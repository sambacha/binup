use crate::cli::GupCli;
use anyhow::Result;
use clap::CommandFactory;
use clap_complete::Shell;
use std::io;

pub fn run_command_completions(shell: Shell) -> Result<()> {
    clap_complete::generate(
        shell,
        &mut GupCli::command(),
        "gup", // Changed from "juliaup" to "gup"
        &mut io::stdout().lock(),
    );
    Ok(())
}
