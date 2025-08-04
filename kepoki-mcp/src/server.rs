use rmcp::{ErrorData as McpError, ServerHandler, model::*, tool, tool_router};

pub struct KepokiMcpServer;

impl Default for KepokiMcpServer {
    fn default() -> Self {
        Self
    }
}

#[tool_router]
impl KepokiMcpServer {
    #[tool(description = "Say hello to the world")]
    async fn say_hello(&self) -> Result<CallToolResult, McpError> {
        Ok(CallToolResult::success(vec![Content::text(
            "Hello, world!",
        )]))
    }
}

impl ServerHandler for KepokiMcpServer {}
