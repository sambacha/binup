use crate::global_paths::GupGlobalPaths;
use crate::state_config::ProjectLocalState;
use anyhow::{Context, Result};
use std::fs;
// use std::path::PathBuf; // PathBuf is not directly used

pub fn load_project_local_state(
    project_name: &str,
    paths: &GupGlobalPaths,
) -> Result<ProjectLocalState> {
    let state_path = paths.project_state_file(project_name);

    if !state_path.exists() {
        // If the state file doesn't exist, return a default state
        return Ok(ProjectLocalState::default());
    }

    let content = fs::read_to_string(&state_path).with_context(|| {
        format!(
            "Could not read project state file for '{}' at '{}'.",
            project_name,
            state_path.display()
        )
    })?;

    let state: ProjectLocalState = serde_json::from_str(&content).with_context(|| {
        format!(
            "Could not parse project state file for '{}' at '{}'.",
            project_name,
            state_path.display()
        )
    })?;

    Ok(state)
}

pub fn save_project_local_state(
    project_name: &str,
    state: &ProjectLocalState,
    paths: &GupGlobalPaths,
) -> Result<()> {
    let state_path = paths.project_state_file(project_name);
    let parent_dir = state_path.parent().ok_or_else(|| {
        anyhow::anyhow!("Could not determine parent directory for project state file.")
    })?;

    fs::create_dir_all(parent_dir).with_context(|| {
        format!(
            "Could not create directory for project state at '{}'.",
            parent_dir.display()
        )
    })?;

    let content = serde_json::to_string_pretty(state)
        .with_context(|| "Could not serialize project local state.")?;

    // Atomic write: write to a temporary file and then rename
    let temp_path = state_path.with_file_name(format!(
        "{}.tmp",
        state_path.file_name().unwrap().to_string_lossy()
    ));
    // Ensure directory exists before writing temp file
    if let Some(parent) = temp_path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "Could not create directory for temporary project state file at '{}'.",
                parent.display()
            )
        })?;
    }

    fs::write(&temp_path, content).with_context(|| {
        format!(
            "Could not write temporary project state file to '{}'.",
            temp_path.display()
        )
    })?;

    fs::rename(&temp_path, &state_path).with_context(|| {
        format!(
            "Could not rename temporary project state file from '{}' to '{}'.",
            temp_path.display(),
            state_path.display()
        )
    })?;

    Ok(())
}
