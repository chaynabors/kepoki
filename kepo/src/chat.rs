use std::process::ExitCode;

use anyhow::Result;
use clap::Args;

#[derive(Debug, Default, Args)]
pub struct ChatArgs;

impl ChatArgs {
    pub async fn invoke(self) -> Result<ExitCode> {
        Ok(ExitCode::SUCCESS)
    }
}
