use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;
use std::str::FromStr;

use anyhow::Result;
use aws_sdk_bedrockruntime::Config;
use clap::Args;
use kepoki::agent::Agent;
use kepoki::runtime::Runtime;
use kepoki::runtime::agent::AgentCommand;
use kepoki::runtime::agent::AgentEvent;
use kepoki_bedrock::BedrockBackend;
use tokio::io::AsyncBufReadExt;
use tokio::io::BufReader;
use tokio::select;

enum AgentIdentifier {
    Named(String),
    Path(PathBuf),
}

impl FromStr for AgentIdentifier {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // If the path exists as a file the user has access to, treat it as a path.
        if std::fs::metadata(s).is_ok() {
            return Ok(AgentIdentifier::Path(PathBuf::from(s)));
        }

        // Otherwise check if it meets the naming convention for an agent.
        let regex = regex::Regex::new(r"^([a-z][a-z0-9]*)(-[a-z0-9]+)*$")?;
        if regex.is_match(s) {
            return Ok(AgentIdentifier::Named(s.to_string()));
        }

        // Otherwise, return an error.
        Err(anyhow::anyhow!("Invalid agent identifier: {}", s))
    }
}

#[derive(Debug, Args)]
pub struct RunArgs {
    /// The agent to run
    agent: String,
}

impl RunArgs {
    pub async fn invoke(self) -> Result<ExitCode> {
        let agent_identifier = AgentIdentifier::from_str(&self.agent)?;
        let agent: Agent = match agent_identifier {
            AgentIdentifier::Named(name) => todo!(),
            AgentIdentifier::Path(path) => {
                serde_json::from_reader(std::io::BufReader::new(File::open(&path)?))?
            }
        };

        let backend = BedrockBackend::new(
            Config::builder()
                .region(aws_sdk_bedrockruntime::config::Region::new("us-west-2"))
                .build(),
        );

        let mut runtime = Runtime::new();
        let agent = runtime.spawn_agent(backend, "".to_string(), agent);

        let mut stdout = std::io::stdout();
        let mut stdin = BufReader::new(tokio::io::stdin());
        let mut buf = String::new();

        loop {
            select! {
                event = runtime.recv() => {
                    match event {
                        Ok(event) => {
                            stdout.write_all(serde_json::to_string(&event).unwrap().as_bytes())?;
                            match event {
                                AgentEvent::Terminated(_) => {
                                    return Ok(ExitCode::FAILURE);
                                },
                                AgentEvent::Completed(_) => {
                                    return Ok(ExitCode::SUCCESS);
                                },
                                _ => {}
                            }
                        }
                        Err(err) => eprintln!("Error receiving event: {}", err),
                    }
                }
                _ = stdin.read_line(&mut buf) => {
                    match serde_json::from_str::<AgentCommand>(&buf) {
                        Ok(command) => runtime.send(&agent, command)?,
                        Err(_) => eprintln!("Failed to parse command: {}", buf),
                    }
                    buf.clear();
                }
            }
        }
    }
}
