use crate::global_config_manager::load_global_config;
use crate::global_paths::GupGlobalPaths;
use crate::operations_gc::garbage_collect_versions;
use crate::operations_metadata::{resolve_channel_to_version, sync_project_metadata};
use crate::operations_symlink::remove_executable_symlink;
use crate::project_state_manager::{load_project_local_state, save_project_local_state};
use anyhow::{anyhow, bail, Context, Result};
use console::style;
use std::fs;

pub fn run_command_remove(
    project_name: &str,
    version_or_link_name_to_remove: &str,
    paths: &GupGlobalPaths,
) -> Result<()> {
    eprintln!(
        "{} '{}' from project '{}'.",
        style("Removing").cyan().bold(),
        version_or_link_name_to_remove,
        project_name
    );

    // 1. Load GupGlobalConfig
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

    // 2. Load ProjectMetadata (needed for channel resolution and context)
    let source_url = url::Url::parse(&project_info.source_of_truth_url).with_context(|| {
        format!(
            "Failed to parse source_of_truth_url: '{}'",
            project_info.source_of_truth_url
        )
    })?;
    let project_metadata = sync_project_metadata(project_name, &source_url, paths)?;

    // 3. Load ProjectLocalState
    let mut project_state = load_project_local_state(project_name, paths)?;

    // 4. Determine if it's an installed version or a linked path
    if project_state
        .linked_paths
        .contains_key(version_or_link_name_to_remove)
    {
        // It's a linked path
        project_state
            .linked_paths
            .remove(version_or_link_name_to_remove);

        // Remove its symlink from gup_bin_dir
        remove_executable_symlink(version_or_link_name_to_remove, paths)?;

        eprintln!(
            "{} link '{}' for project '{}'.",
            style("Successfully removed").green().bold(),
            version_or_link_name_to_remove,
            project_name
        );
    } else if let Some(installed_version_info) = project_state
        .installed_versions
        .get(version_or_link_name_to_remove)
    {
        // It's an installed version
        // Check if it's the default
        if project_state.default_version_or_channel_name.as_deref()
            == Some(version_or_link_name_to_remove)
        {
            bail!(
                "Version '{}' for project '{}' cannot be removed because it is currently configured as the default. \
                Set a different default first using `gup default {} <other_version_or_channel>`.",
                version_or_link_name_to_remove, project_name, project_name
            );
        }

        // Check if any active channels resolve to this version
        for (channel_name, active_channel_info) in &project_state.active_channels {
            if active_channel_info.resolved_version == version_or_link_name_to_remove.to_string() {
                bail!(
                    "Version '{}' for project '{}' cannot be removed because it is currently used by active channel '{}'. \
                    Remove the channel first or switch it to another version.",
                    version_or_link_name_to_remove, project_name, channel_name
                );
            }
        }

        // Check directory overrides
        for override_info in &project_state.overrides {
            if let Ok(resolved_override_version) =
                resolve_channel_to_version(&override_info.version_or_channel, &project_metadata)
            {
                if resolved_override_version == version_or_link_name_to_remove.to_string() {
                    bail!(
                        "Version '{}' for project '{}' cannot be removed because it is currently used in a directory override at '{}'. \
                        Unset the override first using `gup override unset {} {}`.",
                        version_or_link_name_to_remove, project_name, override_info.path.display(), project_name, override_info.path.display()
                    );
                }
            }
        }

        let version_install_path = paths
            .project_versions_dir(project_name)
            .join(&installed_version_info.path_segment);

        if version_install_path.exists() {
            eprintln!(
                "  Removing installed version directory: {}",
                style(version_install_path.display()).yellow()
            );
            if let Err(e) = fs::remove_dir_all(&version_install_path) {
                eprintln!(
                    "{} Failed to delete directory {}. You can try to delete at a later point by running `gup gc {}`.\nError: {}",
                    style("WARNING:").yellow().bold(),
                    version_install_path.display(),
                    project_name,
                    e
                );
            }
        }
        project_state
            .installed_versions
            .remove(version_or_link_name_to_remove);
        eprintln!(
            "{} version '{}' for project '{}'.",
            style("Successfully removed").green().bold(),
            version_or_link_name_to_remove,
            project_name
        );
    } else {
        bail!(
            "'{}' is not an installed version or link for project '{}'.",
            version_or_link_name_to_remove,
            project_name
        );
    }

    // 5. Run garbage collection for the project (good practice after removing a version)
    garbage_collect_versions(
        project_name,
        false,
        &project_metadata,
        &mut project_state,
        paths,
    )?;

    // 6. Save ProjectLocalState
    save_project_local_state(project_name, &project_state, paths)?;

    Ok(())
}
