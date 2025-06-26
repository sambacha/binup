use crate::global_config_manager::load_global_config;
use crate::global_paths::GupGlobalPaths;
use crate::operations_metadata::{resolve_channel_to_version, sync_project_metadata};
use crate::project_state_manager::load_project_local_state;
use crate::state_config::GupGlobalConfig; // Added import
use anyhow::{anyhow, bail, Context, Result};
use std::env;
use std::ffi::OsStr;
#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command; // For exec

// Helper function to find the project and executable to run.
// Order of precedence:
// 1. Directory Override for the invoked command name (or project's default executable if invoked_as is project name)
// 2. Linked command (if invoked_as matches a link_name)
// 3. Default version of a project (if invoked_as matches an executable from the default version)
fn determine_target_executable(
    invoked_as_osstr: &OsStr,
    current_dir: &Path,
    global_config: &GupGlobalConfig,
    paths: &GupGlobalPaths,
) -> Result<Option<(PathBuf, Vec<String>)>> {
    // (executable_path, combined_args)
    let invoked_as_str = invoked_as_osstr.to_string_lossy().into_owned();

    // --- 1. Check Directory Overrides ---
    // Iterate through parent directories of current_dir
    for ancestor_path in current_dir.ancestors() {
        for (project_name, _project_reg_info) in &global_config.managed_projects {
            if let Ok(state) = load_project_local_state(project_name, paths) {
                for dir_override in &state.overrides {
                    if dir_override.path == ancestor_path {
                        // Found an override for this directory and project.
                        // Now, check if `invoked_as_str` matches an executable for this override.
                        let metadata_url = url::Url::parse(&_project_reg_info.source_of_truth_url)?;
                        let metadata = sync_project_metadata(project_name, &metadata_url, paths)
                            .with_context(|| {
                                format!(
                                    "Failed to load metadata for override project '{}'",
                                    project_name
                                )
                            })?;

                        let concrete_version =
                            resolve_channel_to_version(&dir_override.version_or_channel, &metadata)
                                .with_context(|| {
                                    format!(
                                        "Failed to resolve override version '{}' for project '{}'",
                                        dir_override.version_or_channel, project_name
                                    )
                                })?;

                        // Check if the concrete_version is actually installed for this override to be valid
                        if let Some(installed_version_info) =
                            state.installed_versions.get(&concrete_version)
                        {
                            // Executables are defined at the project metadata level
                            for exec_detail_from_meta in &metadata.executables {
                                // Assuming ExecutableDetail has 'name' and 'path_in_bin_subdir' (or similar)
                                // and potentially 'aliases'
                                // For this example, let's assume ExecutableDetail has `name` and `path_in_bin_subdir`
                                // and we need to construct the full path.
                                // The `aliases` check would need `ExecutableDetail` to have an `aliases: Vec<String>` field.
                                // For now, just checking `exec_detail_from_meta.name`.
                                if exec_detail_from_meta.name == invoked_as_str {
                                    // Construct path: project_versions_dir / version_path_segment / bin_subdir / path_in_bin_subdir
                                    let base_install_path = paths
                                        .project_versions_dir(project_name)
                                        .join(&installed_version_info.path_segment);

                                    // Determine bin_subdir (from project default, version override, or artifact override)
                                    // This logic is simplified here; actual MergedInstallConfig should be used.
                                    // For now, assume project_metadata.default_install_config.bin_subdir
                                    let bin_subdir_path = metadata
                                        .default_install_config
                                        .bin_subdir
                                        .as_deref()
                                        .unwrap_or("");

                                    let full_exec_path = base_install_path
                                        .join(bin_subdir_path)
                                        .join(&exec_detail_from_meta.path_in_bin_subdir);

                                    eprintln!("[gup multiplexer] Directory override match: Project '{}', Version '{}', Executable '{}' at '{}'", project_name, concrete_version, exec_detail_from_meta.name, full_exec_path.display());
                                    return Ok(Some((full_exec_path, Vec::new())));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // --- 2. Check Linked Commands ---
    // A link name is global across all projects for simplicity of symlink creation in `gup_bin_dir`.
    // The `gup link` command should ideally check for global link name collisions.
    // Here, we assume `invoked_as_str` could be a link name.
    // We need to iterate through all projects to find if any has this link.
    for (project_name, _project_reg_info) in &global_config.managed_projects {
        if let Ok(state) = load_project_local_state(project_name, paths) {
            if let Some(link_info) = state.linked_paths.get(&invoked_as_str) {
                eprintln!(
                    "[gup multiplexer] Link match: Link '{}' for project '{}' to '{}'",
                    invoked_as_str, project_name, link_info.command_path
                );
                let target_path = PathBuf::from(&link_info.command_path);
                let prepended_args = link_info.arguments.clone().unwrap_or_default();
                return Ok(Some((target_path, prepended_args)));
            }
        }
    }

    // --- 3. Check Default Version of a Project ---
    // This part is more complex: how do we map `invoked_as_str` to a project if it's not an override or link?
    // Option A: `invoked_as_str` is `project_name`, run default executable.
    // Option B: `invoked_as_str` is `executable_name`, find which project's default provides it. (Can be ambiguous)
    // Option C: `invoked_as_str` is `project_name-executable_name`.

    // For now, let's try Option A: if `invoked_as_str` is a known project name.
    if let Some(project_reg_info) = global_config.managed_projects.get(&invoked_as_str) {
        let project_name = &invoked_as_str;
        eprintln!(
            "[gup multiplexer] Attempting to run default for project: {}",
            project_name
        );
        if let Ok(state) = load_project_local_state(project_name, paths) {
            if let Some(default_version_or_channel) = &state.default_version_or_channel_name {
                let metadata_url = url::Url::parse(&project_reg_info.source_of_truth_url)?;
                let metadata = sync_project_metadata(project_name, &metadata_url, paths)?;
                let concrete_version =
                    resolve_channel_to_version(default_version_or_channel, &metadata)?;

                if let Some(installed_version_info) =
                    state.installed_versions.get(&concrete_version)
                {
                    // Use metadata.default_executable_name or fallback
                    let exec_name_to_find = metadata
                        .default_executable_name
                        .as_ref()
                        .or_else(|| metadata.executables.first().map(|e| &e.name)) // Fallback to first defined executable
                        .cloned();

                    if let Some(exec_name_str) = exec_name_to_find {
                        if let Some(exec_detail_from_meta) = metadata
                            .executables
                            .iter()
                            .find(|e| e.name == exec_name_str)
                        {
                            let base_install_path = paths
                                .project_versions_dir(project_name)
                                .join(&installed_version_info.path_segment);
                            // Simplified bin_subdir logic for now
                            let bin_subdir_path = metadata
                                .default_install_config
                                .bin_subdir
                                .as_deref()
                                .unwrap_or("");
                            let full_exec_path = base_install_path
                                .join(bin_subdir_path)
                                .join(&exec_detail_from_meta.path_in_bin_subdir);

                            eprintln!("[gup multiplexer] Project default match: Project '{}', Version '{}', Executable '{}' at '{}'", project_name, concrete_version, exec_name_str, full_exec_path.display());
                            return Ok(Some((full_exec_path, Vec::new())));
                        }
                    }
                }
            }
        }
    }

    // Check if invoked_as_str matches any executable name across all projects' defaults
    for (project_name, project_reg_info) in &global_config.managed_projects {
        if let Ok(state) = load_project_local_state(project_name, paths) {
            if let Some(default_version_or_channel) = &state.default_version_or_channel_name {
                let metadata_url = url::Url::parse(&project_reg_info.source_of_truth_url)?;
                let metadata = sync_project_metadata(project_name, &metadata_url, paths)?;
                if let Ok(concrete_version) =
                    resolve_channel_to_version(default_version_or_channel, &metadata)
                {
                    if let Some(installed_version_info) =
                        state.installed_versions.get(&concrete_version)
                    {
                        // Check if any executable matches the invoked name
                        for exec_detail in &metadata.executables {
                            if exec_detail.name == invoked_as_str {
                                let base_install_path = paths
                                    .project_versions_dir(project_name)
                                    .join(&installed_version_info.path_segment);
                                let bin_subdir_path = metadata
                                    .default_install_config
                                    .bin_subdir
                                    .as_deref()
                                    .unwrap_or("");
                                let full_exec_path = base_install_path
                                    .join(bin_subdir_path)
                                    .join(&exec_detail.path_in_bin_subdir);

                                eprintln!("[gup multiplexer] Executable name match: Found '{}' in project '{}', version '{}' at '{}'", invoked_as_str, project_name, concrete_version, full_exec_path.display());
                                return Ok(Some((full_exec_path, Vec::new())));
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(None) // No target found
}

pub fn run_multiplexed_command(
    invoked_as: &OsStr,
    args: &[String],
    paths: &GupGlobalPaths,
) -> Result<i32> {
    eprintln!(
        "[gup multiplexer] Invoked as: {:?}, Args: {:?}",
        invoked_as, args
    );

    let current_dir = env::current_dir().context("Failed to get current directory")?;
    let global_config = load_global_config(paths)?;

    match determine_target_executable(invoked_as, &current_dir, &global_config, paths)? {
        Some((target_exe_path, prepended_args)) => {
            if !target_exe_path.exists() {
                bail!(
                    "Multiplexer resolved to target executable '{}' but it does not exist.",
                    target_exe_path.display()
                );
            }

            let mut command = Command::new(&target_exe_path);
            command.args(&prepended_args); // Add args from link definition first
            command.args(args); // Then add args passed to the symlink

            eprintln!(
                "[gup multiplexer] Executing: {:?} with args {:?}",
                target_exe_path,
                command.get_args().collect::<Vec<_>>()
            );

            #[cfg(unix)]
            {
                // execvp behavior: uses PATH if command is not absolute,
                // but here target_exe_path should always be absolute.
                let err = command.exec(); // Replaces gup process
                                          // If exec returns, it's an error
                return Err(anyhow!(
                    "exec of '{}' failed: {}",
                    target_exe_path.display(),
                    err
                ));
            }

            #[cfg(not(unix))]
            {
                let mut child = command.spawn().with_context(|| {
                    format!(
                        "Failed to spawn target command '{}'",
                        target_exe_path.display()
                    )
                })?;
                let status = child.wait().with_context(|| {
                    format!(
                        "Failed to wait for target command '{}'",
                        target_exe_path.display()
                    )
                })?;
                return Ok(status.code().unwrap_or(1));
            }
        }
        None => {
            bail!(
                "gup multiplexer could not determine what to run for invocation '{:?}'.\n\
                 Possible reasons:\n\
                 - No directory override is active for the current path.\n\
                 - '{:?}' is not a gup-managed link name.\n\
                 - '{:?}' is not a registered project name with a runnable default.\n\
                 - The target executable within the resolved project version could not be found.",
                invoked_as,
                invoked_as,
                invoked_as
            );
        }
    }
}
