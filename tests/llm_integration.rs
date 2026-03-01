//! Integration tests for LLM client with mock server

use openmanus::llm::{HttpLlmClient, LlmClient, LlmRequest};
use openmanus::schema::Message;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Create an HttpLlmClient that bypasses system proxies, so that requests to
/// the local wiremock server are not intercepted by any proxy running on the host.
fn make_client(base_url: impl Into<String>, api_key: &str, model: &str) -> HttpLlmClient {
    let no_proxy_client = reqwest::Client::builder()
        .no_proxy()
        .build()
        .expect("Failed to build no-proxy reqwest client");
    HttpLlmClient::with_client(no_proxy_client, base_url, api_key, model)
}

fn create_mock_response(content: &str) -> String {
    serde_json::json!({
        "id": "test-id",
        "model": "gpt-4",
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": content
            },
            "finish_reason": "stop"
        }],
        "usage": {
            "prompt_tokens": 10,
            "completion_tokens": 5,
            "total_tokens": 15
        }
    })
    .to_string()
}

#[tokio::test]
async fn test_http_llm_client_successful_request() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .and(header("Authorization", "Bearer test-key"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(create_mock_response("Hello, I am a test response")),
        )
        .mount(&mock_server)
        .await;

    let client = make_client(mock_server.uri(), "test-key", "gpt-4");
    let request = LlmRequest::new("gpt-4", vec![Message::user("Hello")]);

    let response = client.completion(&request).await.unwrap();

    assert_eq!(response.content(), Some("Hello, I am a test response"));
}

#[tokio::test]
async fn test_http_llm_client_with_tool_calls() {
    let mock_server = MockServer::start().await;

    let response_body = serde_json::json!({
        "id": "test-id",
        "model": "gpt-4",
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": null,
                "tool_calls": [{
                    "id": "call_123",
                    "type": "function",
                    "function": {
                        "name": "bash",
                        "arguments": "{\"command\": \"ls\"}"
                    }
                }]
            },
            "finish_reason": "tool_calls"
        }]
    });

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(response_body))
        .mount(&mock_server)
        .await;

    let client = make_client(mock_server.uri(), "test-key", "gpt-4");
    let request = LlmRequest::new("gpt-4", vec![Message::user("List files")]);

    let response = client.completion(&request).await.unwrap();

    assert!(response.has_tool_calls());
    let tool_calls = response.tool_calls().unwrap();
    assert_eq!(tool_calls.len(), 1);
    assert_eq!(tool_calls[0].function.name, "bash");
}

#[tokio::test]
async fn test_http_llm_client_rate_limit_retry() {
    let mock_server = MockServer::start().await;

    // First request returns 429 (rate limit)
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(429))
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    // Second request succeeds
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200).set_body_string(create_mock_response("Success after retry")),
        )
        .mount(&mock_server)
        .await;

    let client = make_client(mock_server.uri(), "test-key", "gpt-4").with_max_retries(3);
    let request = LlmRequest::new("gpt-4", vec![Message::user("Hello")]);

    let response = client.completion(&request).await.unwrap();

    assert_eq!(response.content(), Some("Success after retry"));
}

#[tokio::test]
async fn test_http_llm_client_auth_error() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(401).set_body_string("Unauthorized"))
        .mount(&mock_server)
        .await;

    let client = make_client(mock_server.uri(), "invalid-key", "gpt-4");
    let request = LlmRequest::new("gpt-4", vec![Message::user("Hello")]);

    let result = client.completion(&request).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_http_llm_client_model_name() {
    let client = HttpLlmClient::new("http://localhost", "key", "gpt-4-turbo");
    assert_eq!(client.model(), "gpt-4-turbo");
}

#[tokio::test]
async fn test_http_llm_client_malformed_response() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_string("invalid json"))
        .mount(&mock_server)
        .await;

    let client = make_client(mock_server.uri(), "test-key", "gpt-4");
    let request = LlmRequest::new("gpt-4", vec![Message::user("Hello")]);

    let result = client.completion(&request).await;

    assert!(result.is_err());
}
