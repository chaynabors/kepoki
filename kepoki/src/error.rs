use rmcp::RmcpError;
use thiserror::Error;

use crate::runtime::AgentHandle;

#[derive(Debug, Error)]
pub enum KepokiError {
    #[error("Error with MCP server: {0}")]
    McpServerError(Box<RmcpError>),
    #[error("Failed to join thread: {0}")]
    JoinFailed(#[from] tokio::task::JoinError),
    #[error("Attempted to communicate with the runtime without running agents")]
    NoRunningAgents,
    #[error("Agent does not exist: {0:#?}")]
    AgentNotFound(AgentHandle),
    #[error("Agent manually terminated: {0:#?}")]
    AgentManuallyTerminated(AgentHandle),
    #[error("Agent event receiver closed unexpectedly: {0:#?}")]
    EventReceiverClosed(AgentHandle),
    #[error(transparent)]
    CustomError(Box<dyn std::error::Error + Send + Sync>),
}

impl From<RmcpError> for KepokiError {
    fn from(err: RmcpError) -> Self {
        KepokiError::McpServerError(Box::new(err))
    }
}
