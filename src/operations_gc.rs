use crate::global_paths::GupGlobalPaths;
use crate::operations_metadata::resolve_channel_to_version;
use crate::operations_symlink::remove_executable_symlink;
use crate::project_metadata_m::ProjectMetadata;
use crate::project_state_manager::save_project_local_state;
use crate::state_config::ProjectLocalState;
use anyhow::{Context, Result};
use console::style;
use std::collections::HashSet; // Removed HashMap
use std::fs;
use std::path::PathBuf;

pub fn garbage_collect_versions(
    project_unique_name: &str,
    prune_linked_paths: bool,
    project_metadata: &ProjectMetadata,
    project_state: &mut ProjectLocalState,
    paths: &GupGlobalPaths,
) -> Result<()> {
    eprintln!(
        "{} for project {}.",
        style("Garbage collecting versions").cyan().bold(),
        project_unique_name
    );

    let mut active_versions: HashSet<String> = HashSet::new();

    // Add default version if set
    if let Some(default_channel_or_version) = &project_state.default_version_or_channel_name {
        if let Ok(resolved_version) =
            resolve_channel_to_version(default_channel_or_version, project_metadata)
        {
            active_versions.insert(resolved_version);
        }
    }

    // Add versions from active channels
    for (_channel_name, active_channel_info) in &project_state.active_channels {
        active_versions.insert(active_channel_info.resolved_version.clone());
    }

    // Add versions from overrides
    for override_info in &project_state.overrides {
        if let Ok(resolved_version) =
            resolve_channel_to_version(&override_info.version_or_channel, project_metadata)
        {
            active_versions.insert(resolved_version);
        }
    }

    // Identify versions to remove
    let mut versions_to_remove = Vec::new();
    for (version_string, installed_info) in &project_state.installed_versions {
        if !active_versions.contains(version_string) {
            versions_to_remove.push(installed_info.path_segment.clone());
        }
    }

    // Remove identified versions
    for path_segment in &versions_to_remove {
        let version_dir = paths
            .project_versions_dir(project_unique_name)
            .join(path_segment);
        if version_dir.exists() {
            eprintln!(
                "  Removing unused version directory: {}",
                style(version_dir.display()).yellow()
            );
            fs::remove_dir_all(&version_dir).with_context(|| {
                format!(
                    "Failed to remove unused version directory '{}'.",
                    version_dir.display()
                )
            })?;
        }
        // Remove from installed_versions map
        project_state
            .installed_versions
            .retain(|_, info| info.path_segment != *path_segment);
    }

    // Prune linked paths if requested
    if prune_linked_paths {
        eprintln!(
            "{} linked paths for project {}.",
            style("Pruning").cyan().bold(),
            project_unique_name
        );
        let mut symlinks_to_remove = Vec::new();
        project_state.linked_paths.retain(|link_name, linked_info| {
            let command_path = PathBuf::from(&linked_info.command_path);
            if !command_path.exists() {
                eprintln!(
                    "  Removing dangling linked path '{}' (points to non-existent '{}').",
                    style(link_name).yellow(),
                    command_path.display()
                );
                symlinks_to_remove.push(link_name.clone());
                false // Retain = false means remove this entry
            } else {
                true // Retain = true means keep this entry
            }
        });

        for symlink_name in symlinks_to_remove {
            remove_executable_symlink(&symlink_name, paths)?;
        }
    }

    save_project_local_state(project_unique_name, project_state, paths)?;

    eprintln!(
        "{} garbage collection for project {}.",
        style("Successfully completed").green().bold(),
        project_unique_name
    );

    Ok(())
}
