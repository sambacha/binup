use crate::global_paths::GupGlobalPaths;
use anyhow::{Context, Result};
use console::style;
#[cfg(not(windows))]
use std::os::unix::fs as unix_fs;
use std::path::Path;
use std::path::PathBuf; // Alias for clarity

// _remove_symlink remains largely the same conceptually but will be called by the new remove_symlink.
// Made pub(crate) as it's an internal helper for this module.
pub(crate) fn _remove_symlink(symlink_path: &Path) -> Result<Option<PathBuf>> {
    if symlink_path.exists() {
        // Check existence before attempting to read/remove link
        // Ensure parent directory exists before attempting to remove file,
        // though typically it should if symlink_path is valid.
        if let Some(parent) = symlink_path.parent() {
            std::fs::create_dir_all(parent).with_context(|| {
                format!(
                    "Failed to create parent directory for symlink at {}",
                    parent.display()
                )
            })?;
        }

        let prev_target = if symlink_path.is_symlink() {
            std::fs::read_link(&symlink_path).ok() // Ok if it's not a symlink, or error reading
        } else {
            None
        };
        std::fs::remove_file(symlink_path)
            .with_context(|| format!("Failed to remove symlink at {}", symlink_path.display()))?;
        return Ok(prev_target);
    }

    Ok(None)
}

// Renamed from remove_symlink to remove_executable_symlink for clarity
pub fn remove_executable_symlink(
    executable_name: &str, // e.g., "mytool"
    paths: &GupGlobalPaths,
) -> Result<()> {
    // On Windows, we create .bat files, on Unix we create symlinks
    #[cfg(windows)]
    let symlink_path = paths.gup_bin_dir().join(format!("{}.bat", executable_name));
    #[cfg(not(windows))]
    let symlink_path = paths.gup_bin_dir().join(executable_name);

    eprintln!(
        "{} symlink for {}.",
        style("Deleting").cyan().bold(),
        executable_name
    );

    _remove_symlink(&symlink_path)?;

    Ok(())
}

// This function will create a symlink in `~/.gup/bin/executable_name` that points to the main `gup` binary.
// The `gup` binary will then act as a multiplexer.
#[cfg(not(windows))]
pub fn create_executable_symlink(
    executable_name: &str,      // The desired command name, e.g., "mytool"
    gup_executable_path: &Path, // Path to the main `gup` executable itself
    paths: &GupGlobalPaths,
) -> Result<()> {
    let symlink_folder = paths.gup_bin_dir();
    std::fs::create_dir_all(&symlink_folder).with_context(|| {
        format!(
            "Failed to create gup bin directory at {}",
            symlink_folder.display()
        )
    })?;

    let symlink_path = symlink_folder.join(executable_name);

    let updating = _remove_symlink(&symlink_path).with_context(|| {
        format!(
            "Failed to remove existing symlink at {}",
            symlink_path.display()
        )
    })?;

    if updating.is_some() {
        eprintln!(
            "{} symlink for {} to point to gup multiplexer.",
            style("Updating").cyan().bold(),
            executable_name
        );
    } else {
        eprintln!(
            "{} symlink for {} to point to gup multiplexer.",
            style("Creating").cyan().bold(),
            executable_name
        );
    }

    unix_fs::symlink(&gup_executable_path, &symlink_path).with_context(|| {
        format!(
            "Failed to create symlink from {} to {}.",
            gup_executable_path.display(),
            symlink_path.display()
        )
    })?;

    if updating.is_none() {
        if let Ok(path_env) = std::env::var("PATH") {
            if !path_env.split(':').any(|p| Path::new(p) == symlink_folder) {
                eprintln!(
                "Symlink {} added in {}. Add this directory to the system PATH to make the command available in your shell.",
                executable_name, symlink_folder.display(),
            );
            }
        }
    }

    Ok(())
}

#[cfg(windows)]
pub fn create_executable_symlink(
    executable_name: &str,
    gup_executable_path: &Path,
    paths: &GupGlobalPaths,
) -> Result<()> {
    use std::fs;

    // Create a .bat shim file that redirects to the gup multiplexer
    let shim_path = paths.gup_bin_dir().join(format!("{}.bat", executable_name));

    // Ensure the bin directory exists
    let bin_dir = paths.gup_bin_dir();
    fs::create_dir_all(&bin_dir).with_context(|| {
        format!(
            "Failed to create gup bin directory at '{}'",
            bin_dir.display()
        )
    })?;

    // Create batch file content that passes all arguments to gup
    // Use %* to pass all command line arguments
    let shim_content = format!("@echo off\n\"{}\" %*\n", gup_executable_path.display());

    fs::write(&shim_path, shim_content).with_context(|| {
        format!(
            "Failed to create Windows batch shim '{}' for executable '{}'",
            shim_path.display(),
            executable_name
        )
    })?;

    eprintln!(
        "Created Windows batch shim '{}' -> '{}'",
        shim_path.display(),
        gup_executable_path.display()
    );

    Ok(())
}

/// Updates symlinks for the default executable(s) of a project when the default version changes
pub fn update_default_symlinks_for_project(
    project_name: &str,
    paths: &GupGlobalPaths,
) -> Result<()> {
    use crate::global_config_manager::load_global_config;
    use crate::operations_metadata::sync_project_metadata;
    use crate::project_state_manager::load_project_local_state;

    // Load project configuration
    let global_config = load_global_config(paths)?;
    let project_info = global_config
        .managed_projects
        .get(project_name)
        .ok_or_else(|| anyhow::anyhow!("Project '{}' not found in global config", project_name))?;

    // Load project state to get default version
    let project_state = load_project_local_state(project_name, paths)?;
    let default_version = project_state
        .default_version_or_channel_name
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("No default version set for project '{}'", project_name))?;

    // Load project metadata to get executable information
    let source_url = url::Url::parse(&project_info.source_of_truth_url)
        .with_context(|| format!("Failed to parse source URL for project '{}'", project_name))?;
    let project_metadata = sync_project_metadata(project_name, &source_url, paths)?;

    // Get the gup executable path for symlink target
    let gup_executable_path =
        std::env::current_exe().context("Failed to get current executable path")?;

    eprintln!(
        "{} default symlinks for project '{}' (default: {})",
        style("Updating").cyan().bold(),
        project_name,
        style(default_version).green()
    );

    // Create symlinks for all executables defined in the project metadata
    for executable in &project_metadata.executables {
        let executable_name = if project_metadata.executables.len() == 1
            && project_metadata.default_executable_name.as_ref() == Some(&executable.name)
        {
            // For single-executable projects where this is the default, use the project name as the command
            project_name.to_string()
        } else {
            // For multi-executable projects or non-default executables, prefix with project name
            format!("{}_{}", project_name, executable.name)
        };

        // Create or update the symlink
        create_executable_symlink(&executable_name, &gup_executable_path, paths).with_context(
            || {
                format!(
                    "Failed to create symlink for executable '{}'",
                    executable_name
                )
            },
        )?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    #[test]
    fn test_symlink_path_generation() {
        use std::env;

        // Test that Windows generates .bat files and Unix generates regular files
        let temp_dir = TempDir::new().unwrap();

        // Set environment variable to use temp directory
        let original_env = env::var("GUP_DEPOT_PATH").ok();
        env::set_var("GUP_DEPOT_PATH", temp_dir.path());

        let paths = crate::global_paths::get_gup_paths().unwrap();

        // Test executable name logic in remove function
        #[cfg(windows)]
        let expected_path = paths.gup_bin_dir().join("test-exe.bat");
        #[cfg(not(windows))]
        let expected_path = paths.gup_bin_dir().join("test-exe");

        // This tests the path generation logic used in remove_executable_symlink
        #[cfg(windows)]
        let actual_path = paths.gup_bin_dir().join(format!("{}.bat", "test-exe"));
        #[cfg(not(windows))]
        let actual_path = paths.gup_bin_dir().join("test-exe");

        assert_eq!(expected_path, actual_path);

        // Restore environment
        match original_env {
            Some(val) => env::set_var("GUP_DEPOT_PATH", val),
            None => env::remove_var("GUP_DEPOT_PATH"),
        }
    }
}
