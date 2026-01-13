use crate::planner::llm::{LlmClient, LlmMessage, LlmRequest, LlmRole};
use crate::schema::registry::WorkspaceRegistry;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentResult {
    pub workspace: String,
    pub task: String,
    pub filter_hints: Vec<String>,
}

/// Classification result with confidence score
#[derive(Debug, Clone)]
pub struct WorkspaceClassification {
    pub workspace: String,
    pub confidence: ClassificationConfidence,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ClassificationConfidence {
    High,   // Keyword match or clear LLM signal
    Medium, // LLM classification with some ambiguity
    Low,    // Fallback or unclear
}

/// Intent classification agent that routes queries to appropriate workspaces
pub struct IntentAgent {
    registry: WorkspaceRegistry,
    llm_client: Option<Box<dyn LlmClient>>,
    model: String,
}

impl IntentAgent {
    /// Create a new IntentAgent with workspace registry and LLM client
    pub fn new(
        registry: WorkspaceRegistry,
        llm_client: Option<Box<dyn LlmClient>>,
        model: String,
    ) -> Self {
        Self {
            registry,
            llm_client,
            model,
        }
    }

    /// Classify a user prompt to determine the appropriate workspace
    pub async fn classify_workspace(&self, user_prompt: &str) -> Result<WorkspaceClassification> {
        // Fast path: Try keyword-based classification first
        if let Some(classification) = self.keyword_classify(user_prompt) {
            return Ok(classification);
        }

        // Slow path: Use LLM for classification if available
        if let Some(ref client) = self.llm_client {
            self.llm_classify(user_prompt, client.as_ref()).await
        } else {
            // Fallback to default workspace if no LLM available
            Ok(WorkspaceClassification {
                workspace: "campaigns_offers".to_string(),
                confidence: ClassificationConfidence::Low,
                reason: "No LLM available, using default workspace".to_string(),
            })
        }
    }

    /// Fast keyword-based classification for obvious cases
    fn keyword_classify(&self, prompt: &str) -> Option<WorkspaceClassification> {
        let prompt_lower = prompt.to_lowercase();

        // Define keyword patterns with weights
        // entity_keywords: Primary entities (campaigns, offers, products, skus) - HIGH priority
        // attribute_keywords: Attributes/filters (price, discount, region) - MEDIUM priority
        let workspace_patterns: Vec<(&str, Vec<&str>, Vec<&str>)> = vec![
            (
                "campaigns_offers",
                vec!["campaign", "offer", "promo", "promotion", "phase"], // entities
                vec!["export", "prepaid", "apac", "live", "active"],      // attributes
            ),
            (
                "distribution",
                vec!["sku", "partner", "channel"], // entities
                vec![
                    "distribution",
                    "platform",
                    "country",
                    "region",
                    "marketplace",
                    "distributor",
                ], // attributes
            ),
            (
                "pricing_discounts",
                vec![], // no primary entities - this is a supporting workspace
                vec![
                    "price",
                    "pricing",
                    "discount",
                    "revenue",
                    "cost",
                    "currency",
                    "amount",
                    "price in",
                    "how much",
                    "discounted",
                    "discount percentage",
                ],
            ),
        ];

        // Score each workspace based on weighted matches
        let mut scored_matches: Vec<(&str, i32, String)> = Vec::new();

        for (workspace, entity_keywords, attribute_keywords) in &workspace_patterns {
            let entity_matches: Vec<String> = entity_keywords
                .iter()
                .filter(|keyword| prompt_lower.contains(*keyword))
                .map(|s| s.to_string())
                .collect();

            let attribute_matches: Vec<String> = attribute_keywords
                .iter()
                .filter(|keyword| prompt_lower.contains(*keyword))
                .map(|s| s.to_string())
                .collect();

            // Weighted scoring:
            // - Entity match (campaigns, offers, products, skus) = 10 points each
            // - Attribute match (price, discount, region) = 1 point each
            let entity_score = entity_matches.len() as i32 * 10;
            let attribute_score = attribute_matches.len() as i32;
            let total_score = entity_score + attribute_score;

            if total_score > 0 {
                let reason = if !entity_matches.is_empty() {
                    format!(
                        "Entity match: {} (score: {})",
                        entity_matches.join(", "),
                        total_score
                    )
                } else {
                    format!(
                        "Attribute match: {} (score: {})",
                        attribute_matches.join(", "),
                        total_score
                    )
                };
                scored_matches.push((workspace, total_score, reason));
            }
        }

        // Sort by score (highest first)
        scored_matches.sort_by(|a, b| b.1.cmp(&a.1));

        // Return the best match if it exists and workspace is valid
        if let Some((workspace, score, reason)) = scored_matches.first() {
            if *score > 0 && self.registry.has_workspace(workspace) {
                return Some(WorkspaceClassification {
                    workspace: workspace.to_string(),
                    confidence: ClassificationConfidence::High,
                    reason: reason.clone(),
                });
            }
        }

        None
    }

    /// LLM-based classification for ambiguous cases
    async fn llm_classify(
        &self,
        user_prompt: &str,
        client: &dyn LlmClient,
    ) -> Result<WorkspaceClassification> {
        let workspaces = self.build_workspace_descriptions();

        let system_prompt = format!(
            r#"You are a workspace classifier for a natural language SQL system.

Available workspaces:
{}

Your task: Analyze the user's query and determine which workspace is most appropriate.

Rules:
- Return ONLY the workspace name, nothing else
- Choose the workspace whose domain best matches the query intent
- Consider the entities and tags when making your decision
- If multiple workspaces could work, choose the most specific one
- Valid workspace names: {}

Respond with ONLY the workspace name (e.g., "pricing_discounts" or "distribution" or "campaigns_offers")."#,
            workspaces,
            self.registry.list_workspaces().join(", ")
        );

        let request = LlmRequest {
            messages: vec![
                LlmMessage {
                    role: LlmRole::System,
                    content: system_prompt,
                },
                LlmMessage {
                    role: LlmRole::User,
                    content: user_prompt.to_string(),
                },
            ],
            model: self.model.clone(),
            temperature: 0.0,     // Deterministic classification
            max_tokens: Some(50), // We only need the workspace name
        };

        let response = client
            .complete(request)
            .await
            .context("LLM workspace classification failed")?;

        let workspace = response.content.trim().to_lowercase();

        // Validate the workspace exists
        if self.registry.has_workspace(&workspace) {
            Ok(WorkspaceClassification {
                workspace,
                confidence: ClassificationConfidence::Medium,
                reason: "LLM classification".to_string(),
            })
        } else {
            // LLM returned invalid workspace, fall back to keyword or default
            eprintln!(
                "[WARN] LLM returned invalid workspace '{}', falling back to default",
                workspace
            );
            Ok(WorkspaceClassification {
                workspace: "campaigns_offers".to_string(),
                confidence: ClassificationConfidence::Low,
                reason: format!(
                    "LLM returned invalid workspace '{}', using default",
                    workspace
                ),
            })
        }
    }

    /// Build workspace descriptions for LLM prompt
    fn build_workspace_descriptions(&self) -> String {
        let mut descriptions = Vec::new();

        for metadata in self.registry.all_metadata() {
            descriptions.push(format!(
                "- {}: {}\n  Tags: [{}]\n  Entities: [{}]",
                metadata.name,
                metadata.description,
                metadata.tags.join(", "),
                metadata.entities.join(", ")
            ));
        }

        descriptions.join("\n\n")
    }
}

/// Legacy stub: classify query to workspace/task.
///
/// DEPRECATED: Use IntentAgent::classify_workspace instead
pub fn classify(_user_prompt: &str) -> IntentResult {
    IntentResult {
        workspace: "campaigns_offers".to_string(),
        task: "export".to_string(),
        filter_hints: vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planner::llm::{LlmResponse, LlmUsage};
    use std::path::PathBuf;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    // Mock LLM client for testing
    struct MockLlmClient {
        responses: Arc<Mutex<Vec<String>>>,
    }

    impl MockLlmClient {
        fn new(responses: Vec<String>) -> Self {
            Self {
                responses: Arc::new(Mutex::new(responses)),
            }
        }
    }

    #[async_trait::async_trait]
    impl LlmClient for MockLlmClient {
        async fn complete(&self, _req: LlmRequest) -> Result<LlmResponse> {
            let mut responses = self.responses.lock().await;
            let response = responses
                .pop()
                .unwrap_or_else(|| "campaigns_offers".to_string());

            Ok(LlmResponse {
                content: response,
                usage: Some(LlmUsage {
                    prompt_tokens: 100,
                    completion_tokens: 10,
                    total_tokens: 110,
                }),
            })
        }
    }

    fn create_test_registry() -> WorkspaceRegistry {
        // Change to repo root directory for schema loading
        let crate_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let repo_root = crate_root
            .parent()
            .and_then(|p| p.parent())
            .expect("resolve repo root from CARGO_MANIFEST_DIR");

        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(repo_root).expect("change to repo root");

        let workspaces_dir = PathBuf::from("config/workspaces");
        if !workspaces_dir.exists() {
            panic!("Test requires config/workspaces directory");
        }

        let result = WorkspaceRegistry::from_directory(&workspaces_dir)
            .expect("Failed to create test registry");

        std::env::set_current_dir(original_dir).expect("restore original directory");
        result
    }

    #[tokio::test]
    async fn test_keyword_classification_pricing() {
        let registry = create_test_registry();
        let agent = IntentAgent::new(registry, None, "gpt-4".to_string());

        let classification = agent
            .classify_workspace("Show me the pricing and discounts for all items")
            .await
            .expect("Classification should succeed");

        assert_eq!(classification.workspace, "pricing_discounts");
        assert_eq!(classification.confidence, ClassificationConfidence::High);
    }

    #[tokio::test]
    async fn test_keyword_classification_distribution() {
        let registry = create_test_registry();
        let agent = IntentAgent::new(registry, None, "gpt-4".to_string());

        let classification = agent
            .classify_workspace("List all SKUs available in US marketplace")
            .await
            .expect("Classification should succeed");

        assert_eq!(classification.workspace, "distribution");
        assert_eq!(classification.confidence, ClassificationConfidence::High);
    }

    #[tokio::test]
    async fn test_keyword_classification_campaigns() {
        let registry = create_test_registry();
        let agent = IntentAgent::new(registry, None, "gpt-4".to_string());

        let classification = agent
            .classify_workspace("Export all APAC prepaid campaigns")
            .await
            .expect("Classification should succeed");

        assert_eq!(classification.workspace, "campaigns_offers");
        assert_eq!(classification.confidence, ClassificationConfidence::High);
    }

    #[tokio::test]
    async fn test_llm_classification() {
        let registry = create_test_registry();

        // Skip if registry doesn't have pricing_discounts (test isolation issue)
        if !registry.has_workspace("pricing_discounts") {
            eprintln!("Skipping test: pricing_discounts workspace not found");
            return;
        }

        let mock_client = Box::new(MockLlmClient::new(vec!["pricing_discounts".to_string()]));
        let agent = IntentAgent::new(registry, Some(mock_client), "gpt-4".to_string());

        // Use a query without clear keywords to force LLM classification
        let classification = agent
            .classify_workspace("Show me the financial data for item X")
            .await
            .expect("Classification should succeed");

        assert_eq!(classification.workspace, "pricing_discounts");
        assert_eq!(classification.confidence, ClassificationConfidence::Medium);
    }

    #[tokio::test]
    async fn test_fallback_on_invalid_llm_response() {
        let registry = create_test_registry();
        let mock_client = Box::new(MockLlmClient::new(vec!["invalid_workspace".to_string()]));
        let agent = IntentAgent::new(registry, Some(mock_client), "gpt-4".to_string());

        let classification = agent
            .classify_workspace("Some ambiguous query")
            .await
            .expect("Classification should succeed");

        assert_eq!(classification.workspace, "campaigns_offers");
        assert_eq!(classification.confidence, ClassificationConfidence::Low);
    }

    #[tokio::test]
    async fn test_no_llm_fallback() {
        let registry = create_test_registry();
        let agent = IntentAgent::new(registry, None, "gpt-4".to_string());

        let classification = agent
            .classify_workspace("Some query without clear keywords")
            .await
            .expect("Classification should succeed");

        assert_eq!(classification.workspace, "campaigns_offers");
        assert_eq!(classification.confidence, ClassificationConfidence::Low);
    }
}
