pub mod agent;

use std::collections::HashMap;
use std::collections::VecDeque;
use std::fmt::Display;
use std::process::ExitCode;

use serde::Deserialize;
use serde::Serialize;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::mpsc::UnboundedSender;
use tokio::task::JoinSet;
use uuid::Uuid;

use crate::backend::Backend;
use crate::error::KepokiError;
use crate::runtime::agent::AgentCommand;
use crate::runtime::agent::AgentEvent;
use crate::runtime::agent::AgentState;

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct AgentHandle {
    name: String,
    uuid: Uuid,
}

impl Display for AgentHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

#[derive(Debug, Default)]
#[allow(clippy::type_complexity)] // Private API so allowed.
pub struct Runtime {
    thread_join_set: JoinSet<(AgentHandle, Result<ExitCode, KepokiError>)>,
    recv_join_set: JoinSet<(
        AgentHandle,
        Option<(UnboundedReceiver<AgentEvent>, AgentEvent)>,
    )>,
    command_emitters: HashMap<AgentHandle, UnboundedSender<AgentCommand>>,
}

impl Runtime {
    pub fn new() -> Self {
        Self {
            thread_join_set: JoinSet::new(),
            recv_join_set: JoinSet::new(),
            command_emitters: HashMap::new(),
        }
    }

    pub fn spawn_agent<B: Backend>(
        &mut self,
        backend: B,
        model: B::Model,
        agent: crate::agent::Agent,
    ) -> AgentHandle {
        let agent_handle = AgentHandle {
            name: agent.name.clone(),
            uuid: Uuid::new_v4(),
        };

        let (command_emitter, command_receiver) = tokio::sync::mpsc::unbounded_channel();
        let (event_emitter, mut event_receiver) = tokio::sync::mpsc::unbounded_channel();

        let join_handle = tokio::spawn(
            agent::Agent {
                backend,
                model,
                handle: agent_handle.clone(),
                command_receiver,
                event_emitter,
                state: AgentState {
                    definition: agent,
                    messages: VecDeque::new(),
                    paused: false,
                },
            }
            .run(),
        );

        let handle = agent_handle.clone();
        self.thread_join_set.spawn(async move {
            match join_handle.await {
                Ok(result) => (handle, result),
                Err(e) => (handle, Err(KepokiError::JoinFailed(e))),
            }
        });

        let handle = agent_handle.clone();
        self.recv_join_set.spawn(async {
            match event_receiver.recv().await {
                Some(event) => (handle, Some((event_receiver, event))),
                None => (handle, None),
            }
        });

        self.command_emitters
            .insert(agent_handle.clone(), command_emitter);

        agent_handle
    }

    pub fn send(&mut self, agent: &AgentHandle, command: AgentCommand) -> Result<(), KepokiError> {
        // Intercept runtime commands
        if matches!(command, AgentCommand::Terminate) {
            todo!()
        }

        match self.command_emitters.get(agent) {
            Some(emitter) => emitter
                .send(command)
                .map_err(|_| KepokiError::AgentNotFound(agent.clone())),
            None => {
                tracing::error!("No command emitter found for agent: {:?}", agent);
                Err(KepokiError::AgentNotFound(agent.clone()))
            }
        }
    }

    pub async fn recv(&mut self) -> Result<AgentEvent, KepokiError> {
        if let Some(stopped) = self.thread_join_set.try_join_next() {
            let (agent, result) = stopped?;
            return match result {
                Ok(_) => Ok(AgentEvent::Completed(agent)),
                Err(err) => Ok(AgentEvent::Terminated(err.to_string())),
            };
        }

        let (handle, output) = self
            .recv_join_set
            .join_next()
            .await
            .ok_or(KepokiError::NoRunningAgents)??;

        let (mut event_receiver, event) =
            output.ok_or(KepokiError::AgentNotFound(handle.clone()))?;

        self.recv_join_set.spawn(async move {
            match event_receiver.recv().await {
                Some(event) => (handle, Some((event_receiver, event))),
                None => (handle, None),
            }
        });

        Ok(event)
    }
}
