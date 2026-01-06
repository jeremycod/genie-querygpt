use querygpt_core::planner::llm::{LlmClient, LlmMessage, LlmRequest, LlmRole};
use querygpt_core::planner::openai_client::OpenAIClient;

#[tokio::test]
#[ignore] // Requires OPENAI_API_KEY
async fn test_openai_client_integration() {
    let client = OpenAIClient::from_env().expect("OPENAI_API_KEY not set");

    let request = LlmRequest {
        messages: vec![
            LlmMessage {
                role: LlmRole::System,
                content: "You are a helpful assistant. Respond with valid JSON only.".to_string(),
            },
            LlmMessage {
                role: LlmRole::User,
                content: "Generate a simple JSON object with a greeting field.".to_string(),
            },
        ],
        model: "gpt-3.5-turbo".to_string(),
        temperature: 0.1,
        max_tokens: Some(100),
    };

    let response = client.complete(request).await;
    assert!(response.is_ok());

    let response = response.unwrap();
    assert!(!response.content.is_empty());
    println!("OpenAI Response: {}", response.content);
}
