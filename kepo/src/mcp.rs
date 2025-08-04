use std::process::ExitCode;

use anyhow::Result;
use clap::Args;
use kepoki_mcp::KepokiMcpServer;
use rmcp::serve_server;

#[derive(Debug, Args)]
pub struct McpArgs;

impl McpArgs {
    pub async fn invoke(self) -> Result<ExitCode> {
        let io = (tokio::io::stdin(), tokio::io::stdout());
        serve_server(KepokiMcpServer, io).await?;
        Ok(ExitCode::SUCCESS)
    }
}
