use std::borrow::Cow;

use anthropic_client::AnthropicClient;
use anthropic_client::ApiVersion;
use anthropic_client::MessagesRequestBody;
use anthropic_client::Model;
use anthropic_client::ToolChoice as AnthropicToolChoice;
use kepoki::backend::MessageStream;
use kepoki::backend::ToolChoice;
use kepoki::error::KepokiError;

pub struct AnthropicMessageStream(anthropic_client::MessageStream);

impl MessageStream for AnthropicMessageStream {
    fn recv(&mut self) -> Result<Option<kepoki::backend::MessagesResponseEvent>, KepokiError> {
        match smol::block_on(self.0.recv()) {
            Ok(Some(event)) => Ok(Some(match event {
                anthropic_client::MessagesResponseEvent::Ping => {
                    kepoki::backend::MessagesResponseEvent::Ping
                }
                anthropic_client::MessagesResponseEvent::MessageStart { message } => {
                    kepoki::backend::MessagesResponseEvent::MessageStart(message.into())
                }
                anthropic_client::MessagesResponseEvent::MessageDelta { delta } => {
                    kepoki::backend::MessagesResponseEvent::MessageDelta(delta.into())
                }
                anthropic_client::MessagesResponseEvent::MessageStop => {
                    kepoki::backend::MessagesResponseEvent::MessageStop
                }
                anthropic_client::MessagesResponseEvent::ContentBlockStart {
                    index,
                    content_block,
                } => kepoki::backend::MessagesResponseEvent::ContentBlockStart(
                    kepoki::backend::ContentBlockStart {
                        index,
                        content_block: reverse_convert_content_block(content_block),
                    },
                ),
                anthropic_client::MessagesResponseEvent::ContentBlockDelta { index, delta } => {
                    kepoki::backend::MessagesResponseEvent::ContentBlockDelta(match delta {
                        anthropic_client::ContentBlockDelta::TextDelta { text } => {
                            kepoki::backend::ContentBlockDelta::Text { id, text }
                        }
                        anthropic_client::ContentBlockDelta::InputJsonDelta { partial_json } => {
                            kepoki::backend::ContentBlockDelta::InputJson { id, partial_json }
                        }
                    })
                }
                anthropic_client::MessagesResponseEvent::ContentBlockStop { index } => {
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

impl kepoki::backend::Backend for AnthropicBackend {
    type Model = Model;
    type MessagesEventStream = AnthropicMessageStream;

    fn messages(
        &self,
        request: kepoki::backend::MessagesRequest<Self>,
    ) -> Result<Self::MessagesEventStream, KepokiError> {
        Ok(AnthropicMessageStream(
            smol::block_on(
                self.client
                    .messages_stream(&anthropic_client::MessagesRequest {
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
            .map_err(|err| KepokiError::CustomError(Box::new(err)))?,
        ))
    }
}

fn convert_message(message: kepoki::backend::InputMessage) -> anthropic_client::InputMessage {
    anthropic_client::InputMessage {
        role: convert_role(message.role),
        content: convert_content(message.content),
    }
}

fn convert_role(role: kepoki::backend::Role) -> anthropic_client::Role {
    match role {
        kepoki::backend::Role::User => anthropic_client::Role::User,
        kepoki::backend::Role::Assistant => anthropic_client::Role::Assistant,
    }
}

fn convert_content(content: Vec<kepoki::backend::ContentBlock>) -> anthropic_client::Content {
    anthropic_client::Content::Blocks(content.into_iter().map(convert_content_block).collect())
}

fn convert_content_block(block: kepoki::backend::ContentBlock) -> anthropic_client::ContentBlock {
    match block {
        kepoki::backend::ContentBlock::Text { text } => anthropic_client::ContentBlock::Text {
            text,
            cache_control: None,
            citations: None,
        },
        kepoki::backend::ContentBlock::Image { source } => anthropic_client::ContentBlock::Image {
            source: convert_source(source),
            cache_control: None,
        },
        kepoki::backend::ContentBlock::ToolUse { id, input, name } => {
            anthropic_client::ContentBlock::ToolUse {
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
        } => anthropic_client::ContentBlock::ToolResult {
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

fn reverse_convert_content_block(
    block: anthropic_client::ContentBlock,
) -> kepoki::backend::ContentBlock {
    match block {
        anthropic_client::ContentBlock::Text { text, .. } => {
            kepoki::backend::ContentBlock::Text { text }
        }
        anthropic_client::ContentBlock::Image { source, .. } => {
            kepoki::backend::ContentBlock::Image {
                source: reverse_convert_source(source),
            }
        }
        anthropic_client::ContentBlock::ToolUse {
            id, input, name, ..
        } => kepoki::backend::ContentBlock::ToolUse { id, input, name },
        anthropic_client::ContentBlock::ToolResult {
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

fn convert_source(source: kepoki::backend::ImageSource) -> anthropic_client::ImageSource {
    match source {
        kepoki::backend::ImageSource::Base64 { data, media_type } => {
            anthropic_client::ImageSource::Base64 {
                data,
                media_type: convert_media_type(media_type),
            }
        }
    }
}

fn reverse_convert_source(source: anthropic_client::ImageSource) -> kepoki::backend::ImageSource {
    match source {
        anthropic_client::ImageSource::Base64 { data, media_type } => {
            kepoki::backend::ImageSource::Base64 {
                data,
                media_type: reverse_convert_media_type(media_type),
            }
        }
        _ => todo!(),
    }
}

fn convert_media_type(
    media_type: kepoki::backend::ImageMediaType,
) -> anthropic_client::ImageMediaType {
    match media_type {
        kepoki::backend::ImageMediaType::Jpeg => anthropic_client::ImageMediaType::Jpeg,
        kepoki::backend::ImageMediaType::Png => anthropic_client::ImageMediaType::Png,
        kepoki::backend::ImageMediaType::Gif => anthropic_client::ImageMediaType::Gif,
        kepoki::backend::ImageMediaType::Webp => anthropic_client::ImageMediaType::Webp,
    }
}

fn reverse_convert_media_type(
    media_type: anthropic_client::ImageMediaType,
) -> kepoki::backend::ImageMediaType {
    match media_type {
        anthropic_client::ImageMediaType::Jpeg => kepoki::backend::ImageMediaType::Jpeg,
        anthropic_client::ImageMediaType::Png => kepoki::backend::ImageMediaType::Png,
        anthropic_client::ImageMediaType::Gif => kepoki::backend::ImageMediaType::Gif,
        anthropic_client::ImageMediaType::Webp => kepoki::backend::ImageMediaType::Webp,
    }
}

fn convert_tool_result_content_block(
    block: kepoki::backend::ToolResultContentBlock,
) -> anthropic_client::ToolResultContentBlock {
    match block {
        kepoki::backend::ToolResultContentBlock::Text { text } => {
            anthropic_client::ToolResultContentBlock::Text { text }
        }
        kepoki::backend::ToolResultContentBlock::Image { source } => {
            anthropic_client::ToolResultContentBlock::Image {
                source: convert_source(source),
            }
        }
    }
}

fn reverse_convert_tool_result_content_block(
    block: anthropic_client::ToolResultContentBlock,
) -> kepoki::backend::ToolResultContentBlock {
    match block {
        anthropic_client::ToolResultContentBlock::Text { text } => {
            kepoki::backend::ToolResultContentBlock::Text { text }
        }
        anthropic_client::ToolResultContentBlock::Image { source } => {
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

fn convert_tool<'a>(tool: kepoki::backend::Tool<'a>) -> anthropic_client::Tool<'a> {
    anthropic_client::Tool {
        name: tool.name,
        description: tool.description,
        input_schema: tool.input_schema,
        cache_control: None,
    }
}
