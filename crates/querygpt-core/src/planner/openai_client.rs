use super::llm::{LlmClient, LlmError, LlmRequest, LlmResponse, LlmRole, LlmUsage};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

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

#[derive(Deserialize)]
struct OpenAIErrorResponse {
    error: OpenAIErrorDetail,
}

#[derive(Deserialize)]
struct OpenAIErrorDetail {
    message: String,
    #[allow(dead_code)]
    #[serde(rename = "type")]
    error_type: Option<String>,
    #[allow(dead_code)]
    code: Option<String>,
}

impl OpenAIClient {
    pub fn new(api_key: String) -> Self {
        Self::with_timeout(api_key, Duration::from_secs(30))
    }

    pub fn with_timeout(api_key: String, timeout: Duration) -> Self {
        let client = Client::builder()
            .timeout(timeout)
            .build()
            .expect("failed to build HTTP client");

        Self {
            api_key,
            base_url: "https://api.openai.com/v1".to_string(),
            client,
        }
    }

    pub fn from_env() -> anyhow::Result<Self> {
        let api_key = std::env::var("OPENAI_API_KEY")
            .map_err(|_| anyhow::anyhow!("OPENAI_API_KEY environment variable not set"))?;
        Ok(Self::new(api_key))
    }

    /// Parse OpenAI error response and convert to typed error
    async fn parse_error_response(
        status: reqwest::StatusCode,
        response: reqwest::Response,
    ) -> LlmError {
        let status_code = status.as_u16();

        // Try to parse OpenAI error format
        if let Ok(error_response) = response.json::<OpenAIErrorResponse>().await {
            let message = error_response.error.message;

            match status_code {
                401 => LlmError::AuthenticationFailed { message },
                429 => {
                    // Parse retry-after from message if present
                    let retry_after = message
                        .split("try again in ")
                        .nth(1)
                        .and_then(|s| s.split('s').next())
                        .and_then(|s| s.trim().parse::<u64>().ok());

                    LlmError::RateLimit {
                        message: format!("Rate limit exceeded. {}", message),
                        retry_after,
                    }
                }
                _ => LlmError::ApiError {
                    status: status_code,
                    message,
                },
            }
        } else {
            // Fallback for unparseable responses
            match status_code {
                401 => LlmError::AuthenticationFailed {
                    message: "Invalid or expired API key".to_string(),
                },
                429 => LlmError::RateLimit {
                    message: "Rate limit exceeded".to_string(),
                    retry_after: None,
                },
                _ => LlmError::ApiError {
                    status: status_code,
                    message: format!("HTTP {}", status_code),
                },
            }
        }
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
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&openai_req)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    LlmError::Timeout { timeout_secs: 30 }
                } else if e.is_connect() || e.is_request() {
                    LlmError::NetworkError(format!("Failed to connect to OpenAI API: {}", e))
                } else {
                    LlmError::ClientError(format!("HTTP request failed: {}", e))
                }
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error = Self::parse_error_response(status, response).await;
            return Err(error.into());
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
