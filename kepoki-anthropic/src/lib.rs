use std::borrow::Cow;

use anthropoki::AnthropicClient;
use anthropoki::ApiVersion;
use anthropoki::MessagesRequestBody;
use anthropoki::Model;
use anthropoki::ToolChoice as AnthropicToolChoice;
use kepoki::backend::MessageStream;
use kepoki::backend::ToolChoice;
use kepoki::error::KepokiError;

pub struct AnthropicMessageStream(anthropoki::MessageStream);

impl MessageStream for AnthropicMessageStream {
    fn recv(&mut self) -> Result<Option<kepoki::backend::MessagesResponseEvent>, KepokiError> {
        match futures::executor::block_on(self.0.recv()) {
            Ok(Some(event)) => Ok(Some(match event {
                anthropoki::MessagesResponseEvent::Ping => {
                    kepoki::backend::MessagesResponseEvent::Ping
                }
                anthropoki::MessagesResponseEvent::MessageStart { message } => {
                    kepoki::backend::MessagesResponseEvent::MessageStart(reverse_convert_message(
                        message,
                    ))
                }
                anthropoki::MessagesResponseEvent::MessageDelta { delta } => {
                    kepoki::backend::MessagesResponseEvent::MessageDelta(
                        reverse_convert_message_delta(delta),
                    )
                }
                anthropoki::MessagesResponseEvent::MessageStop => {
                    kepoki::backend::MessagesResponseEvent::MessageStop
                }
                anthropoki::MessagesResponseEvent::ContentBlockStart {
                    index,
                    content_block,
                } => kepoki::backend::MessagesResponseEvent::ContentBlockStart(
                    kepoki::backend::ContentBlockStart {
                        index,
                        content_block: reverse_convert_content_block(content_block),
                    },
                ),
                anthropoki::MessagesResponseEvent::ContentBlockDelta { index, delta } => {
                    kepoki::backend::MessagesResponseEvent::ContentBlockDelta(match delta {
                        anthropoki::ContentBlockDelta::TextDelta { text } => {
                            kepoki::backend::ContentBlockDelta::Text { index, text }
                        }
                        anthropoki::ContentBlockDelta::InputJsonDelta { partial_json } => {
                            kepoki::backend::ContentBlockDelta::InputJson {
                                index,
                                partial_json,
                            }
                        }
                    })
                }
                anthropoki::MessagesResponseEvent::ContentBlockStop { index } => {
                    kepoki::backend::MessagesResponseEvent::ContentBlockStop(
                        kepoki::backend::ContentBlockStop { index },
                    )
                }
            })),
            Ok(None) => Ok(None),
            Err(err) => Err(KepokiError::CustomError(Box::new(err))),
        }
    }
}

pub struct AnthropicBackend {
    betas: Option<Vec<String>>,
    version: ApiVersion,
    api_key: String,

    client: AnthropicClient,
}

impl AnthropicBackend {
    pub fn new(api_key: String, version: ApiVersion, betas: Option<Vec<String>>) -> Self {
        Self {
            betas,
            version,
            api_key,
            client: AnthropicClient::new(),
        }
    }
}

impl kepoki::backend::Backend for AnthropicBackend {
    type Model = Model;
    type MessagesEventStream = AnthropicMessageStream;

    fn messages(
        &self,
        request: kepoki::backend::MessagesRequest<Self>,
    ) -> Result<Self::MessagesEventStream, KepokiError> {
        Ok(AnthropicMessageStream(
            futures::executor::block_on(
                self.client.messages_stream(&anthropoki::MessagesRequest {
                    anthropic_beta: self
                        .betas
                        .as_ref()
                        .map(|b| b.iter().map(|s| Cow::Borrowed(s.as_str())).collect()),
                    anthropic_version: self.version,
                    x_api_key: self.api_key.clone().into(),
                    body: MessagesRequestBody {
                        model: request.model,
                        messages: request.messages.into_iter().map(convert_message).collect(),
                        max_tokens: request.max_tokens,
                        stream: true,
                        system: request.system,
                        temperature: request.temperature,
                        tool_choice: request.tool_choice.map(convert_tool_choice),
                        tools: request
                            .tools
                            .map(|tools| tools.into_iter().map(convert_tool).collect()),
                        ..Default::default()
                    },
                }),
            )
            .map_err(|err| KepokiError::CustomError(Box::new(err)))
            .unwrap(),
        ))
    }
}

fn convert_message(message: kepoki::backend::InputMessage) -> anthropoki::InputMessage {
    anthropoki::InputMessage {
        role: convert_role(message.role),
        content: convert_content(message.content),
    }
}

fn reverse_convert_message(message: anthropoki::Message) -> kepoki::backend::Message {
    kepoki::backend::Message {
        id: message.id,
        content: reverse_convert_content(message.content),
        stop_reason: message.stop_reason.map(reverse_convert_stop_reason),
        stop_sequence: message.stop_sequence,
        usage: None,
    }
}

fn convert_role(role: kepoki::backend::Role) -> anthropoki::Role {
    match role {
        kepoki::backend::Role::User => anthropoki::Role::User,
        kepoki::backend::Role::Assistant => anthropoki::Role::Assistant,
    }
}

fn convert_content(content: Vec<kepoki::backend::ContentBlock>) -> anthropoki::Content {
    anthropoki::Content::Blocks(content.into_iter().map(convert_content_block).collect())
}

fn reverse_convert_content(content: anthropoki::Content) -> Vec<kepoki::backend::ContentBlock> {
    match content {
        anthropoki::Content::Blocks(blocks) => blocks
            .into_iter()
            .map(reverse_convert_content_block)
            .collect(),
        _ => todo!("Unsupported content type: {:?}", content),
    }
}

fn convert_content_block(block: kepoki::backend::ContentBlock) -> anthropoki::ContentBlock {
    match block {
        kepoki::backend::ContentBlock::Text { text } => anthropoki::ContentBlock::Text {
            text,
            cache_control: None,
            citations: None,
        },
        kepoki::backend::ContentBlock::Image { source } => anthropoki::ContentBlock::Image {
            source: convert_source(source),
            cache_control: None,
        },
        kepoki::backend::ContentBlock::ToolUse { id, input, name } => {
            anthropoki::ContentBlock::ToolUse {
                id,
                input,
                name,
                cache_control: None,
            }
        }
        kepoki::backend::ContentBlock::ToolResult {
            tool_use_id,
            content,
            is_error,
        } => anthropoki::ContentBlock::ToolResult {
            tool_use_id,
            content: content.map(|c| {
                c.into_iter()
                    .map(convert_tool_result_content_block)
                    .collect()
            }),
            is_error,
            cache_control: None,
        },
    }
}

fn reverse_convert_content_block(block: anthropoki::ContentBlock) -> kepoki::backend::ContentBlock {
    match block {
        anthropoki::ContentBlock::Text { text, .. } => kepoki::backend::ContentBlock::Text { text },
        anthropoki::ContentBlock::Image { source, .. } => kepoki::backend::ContentBlock::Image {
            source: reverse_convert_source(source),
        },
        anthropoki::ContentBlock::ToolUse {
            id, input, name, ..
        } => kepoki::backend::ContentBlock::ToolUse { id, input, name },
        anthropoki::ContentBlock::ToolResult {
            tool_use_id,
            content,
            is_error,
            ..
        } => kepoki::backend::ContentBlock::ToolResult {
            tool_use_id,
            content: content.map(|c| {
                c.into_iter()
                    .map(reverse_convert_tool_result_content_block)
                    .collect()
            }),
            is_error,
        },
        _ => todo!("Unsupported content block type: {:?}", block),
    }
}

fn convert_source(source: kepoki::backend::ImageSource) -> anthropoki::ImageSource {
    match source {
        kepoki::backend::ImageSource::Base64 { data, media_type } => {
            anthropoki::ImageSource::Base64 {
                data,
                media_type: convert_media_type(media_type),
            }
        }
    }
}

fn reverse_convert_source(source: anthropoki::ImageSource) -> kepoki::backend::ImageSource {
    match source {
        anthropoki::ImageSource::Base64 { data, media_type } => {
            kepoki::backend::ImageSource::Base64 {
                data,
                media_type: reverse_convert_media_type(media_type),
            }
        }
        _ => todo!(),
    }
}

fn convert_media_type(media_type: kepoki::backend::ImageMediaType) -> anthropoki::ImageMediaType {
    match media_type {
        kepoki::backend::ImageMediaType::Jpeg => anthropoki::ImageMediaType::Jpeg,
        kepoki::backend::ImageMediaType::Png => anthropoki::ImageMediaType::Png,
        kepoki::backend::ImageMediaType::Gif => anthropoki::ImageMediaType::Gif,
        kepoki::backend::ImageMediaType::Webp => anthropoki::ImageMediaType::Webp,
    }
}

fn reverse_convert_media_type(
    media_type: anthropoki::ImageMediaType,
) -> kepoki::backend::ImageMediaType {
    match media_type {
        anthropoki::ImageMediaType::Jpeg => kepoki::backend::ImageMediaType::Jpeg,
        anthropoki::ImageMediaType::Png => kepoki::backend::ImageMediaType::Png,
        anthropoki::ImageMediaType::Gif => kepoki::backend::ImageMediaType::Gif,
        anthropoki::ImageMediaType::Webp => kepoki::backend::ImageMediaType::Webp,
    }
}

fn convert_tool_result_content_block(
    block: kepoki::backend::ToolResultContentBlock,
) -> anthropoki::ToolResultContentBlock {
    match block {
        kepoki::backend::ToolResultContentBlock::Text { text } => {
            anthropoki::ToolResultContentBlock::Text { text }
        }
        kepoki::backend::ToolResultContentBlock::Image { source } => {
            anthropoki::ToolResultContentBlock::Image {
                source: convert_source(source),
            }
        }
    }
}

fn reverse_convert_tool_result_content_block(
    block: anthropoki::ToolResultContentBlock,
) -> kepoki::backend::ToolResultContentBlock {
    match block {
        anthropoki::ToolResultContentBlock::Text { text } => {
            kepoki::backend::ToolResultContentBlock::Text { text }
        }
        anthropoki::ToolResultContentBlock::Image { source } => {
            kepoki::backend::ToolResultContentBlock::Image {
                source: reverse_convert_source(source),
            }
        }
    }
}

fn convert_tool_choice(tool_choice: ToolChoice) -> AnthropicToolChoice {
    match tool_choice {
        ToolChoice::Auto {
            disable_parallel_tool_use,
        } => AnthropicToolChoice::Auto {
            disable_parallel_tool_use,
        },
        ToolChoice::Any {
            disable_parallel_tool_use,
        } => AnthropicToolChoice::Any {
            disable_parallel_tool_use,
        },
        ToolChoice::Tool {
            tool_name,
            disable_parallel_tool_use,
        } => AnthropicToolChoice::Tool {
            tool_name,
            disable_parallel_tool_use,
        },
    }
}

fn convert_tool<'a>(tool: kepoki::backend::Tool<'a>) -> anthropoki::Tool<'a> {
    anthropoki::Tool {
        name: tool.name,
        description: tool.description,
        input_schema: tool.input_schema,
        cache_control: None,
    }
}

fn reverse_convert_stop_reason(stop_reason: anthropoki::StopReason) -> kepoki::backend::StopReason {
    match stop_reason {
        anthropoki::StopReason::EndTurn => kepoki::backend::StopReason::EndTurn,
        anthropoki::StopReason::MaxTokens => kepoki::backend::StopReason::MaxTokens,
        anthropoki::StopReason::StopSequence => kepoki::backend::StopReason::StopSequence,
        anthropoki::StopReason::ToolUse => kepoki::backend::StopReason::ToolUse,
        anthropoki::StopReason::PauseTurn => kepoki::backend::StopReason::PauseTurn,
        anthropoki::StopReason::Refusal => kepoki::backend::StopReason::Refusal,
    }
}

fn reverse_convert_message_delta(delta: anthropoki::MessageDelta) -> kepoki::backend::MessageDelta {
    kepoki::backend::MessageDelta {
        stop_reason: delta.stop_reason.map(reverse_convert_stop_reason),
        stop_sequence: delta.stop_sequence,
        usage: None,
    }
}

#[cfg(test)]
mod tests {
    use kepoki::runtime::agent::AgentCommand;
    use kepoki::runtime::agent::AgentEvent;

    use super::*;

    #[ignore]
    #[tokio::test]
    async fn test_message_stream() {
        tracing_subscriber::fmt::init();

        let api_key = std::env::var("ANTHROPIC_API_KEY").unwrap();
        let backend = AnthropicBackend::new(api_key, ApiVersion::Latest, None);
        let mut runtime = kepoki::runtime::Runtime::new();
        let agent = runtime.spawn_agent(
            backend,
            Model::ClaudeSonnet3_5,
            kepoki::agent::Agent {
                prompt: "You are an agent that does everything for me without asking".into(),
                ..Default::default()
            },
        );

        runtime
            .send(
                &agent,
                AgentCommand::UserMessage("Hello! Who are you?".to_string()),
            )
            .unwrap();

        while let Ok(event) = runtime.recv().await {
            tracing::info!("Received event: {:?}", event);
            if matches!(event, AgentEvent::Message(_)) {
                break;
            }
        }
    }
}
