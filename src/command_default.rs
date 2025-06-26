use crate::global_config_manager::load_global_config;
use crate::global_paths::GupGlobalPaths;
use crate::operations_metadata::{resolve_channel_to_version, sync_project_metadata};
use crate::project_state_manager::{load_project_local_state, save_project_local_state};
use anyhow::{anyhow, bail, Context, Result};
use console::style;

pub fn run_command_default(
    project_name: &str,
    version_or_channel_to_set_as_default: &str,
    paths: &GupGlobalPaths,
) -> Result<()> {
    eprintln!(
        "{} default for project '{}' to '{}'.",
        style("Setting").cyan().bold(),
        project_name,
        version_or_channel_to_set_as_default
    );

    // 1. Load GupGlobalConfig to check if project_name is managed
    let global_config = load_global_config(paths)?;
    let project_info = global_config
        .managed_projects
        .get(project_name)
        .ok_or_else(|| {
            anyhow!(
                "Project '{}' is not registered. Use `gup project add` first.",
                project_name
            )
        })?;

    // 2. Load/Sync ProjectMetadata for `project_name` (needed to validate channel/version)
    let source_url = url::Url::parse(&project_info.source_of_truth_url).with_context(|| {
        format!(
            "Failed to parse source_of_truth_url: '{}'",
            project_info.source_of_truth_url
        )
    })?;
    let project_metadata = sync_project_metadata(project_name, &source_url, paths)?;

    // 3. Load ProjectLocalState for `project_name`
    let mut project_state = load_project_local_state(project_name, paths)?;

    // 4. Validate `version_or_channel_to_set_as_default`:
    let concrete_version: String;
    let is_channel = project_metadata
        .channels
        .contains_key(version_or_channel_to_set_as_default);

    if is_channel {
        concrete_version =
            resolve_channel_to_version(version_or_channel_to_set_as_default, &project_metadata)
                .with_context(|| {
                    format!(
                        "Failed to resolve channel '{}' for project '{}'.",
                        version_or_channel_to_set_as_default, project_name
                    )
                })?;
    } else if project_metadata
        .available_versions
        .contains_key(version_or_channel_to_set_as_default)
    {
        concrete_version = version_or_channel_to_set_as_default.to_string();
    } else {
        bail!(
            "'{}' is not a valid version or channel for project '{}'.",
            version_or_channel_to_set_as_default,
            project_name
        );
    }

    // Check if the concrete version is installed
    if !project_state
        .installed_versions
        .contains_key(&concrete_version)
    {
        bail!(
            "Version '{}' (resolved from '{}') for project '{}' is not installed. Install it first using `gup add {} {}`.",
            concrete_version,
            version_or_channel_to_set_as_default,
            project_name,
            project_name,
            version_or_channel_to_set_as_default // Use the original input for the add command
        );
    }

    // 5. Update ProjectLocalState
    project_state.default_version_or_channel_name =
        Some(version_or_channel_to_set_as_default.to_string());

    // 6. Save ProjectLocalState
    save_project_local_state(project_name, &project_state, paths)?;

    eprintln!(
        "{} default for project '{}' to '{}'.",
        style("Successfully set").green().bold(),
        project_name,
        version_or_channel_to_set_as_default
    );

    Ok(())
}
