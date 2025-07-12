use crate::{global_paths::GupGlobalPaths, operations::update_version_db};
use anyhow::{Context, Result};

pub fn run_command_update_version_db(paths: &GupGlobalPaths) -> Result<()> {
    update_version_db(paths).with_context(|| "Failed to update version db.")?;

    Ok(())
}
