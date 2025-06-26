use crate::global_config_manager::load_global_config;
use crate::global_paths::GupGlobalPaths;
use crate::operations_metadata::{resolve_channel_to_version, sync_project_metadata};
use crate::project_metadata_m::ChannelDetail;
use crate::project_state_manager::load_project_local_state;
// GupGlobalConfig and ProjectLocalState types are not directly used, but their instances are.
// ProjectMetadata type is used.

use anyhow::{Context, Result}; // Removed bail
use cli_table::{
    format::{Border, Separator}, // Removed HorizontalLine
    print_stdout,
    Table,
    WithTitle,
};
use human_sort::compare;
use itertools::Itertools;
use std::collections::HashSet;

#[derive(Table)]
struct ManagedProjectRow {
    #[table(title = "Project Name")]
    name: String,
    #[table(title = "Display Name")]
    display_name: String,
    #[table(title = "Source URL")]
    source_url: String,
}

#[derive(Table)]
struct ProjectChannelRow {
    #[table(title = "Channel")]
    name: String,
    #[table(title = "Details")]
    version_info: String,
    #[table(title = "Resolves To")]
    resolved_version: String,
    #[table(title = "Status")]
    status: String,
}

#[derive(Table)]
struct ProjectVersionRow {
    #[table(title = "Version")]
    version: String,
    #[table(title = "Status")]
    status: String,
}

#[derive(Table)]
struct ProjectLinkRow {
    #[table(title = "Link Name")]
    link_name: String,
    #[table(title = "Target Path")]
    target_path: String,
    #[table(title = "Arguments")]
    arguments: String,
}

pub fn run_command_list(project_name_opt: Option<&str>, paths: &GupGlobalPaths) -> Result<()> {
    let border = Border::builder().build();
    let separator = Separator::builder().build();

    if let Some(project_name) = project_name_opt {
        // List details for a specific project
        let global_config = load_global_config(paths)
            .with_context(|| "Failed to load global gup configuration.")?;

        let project_reg_info = global_config
            .managed_projects
            .get(project_name)
            .ok_or_else(|| anyhow::anyhow!("Project '{}' is not managed by gup.", project_name))?;

        let metadata_url =
            url::Url::parse(&project_reg_info.source_of_truth_url).with_context(|| {
                format!(
                    "Failed to parse metadata URL for project '{}': {}",
                    project_name, project_reg_info.source_of_truth_url
                )
            })?;
        let metadata = sync_project_metadata(project_name, &metadata_url, paths)
            .with_context(|| format!("Failed to load metadata for project '{}'.", project_name))?;
        let state = load_project_local_state(project_name, paths).with_context(|| {
            format!("Failed to load local state for project '{}'.", project_name)
        })?;

        println!("Details for project: {}", metadata.project_name);
        println!(
            "Description: {}",
            metadata.description.as_deref().unwrap_or("N/A")
        );
        println!(
            "Homepage: {}",
            metadata.homepage_url.as_deref().unwrap_or("N/A")
        );
        if let Some(default_name) = &state.default_version_or_channel_name {
            println!("Default: {}", default_name);
        } else {
            println!("Default: (not set)");
        }
        println!("---");

        // Channels
        if !metadata.channels.is_empty() {
            let mut channel_rows: Vec<ProjectChannelRow> = Vec::new();
            for (name, detail) in metadata
                .channels
                .iter()
                .sorted_by(|(a, _), (b, _)| compare(a, b))
            {
                let resolved_version = resolve_channel_to_version(name, &metadata)
                    .unwrap_or_else(|e| format!("Error: {}", e));

                let mut status_parts = Vec::new();
                if state.default_version_or_channel_name.as_deref() == Some(name) {
                    status_parts.push("Default");
                }
                if state.installed_versions.contains_key(&resolved_version) {
                    status_parts.push("Installed");
                } else if metadata.available_versions.contains_key(&resolved_version) {
                    status_parts.push("Available");
                }

                channel_rows.push(ProjectChannelRow {
                    name: name.clone(),
                    version_info: format_channel_detail(detail),
                    resolved_version,
                    status: if status_parts.is_empty() {
                        "-".to_string()
                    } else {
                        status_parts.join(", ")
                    },
                });
            }
            print_stdout(
                channel_rows
                    .with_title()
                    .border(border.clone())
                    .separator(separator.clone()),
            )?;
            println!("---");
        }

        // Versions (Installed and Available)
        let mut all_versions_set: HashSet<String> = HashSet::new();
        all_versions_set.extend(metadata.available_versions.keys().cloned());
        all_versions_set.extend(state.installed_versions.keys().cloned());

        if !all_versions_set.is_empty() {
            let mut version_rows: Vec<ProjectVersionRow> = Vec::new();
            for version_str in all_versions_set.iter().sorted_by(|a, b| compare(a, b)) {
                let mut status_parts = Vec::new();
                let is_default_version_itself =
                    state.default_version_or_channel_name.as_deref() == Some(version_str.as_str());
                let is_default_channel_resolves_to_this =
                    if let Some(default_channel_name) = &state.default_version_or_channel_name {
                        if metadata.channels.contains_key(default_channel_name) {
                            resolve_channel_to_version(default_channel_name, &metadata)
                                .map_or(false, |rv| rv == *version_str)
                        } else {
                            false
                        }
                    } else {
                        false
                    };

                if is_default_version_itself || is_default_channel_resolves_to_this {
                    status_parts.push("Default");
                }
                if state.installed_versions.contains_key(version_str) {
                    status_parts.push("Installed");
                } else if metadata.available_versions.contains_key(version_str) {
                    status_parts.push("Available");
                }

                version_rows.push(ProjectVersionRow {
                    version: version_str.clone(),
                    status: if status_parts.is_empty() {
                        "-".to_string()
                    } else {
                        status_parts.join(", ")
                    },
                });
            }
            print_stdout(
                version_rows
                    .with_title()
                    .border(border.clone())
                    .separator(separator.clone()),
            )?;
            println!("---");
        }

        // Links
        if !state.linked_paths.is_empty() {
            let link_rows: Vec<_> = state
                .linked_paths
                .iter()
                .map(|(name, info)| ProjectLinkRow {
                    link_name: name.clone(),
                    target_path: info.command_path.clone(),
                    arguments: info
                        .arguments
                        .as_deref()
                        .map_or_else(String::new, |args_slice| args_slice.join(" ")),
                })
                .sorted_by(|a, b| compare(&a.link_name, &b.link_name))
                .collect();
            print_stdout(
                link_rows
                    .with_title()
                    .border(border.clone())
                    .separator(separator.clone()),
            )?;
        }
    } else {
        // List all managed projects
        let global_config = load_global_config(paths)
            .with_context(|| "Failed to load global gup configuration.")?;

        if global_config.managed_projects.is_empty() {
            println!("No projects are currently managed by gup.");
            println!("Use `gup project add <registration_file_or_url>` to add one.");
            return Ok(());
        }

        let project_rows: Vec<_> = global_config
            .managed_projects
            .values()
            .map(|p_info| ManagedProjectRow {
                name: p_info.unique_name.clone(),
                display_name: p_info.display_name.clone(),
                source_url: p_info.source_of_truth_url.clone(),
            })
            .sorted_by(|a, b| compare(&a.name, &b.name))
            .collect();
        print_stdout(
            project_rows
                .with_title()
                .border(border)
                .separator(separator),
        )?;
    }

    Ok(())
}

fn format_channel_detail(detail: &ChannelDetail) -> String {
    match detail.resolution_strategy.as_str() {
        "latest_semver" => {
            if let Some(prefix) = &detail.version_prefix {
                format!("Strategy: latest_semver, Prefix: {}", prefix)
            } else {
                "Strategy: latest_semver (no prefix)".to_string()
            }
        }
        "exact_match" => {
            format!(
                "Strategy: exact_match, Version: {}",
                detail.version_prefix.as_deref().unwrap_or("N/A")
            )
        }
        _ => format!("Strategy: {}", detail.resolution_strategy),
    }
}
