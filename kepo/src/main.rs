mod chat;
mod mcp;
mod run;

use std::process::ExitCode;

use anyhow::Result;
use clap::Parser;
use mcp::McpArgs;

use crate::chat::ChatArgs;
use crate::run::RunArgs;

#[derive(Debug, Parser)]
struct Cli {
    #[clap(subcommand)]
    command: Option<Subcommand>,
}

#[derive(Debug, clap::Subcommand)]
enum Subcommand {
    Chat(ChatArgs),
    Run(RunArgs),
    Mcp(McpArgs),
}

#[tokio::main]
async fn main() -> Result<ExitCode> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();
    let subcommand = cli.command.unwrap_or(Subcommand::Chat(ChatArgs::default()));

    match subcommand {
        Subcommand::Chat(args) => args.invoke().await,
        Subcommand::Run(args) => args.invoke().await,
        Subcommand::Mcp(args) => args.invoke().await,
    }
}
