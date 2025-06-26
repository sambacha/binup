use crate::global_config_manager::load_global_config;
use crate::global_paths::GupGlobalPaths;
use crate::operations_metadata::sync_project_metadata;
use crate::operations_symlink::create_executable_symlink;
use crate::project_state_manager::{load_project_local_state, save_project_local_state};
use crate::state_config::LinkedProjectInfo; // This was missing in the previous attempt's SEARCH block
use anyhow::{bail, Context, Result};
use console::style;
use path_absolutize::Absolutize;
use std::path::Path;

pub fn run_command_link(
    project_name: &str,
    link_name: &str,
    file_path_str: &str,
    args: &[String],
    paths: &GupGlobalPaths,
) -> Result<()> {
    eprintln!(
        "{} link '{}' for project '{}' to '{}'.",
        style("Creating").cyan().bold(),
        link_name,
        project_name,
        file_path_str
    );

    // 1. Load GupGlobalConfig
    let global_config = load_global_config(paths)?;
    if !global_config.managed_projects.contains_key(project_name) {
        bail!(
            "Project '{}' is not registered. Use `gup project add` first.",
            project_name
        );
    }

    // 2. Load ProjectMetadata (to check for name collisions with existing channels/versions)
    let project_info = global_config.managed_projects.get(project_name).unwrap(); // Should be safe due to earlier check
    let source_url = url::Url::parse(&project_info.source_of_truth_url).with_context(|| {
        format!(
            "Failed to parse source_of_truth_url: '{}'",
            project_info.source_of_truth_url
        )
    })?;
    let project_metadata = sync_project_metadata(project_name, &source_url, paths)?;

    // 3. Load ProjectLocalState
    let mut project_state = load_project_local_state(project_name, paths)?;

    // 4. Check for collisions with existing link names, installed versions, or channels from metadata
    if project_state.linked_paths.contains_key(link_name) {
        bail!(
            "Link name '{}' is already used for project '{}'.",
            link_name,
            project_name
        );
    }
    if project_state.installed_versions.contains_key(link_name) {
        bail!(
            "Link name '{}' conflicts with an installed version for project '{}'.",
            link_name,
            project_name
        );
    }
    if project_metadata.channels.contains_key(link_name) {
        eprintln!(
            "{} The link name '{}' for project '{}' is also a defined channel in its metadata. This link will take precedence.",
            style("WARNING:").yellow().bold(),
            link_name,
            project_name
        );
    }
    if project_metadata.available_versions.contains_key(link_name) {
        eprintln!(
            "{} The link name '{}' for project '{}' is also an available version. This link will take precedence.",
            style("WARNING:").yellow().bold(),
            link_name,
            project_name
        );
    }

    // 5. Absolutize and validate file_path_str
    let absolute_file_path = Path::new(file_path_str)
        .absolutize()
        .with_context(|| {
            format!(
                "Failed to convert path '{}' to absolute path.",
                file_path_str
            )
        })?
        .to_path_buf(); // Convert Cow to PathBuf

    if !absolute_file_path.exists() {
        eprintln!(
            "{} The file at '{}' does not exist. The link '{}' for project '{}' might not work.",
            style("WARNING:").yellow().bold(),
            absolute_file_path.display(),
            link_name,
            project_name
        );
    }
    if !absolute_file_path.is_file() {
        eprintln!(
            "{} The path '{}' is not a file. The link '{}' for project '{}' might not work as expected.",
            style("WARNING:").yellow().bold(),
            absolute_file_path.display(),
            link_name,
            project_name
        );
    }

    // 6. Update ProjectLocalState
    project_state.linked_paths.insert(
        link_name.to_string(),
        LinkedProjectInfo {
            command_path: absolute_file_path.to_string_lossy().into_owned(),
            arguments: if args.is_empty() {
                None
            } else {
                Some(args.to_vec())
            },
        },
    );

    // 7. Save ProjectLocalState
    save_project_local_state(project_name, &project_state, paths)?;

    // 8. Create symlink in gup_bin_dir, pointing to the gup executable itself for multiplexing
    let gup_exe_path =
        std::env::current_exe().context("Failed to get current executable path for gup.")?;

    create_executable_symlink(link_name, &gup_exe_path, paths)?;

    eprintln!(
        "{} link '{}' for project '{}' to '{}'.",
        style("Successfully created").green().bold(),
        link_name,
        project_name,
        file_path_str
    );

    Ok(())
}
