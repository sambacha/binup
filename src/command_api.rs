use crate::global_paths::GupGlobalPaths;
use anyhow::{bail, Result};

pub fn run_command_api(command: &str, _paths: &GupGlobalPaths) -> Result<()> {
    // API command is Julia-specific and not applicable to generic gup
    bail!(
        "API command '{}' is not supported in gup. This was a Julia-specific feature.",
        command
    );
}
