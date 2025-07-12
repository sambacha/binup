use crate::error_handling::GupError;
use crate::global_config_manager::{add_managed_project, is_project_managed, load_global_config};
use crate::global_paths::GupGlobalPaths;
use crate::operations_metadata::sync_project_metadata;
use crate::project_state_manager::save_project_local_state;
use crate::state_config::ProjectLocalState;
use anyhow::{Context, Result};
use console::style;

pub fn run_command_project_add(registration_source: &str, paths: &GupGlobalPaths) -> Result<()> {
    eprintln!(
        "{} project from source: {}",
        style("Adding").cyan().bold(),
        registration_source
    );

    // Parse registration_source as URL
    let metadata_url = url::Url::parse(registration_source)
        .map_err(|_| GupError::invalid_url(registration_source, "project registration"))?;

    // Fetch and parse metadata to get project information
    eprintln!("  Fetching project metadata...");
    let response = reqwest::blocking::get(metadata_url.clone())
        .map_err(|e| GupError::network_error(registration_source, &e.into()))?;

    if !response.status().is_success() {
        return Err(GupError::http_error(
            registration_source,
            response.status().as_u16(),
        ));
    }

    let metadata_content = response
        .text()
        .map_err(|e| GupError::network_error(registration_source, &e.into()))?;

    let project_metadata: crate::project_metadata_m::ProjectMetadata =
        serde_json::from_str(&metadata_content)
            .map_err(|e| GupError::invalid_metadata(registration_source, &e))?;

    let project_name = &project_metadata.project_name;

    // Check if project is already managed
    if is_project_managed(project_name, paths)? {
        eprintln!(
            "  Project '{}' is already managed by gup.",
            style(project_name).yellow()
        );
        return Ok(());
    }

    // Add project to global configuration
    add_managed_project(
        project_name,
        &project_metadata
            .display_name
            .unwrap_or_else(|| project_name.clone()),
        registration_source,
        paths,
    )?;

    // Create initial project state
    let initial_state = ProjectLocalState::default();
    save_project_local_state(project_name, &initial_state, paths)?;

    // Cache the metadata
    sync_project_metadata(project_name, &metadata_url, paths)?;

    eprintln!(
        "{} project '{}' from {}",
        style("Successfully added").green().bold(),
        project_name,
        registration_source
    );
    eprintln!(
        "  Use `gup add {} <version>` to install a version.",
        project_name
    );

    Ok(())
}

pub fn run_command_project_remove(project_name: &str, paths: &GupGlobalPaths) -> Result<()> {
    eprintln!(
        "{} project '{}'",
        style("Removing").red().bold(),
        project_name
    );

    // Check if project exists
    if !crate::global_config_manager::is_project_managed(project_name, paths)? {
        anyhow::bail!("Project '{}' is not managed by gup.", project_name);
    }

    // Remove from global configuration
    let removed = crate::global_config_manager::remove_managed_project(project_name, paths)?;
    if !removed {
        anyhow::bail!(
            "Failed to remove project '{}' from configuration.",
            project_name
        );
    }

    // Remove project directory
    let project_dir = paths.project_dir(project_name);
    if project_dir.exists() {
        std::fs::remove_dir_all(&project_dir).with_context(|| {
            format!(
                "Failed to remove project directory: {}",
                project_dir.display()
            )
        })?;
        eprintln!("  Removed project directory: {}", project_dir.display());
    }

    // TODO: Remove symlinks from ~/.gup/bin when multiplexer is implemented

    eprintln!(
        "{} project '{}'",
        style("Successfully removed").green().bold(),
        project_name
    );

    Ok(())
}

pub fn run_command_project_list(paths: &GupGlobalPaths) -> Result<()> {
    // Delegate to the general list command which handles listing all projects
    crate::command_list::run_command_list(None, paths)
}

pub fn run_command_project_update_metadata(
    project_name: &str,
    paths: &GupGlobalPaths,
) -> Result<()> {
    eprintln!(
        "{} metadata for project '{}'",
        style("Updating").cyan().bold(),
        project_name
    );

    // Load global config to get project's metadata source
    let global_config = load_global_config(paths)?;
    let project_info = global_config
        .managed_projects
        .get(project_name)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Project '{}' is not managed by gup. Use `gup project add` first.",
                project_name
            )
        })?;

    // Parse source URL and sync metadata
    let metadata_url = url::Url::parse(&project_info.source_of_truth_url).with_context(|| {
        format!(
            "Failed to parse metadata URL for project '{}': {}",
            project_name, project_info.source_of_truth_url
        )
    })?;

    sync_project_metadata(project_name, &metadata_url, paths)?;

    eprintln!(
        "{} metadata for project '{}'",
        style("Successfully updated").green().bold(),
        project_name
    );

    Ok(())
}
