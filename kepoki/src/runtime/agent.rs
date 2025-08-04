use std::borrow::Cow;
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
}

#[derive(Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub enum AgentEvent {
    Ping,
    MessageStart(Message),
    MessageDelta(MessageDelta),
    MessageStop,
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
    pub async fn run(mut self) -> Result<ExitCode, KepokiError> {
        let messages = vec![InputMessage {
            role: Role::User,
            content: vec![ContentBlock::Text {
                text: "Starting...".to_string(),
            }],
        }];

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
                        if !self.state.paused {
                            break;
                        }
                    }
                    Err(TryRecvError::Disconnected) => {
                        tracing::info!("Agent channel disconnected, shutting down thread.");
                        return Ok(ExitCode::FAILURE);
                    }
                }
            }

            // Continue conversation
            let mut stream = self
                .backend
                .messages(MessagesRequest {
                    model: self.model.clone(),
                    messages: messages.clone(),
                    max_tokens: 8192,
                    system: Some(Cow::Borrowed(&self.state.definition.prompt)),
                    temperature: Some(self.state.definition.temperature),
                    tool_choice: None,
                    tools: None,
                })
                .unwrap();

            while let Some(event) = stream.recv().unwrap() {
                let event = AgentEvent::from(event);
                self.event_emitter
                    .send(event)
                    .map_err(|_| KepokiError::EventReceiverClosed(self.handle.clone()))?;
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
            command => {
                unreachable!("Command not intercepted by the runtime: {command:?}")
            }
        }

        Ok(None)
    }
}
