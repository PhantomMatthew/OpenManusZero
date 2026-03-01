# LLM Client 设计文档

## 概述

实现真实的 LLM Client，支持 OpenAI 兼容 API，包括流式响应和 Token 计数功能。

## 架构

```
src/llm/
├── mod.rs           # 模块导出
├── client.rs        # LlmClient trait + HttpLlmClient 实现
├── types.rs         # 请求/响应类型（已有）
├── token.rs         # TokenCounter 实现（新增）
└── builder.rs       # LlmClientBuilder（新增，可选）
```

## 组件设计

### 1. TokenCounter (token.rs)

```rust
pub struct TokenCounter {
    bpe: CoreBpe,
}

impl TokenCounter {
    pub fn for_model(model: &str) -> Self;
    pub fn new() -> Self;
    pub fn count_text(&self, text: &str) -> usize;
    pub fn count_messages(&self, messages: &[Message]) -> usize;
}
```

**要点：**
- 使用 tiktoken-rs 库
- 自动回退到 cl100k_base 编码器
- 支持按模型名初始化

### 2. LlmError 扩展

```rust
#[derive(Debug, thiserror::Error)]
pub enum LlmError {
    #[error("API error: {0}")]
    ApiError(String),

    #[error("Authentication failed")]
    AuthenticationError,

    #[error("Rate limit exceeded: {0}")]
    RateLimitExceeded(String),

    #[error("Request timeout")]
    Timeout,

    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    #[error("Stream error: {0}")]
    StreamError(String),

    #[error("Token limit exceeded: {0}")]
    TokenLimitExceeded(String),

    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),
}
```

### 3. HttpLlmClient 流式响应

```rust
async fn stream_completion(&self, request: &LlmRequest)
    -> Result<LlmStream, LlmError>
{
    // 使用 bytes_stream 处理响应
    // SSE 格式解析：data: {...}
    // 通过 unbounded_channel 转换为 Stream
}
```

### 4. 重试策略

- 指数退避重试
- 429 错误触发重试
- 认证错误立即失败
- 可配置最大重试次数

### 5. Builder 模式

```rust
let client = HttpLlmClient::builder()
    .from_env()
    .model("gpt-4o")
    .max_tokens(4096)
    .temperature(0.7)
    .build()?;
```

### 6. LlmClient Trait

```rust
#[async_trait]
pub trait LlmClient: Send + Sync {
    fn model(&self) -> &str;
    async fn completion(&self, request: &LlmRequest) -> Result<LlmResponse, LlmError>;
    async fn stream_completion(&self, request: &LlmRequest) -> Result<LlmStream, LlmError>;
    async fn ask(&self, prompt: &str) -> Result<String, LlmError>;
    async fn ask_with_tools(...) -> Result<LlmResponse, LlmError>;
    fn token_counter(&self) -> &TokenCounter;
}
```

## 依赖

```toml
tiktoken-rs = "0.5"
tokio-stream = "0.1"
thiserror = "1.0"
```

## 使用示例

```rust
// 简单对话
let answer = client.ask("What is Rust?").await?;

// 流式响应
let mut stream = client.stream_completion(&request).await?;
while let Some(chunk) = futures::StreamExt::next(&mut stream).await {
    if let Some(content) = chunk?.choices.first().and_then(|c| c.delta.content.as_ref()) {
        print!("{}", content);
    }
}
```

## 实现步骤

1. 添加依赖到 Cargo.toml
2. 创建 token.rs 实现 TokenCounter
3. 扩展 error.rs 中的 LlmError
4. 重构 client.rs：
   - 添加 Builder 模式
   - 实现 stream_completion
   - 改进重试逻辑
5. 更新 mod.rs 导出
6. 添加单元测试
