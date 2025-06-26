use crate::global_config_manager::{load_global_config, save_global_config};
use crate::global_paths::GupGlobalPaths;
use crate::operations_gc::garbage_collect_versions;
use crate::operations_install::install_version;
use crate::operations_metadata::{resolve_channel_to_version, sync_project_metadata};
use crate::project_state_manager::{load_project_local_state, save_project_local_state};
use crate::state_config::ActiveChannelInfo; // Added this line
use anyhow::{anyhow, bail, Context, Result};
use chrono::Utc;
use console::style;

pub fn run_command_update(
    project_name_opt: Option<&str>,
    channel_to_update_opt: Option<&str>,
    paths: &GupGlobalPaths,
) -> Result<()> {
    if let Some(project_name) = project_name_opt {
        eprintln!(
            "{} project '{}' (channel: {:?}).",
            style("Updating").cyan().bold(),
            project_name,
            channel_to_update_opt
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

        // 2. Sync ProjectMetadata
        let source_url = url::Url::parse(&project_info.source_of_truth_url).with_context(|| {
            format!(
                "Failed to parse source_of_truth_url: '{}'",
                project_info.source_of_truth_url
            )
        })?;
        let project_metadata = sync_project_metadata(project_name, &source_url, paths)?;

        // 3. Load ProjectLocalState
        let mut project_state = load_project_local_state(project_name, paths)?;

        if let Some(channel_name) = channel_to_update_opt {
            // Update a specific channel for this project
            if !project_metadata.channels.contains_key(channel_name)
                && !project_metadata
                    .available_versions
                    .contains_key(channel_name)
            {
                bail!(
                    "Channel or version '{}' not found for project '{}'.",
                    channel_name,
                    project_name
                );
            }

            let concrete_version = resolve_channel_to_version(channel_name, &project_metadata)?;
            let current_resolved_opt = project_state
                .active_channels
                .get(channel_name)
                .map(|ac| &ac.resolved_version);

            if current_resolved_opt != Some(&concrete_version)
                || !project_state
                    .installed_versions
                    .contains_key(&concrete_version)
            {
                eprintln!(
                    "  Updating channel '{}' for project '{}' to version '{}'.",
                    style(channel_name).green(),
                    project_name,
                    style(&concrete_version).green()
                );
                install_version(
                    project_name,
                    &concrete_version,
                    &project_metadata,
                    &mut project_state,
                    paths,
                )?;
                project_state.active_channels.insert(
                    channel_name.to_string(),
                    ActiveChannelInfo {
                        resolved_version: concrete_version.clone(),
                        last_checked: Utc::now(),
                    },
                );
            } else {
                eprintln!(
                    "  Channel '{}' for project '{}' is already up to date (version {}).",
                    style(channel_name).green(),
                    project_name,
                    style(&concrete_version).green()
                );
            }
        } else {
            // Update all active channels for this project
            eprintln!(
                "  {} all active channels for project '{}'.",
                style("Updating").cyan(),
                project_name
            );
            // Use a HashSet to ensure each channel is processed once, even if it's active and default.
            let mut channels_to_process_set = project_state
                .active_channels
                .keys()
                .cloned()
                .collect::<std::collections::HashSet<String>>();

            if let Some(default_channel_or_version) = &project_state.default_version_or_channel_name
            {
                if project_metadata
                    .channels
                    .contains_key(default_channel_or_version)
                {
                    channels_to_process_set.insert(default_channel_or_version.clone());
                }
            }

            let mut channels_to_process_vec: Vec<String> =
                channels_to_process_set.into_iter().collect();
            channels_to_process_vec.sort_unstable(); // For deterministic order of operations

            for channel_name in channels_to_process_vec {
                let concrete_version =
                    resolve_channel_to_version(&channel_name, &project_metadata)?;
                let current_resolved_opt = project_state
                    .active_channels
                    .get(&channel_name)
                    .map(|ac| &ac.resolved_version);

                if current_resolved_opt != Some(&concrete_version)
                    || !project_state
                        .installed_versions
                        .contains_key(&concrete_version)
                {
                    eprintln!(
                        "  Updating channel '{}' for project '{}' to version '{}'.",
                        style(&channel_name).green(),
                        project_name,
                        style(&concrete_version).green()
                    );
                    install_version(
                        project_name,
                        &concrete_version,
                        &project_metadata,
                        &mut project_state,
                        paths,
                    )?;
                    project_state.active_channels.insert(
                        channel_name.clone(),
                        ActiveChannelInfo {
                            resolved_version: concrete_version.clone(),
                            last_checked: Utc::now(),
                        },
                    );
                } else {
                    eprintln!(
                        "  Channel '{}' for project '{}' is already up to date (version {}).",
                        style(&channel_name).green(),
                        project_name,
                        style(&concrete_version).green()
                    );
                }
            }
        }

        // Run garbage collection after updates
        garbage_collect_versions(
            project_name,
            false,
            &project_metadata,
            &mut project_state,
            paths,
        )?;
        save_project_local_state(project_name, &project_state, paths)?;

        eprintln!(
            "{} project '{}'.",
            style("Successfully updated").green().bold(),
            project_name
        );
    } else {
        // Update all projects (if channel_to_update_opt is None)
        if channel_to_update_opt.is_some() {
            bail!("Cannot specify a channel to update without specifying a project.");
        }

        eprintln!("{}", style("Updating all managed projects.").cyan().bold());
        let global_config = load_global_config(paths)?; // Removed mut
        let project_names: Vec<String> = global_config.managed_projects.keys().cloned().collect();

        for p_name in project_names {
            if let Err(e) = run_command_update(Some(&p_name), None, paths) {
                eprintln!(
                    "{} to update project '{}': {}",
                    style("Warning: Failed").yellow().bold(),
                    p_name,
                    e
                );
            }
        }
        save_global_config(&global_config, paths)?; // Save global config if any changes were made (e.g., last sync time)

        eprintln!(
            "{}",
            style("Completed updating all managed projects.")
                .green()
                .bold()
        );
    }

    Ok(())
}
