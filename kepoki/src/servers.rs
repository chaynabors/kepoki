use std::collections::HashMap;

use crate::agent::McpServer;

pub struct McpServers {
    servers: HashMap<McpServer, McpServerInstance>,
}

impl McpServers {
    pub fn new() -> Self {
        Self {
            servers: HashMap::new(),
        }
    }

    // pub async fn load(&mut self, server: McpServer) -> Result<(), KepokiError> {
    //     if let Some(server) = self.servers.get(&server) {
    //         tracing::info!("MCP server already loaded: {:?}", server);
    //         return Ok(());
    //     }
    //
    //     let instance = match &server {
    //         McpServer::Remote(_) => McpServerInstance::Remote,
    //         McpServer::Local(server) => {
    //             todo!();
    //             // McpServerInstance::Local(LocalMcpServerInstance::spawn(server).await?)
    //         }
    //     };
    //
    //     self.servers.insert(server, instance);
    //
    //     Ok(())
    // }
}

#[derive(Debug)]
enum McpServerInstance {
    Local(LocalMcpServerInstance),
    Remote,
}

#[derive(Debug)]
struct LocalMcpServerInstance;

impl LocalMcpServerInstance {
    //pub async fn spawn(mcp_server: &LocalMcpServer) -> Result<Self, KepokiError> {
    //    tracing::info!("Spawning local MCP server: {}", mcp_server.command);
    //    let mut command = Command::new(mcp_server.command);
    //    command.args(mcp_server.args).envs(mcp_server.env);
    //    let service = ().serve(TokioChildProcess::new(command)?).await?;
    //
    //    tracing::info!("Connected to server: {:#?}", service.peer_info());
    //
    //    // List tools
    //    let tools = service.list_tools(Default::default()).await?;
    //    println!("Available tools: {tools:#?}");
    //
    //    // Call tool 'git_status' with arguments = {"repo_path": "."}
    //    let tool_result = service
    //        .call_tool(CallToolRequestParam {
    //            name: "git_status".into(),
    //            arguments: serde_json::json!({ "repo_path": "." }).as_object().cloned(),
    //        })
    //        .await?;
    //    println!("Tool result: {tool_result:#?}");
    //
    //    service.cancel().await?;
    //
    //    Ok(McpServerInstance {})
    //}
}
