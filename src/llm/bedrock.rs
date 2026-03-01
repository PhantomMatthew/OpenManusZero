//! AWS Bedrock LLM Client implementation
//!
//! Provides an LLM client that integrates with AWS Bedrock,
//! converting between OpenAI format and Bedrock format.

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::error::LlmError;
use crate::llm::token::TokenCounter;
use crate::llm::{LlmRequest, LlmResponse, ToolChoice};
use crate::schema::{Message, ToolCall};
use crate::tool::ToolDefinition;

use super::LlmClient;

/// AWS Bedrock model IDs
pub mod models {
    /// Claude 3.5 Sonnet
    pub const CLAUDE_3_5_SONNET: &str = "anthropic.claude-3-5-sonnet-20241022-v2:0";
    /// Claude 3 Sonnet
    pub const CLAUDE_3_SONNET: &str = "anthropic.claude-3-sonnet-20240229-v1:0";
    /// Claude 3 Haiku
    pub const CLAUDE_3_HAIKU: &str = "anthropic.claude-3-haiku-20240307-v1:0";
    /// Claude 3 Opus
    pub const CLAUDE_3_OPUS: &str = "anthropic.claude-3-opus-20240229-v1:0";
    /// Claude 2
    pub const CLAUDE_2: &str = "anthropic.claude.v2:1";
    /// Titan Text Express
    pub const TITAN_TEXT_EXPRESS: &str = "amazon.titan-text-express-v1";
    /// Titan Text Lite
    pub const TITAN_TEXT_LITE: &str = "amazon.titan-text-lite-v1";
    /// Llama 3 70B
    pub const LLAMA_3_70B: &str = "meta.llama3-70b-instruct-v1:0";
    /// Llama 3 8B
    pub const LLAMA_3_8B: &str = "meta.llama3-8b-instruct-v1:0";
}

/// AWS Bedrock configuration
#[derive(Debug, Clone)]
pub struct BedrockConfig {
    /// AWS Region
    pub region: String,
    /// AWS Access Key ID
    pub access_key_id: String,
    /// AWS Secret Access Key
    pub secret_access_key: String,
    /// AWS Session Token (optional, for temporary credentials)
    pub session_token: Option<String>,
    /// Default model ID
    pub model_id: String,
    /// Request timeout in seconds
    pub timeout_secs: u64,
}

impl Default for BedrockConfig {
    fn default() -> Self {
        Self {
            region: "us-east-1".to_string(),
            access_key_id: String::new(),
            secret_access_key: String::new(),
            session_token: None,
            model_id: models::CLAUDE_3_5_SONNET.to_string(),
            timeout_secs: 300,
        }
    }
}

impl BedrockConfig {
    /// Create a new configuration
    pub fn new(region: impl Into<String>, model_id: impl Into<String>) -> Self {
        Self {
            region: region.into(),
            model_id: model_id.into(),
            ..Default::default()
        }
    }

    /// Set AWS credentials
    pub fn with_credentials(
        mut self,
        access_key_id: impl Into<String>,
        secret_access_key: impl Into<String>,
    ) -> Self {
        self.access_key_id = access_key_id.into();
        self.secret_access_key = secret_access_key.into();
        self
    }

    /// Set session token
    pub fn with_session_token(mut self, token: impl Into<String>) -> Self {
        self.session_token = Some(token.into());
        self
    }

    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self, LlmError> {
        let region = std::env::var("AWS_REGION")
            .or_else(|_| std::env::var("AWS_DEFAULT_REGION"))
            .unwrap_or_else(|_| "us-east-1".to_string());

        let access_key_id = std::env::var("AWS_ACCESS_KEY_ID")
            .map_err(|_| LlmError::AuthFailed("AWS_ACCESS_KEY_ID not set".to_string()))?;

        let secret_access_key = std::env::var("AWS_SECRET_ACCESS_KEY")
            .map_err(|_| LlmError::AuthFailed("AWS_SECRET_ACCESS_KEY not set".to_string()))?;

        let session_token = std::env::var("AWS_SESSION_TOKEN").ok();

        let model_id = std::env::var("BEDROCK_MODEL_ID")
            .unwrap_or_else(|_| models::CLAUDE_3_5_SONNET.to_string());

        Ok(Self {
            region,
            access_key_id,
            secret_access_key,
            session_token,
            model_id,
            timeout_secs: 300,
        })
    }

    /// Get the Bedrock endpoint URL
    pub fn endpoint(&self) -> String {
        format!(
            "https://bedrock-runtime.{}.amazonaws.com",
            self.region
        )
    }

    /// Get the converse API URL for a model
    pub fn converse_url(&self) -> String {
        format!(
            "{}/model/{}/converse",
            self.endpoint(),
            urlencoding::encode(&self.model_id)
        )
    }
}

/// Bedrock message content block
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
enum BedrockContentBlock {
    Text { text: String },
    ToolUse {
        tool_use_id: String,
        name: String,
        input: serde_json::Value,
    },
    ToolResult {
        tool_use_id: String,
        content: Vec<BedrockContentBlock>,
        #[serde(skip_serializing_if = "Option::is_none")]
        status: Option<String>,
    },
}

/// Bedrock message
#[derive(Debug, Clone, Serialize, Deserialize)]
struct BedrockMessage {
    role: String,
    content: Vec<BedrockContentBlock>,
}

/// Bedrock tool specification
#[derive(Debug, Clone, Serialize, Deserialize)]
struct BedrockToolSpec {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    input_schema: BedrockInputSchema,
}

/// Bedrock input schema
#[derive(Debug, Clone, Serialize, Deserialize)]
struct BedrockInputSchema {
    json: serde_json::Value,
}

/// Bedrock tool
#[derive(Debug, Clone, Serialize, Deserialize)]
struct BedrockTool {
    tool_spec: BedrockToolSpec,
}

/// Bedrock tool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
struct BedrockToolConfig {
    tools: Vec<BedrockTool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_choice: Option<BedrockToolChoice>,
}

/// Bedrock tool choice
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
enum BedrockToolChoice {
    Auto { auto: serde_json::Value },
    Any { any: serde_json::Value },
}

/// Bedrock inference configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
struct BedrockInferenceConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
}

/// Bedrock converse request
#[derive(Debug, Clone, Serialize, Deserialize)]
struct BedrockConverseRequest {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    system: Vec<BedrockSystemBlock>,
    messages: Vec<BedrockMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    inference_config: Option<BedrockInferenceConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_config: Option<BedrockToolConfig>,
}

/// Bedrock system block
#[derive(Debug, Clone, Serialize, Deserialize)]
struct BedrockSystemBlock {
    text: String,
}

/// Bedrock converse response
#[derive(Debug, Clone, Deserialize)]
struct BedrockConverseResponse {
    output: BedrockOutput,
    #[serde(default)]
    stop_reason: Option<String>,
    #[serde(default)]
    usage: BedrockUsage,
}

/// Bedrock output
#[derive(Debug, Clone, Deserialize)]
struct BedrockOutput {
    message: BedrockResponseMessage,
}

/// Bedrock response message
#[derive(Debug, Clone, Deserialize)]
struct BedrockResponseMessage {
    role: String,
    content: Vec<BedrockContentBlockResponse>,
}

/// Bedrock content block in response
#[derive(Debug, Clone, Deserialize)]
struct BedrockContentBlockResponse {
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    tool_use: Option<BedrockToolUseResponse>,
}

/// Bedrock tool use in response
#[derive(Debug, Clone, Deserialize)]
struct BedrockToolUseResponse {
    tool_use_id: String,
    name: String,
    input: serde_json::Value,
}

/// Bedrock usage
#[derive(Debug, Clone, Deserialize, Default)]
struct BedrockUsage {
    #[serde(default)]
    input_tokens: u32,
    #[serde(default)]
    output_tokens: u32,
    #[serde(default)]
    total_tokens: Option<u32>,
}

/// AWS Bedrock LLM Client
pub struct BedrockClient {
    /// Configuration
    config: BedrockConfig,
    /// HTTP client
    http_client: Client,
    /// Token counter
    token_counter: TokenCounter,
    /// AWS SigV4 signer
    signer: AwsSigV4Signer,
}

impl BedrockClient {
    /// Create a new Bedrock client
    pub fn new(config: BedrockConfig) -> Result<Self, LlmError> {
        let http_client = Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_secs))
            .build()
            .map_err(|e| LlmError::NetworkError(format!("Failed to create HTTP client: {}", e)))?;

        let token_counter = TokenCounter::for_model(&config.model_id);
        let signer = AwsSigV4Signer::new(
            config.access_key_id.clone(),
            config.secret_access_key.clone(),
            config.session_token.clone(),
            config.region.clone(),
        );

        Ok(Self {
            config,
            http_client,
            token_counter,
            signer,
        })
    }

    /// Create a client from environment variables
    pub fn from_env() -> Result<Self, LlmError> {
        let config = BedrockConfig::from_env()?;
        Self::new(config)
    }

    /// Create a client with default configuration and a specific model
    pub fn with_model(model_id: impl Into<String>) -> Result<Self, LlmError> {
        let config = BedrockConfig::from_env()?;
        let config = BedrockConfig {
            model_id: model_id.into(),
            ..config
        };
        Self::new(config)
    }

    /// Convert OpenAI-format messages to Bedrock format
    fn convert_messages(&self, messages: &[Message]) -> (Vec<BedrockSystemBlock>, Vec<BedrockMessage>) {
        let mut system_blocks = Vec::new();
        let mut bedrock_messages = Vec::new();
        let mut pending_tool_result_id: Option<String> = None;

        for message in messages {
            match message.role {
                crate::schema::Role::System => {
                    if let Some(content) = &message.content {
                        system_blocks.push(BedrockSystemBlock {
                            text: content.clone(),
                        });
                    }
                }
                crate::schema::Role::User => {
                    let content = message.content.clone().unwrap_or_default();
                    let tool_result_id = pending_tool_result_id.take();
                    let blocks = if let Some(tool_call_id) = tool_result_id {
                        // This is a tool result
                        vec![BedrockContentBlock::ToolResult {
                            tool_use_id: tool_call_id,
                            content: vec![BedrockContentBlock::Text { text: content }],
                            status: None,
                        }]
                    } else {
                        vec![BedrockContentBlock::Text { text: content }]
                    };
                    bedrock_messages.push(BedrockMessage {
                        role: "user".to_string(),
                        content: blocks,
                    });
                }
                crate::schema::Role::Assistant => {
                    let mut blocks = Vec::new();

                    // Add text content
                    if let Some(content) = &message.content {
                        if !content.is_empty() {
                            blocks.push(BedrockContentBlock::Text {
                                text: content.clone(),
                            });
                        }
                    }

                    // Add tool calls
                    if let Some(tool_calls) = &message.tool_calls {
                        for tc in tool_calls {
                            let input: serde_json::Value =
                                serde_json::from_str(&tc.function.arguments).unwrap_or(serde_json::json!({}));
                            blocks.push(BedrockContentBlock::ToolUse {
                                tool_use_id: tc.id.clone(),
                                name: tc.function.name.clone(),
                                input,
                            });
                            pending_tool_result_id = Some(tc.id.clone());
                        }
                    }

                    if !blocks.is_empty() {
                        bedrock_messages.push(BedrockMessage {
                            role: "assistant".to_string(),
                            content: blocks,
                        });
                    }
                }
                crate::schema::Role::Tool => {
                    // Tool messages are handled in the User role case above
                }
            }
        }

        (system_blocks, bedrock_messages)
    }

    /// Convert tools to Bedrock format
    fn convert_tools(&self, tools: &[ToolDefinition]) -> Vec<BedrockTool> {
        tools
            .iter()
            .map(|tool| {
                let json_schema = tool.function.parameters.as_ref()
                    .map(|p| serde_json::to_value(p).unwrap_or(serde_json::json!({"type": "object"})))
                    .unwrap_or(serde_json::json!({"type": "object", "properties": {}, "required": []}));

                BedrockTool {
                    tool_spec: BedrockToolSpec {
                        name: tool.function.name.clone(),
                        description: Some(tool.function.description.clone()),
                        input_schema: BedrockInputSchema {
                            json: json_schema,
                        },
                    },
                }
            })
            .collect()
    }

    /// Convert tool choice to Bedrock format
    fn convert_tool_choice(&self, choice: &ToolChoice) -> Option<BedrockToolChoice> {
        match choice {
            ToolChoice::Auto => Some(BedrockToolChoice::Auto {
                auto: serde_json::json!({}),
            }),
            ToolChoice::Required => Some(BedrockToolChoice::Any {
                any: serde_json::json!({}),
            }),
            ToolChoice::None => None,
        }
    }

    /// Convert Bedrock response to LlmResponse
    fn convert_response(&self, response: BedrockConverseResponse) -> LlmResponse {
        let mut content = String::new();
        let mut tool_calls = Vec::new();

        for block in &response.output.message.content {
            if let Some(text) = &block.text {
                content.push_str(text);
            }
            if let Some(tool_use) = &block.tool_use {
                tool_calls.push(ToolCall::new(
                    &tool_use.tool_use_id,
                    &tool_use.name,
                    &serde_json::to_string(&tool_use.input).unwrap_or_default(),
                ));
            }
        }

        // Ensure content is not empty if there are tool calls
        if content.is_empty() && !tool_calls.is_empty() {
            content = ".".to_string();
        }

        LlmResponse {
            id: Some(format!("bedrock-{}", uuid::Uuid::new_v4())),
            model: self.config.model_id.clone(),
            choices: vec![super::types::LlmResponseChoice {
                index: 0,
                message: super::types::LlmResponseMessage {
                    role: "assistant".to_string(),
                    content: if content.is_empty() { None } else { Some(content) },
                    tool_calls: if tool_calls.is_empty() {
                        None
                    } else {
                        Some(tool_calls)
                    },
                },
                finish_reason: response.stop_reason.clone(),
            }],
            usage: Some(super::types::LlmUsage {
                prompt_tokens: response.usage.input_tokens as u64,
                completion_tokens: response.usage.output_tokens as u64,
                total_tokens: response.usage.total_tokens.unwrap_or(
                    response.usage.input_tokens + response.usage.output_tokens
                ) as u64,
            }),
        }
    }

    /// Make a signed request to Bedrock
    async fn make_request(&self, body: &BedrockConverseRequest) -> Result<BedrockConverseResponse, LlmError> {
        let url = self.config.converse_url();
        let body_str = serde_json::to_string(body)
            .map_err(|e| LlmError::InvalidResponse(format!("Failed to serialize request: {}", e)))?;

        let request_builder = self
            .http_client
            .post(&url)
            .header("Content-Type", "application/json")
            .body(body_str.clone());

        let signed_request = self
            .signer
            .sign(request_builder, "bedrock", "POST", &url, &body_str)?;

        let response = self
            .http_client
            .execute(signed_request)
            .await
            .map_err(|e| LlmError::NetworkError(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(LlmError::ApiError(format!(
                "Bedrock API error ({}): {}",
                status, error_text
            )));
        }

        let bedrock_response: BedrockConverseResponse = response
            .json()
            .await
            .map_err(|e| LlmError::InvalidResponse(format!("Failed to parse response: {}", e)))?;

        Ok(bedrock_response)
    }
}

#[async_trait]
impl LlmClient for BedrockClient {
    fn model(&self) -> &str {
        &self.config.model_id
    }

    async fn completion(&self, request: &LlmRequest) -> Result<LlmResponse, LlmError> {
        let (system, messages) = self.convert_messages(&request.messages);

        let inference_config = BedrockInferenceConfig {
            max_tokens: Some(request.max_tokens.unwrap_or(4096) as u32),
            temperature: Some(request.temperature.unwrap_or(0.7)),
            top_p: None,
        };

        let tool_config = if let Some(tools) = &request.tools {
            if !tools.is_empty() {
                Some(BedrockToolConfig {
                    tools: self.convert_tools(tools),
                    tool_choice: request.tool_choice.as_ref().and_then(|tc| self.convert_tool_choice(tc)),
                })
            } else {
                None
            }
        } else {
            None
        };

        let bedrock_request = BedrockConverseRequest {
            system,
            messages,
            inference_config: Some(inference_config),
            tool_config,
        };

        let response = self.make_request(&bedrock_request).await?;
        Ok(self.convert_response(response))
    }

    fn token_counter(&self) -> &TokenCounter {
        &self.token_counter
    }
}

/// AWS Signature Version 4 signer
struct AwsSigV4Signer {
    access_key_id: String,
    secret_access_key: String,
    session_token: Option<String>,
    region: String,
}

impl AwsSigV4Signer {
    fn new(
        access_key_id: String,
        secret_access_key: String,
        session_token: Option<String>,
        region: String,
    ) -> Self {
        Self {
            access_key_id,
            secret_access_key,
            session_token,
            region,
        }
    }

    fn sign(
        &self,
        request_builder: reqwest::RequestBuilder,
        service: &str,
        method: &str,
        url: &str,
        payload: &str,
    ) -> Result<reqwest::Request, LlmError> {
        use hmac::Mac;
        use sha2::{Digest, Sha256};

        type HmacSha256 = hmac::Hmac<Sha256>;

        let now = chrono::Utc::now();
        let amz_date = now.format("%Y%m%dT%H%M%SZ").to_string();
        let date_stamp = now.format("%Y%m%d").to_string();

        // Parse URL to get host and path
        let parsed_url = url::Url::parse(url)
            .map_err(|e| LlmError::BuilderError(format!("Invalid URL: {}", e)))?;
        let host = parsed_url
            .host_str()
            .ok_or_else(|| LlmError::BuilderError("Missing host in URL".to_string()))?;
        let path = parsed_url.path();

        // Calculate payload hash
        let payload_hash = hex::encode(Sha256::digest(payload.as_bytes()));

        // Build canonical request - with or without session token
        let (canonical_headers, signed_headers) = if let Some(token) = &self.session_token {
            (
                format!(
                    "content-type:application/json\nhost:{}\nx-amz-date:{}\nx-amz-security-token:{}\n",
                    host, amz_date, token
                ),
                "content-type;host;x-amz-date;x-amz-security-token",
            )
        } else {
            (
                format!(
                    "content-type:application/json\nhost:{}\nx-amz-date:{}\n",
                    host, amz_date
                ),
                "content-type;host;x-amz-date",
            )
        };

        let canonical_request = format!(
            "{}\n{}\n\n{}\n{}\n{}",
            method, path, canonical_headers, signed_headers, payload_hash
        );

        // Create string to sign
        let credential_scope = format!("{}/{}/{}/aws4_request", date_stamp, self.region, service);
        let canonical_request_hash = hex::encode(Sha256::digest(canonical_request.as_bytes()));
        let string_to_sign = format!(
            "AWS4-HMAC-SHA256\n{}\n{}\n{}",
            amz_date, credential_scope, canonical_request_hash
        );

        // Calculate signature
        let k_date = Self::hmac_sha256(
            format!("AWS4{}", self.secret_access_key).as_bytes(),
            date_stamp.as_bytes(),
        );
        let k_region = Self::hmac_sha256(&k_date, self.region.as_bytes());
        let k_service = Self::hmac_sha256(&k_region, service.as_bytes());
        let k_signing = Self::hmac_sha256(&k_service, b"aws4_request");
        let signature = hex::encode(Self::hmac_sha256(&k_signing, string_to_sign.as_bytes()));

        // Build authorization header
        let authorization = format!(
            "AWS4-HMAC-SHA256 Credential={}/{}, SignedHeaders={}, Signature={}",
            self.access_key_id, credential_scope, signed_headers, signature
        );

        // Build request with headers
        let mut request = request_builder
            .header("X-Amz-Date", &amz_date)
            .header("Authorization", &authorization)
            .build()
            .map_err(|e| LlmError::NetworkError(format!("Failed to build request: {}", e)))?;

        // Add session token header if present
        if let Some(token) = &self.session_token {
            request.headers_mut().insert(
                reqwest::header::HeaderName::from_static("x-amz-security-token"),
                reqwest::header::HeaderValue::from_str(token)
                    .map_err(|_| LlmError::BuilderError("Invalid session token".to_string()))?,
            );
        }

        Ok(request)
    }

    fn hmac_sha256(key: &[u8], data: &[u8]) -> Vec<u8> {
        use hmac::Mac;
        type HmacSha256 = hmac::Hmac<sha2::Sha256>;
        let mut mac = <HmacSha256 as Mac>::new_from_slice(key).expect("HMAC can take key of any size");
        mac.update(data);
        mac.finalize().into_bytes().to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bedrock_config_default() {
        let config = BedrockConfig::default();
        assert_eq!(config.region, "us-east-1");
        assert_eq!(config.timeout_secs, 300);
    }

    #[test]
    fn test_bedrock_config_with_credentials() {
        let config = BedrockConfig::new("us-west-2", models::CLAUDE_3_HAIKU)
            .with_credentials("key", "secret")
            .with_session_token("token");

        assert_eq!(config.region, "us-west-2");
        assert_eq!(config.access_key_id, "key");
        assert_eq!(config.secret_access_key, "secret");
        assert_eq!(config.session_token, Some("token".to_string()));
    }

    #[test]
    fn test_bedrock_endpoint() {
        let config = BedrockConfig::new("eu-west-1", models::CLAUDE_3_HAIKU);
        assert_eq!(
            config.endpoint(),
            "https://bedrock-runtime.eu-west-1.amazonaws.com"
        );
    }

    #[test]
    fn test_bedrock_converse_url() {
        let config = BedrockConfig::new("us-east-1", "anthropic.claude-3-sonnet-20240229-v1:0");
        assert_eq!(
            config.converse_url(),
            "https://bedrock-runtime.us-east-1.amazonaws.com/model/anthropic.claude-3-sonnet-20240229-v1%3A0/converse"
        );
    }

    #[test]
    fn test_models_constants() {
        assert!(models::CLAUDE_3_5_SONNET.contains("claude"));
        assert!(models::TITAN_TEXT_EXPRESS.contains("titan"));
        assert!(models::LLAMA_3_70B.contains("llama"));
    }

    #[test]
    fn test_bedrock_content_block_serialize() {
        let block = BedrockContentBlock::Text {
            text: "Hello".to_string(),
        };
        let json = serde_json::to_string(&block).unwrap();
        assert!(json.contains("Hello"));
    }

    #[test]
    fn test_bedrock_inference_config_serialize() {
        let config = BedrockInferenceConfig {
            max_tokens: Some(1024),
            temperature: Some(0.7),
            top_p: None,
        };
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("1024"));
        assert!(json.contains("0.7"));
    }
}
