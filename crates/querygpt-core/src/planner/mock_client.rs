use super::llm::{LlmClient, LlmRequest, LlmResponse, LlmUsage};
use std::collections::HashMap;

/// Mock LLM client for testing with configurable responses
pub struct MockClient {
    responses: HashMap<String, String>,
    default_response: Option<String>,
}

impl MockClient {
    pub fn new() -> Self {
        Self {
            responses: HashMap::new(),
            default_response: None,
        }
    }

    pub fn with_response(mut self, prompt_key: String, response: String) -> Self {
        self.responses.insert(prompt_key, response);
        self
    }

    pub fn with_default_response(mut self, response: String) -> Self {
        self.default_response = Some(response);
        self
    }

    pub fn add_response(&mut self, prompt_key: String, response: String) {
        self.responses.insert(prompt_key, response);
    }

    /// Generate a simple key from the user message for response lookup
    fn extract_prompt_key(&self, request: &LlmRequest) -> String {
        request
            .messages
            .iter()
            .find(|msg| matches!(msg.role, super::llm::LlmRole::User))
            .map(|msg| msg.content.clone())
            .unwrap_or_default()
    }
}

impl Default for MockClient {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl LlmClient for MockClient {
    async fn complete(&self, req: LlmRequest) -> anyhow::Result<LlmResponse> {
        let prompt_key = self.extract_prompt_key(&req);

        let content = self
            .responses
            .get(&prompt_key)
            .or(self.default_response.as_ref())
            .ok_or_else(|| anyhow::anyhow!("No mock response configured for prompt"))?
            .clone();

        Ok(LlmResponse {
            content,
            usage: Some(LlmUsage {
                prompt_tokens: 100,
                completion_tokens: 50,
                total_tokens: 150,
            }),
        })
    }
}
