use crate::global_config_manager::load_global_config;
use crate::global_paths::GupGlobalPaths;
use crate::operations_metadata::{resolve_channel_to_version, sync_project_metadata};
use crate::project_state_manager::load_project_local_state;

use anyhow::{anyhow, Context, Result}; // Removed bail
use cli_table::{
    format::{Border, Justify, Separator},
    print_stdout, Table, WithTitle,
}; // ColorChoice removed
use human_sort::compare;
use itertools::Itertools;
use semver::Version;

#[derive(Table)]
struct AllProjectsStatusRow {
    #[table(title = "Project Name")]
    name: String,
    #[table(title = "Default")]
    default: String,
    #[table(title = "Installed")]
    installed_count: usize,
}

#[derive(Table)]
struct ProjectStatusRow {
    #[table(title = " ", justify = "Justify::Right")]
    is_default_marker: String,
    #[table(title = "Name")]
    name: String,
    #[table(title = "Type")]
    item_type: String,
    #[table(title = "Details")]
    details: String,
    #[table(title = "Update")]
    update_available: String,
}

pub fn run_command_status(project_name_opt: Option<&str>, paths: &GupGlobalPaths) -> Result<()> {
    let border = Border::builder().build();
    let separator = Separator::builder().build();

    if let Some(project_name) = project_name_opt {
        // Display status for a specific project
        let global_config = load_global_config(paths)
            .with_context(|| "Failed to load global gup configuration.")?;
        let project_reg_info = global_config
            .managed_projects
            .get(project_name)
            .ok_or_else(|| anyhow!("Project '{}' is not managed by gup.", project_name))?;

        let metadata_url =
            url::Url::parse(&project_reg_info.source_of_truth_url).with_context(|| {
                format!(
                    "Failed to parse metadata URL for project '{}'",
                    project_name
                )
            })?;
        let metadata = sync_project_metadata(project_name, &metadata_url, paths)
            .with_context(|| format!("Failed to load metadata for project '{}'", project_name))?;
        let state = load_project_local_state(project_name, paths).with_context(|| {
            format!("Failed to load local state for project '{}'", project_name)
        })?;

        let mut rows: Vec<ProjectStatusRow> = Vec::new();
        let default_target_name = state.default_version_or_channel_name.as_deref();

        // Add installed versions
        for (version_str, installed_info) in state
            .installed_versions
            .iter()
            .sorted_by(|a, b| compare(a.0, b.0))
        {
            let is_default = default_target_name == Some(version_str);
            let mut update_msg = "-".to_string();

            // Check if this installed version is managed by an active channel
            let is_managed_by_channel = state
                .active_channels
                .values()
                .any(|ci| ci.resolved_version == *version_str);

            if !is_managed_by_channel {
                // Only check for direct updates if not managed by a channel
                if let Ok(current_semver) = Version::parse(version_str) {
                    let newer_available = metadata
                        .available_versions
                        .keys()
                        .filter_map(|v_str| Version::parse(v_str).ok())
                        .any(|av_semver| {
                            av_semver > current_semver
                                && av_semver.pre.is_empty() == current_semver.pre.is_empty()
                        }); // Basic check, could be more sophisticated
                    if newer_available {
                        update_msg = "Yes (direct)".to_string();
                    }
                }
            }

            rows.push(ProjectStatusRow {
                is_default_marker: if is_default { "*" } else { " " }.to_string(),
                name: version_str.clone(),
                item_type: "Version".to_string(),
                details: format!(
                    "Installed: {}",
                    installed_info.installed_at.format("%Y-%m-%d")
                ),
                update_available: update_msg,
            });
        }

        // Add active channels
        for (channel_name, active_info) in state
            .active_channels
            .iter()
            .sorted_by(|a, b| compare(a.0, b.0))
        {
            let is_default = default_target_name == Some(channel_name);
            let metadata_channel_info = metadata.channels.get(channel_name);
            let points_to = metadata_channel_info.map_or("N/A".to_string(), |ci| {
                if let Some(prefix) = &ci.version_prefix {
                    format!("{} (Prefix: {})", ci.resolution_strategy, prefix)
                } else {
                    ci.resolution_strategy.clone()
                }
            });

            let mut update_msg = "-".to_string();
            if let Ok(latest_resolved_version) = resolve_channel_to_version(channel_name, &metadata)
            {
                if latest_resolved_version != active_info.resolved_version {
                    update_msg = format!("Yes (to {})", latest_resolved_version);
                }
            }

            rows.push(ProjectStatusRow {
                is_default_marker: if is_default { "*" } else { " " }.to_string(),
                name: channel_name.clone(),
                item_type: "Channel".to_string(),
                details: format!(
                    "Tracks: {}, Currently: {}",
                    points_to, active_info.resolved_version
                ),
                update_available: update_msg,
            });
        }

        // Add linked paths
        for (link_name, link_info) in state
            .linked_paths
            .iter()
            .sorted_by(|a, b| compare(a.0, b.0))
        {
            let is_default = default_target_name == Some(link_name);
            rows.push(ProjectStatusRow {
                is_default_marker: if is_default { "*" } else { " " }.to_string(),
                name: link_name.clone(),
                item_type: "Link".to_string(),
                details: format!(
                    "-> {} {}",
                    link_info.command_path,
                    link_info.arguments.as_deref().unwrap_or_default().join(" ")
                ),
                update_available: "N/A".to_string(),
            });
        }

        if rows.is_empty() {
            println!(
                "No versions, channels, or links configured for project '{}'.",
                project_name
            );
        } else {
            print_stdout(rows.with_title().border(border).separator(separator))?;
        }
    } else {
        // Display status for all managed projects
        let global_config = load_global_config(paths)
            .with_context(|| "Failed to load global gup configuration.")?;

        if global_config.managed_projects.is_empty() {
            println!("No projects are currently managed by gup.");
            println!("Use `gup project add <registration_file_or_url>` to add one.");
            return Ok(());
        }

        let mut project_rows = Vec::new();
        for (name, _managed_info) in global_config
            .managed_projects
            .iter()
            .sorted_by(|a, b| compare(a.0, b.0))
        {
            // Try to load state, but don't fail if it's missing (e.g., project added but no versions yet)
            let state = load_project_local_state(name, paths).ok();
            project_rows.push(AllProjectsStatusRow {
                name: name.clone(),
                default: state
                    .as_ref()
                    .and_then(|s| s.default_version_or_channel_name.clone())
                    .unwrap_or_else(|| "-".to_string()),
                installed_count: state.map_or(0, |s| s.installed_versions.len()),
            });
        }
        print_stdout(
            project_rows
                .with_title()
                .border(border)
                .separator(separator),
        )?;
    }
    Ok(())
}
