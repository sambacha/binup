use std::{env::current_dir, path::PathBuf};

use crate::global_config_manager::load_global_config;
use crate::global_paths::GupGlobalPaths;
use crate::operations_metadata::{resolve_channel_to_version, sync_project_metadata};
use crate::project_state_manager::{load_project_local_state, save_project_local_state};
use crate::state_config::DirectoryOverride;
use anyhow::{anyhow, bail, Context, Result};
use cli_table::{
    format::{Border, Separator},
    print_stdout, ColorChoice, Table, WithTitle,
};
use console::style;
use itertools::Itertools;
use path_absolutize::Absolutize;

#[derive(Table)]
struct OverrideRow {
    #[table(title = "Path")]
    path: String,
    #[table(title = "Version/Channel")]
    version_or_channel: String,
}

pub fn run_command_override_status(project_name: &str, paths: &GupGlobalPaths) -> Result<()> {
    eprintln!(
        "{} override status for project '{}'.",
        style("Checking").cyan().bold(),
        project_name
    );

    // 1. Load GupGlobalConfig to ensure project is managed
    let global_config = load_global_config(paths)?;
    if !global_config.managed_projects.contains_key(project_name) {
        bail!(
            "Project '{}' is not registered. Use `gup project add` first.",
            project_name
        );
    }

    // 2. Load ProjectLocalState for project_name
    let project_state = load_project_local_state(project_name, paths)?;

    // 3. Populate and print table from state.overrides
    let rows_in_table: Vec<_> = project_state
        .overrides
        .iter()
        .sorted_by_key(|ov| ov.path.clone())
        .map(|ov| OverrideRow {
            path: ov.path.to_string_lossy().into_owned(),
            version_or_channel: ov.version_or_channel.clone(),
        })
        .collect();

    if rows_in_table.is_empty() {
        eprintln!(
            "{} No directory overrides found for project '{}'.",
            style("Info:").bold(),
            project_name
        );
    } else {
        print_stdout(
            rows_in_table // Removed .table() here, as rows_in_table is Vec<OverrideRow> which derives Table
                .with_title()
                .color_choice(ColorChoice::Auto)
                .border(Border::builder().build())
                .separator(Separator::builder().build()),
        )?;
    }

    Ok(())
}

pub fn run_command_override_set(
    project_name: &str,
    version_or_channel: &str,
    path_opt: Option<String>,
    paths: &GupGlobalPaths,
) -> Result<()> {
    eprintln!(
        "{} override for project '{}' to '{}' for path '{:?}'.",
        style("Setting").cyan().bold(),
        project_name,
        version_or_channel,
        path_opt
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

    // 2. Load ProjectMetadata (needed to validate channel/version)
    let source_url = url::Url::parse(&project_info.source_of_truth_url).with_context(|| {
        format!(
            "Failed to parse source_of_truth_url: '{}'",
            project_info.source_of_truth_url
        )
    })?;
    let project_metadata = sync_project_metadata(project_name, &source_url, paths)?;

    // 3. Load ProjectLocalState
    let mut project_state = load_project_local_state(project_name, paths)?;

    // 4. Validate version_or_channel
    let concrete_version = resolve_channel_to_version(version_or_channel, &project_metadata)
        .with_context(|| {
            format!(
                "'{}' is not a valid version or channel for project '{}'.",
                version_or_channel, project_name
            )
        })?;

    // Check if the concrete version is installed
    if !project_state
        .installed_versions
        .contains_key(&concrete_version)
    {
        bail!(
            "Version '{}' (resolved from '{}') for project '{}' is not installed. Install it first using `gup add {} {}`.",
            concrete_version,
            version_or_channel,
            project_name,
            project_name,
            version_or_channel // Use the original input for the add command
        );
    }

    // 5. Resolve path_opt to an absolute, canonicalized path (defaulting to current_dir).
    let target_path = match path_opt {
        Some(p_str) => PathBuf::from(p_str),
        None => current_dir()?,
    }
    .absolutize()
    .with_context(|| format!("Failed to convert path to absolute path."))?
    .to_path_buf();

    // 6. Add or update the override in project_state.overrides.
    let new_override = DirectoryOverride {
        path: target_path.clone(),
        version_or_channel: version_or_channel.to_string(),
    };

    let mut found_existing = false;
    for existing_ov in project_state.overrides.iter_mut() {
        if existing_ov.path == target_path {
            existing_ov.version_or_channel = version_or_channel.to_string();
            found_existing = true;
            break;
        }
    }
    if !found_existing {
        project_state.overrides.push(new_override);
    }

    // 7. Save ProjectLocalState.
    save_project_local_state(project_name, &project_state, paths)?;

    eprintln!(
        "{} override for project '{}' to '{}' for path '{}'.",
        style("Successfully set").green().bold(),
        project_name,
        version_or_channel,
        target_path.display()
    );

    Ok(())
}

pub fn run_command_override_unset(
    project_name: &str,
    nonexistent: bool,
    path_opt: Option<String>,
    paths: &GupGlobalPaths,
) -> Result<()> {
    eprintln!(
        "{} override for project '{}' (path: '{:?}', nonexistent: {}).",
        style("Unsetting").cyan().bold(),
        project_name,
        path_opt,
        nonexistent
    );

    // 1. Load GupGlobalConfig
    let global_config = load_global_config(paths)?;
    if !global_config.managed_projects.contains_key(project_name) {
        bail!(
            "Project '{}' is not registered. Use `gup project add` first.",
            project_name
        );
    }

    // 2. Load ProjectLocalState for project_name.
    let mut project_state = load_project_local_state(project_name, paths)?;
    let initial_override_count = project_state.overrides.len();

    if nonexistent {
        // Remove overrides pointing to non-existent paths
        project_state.overrides.retain(|ov| ov.path.exists());
        eprintln!("  Removed overrides pointing to non-existent paths.");
    } else {
        // Unset a specific path
        let target_path = match path_opt {
            Some(p_str) => PathBuf::from(p_str),
            None => current_dir()?,
        }
        .absolutize()
        .with_context(|| format!("Failed to convert path to absolute path."))?
        .to_path_buf();

        project_state.overrides.retain(|ov| ov.path != target_path);
        eprintln!("  Removed override for path '{}'.", target_path.display());
    }

    // 3. Save ProjectLocalState.
    if project_state.overrides.len() < initial_override_count {
        save_project_local_state(project_name, &project_state, paths)?;
        eprintln!(
            "{} override for project '{}'.",
            style("Successfully unset").green().bold(),
            project_name
        );
    } else {
        eprintln!(
            "{} No matching override found for project '{}' to unset.",
            style("Info:").bold(),
            project_name
        );
    }

    Ok(())
}
