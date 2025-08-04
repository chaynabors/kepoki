use aws_sdk_bedrockruntime::Client;
use aws_sdk_bedrockruntime::Config;
use aws_sdk_bedrockruntime::primitives::event_stream::EventReceiver;
use aws_sdk_bedrockruntime::types::AnyToolChoice;
use aws_sdk_bedrockruntime::types::AutoToolChoice;
use aws_sdk_bedrockruntime::types::ContentBlock;
use aws_sdk_bedrockruntime::types::ContentBlockDelta;
use aws_sdk_bedrockruntime::types::ContentBlockStart;
use aws_sdk_bedrockruntime::types::ConversationRole;
use aws_sdk_bedrockruntime::types::ConverseStreamOutput;
use aws_sdk_bedrockruntime::types::ImageBlock;
use aws_sdk_bedrockruntime::types::ImageFormat;
use aws_sdk_bedrockruntime::types::ImageSource;
use aws_sdk_bedrockruntime::types::InferenceConfiguration;
use aws_sdk_bedrockruntime::types::SpecificToolChoice;
use aws_sdk_bedrockruntime::types::SystemContentBlock;
use aws_sdk_bedrockruntime::types::ToolConfiguration;
use aws_sdk_bedrockruntime::types::ToolResultBlock;
use aws_sdk_bedrockruntime::types::ToolResultContentBlock;
use aws_sdk_bedrockruntime::types::ToolResultStatus;
use aws_sdk_bedrockruntime::types::ToolSpecification;
use aws_sdk_bedrockruntime::types::ToolUseBlock;
use aws_sdk_bedrockruntime::types::ToolUseBlockDelta;
use aws_sdk_bedrockruntime::types::error::ConverseStreamOutputError;
use aws_smithy_types::Blob;
use aws_smithy_types::Document;
use kepoki::backend::Backend;
use kepoki::backend::MessageStream;
use kepoki::backend::MessagesResponseEvent;
use kepoki::error::KepokiError;

pub struct BedrockMessagesEventStream {
    stream: EventReceiver<ConverseStreamOutput, ConverseStreamOutputError>,
}

impl MessageStream for BedrockMessagesEventStream {
    fn recv(&mut self) -> Result<Option<MessagesResponseEvent>, KepokiError> {
        loop {
            let Some(output) = smol::block_on(self.stream.recv())
                .map_err(|err| KepokiError::CustomError(Box::new(err)))?
            else {
                return Ok(None);
            };

            return Ok(Some(match output {
                ConverseStreamOutput::ContentBlockDelta(content_block_delta_event) => {
                    if let Some(content_block_delta_event) = content_block_delta_event.delta {
                        MessagesResponseEvent::ContentBlockDelta(content_block_delta_event)
                    } else {
                        continue;
                    }
                    MessagesResponseEvent::ContentBlockDelta(content_block_delta_event)
                }
                ConverseStreamOutput::ContentBlockStart(content_block_start_event) => {
                    if let Some(content_block_start_event) = content_block_start_event.start {
                        match content_block_start_event {
                            ContentBlockStart::ToolUse(tool_use_block_start) => {
                                MessagesResponseEvent::ContentBlockStart(
                                    kepoki::backend::ContentBlock::ToolUse {
                                        id: tool_use_block_start.tool_use_id,
                                        name: tool_use_block_start.name,
                                        input: String::new(),
                                    },
                                )
                            }
                            _ => todo!(),
                        }
                    } else {
                        continue;
                    }
                }
                ConverseStreamOutput::ContentBlockStop(content_block_stop_event) => todo!(),
                ConverseStreamOutput::MessageStart(message_start_event) => todo!(),
                ConverseStreamOutput::MessageStop(message_stop_event) => todo!(),
                ConverseStreamOutput::Metadata(converse_stream_metadata_event) => todo!(),
                _ => {
                    tracing::warn!("Received unexpected event type from Bedrock: {:?}", output);
                    return Ok(None);
                }
            }));
        }
    }
}

pub struct BedrockBackend {
    client: Client,
}

impl BedrockBackend {
    pub fn new(config: Config) -> Self {
        let client = Client::from_conf(config);

        Self { client }
    }
}

impl Backend for BedrockBackend {
    type Model = String;
    type MessagesEventStream = BedrockMessagesEventStream;

    fn messages(
        &self,
        request: kepoki::backend::MessagesRequest<Self>,
    ) -> Result<Self::MessagesEventStream, KepokiError> {
        let mut request_builder = self
            .client
            .converse_stream()
            .model_id(request.model.clone())
            .inference_config(build_inference_config(&request)?)
            .tool_config(build_tool_config(&request)?);

        for message in &request.messages {
            request_builder = request_builder.messages(build_message(message)?);
        }

        if let Some(system) = &request.system {
            request_builder = request_builder.system(SystemContentBlock::Text(system.to_string()));
        }

        let stream = smol::block_on(request_builder.send())
            .map_err(|err| KepokiError::CustomError(Box::new(err)))?
            .stream;

        Ok(BedrockMessagesEventStream { stream })
    }
}

fn build_inference_config(
    request: &kepoki::backend::MessagesRequest<BedrockBackend>,
) -> Result<InferenceConfiguration, KepokiError> {
    let mut inference = InferenceConfiguration::builder();
    if let Ok(max_tokens) = i32::try_from(request.max_tokens) {
        inference = inference.max_tokens(max_tokens);
    }
    if let Some(temperature) = request.temperature {
        inference = inference.temperature(temperature);
    }
    Ok(inference.build())
}

fn build_tool_config(
    request: &kepoki::backend::MessagesRequest<BedrockBackend>,
) -> Result<ToolConfiguration, KepokiError> {
    let mut builder = ToolConfiguration::builder();
    if let Some(tool_choice) = &request.tool_choice {
        builder = builder.tool_choice(match tool_choice {
            kepoki::backend::ToolChoice::Auto { .. } => {
                aws_sdk_bedrockruntime::types::ToolChoice::Auto(AutoToolChoice::builder().build())
            }
            kepoki::backend::ToolChoice::Any { .. } => {
                aws_sdk_bedrockruntime::types::ToolChoice::Any(AnyToolChoice::builder().build())
            }
            kepoki::backend::ToolChoice::Tool { tool_name, .. } => {
                aws_sdk_bedrockruntime::types::ToolChoice::Tool(
                    SpecificToolChoice::builder()
                        .name(tool_name)
                        .build()
                        .map_err(|err| KepokiError::CustomError(Box::new(err)))?,
                )
            }
        });
    }

    if let Some(tools) = &request.tools {
        for tool in tools.iter() {
            builder = builder.tools(aws_sdk_bedrockruntime::types::Tool::ToolSpec({
                let mut builder = ToolSpecification::builder().name(tool.name.clone());
                if let Some(description) = &tool.description {
                    builder = builder.description(description.clone());
                }
                builder
                    .build()
                    .map_err(|err| KepokiError::CustomError(Box::new(err)))?
            }))
        }
    }

    builder
        .build()
        .map_err(|err| KepokiError::CustomError(Box::new(err)))
}

fn build_message(
    message: &kepoki::backend::InputMessage,
) -> Result<aws_sdk_bedrockruntime::types::Message, KepokiError> {
    let mut builder = aws_sdk_bedrockruntime::types::Message::builder().role(match message.role {
        kepoki::backend::Role::User => ConversationRole::User,
        kepoki::backend::Role::Assistant => ConversationRole::Assistant,
    });

    for content in &message.content {
        builder = builder.content(match content {
            kepoki::backend::ContentBlock::Text { text } => ContentBlock::Text(text.to_owned()),
            kepoki::backend::ContentBlock::Image { source } => {
                ContentBlock::Image(build_image_block(source)?)
            }
            kepoki::backend::ContentBlock::ToolUse { id, input, name } => {
                ContentBlock::ToolUse(build_tool_use(id, input, name)?)
            }
            kepoki::backend::ContentBlock::ToolResult {
                tool_use_id,
                content,
                is_error,
            } => ContentBlock::ToolResult(build_tool_result(tool_use_id, content, *is_error)?),
        });
    }

    builder
        .build()
        .map_err(|err| KepokiError::CustomError(Box::new(err)))
}

fn build_image_block(source: &kepoki::backend::ImageSource) -> Result<ImageBlock, KepokiError> {
    let mut builder = ImageBlock::builder();
    match source {
        kepoki::backend::ImageSource::Base64 { data, media_type } => {
            builder = builder.format(match media_type {
                kepoki::backend::ImageMediaType::Jpeg => ImageFormat::Jpeg,
                kepoki::backend::ImageMediaType::Png => ImageFormat::Png,
                kepoki::backend::ImageMediaType::Gif => ImageFormat::Gif,
                kepoki::backend::ImageMediaType::Webp => ImageFormat::Webp,
            });

            builder = builder.source(ImageSource::Bytes(Blob::new(data.as_bytes())))
        }
    }

    builder
        .build()
        .map_err(|err| KepokiError::CustomError(Box::new(err)))
}

fn build_tool_use(
    id: &str,
    input: &str,
    name: &str,
) -> Result<aws_sdk_bedrockruntime::types::ToolUseBlock, KepokiError> {
    ToolUseBlock::builder()
        .tool_use_id(id.to_owned())
        .name(name.to_owned())
        .input(Document::String(input.to_string()))
        .build()
        .map_err(|err| KepokiError::CustomError(Box::new(err)))
}

fn build_tool_result(
    tool_use_id: &String,
    content: &Option<Vec<kepoki::backend::ToolResultContentBlock>>,
    is_error: Option<bool>,
) -> Result<ToolResultBlock, KepokiError> {
    let mut builder = ToolResultBlock::builder().tool_use_id(tool_use_id.to_owned());

    if let Some(content) = content {
        for content in content.iter() {
            builder = builder.content(match content {
                kepoki::backend::ToolResultContentBlock::Text { text } => {
                    ToolResultContentBlock::Text(text.to_owned())
                }
                kepoki::backend::ToolResultContentBlock::Image { source } => {
                    ToolResultContentBlock::Image(build_image_block(source)?)
                }
            });
        }
    }

    if let Some(is_error) = is_error {
        builder = builder.status(match is_error {
            true => ToolResultStatus::Error,
            false => ToolResultStatus::Success,
        });
    }

    builder
        .build()
        .map_err(|err| KepokiError::CustomError(Box::new(err)))
}

fn convert_content_block_delta(
    content_block_delta: ContentBlockDelta,
) -> Option<kepoki::backend::ContentBlock> {
    Some(match content_block_delta {
        ContentBlockDelta::Text(text) => kepoki::backend::ContentBlock::Text { text },
        ContentBlockDelta::ToolUse(ToolUseBlockDelta { input, .. }) => {
            kepoki::backend::ContentBlock::ToolUse {
                id: (),
                input,
                name: (),
            }
        }
        _ => {
            tracing::warn!(
                "Received unhandled content block delta type from Bedrock: {:?}",
                content_block_delta
            );

            return None;
        }
    })
}
