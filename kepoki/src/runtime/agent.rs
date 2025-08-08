use std::borrow::Cow;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::process::ExitCode;

use serde::Deserialize;
use serde::Serialize;
use tokio::sync::mpsc::error::TryRecvError;

use crate::backend::Backend;
use crate::backend::ContentBlock;
use crate::backend::ContentBlockDelta;
use crate::backend::ContentBlockStart;
use crate::backend::ContentBlockStop;
use crate::backend::InputMessage;
use crate::backend::Message;
use crate::backend::MessageDelta;
use crate::backend::MessageStream;
use crate::backend::MessagesRequest;
use crate::backend::MessagesResponseEvent;
use crate::backend::Role;
use crate::error::KepokiError;
use crate::runtime::AgentHandle;

#[derive(Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub enum AgentCommand {
    Exit,
    Pause,
    Unpause,
    Terminate,
    DumpState,
    UserMessage(String),
}

#[derive(Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub enum AgentEvent {
    Ping,
    MessageStart(Message),
    MessageDelta(MessageDelta),
    MessageStop,
    /// Represents a message that has been fully received and processed.
    Message(Message),
    ContentBlockStart(ContentBlockStart),
    ContentBlockDelta(ContentBlockDelta),
    ContentBlockStop(ContentBlockStop),
    Terminated(String),
    Completed(AgentHandle),
    StateDump(Box<AgentState>),
}

impl From<MessagesResponseEvent> for AgentEvent {
    fn from(event: MessagesResponseEvent) -> Self {
        match event {
            MessagesResponseEvent::Ping => Self::Ping,
            MessagesResponseEvent::MessageStart(event) => Self::MessageStart(event),
            MessagesResponseEvent::MessageDelta(event) => Self::MessageDelta(event),
            MessagesResponseEvent::MessageStop => Self::MessageStop,
            MessagesResponseEvent::ContentBlockStart(event) => Self::ContentBlockStart(event),
            MessagesResponseEvent::ContentBlockDelta(event) => Self::ContentBlockDelta(event),
            MessagesResponseEvent::ContentBlockStop(event) => Self::ContentBlockStop(event),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct AgentState {
    pub definition: crate::agent::Agent,
    pub messages: VecDeque<InputMessage>,
    pub paused: bool,
}

pub struct Agent<B: Backend> {
    pub backend: B,
    pub model: B::Model,
    pub handle: AgentHandle,
    pub command_receiver: tokio::sync::mpsc::UnboundedReceiver<AgentCommand>,
    pub event_emitter: tokio::sync::mpsc::UnboundedSender<AgentEvent>,
    pub state: AgentState,
}

impl<B: Backend> Agent<B> {
    pub fn run(mut self) -> Result<ExitCode, KepokiError> {
        loop {
            // Handle incoming commands
            loop {
                match self.command_receiver.try_recv() {
                    Ok(command) => {
                        if let Some(exit_code) = self.handle_command(command)? {
                            return Ok(exit_code);
                        }
                    }
                    Err(TryRecvError::Empty) => {
                        if let Some(message) = self.state.messages.back() {
                            if message.role == Role::User && !self.state.paused {
                                break;
                            }
                        }

                        std::thread::sleep(std::time::Duration::from_millis(100));
                    }
                    Err(TryRecvError::Disconnected) => {
                        tracing::info!("Agent channel disconnected, shutting down thread.");
                        return Ok(ExitCode::FAILURE);
                    }
                }
            }

            // Continue conversation
            let mut stream = self.backend.messages(MessagesRequest {
                model: self.model.clone(),
                messages: self.state.messages.clone().into(),
                max_tokens: 8192,
                system: Some(Cow::Borrowed(&self.state.definition.prompt)),
                temperature: Some(self.state.definition.temperature),
                tool_choice: None,
                tools: None,
            })?;

            let mut message = None;
            let mut blocks = HashMap::new();
            while let Some(event) = stream.recv()? {
                self.event_emitter
                    .send(AgentEvent::from(event.clone()))
                    .map_err(|_| KepokiError::EventReceiverClosed(self.handle.clone()))?;

                match event {
                    MessagesResponseEvent::Ping => (),
                    MessagesResponseEvent::MessageStart(start) => {
                        if message.is_some() {
                            return Err(KepokiError::UnexpectedEvent(self.handle.clone()));
                        }

                        message = Some(start);
                    }
                    MessagesResponseEvent::MessageDelta(delta) => {
                        let message = message
                            .as_mut()
                            .ok_or_else(|| KepokiError::UnexpectedEvent(self.handle.clone()))?;

                        if let Some(stop_reason) = delta.stop_reason {
                            message.stop_reason = Some(stop_reason);
                        }

                        if let Some(stop_sequence) = delta.stop_sequence {
                            message.stop_sequence = Some(stop_sequence);
                        }

                        if let Some(usage) = delta.usage {
                            message.usage = Some(usage);
                        }
                    }
                    MessagesResponseEvent::MessageStop => {
                        if message.is_none() {
                            return Err(KepokiError::UnexpectedEvent(self.handle.clone()));
                        }
                    }
                    MessagesResponseEvent::ContentBlockStart(block) => {
                        if blocks.insert(block.index, block.content_block).is_some() {
                            return Err(KepokiError::UnexpectedEvent(self.handle.clone()));
                        }
                    }
                    MessagesResponseEvent::ContentBlockDelta(delta) => match delta {
                        ContentBlockDelta::Text { index, text } => {
                            let Some(block) = blocks.get_mut(&index) else {
                                return Err(KepokiError::UnexpectedEvent(self.handle.clone()));
                            };

                            match block {
                                ContentBlock::Text { text: block_text } => {
                                    block_text.push_str(&text);
                                }
                                _ => {
                                    return Err(KepokiError::UnexpectedEvent(self.handle.clone()));
                                }
                            }
                        }
                        ContentBlockDelta::InputJson {
                            index,
                            partial_json,
                        } => {
                            let Some(block) = blocks.get_mut(&index) else {
                                return Err(KepokiError::UnexpectedEvent(self.handle.clone()));
                            };

                            match block {
                                ContentBlock::ToolUse { input, .. } => {
                                    input.push_str(&partial_json);
                                }
                                _ => {
                                    return Err(KepokiError::UnexpectedEvent(self.handle.clone()));
                                }
                            }
                        }
                    },
                    MessagesResponseEvent::ContentBlockStop(content_block_stop) => {
                        if blocks.contains_key(&content_block_stop.index) {
                            blocks.remove(&content_block_stop.index);
                        }
                    }
                }
            }

            match message {
                Some(mut msg) => {
                    msg.content = blocks.into_values().collect();
                    self.state.messages.push_back(InputMessage {
                        role: Role::Assistant,
                        content: msg.content.clone(),
                    });
                    self.event_emitter
                        .send(AgentEvent::Message(msg))
                        .map_err(|_| KepokiError::EventReceiverClosed(self.handle.clone()))?;
                }
                None => return Err(KepokiError::NoMessageReceived(self.handle.clone())),
            }
        }
    }

    fn handle_command(&mut self, command: AgentCommand) -> Result<Option<ExitCode>, KepokiError> {
        match command {
            AgentCommand::Exit => {
                tracing::info!("Agent {} exiting", self.handle);
                return Ok(Some(ExitCode::SUCCESS));
            }
            AgentCommand::Pause => {
                tracing::info!("Agent {} paused", self.handle);
                self.state.paused = true;
            }
            AgentCommand::Unpause => {
                tracing::info!("Agent {} unpaused", self.handle);
                self.state.paused = false;
            }
            AgentCommand::DumpState => {
                tracing::info!("Dumping state for agent {}", self.handle);
                self.event_emitter
                    .send(AgentEvent::StateDump(Box::new(self.state.clone())))
                    .map_err(|_| KepokiError::EventReceiverClosed(self.handle.clone()))?;
            }
            AgentCommand::UserMessage(message) => {
                tracing::info!("Received user message for agent {}", self.handle);
                self.state.messages.push_back(InputMessage {
                    role: Role::User,
                    content: vec![ContentBlock::Text { text: message }],
                });
            }
            command => {
                unreachable!("Command not intercepted by the runtime: {command:?}")
            }
        }

        Ok(None)
    }
}
