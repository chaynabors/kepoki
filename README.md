# Kepoki

A Rust framework for building AI agents and applications with multi-provider 
support, streaming responses, and comprehensive tool support.

## Crates

* **kepoki** - Core agent framework with runtime, backend abstraction, and tool integration. What this README is about.
* **anthropoki** - Standalone Anthropic API client with streaming support
* **kepoki-anthropic** - Anthropic backend adapter for the kepoki framework
* **kepoki-bedrock** - AWS Bedrock backend adapter for the kepoki framework

## Features

* **Multi-Provider Backends**: Support for Anthropic, AWS Bedrock, and other AI services through a unified interface
* **Agent Runtime**: Complete lifecycle management with state persistence and error recovery
* **Streaming Responses**: Real-time message streaming from AI providers
* **Tool Integration**: Built-in tool framework and MCP support
* **Multi-Agent Communication**: Agent-to-agent messaging for complex workflows
* **Rich Content**: Support for text, images, and structured data

## Quick Start

Add to your Cargo.toml:

```toml
[dependencies]
kepoki = "0.2.0"
kepoki-anthropic = "0.2.0"
```

Basic usage:

```rust
use kepoki::*;
use kepoki_anthropic::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let backend = AnthropicBackend::new(
        std::env::var("ANTHROPIC_API_KEY")?,
        ApiVersion::Latest,
        None
    );
    
    let mut runtime = Runtime::new();
    let agent = runtime.spawn_agent(
        backend,
        Model::ClaudeSonnet3_5,
        Agent {
            name: "assistant".to_string(),
            prompt: "You are a helpful assistant.".to_string(),
            ..Default::default()
        }
    );

    runtime.send(&agent, AgentCommand::UserMessage("Hello!".to_string()))?;
    
    while let Ok(event) = runtime.recv().await {
        match event {
            AgentEvent::Message(msg) => {
                println!("Assistant: {:?}", msg.content);
                break;
            }
            _ => {}
        }
    }
    
    Ok(())
}
```

## Agent Configuration

```rust
let agent = Agent {
    name: "code-assistant".to_string(),
    description: "Helps with coding tasks".to_string(),
    prompt: "You are an expert programmer.".to_string(),
    temperature: 0.7,
    tools: vec!["git".parse()?, "file-ops".parse()?],
    mcp_servers: HashMap::from([
        ("git".to_string(), McpServer::Local(LocalMcpServer {
            command: "git-mcp-server".to_string(),
            args: vec![],
            env: HashMap::new(),
        }))
    ]),
    ..Default::default()
};
```


## Streaming Responses

```rust
while let Ok(event) = runtime.recv().await {
    match event {
        AgentEvent::ContentBlockDelta(delta) => {
            match delta {
                ContentBlockDelta::Text { text, .. } => print!("{}", text),
                _ => {}
            }
        }
        AgentEvent::Message(msg) => break,
        _ => {}
    }
}
```

## Multi-Agent Communication

TODO: A2A

```rust
let code_agent = runtime.spawn_agent(backend.clone(), model, Agent {
    name: "coder".to_string(),
    prompt: "You write code.".to_string(),
    ..Default::default()
});

let review_agent = runtime.spawn_agent(backend, model, Agent {
    name: "reviewer".to_string(), 
    prompt: "You review code for quality.".to_string(),
    ..Default::default()
});

// Agents can communicate through the runtime
runtime.send(&code_agent, AgentCommand::UserMessage("Write a function".to_string()))?;
// ... get code response ...
runtime.send(&review_agent, AgentCommand::UserMessage("Review this code".to_string()))?;
```

## Backends

### Anthropic
```bash
cargo add kepoki-anthropic
```

### AWS Bedrock
```bash
cargo add kepoki-bedrock
```

## Tools and MCP

Kepoki supports both built-in tools and MCP servers:

TODO

## License

Licensed under either of Apache 2.0 OR MIT at your leisure.
