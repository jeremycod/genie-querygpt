use super::llm::{LlmClient, LlmRequest, LlmResponse, LlmRole, LlmUsage};
use reqwest::Client;
use serde::{Deserialize, Serialize};

/// OpenAI API client for real LLM integration
pub struct OpenAIClient {
    api_key: String,
    base_url: String,
    client: Client,
}

#[derive(Serialize)]
struct OpenAIRequest {
    model: String,
    messages: Vec<OpenAIMessage>,
    temperature: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
}

#[derive(Serialize)]
struct OpenAIMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct OpenAIResponse {
    choices: Vec<OpenAIChoice>,
    usage: Option<OpenAIUsage>,
}

#[derive(Deserialize)]
struct OpenAIChoice {
    message: OpenAIResponseMessage,
}

#[derive(Deserialize)]
struct OpenAIResponseMessage {
    content: String,
}

#[derive(Deserialize)]
struct OpenAIUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

impl OpenAIClient {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            base_url: "https://api.openai.com/v1".to_string(),
            client: Client::new(),
        }
    }

    pub fn from_env() -> anyhow::Result<Self> {
        let api_key = std::env::var("OPENAI_API_KEY")
            .map_err(|_| anyhow::anyhow!("OPENAI_API_KEY environment variable not set"))?;
        Ok(Self::new(api_key))
    }
}

#[async_trait::async_trait]
impl LlmClient for OpenAIClient {
    async fn complete(&self, req: LlmRequest) -> anyhow::Result<LlmResponse> {
        let openai_req = OpenAIRequest {
            model: req.model,
            messages: req
                .messages
                .into_iter()
                .map(|msg| OpenAIMessage {
                    role: match msg.role {
                        LlmRole::System => "system".to_string(),
                        LlmRole::User => "user".to_string(),
                        LlmRole::Assistant => "assistant".to_string(),
                    },
                    content: msg.content,
                })
                .collect(),
            temperature: req.temperature,
            max_tokens: req.max_tokens,
        };

        let response = self
            .client
            .post(&format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&openai_req)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("HTTP request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "OpenAI API error {}: {}",
                status,
                error_text
            ));
        }

        let openai_response: OpenAIResponse = response
            .json()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to parse OpenAI response: {}", e))?;

        let choice = openai_response
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("No choices in OpenAI response"))?;

        let usage = openai_response.usage.map(|u| LlmUsage {
            prompt_tokens: u.prompt_tokens,
            completion_tokens: u.completion_tokens,
            total_tokens: u.total_tokens,
        });

        Ok(LlmResponse {
            content: choice.message.content,
            usage,
        })
    }
}
