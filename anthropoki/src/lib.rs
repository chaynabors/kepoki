use std::borrow::Cow;
use std::fmt::Display;
use std::pin::Pin;

use bytes::Bytes;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
pub enum ApiVersion {
    #[default]
    Latest,
    #[serde(rename = "2023-06-01")]
    V2023_06_01,
    #[serde(rename = "2023-01-01")]
    V2023_01_01,
}

impl AsRef<str> for ApiVersion {
    fn as_ref(&self) -> &str {
        match self {
            Self::Latest => "2023-06-01",
            Self::V2023_06_01 => "2023-06-01",
            Self::V2023_01_01 => "2023-01-01",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub enum Model {
    #[serde(rename = "claude-sonnet-4-5-20250929")]
    ClaudeSonnet4_5,
    #[serde(rename = "claude-haiku-4-5-20251001")]
    ClaudeHaiku4_5,
    #[serde(rename = "claude-opus-4-5-20251101")]
    ClaudeOpus4_5,
    #[serde(rename = "claude-opus-4-1-20250805")]
    ClaudeOpus4_1,
    #[serde(rename = "claude-opus-4-20250514")]
    ClaudeOpus4,
    #[serde(rename = "claude-sonnet-4-20250514")]
    ClaudeSonnet4,
    #[serde(rename = "claude-3-7-sonnet-20250219")]
    ClaudeSonnet3_7,
    #[serde(rename = "claude-3-5-sonnet-20241022")]
    ClaudeSonnet3_5V2,
    #[serde(rename = "claude-3-5-sonnet-20240620")]
    ClaudeSonnet3_5,
    #[serde(rename = "claude-3-5-haiku-20241022")]
    ClaudeHaiku3_5,
    #[serde(rename = "claude-3-haiku-20240307")]
    ClaudeHaiku3,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    User,
    Assistant,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Content {
    String(String),
    Blocks(Vec<ContentBlock>),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum ContentBlock {
    Text {
        text: String,
        #[serde(default)]
        cache_control: Option<CacheControl>,
        #[serde(default)]
        citations: Option<Vec<Citation>>,
    },
    Image {
        source: ImageSource,
        #[serde(default)]
        cache_control: Option<CacheControl>,
    },
    Document {
        source: DocumentSource,
        #[serde(default)]
        cache_control: Option<CacheControl>,
        #[serde(default)]
        citations: Option<Vec<Citation>>,
        #[serde(default)]
        context: Option<String>,
        #[serde(default)]
        title: Option<String>,
    },
    ToolUse {
        id: String,
        input: String,
        name: String,
        #[serde(default)]
        cache_control: Option<CacheControl>,
    },
    ToolResult {
        tool_use_id: String,
        #[serde(default)]
        cache_control: Option<CacheControl>,
        #[serde(default)]
        content: Option<Vec<ToolResultContentBlock>>,
        #[serde(default)]
        is_error: Option<bool>,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum ToolResultContentBlock {
    Text { text: String },
    Image { source: ImageSource },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum ContentBlockDelta {
    TextDelta { text: String },
    InputJsonDelta { partial_json: String },
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub enum CacheControl {
    /// The time-to-live for the cache control breakpoint.
    Ephemeral { ttl: Ttl },
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub enum Ttl {
    FiveMinutes,
    OneHour,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Citation {
    CharacterLocation {
        cited_text: String,
        document_index: u32,
        document_title: Option<String>,
        end_char_index: u32,
        start_char_index: u32,
    },
    PageLocation {
        cited_text: String,
        document_index: u32,
        document_title: Option<String>,
        end_page_number: u32,
        start_page_number: u32,
    },
    ContentBlockLocation {
        cited_text: String,
        document_index: u32,
        document_title: Option<String>,
        end_block_index: u32,
        start_block_index: u32,
    },
    RequestWebSearchResultLocationCitation {
        cited_text: String,
        encrypted_index: String,
        title: Option<String>,
        url: String,
    },
    RequestSerarchResultLocationCitation {
        cited_text: String,
        end_block_index: u32,
        search_result_index: u32,
        source: String,
        start_block_index: u32,
        title: Option<String>,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum ImageSource {
    Base64 {
        data: String,
        media_type: ImageMediaType,
    },
    Url {
        url: String,
    },
    File {
        file_id: String,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum DocumentSource {
    PdfBase64 {
        data: String,
        media_type: DocumentMediaType,
    },
    PlainText {
        data: String,
        media_type: DocumentMediaType,
    },
    ContentBlock {
        content: Content,
    },
    PdfUrl {
        url: String,
    },
    FileDocument {
        file_id: String,
    },
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub enum DocumentMediaType {
    Pdf,
    Plain,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub enum ImageMediaType {
    Jpeg,
    Png,
    Gif,
    Webp,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[allow(clippy::manual_non_exhaustive)]
pub struct InputMessage {
    pub role: Role,
    pub content: Content,
    #[serde(skip)]
    pub _ne: (),
}

impl Default for InputMessage {
    fn default() -> Self {
        Self {
            role: Role::User,
            content: Content::String(String::new()),
            _ne: (),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[allow(clippy::manual_non_exhaustive)]
pub struct McpServer {
    pub name: String,
    pub url: String,
    pub authorization_token: Option<String>,
    pub tool_configuration: Option<ToolConfiguration>,
    #[serde(skip)]
    _ne: (),
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[allow(clippy::manual_non_exhaustive)]
pub struct ToolConfiguration {
    pub allowed_tools: Option<Vec<String>>,
    pub enabled: Option<bool>,
    #[serde(skip)]
    _ne: (),
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[allow(clippy::manual_non_exhaustive)]
pub struct Metadata<'a> {
    /// An external identifier for the user who is associated with the request.
    pub user_id: Option<Cow<'a, str>>,
    #[serde(skip)]
    _ne: (),
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub enum ServiceTier {
    Auto,
    StandardOnly,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub enum Thinking {
    Enabled {
        /// Determines how many tokens Claude can use for its internal reasoning process.
        budget_tokens: u32,
    },
    Disabled,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum ToolChoice {
    Auto {
        /// Whether to disable parallel tool use.
        disable_parallel_tool_use: bool,
    },
    Any {
        /// Whether to disable parallel tool use.
        disable_parallel_tool_use: bool,
    },
    Tool {
        /// The name of the tool to use.
        tool_name: String,
        /// Whether to disable parallel tool use.
        disable_parallel_tool_use: bool,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[allow(clippy::manual_non_exhaustive)]
pub struct Tool<'a> {
    /// Name of the tool.
    pub name: Cow<'a, str>,
    /// JSON schema for this tool's input.
    pub input_schema: Option<Cow<'a, str>>,
    /// Description of what this tool does.
    pub description: Option<Cow<'a, str>>,
    /// Create a cache control breakpoint at this content block.
    pub cache_control: Option<CacheControl>,
    #[serde(skip)]
    pub _ne: (),
}

impl Default for Tool<'_> {
    fn default() -> Self {
        Self {
            name: Cow::Borrowed(""),
            input_schema: None,
            description: None,
            cache_control: None,
            _ne: (),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[allow(clippy::manual_non_exhaustive)]
pub struct MessagesRequest<'a> {
    /// Optional header to specify the beta version(s) you want to use.
    #[serde(skip)]
    pub anthropic_beta: Option<Vec<Cow<'a, str>>>,
    /// The version of the Anthropic API you want to use.
    #[serde(skip)]
    pub anthropic_version: ApiVersion,
    /// Your unique API key for authentication.
    #[serde(skip)]
    pub x_api_key: Cow<'a, str>,
    /// The body of the request.
    pub body: MessagesRequestBody<'a>,
    #[serde(skip)]
    pub _ne: (),
}

impl Default for MessagesRequest<'_> {
    fn default() -> Self {
        MessagesRequest {
            anthropic_beta: None,
            anthropic_version: ApiVersion::Latest,
            x_api_key: "".into(),
            body: MessagesRequestBody::default(),
            _ne: (),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[allow(clippy::manual_non_exhaustive)]
pub struct MessagesRequestBody<'a> {
    /// The model that will complete your prompt.
    pub model: Model,
    /// Input messages.
    pub messages: Vec<InputMessage>,
    /// The maximum number of tokens to generate before stopping.
    pub max_tokens: u32,
    /// Container identifier for reuse across requests.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container: Option<Cow<'a, str>>,
    /// MCP servers to be utilized in this request
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mcp_servers: Option<Vec<McpServer>>,
    /// An object describing metadata about the request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Metadata<'a>>,
    /// Determines whether to use priority capacity (if available) or standard capacity for this request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_tier: Option<ServiceTier>,
    /// Custom text sequences that will cause the model to stop generating.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<Cow<'a, str>>>,
    /// Whether to incrementally stream the response using server-sent events.
    pub stream: bool,
    /// System prompt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<Cow<'a, str>>,
    /// Amount of randomness injected into the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Configuration for enabling Claude's extended thinking.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<Thinking>,
    /// How the model should use the provided tools.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ToolChoice>,
    /// Definitions of tools that the model may use.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool<'a>>>,
    /// Only sample from the top K options for each subsequent token.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,
    /// Use nucleus sampling.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip)]
    pub _ne: (),
}

impl Default for MessagesRequestBody<'_> {
    fn default() -> Self {
        MessagesRequestBody {
            model: Model::ClaudeSonnet4_5,
            messages: vec![],
            max_tokens: 2048,
            container: None,
            mcp_servers: None,
            metadata: None,
            service_tier: None,
            stop_sequences: None,
            stream: false,
            system: None,
            temperature: Some(1.0),
            thinking: None,
            tool_choice: None,
            tools: None,
            top_k: None,
            top_p: None,
            _ne: (),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum MessagesResponse {
    Error(ApiError),
    Message(Message),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[allow(clippy::manual_non_exhaustive)]
pub struct Message {
    /// Unique object identifier.
    pub id: String,
    /// Conversational role of the generated message.
    pub role: Role,
    /// Content generated by the model.
    pub content: Content,
    /// The model that handled the request.
    pub model: Model,
    /// The reason that we stopped.
    pub stop_reason: Option<StopReason>,
    /// Which custom stop sequence was generated, if any.
    pub stop_sequence: Option<String>,
    // TODO: usage
    // TODO: container
    #[serde(skip)]
    _ne: (),
}

impl Default for Message {
    fn default() -> Self {
        Self {
            id: String::new(),
            role: Role::Assistant,
            content: Content::String(String::new()),
            model: Model::ClaudeSonnet3_5,
            stop_reason: None,
            stop_sequence: None,
            _ne: (),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[allow(clippy::manual_non_exhaustive)]
pub struct MessageDelta {
    /// The reason that we stopped.
    pub stop_reason: Option<StopReason>,
    /// Which custom stop sequence was generated, if any.
    pub stop_sequence: Option<String>,
    #[serde(skip)]
    _ne: (),
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    /// The model reached a natural stopping point
    EndTurn,
    /// We exceeded the requested max_tokens or the model's maximum
    MaxTokens,
    /// One of your provided custom stop_sequences was generated
    StopSequence,
    /// The model invoked one or more tools
    ToolUse,
    /// We paused a long-running turn. You may provide the response back as-is in a subsequent request to let the model continue.
    PauseTurn,
    /// When streaming classifiers intervene to handle potential policy violations
    Refusal,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum MessagesResponseEvent {
    Ping,
    MessageStart {
        message: Message,
    },
    MessageDelta {
        delta: MessageDelta,
    },
    MessageStop,
    ContentBlockStart {
        index: usize,
        content_block: ContentBlock,
    },
    ContentBlockDelta {
        index: usize,
        delta: ContentBlockDelta,
    },
    ContentBlockStop {
        index: usize,
    },
}

#[derive(Debug, Error)]
pub enum AnthropicError {
    #[error("You must set stream to false to use messages")]
    StreamEnabled,
    #[error("You must set stream to true to use messages_stream")]
    StreamNotEnabled,
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
    #[error(transparent)]
    Serde(#[from] serde_json::Error),
    #[error(transparent)]
    Api(#[from] ApiError),
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[allow(clippy::manual_non_exhaustive)]
pub struct ApiError {
    pub error: ApiErrorDetails,
    #[serde(skip)]
    _ne: (),
}

impl Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} - {}", self.error.r#type, self.error.message)
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[allow(clippy::manual_non_exhaustive)]
pub struct ApiErrorDetails {
    pub r#type: String,
    pub message: String,
    #[serde(skip)]
    _ne: (),
}

impl std::error::Error for ApiError {}

pub struct MessageStream {
    stream: Pin<Box<dyn futures_core::Stream<Item = reqwest::Result<Bytes>> + Send>>,
    buf: Vec<u8>,
}

impl MessageStream {
    pub async fn recv(&mut self) -> Result<Option<MessagesResponseEvent>, AnthropicError> {
        let mut lines_parsed = 0;
        let mut data = None;
        loop {
            while let Some(at) = self.buf.iter().position(|&b| b == b'\n') {
                let line = self.buf.drain(..=at).collect::<Vec<u8>>();
                let line = String::from_utf8_lossy(&line);
                let line = line.trim();

                match lines_parsed {
                    0 => assert!(line.strip_prefix("event: ").is_some()),
                    1 => data = Some(serde_json::from_str(line.strip_prefix("data: ").unwrap())?),
                    2 => return Ok(data.unwrap()),
                    _ => unreachable!(),
                }

                lines_parsed += 1;
                lines_parsed %= 3;
            }

            match self.stream.next().await {
                Some(Ok(bytes)) => self.buf.extend_from_slice(&bytes),
                Some(Err(err)) => return Err(AnthropicError::Reqwest(err)),
                None => return Ok(None),
            }
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct AnthropicClient {
    client: reqwest::Client,
}

impl AnthropicClient {
    pub fn new() -> Self {
        AnthropicClient {
            client: reqwest::Client::new(),
        }
    }

    /// Send a structured list of input messages with text and/or image content, and the model will generate the next message in the conversation.
    pub async fn messages(&self, request: &MessagesRequest<'_>) -> Result<Message, AnthropicError> {
        if request.body.stream {
            return Err(AnthropicError::StreamEnabled);
        }

        let mut post = self.client.post("https://api.anthropic.com/v1/messages");

        if let Some(beta) = &request.anthropic_beta {
            post = post.header("anthropic-beta", beta.join(","));
        }

        let response = post
            .header("anthropic-version", request.anthropic_version.as_ref())
            .header("x-api-key", request.x_api_key.as_ref())
            .body(serde_json::to_string(&request.body)?)
            .send()
            .await?;

        match serde_json::from_str::<MessagesResponse>(&response.text().await?)? {
            MessagesResponse::Message(messages_response) => Ok(messages_response),
            MessagesResponse::Error(api_error) => Err(AnthropicError::Api(api_error)),
        }
    }

    /// Send a structured list of input messages with text and/or image content, and the model will generate the next message in the conversation.
    pub async fn messages_stream(
        &self,
        request: &MessagesRequest<'_>,
    ) -> Result<MessageStream, AnthropicError> {
        if !request.body.stream {
            return Err(AnthropicError::StreamNotEnabled);
        }

        let mut post = self.client.post("https://api.anthropic.com/v1/messages");

        if let Some(beta) = &request.anthropic_beta {
            post = post.header("anthropic-beta", beta.join(","));
        }

        let response = post
            .header("anthropic-version", request.anthropic_version.as_ref())
            .header("x-api-key", request.x_api_key.as_ref())
            .body(serde_json::to_string(&request.body)?)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await?;
            if let Ok(api_error) = serde_json::from_str::<ApiError>(&error_text) {
                return Err(AnthropicError::Api(api_error));
            }

            return Err(AnthropicError::Api(ApiError {
                error: ApiErrorDetails {
                    r#type: format!("http_error_{}", status.as_u16()),
                    message: error_text,
                    ..Default::default()
                },
                ..Default::default()
            }));
        }

        Ok(MessageStream {
            stream: Box::pin(response.bytes_stream()),
            buf: vec![],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[ignore]
    #[tokio::test]
    async fn test_messages() {
        AnthropicClient::new()
            .messages(&MessagesRequest {
                anthropic_beta: None,
                anthropic_version: ApiVersion::Latest,
                x_api_key: env!("ANTHROPIC_API_KEY").into(),
                body: MessagesRequestBody {
                    model: Model::ClaudeSonnet3_5,
                    messages: vec![InputMessage {
                        role: Role::User,
                        content: Content::String("Hello, how are you?".to_string()),
                        ..Default::default()
                    }],
                    ..Default::default()
                },
                ..Default::default()
            })
            .await
            .unwrap();
    }

    #[ignore]
    #[tokio::test]
    async fn test_messages_stream() {
        tracing_subscriber::fmt::init();

        let mut stream = AnthropicClient::new()
            .messages_stream(&MessagesRequest {
                anthropic_beta: None,
                anthropic_version: ApiVersion::Latest,
                x_api_key: env!("ANTHROPIC_API_KEY").into(),
                body: MessagesRequestBody {
                    model: Model::ClaudeSonnet3_5,
                    messages: vec![InputMessage {
                        role: Role::User,
                        content: Content::String("Hello, how are you?".to_string()),
                        ..Default::default()
                    }],
                    stream: true,
                    ..Default::default()
                },
                ..Default::default()
            })
            .await
            .unwrap();

        while let Some(event) = stream.recv().await.unwrap() {
            println!("{:?}", event);
        }
    }
}
