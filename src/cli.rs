#![allow(clippy::print_stderr)]
#![allow(clippy::print_stdout)]

use std::io::{self, Write as _};

use clap::{Parser, Subcommand};

use crate::updater::{guess_installation_method, update_sync, UpdateStatus};

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
        match self.subcommand.as_ref() {
            Some(SubCommand::SelfUpdate) => {
                run_update();
                Some(())
            }
            None | Some(SubCommand::Start) => None,
        }
    }
}

fn run_update() {
    let method = guess_installation_method();
    if !method.can_be_updated() {
        println!("The application appears to be installed via {method}.");
        println!(
            "It is recommended to update it using the same package manager."
        );
        if !confirm_default_no("Would you like to continue anyway?") {
            eprintln!("Update aborted.");
            return;
        }
    }
    match update_sync(true) {
        Err(err) => {
            eprintln!("Failed to update: {err}.");
        }
        Ok(UpdateStatus::UpToDate(version)) => {
            println!("\ntoggl-track {version} is up to date.");
        }
        Ok(UpdateStatus::Updated(version)) => {
            println!("\ntoggl-track updated to {version}.");
        }
    }
}

fn confirm_default_no(msg: &str) -> bool {
    print!("{msg} [y/N] ");
    io::stdout().flush().expect("Cannot flush stdout");

    let mut s = String::new();
    // OK to discard: will just reject on error as desired.
    let _ = io::stdin().read_line(&mut s);
    let s = s.trim().to_lowercase();
    s == "y"
}
