use clap::{Parser, Subcommand};

use crate::updater::{update, UpdateStatus};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct CliArgs {
    #[clap(subcommand)]
    subcommand: Option<SubCommand>,
}

#[derive(Debug, Default, Subcommand)]
pub enum SubCommand {
    #[default]
    #[clap(about = "Launch the GUI")]
    Start,
    #[clap(about = "Update the application")]
    SelfUpdate,
}

impl CliArgs {
    pub fn run(&self) -> Option<()> {
        //! Returns None if not given a CLI command.
        async_std::task::block_on(self.run_internal())
    }
    async fn run_internal(&self) -> Option<()> {
        match self.subcommand.as_ref().unwrap_or(&SubCommand::default()) {
            SubCommand::SelfUpdate => {
                match update(true).await {
                    Err(err) => {
                        eprintln!("Failed to update: {err}.");
                    }
                    Ok(UpdateStatus::UpToDate(version)) => {
                        println!("\ntoggl-track {version} is up to date.");
                    }
                    Ok(UpdateStatus::Updated(version)) => {
                        println!("\ntoggl-track updated to {version}.");
                    }
                };
                Some(())
            }
            SubCommand::Start => None,
        }
    }
}
