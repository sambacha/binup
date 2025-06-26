use crate::error_handling::GupError;
use crate::global_config_manager::load_global_config;
use crate::global_paths::GupGlobalPaths;
use crate::operations_install::install_version;
use crate::operations_metadata::{resolve_channel_to_version, sync_project_metadata};
use crate::project_state_manager::{load_project_local_state, save_project_local_state};
use crate::state_config::ActiveChannelInfo;
use anyhow::{bail, Context, Result};
use chrono::Utc;
use console::style;

pub fn run_command_add(
    project_name: &str,
    version_or_channel_to_add: &str,
    paths: &GupGlobalPaths,
) -> Result<()> {
    eprintln!(
        "{} version/channel '{}' for project {}.",
        style("Adding").cyan().bold(),
        version_or_channel_to_add,
        project_name
    );

    // 1. Load GupGlobalConfig to find project's source_of_truth_url
    let global_config = load_global_config(paths)?;
    let project_info = global_config
        .managed_projects
        .get(project_name)
        .ok_or_else(|| GupError::project_not_found(project_name))?;

    // 2. Load/Sync ProjectMetadata for `project_name`
    let source_url = url::Url::parse(&project_info.source_of_truth_url).with_context(|| {
        format!(
            "Failed to parse source_of_truth_url: '{}'",
            project_info.source_of_truth_url
        )
    })?;
    let project_metadata = sync_project_metadata(project_name, &source_url, paths)?;

    // 3. Load ProjectLocalState for `project_name`
    let mut project_state = load_project_local_state(project_name, paths)?;

    // 4. Resolve version_or_channel_to_add to a concrete version string
    let concrete_version_to_install: String;
    let mut is_channel_installation = false;

    if project_metadata
        .channels
        .contains_key(version_or_channel_to_add)
    {
        is_channel_installation = true;
        concrete_version_to_install =
            resolve_channel_to_version(version_or_channel_to_add, &project_metadata)?;
        eprintln!(
            "  Channel '{}' resolved to version '{}'.",
            style(version_or_channel_to_add).green(),
            style(&concrete_version_to_install).green()
        );
    } else if project_metadata
        .available_versions
        .contains_key(version_or_channel_to_add)
    {
        concrete_version_to_install = version_or_channel_to_add.to_string();
    } else {
        bail!(
            "'{}' is not a valid version or channel for project '{}'.",
            version_or_channel_to_add,
            project_name
        );
    }

    // 5. Call operations_install::install_version (handles already installed check internally)
    install_version(
        project_name,
        &concrete_version_to_install,
        &project_metadata,
        &mut project_state,
        paths,
    )?;

    // 6. Update ProjectLocalState
    if is_channel_installation {
        project_state.active_channels.insert(
            version_or_channel_to_add.to_string(),
            ActiveChannelInfo {
                resolved_version: concrete_version_to_install.clone(),
                last_checked: Utc::now(),
            },
        );
    }

    // If no default is set, make this the default
    if project_state.default_version_or_channel_name.is_none() {
        eprintln!(
            "  No default version set for project '{}'. Setting '{}' as default.",
            project_name,
            style(version_or_channel_to_add).green()
        );
        project_state.default_version_or_channel_name = Some(version_or_channel_to_add.to_string());

        // Save state first, then update symlinks
        save_project_local_state(project_name, &project_state, paths)?;

        // Update symlinks for the new default
        use crate::operations_symlink::update_default_symlinks_for_project;
        update_default_symlinks_for_project(project_name, paths).with_context(|| {
            format!(
                "Failed to update default symlinks for project '{}'",
                project_name
            )
        })?;

        eprintln!(
            "{} version/channel '{}' for project {}.",
            style("Successfully added").green().bold(),
            version_or_channel_to_add,
            project_name
        );
        return Ok(());
    }

    // 7. Save ProjectLocalState
    save_project_local_state(project_name, &project_state, paths)?;

    eprintln!(
        "{} version/channel '{}' for project {}.",
        style("Successfully added").green().bold(),
        version_or_channel_to_add,
        project_name
    );

    Ok(())
}
