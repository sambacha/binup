//! # Global Path Management
//!
//! This module provides centralized path management for gup's directory structure
//! and file locations. It handles cross-platform differences and ensures consistent
//! file organization across all operations.
//!
//! ## Directory Structure
//!
//! ```text
//! ~/.gup/                           # Root gup directory
//! ├── global_config.json            # Global configuration
//! ├── .gup-lock                     # Global lock file
//! ├── bin/                          # Symlinks/shims for executables
//! └── projects/                     # Per-project data
//!     └── <project_name>/
//!         ├── metadata_cache.json   # Cached project metadata  
//!         ├── project_state.json    # Local project state
//!         └── versions/             # Installed versions
//!             └── <version_dir>/    # Individual version installations
//! ```
//!
//! ## Platform Differences
//!
//! The paths are automatically adjusted for different platforms:
//! - **Unix/Linux/macOS**: Uses `~/.gup/`
//! - **Windows**: Uses appropriate Windows directories following conventions

#[cfg(feature = "selfupdate")]
use anyhow::Context;
use anyhow::{anyhow, bail, Result};
use std::path::PathBuf;

/// Global path manager for gup's directory structure
///
/// Provides a centralized way to access all important paths and directories
/// used by gup, ensuring consistency across the application.
pub struct GupGlobalPaths {
    /// Root directory for gup (e.g., ~/.gup)
    guphome: PathBuf,
    #[cfg(feature = "selfupdate")]
    /// Directory for gup's own installation files when self-updating
    gupselfhome: PathBuf,
    #[cfg(feature = "selfupdate")]
    /// Binary directory for gup's own installation
    gupselfbin: PathBuf,
}

impl GupGlobalPaths {
    /// Returns the root gup directory (e.g., `~/.gup`)
    ///
    /// This is the base directory where all gup data is stored.
    pub fn guphome(&self) -> &PathBuf {
        &self.guphome
    }

    /// Returns the global configuration file path
    ///
    /// Returns the path to gup's global configuration file (e.g., `~/.gup/global_config.json`)
    /// which contains settings and the registry of managed projects.
    pub fn global_config_file(&self) -> PathBuf {
        self.guphome.join("global_config.json")
    }

    /// Returns the global lock file path
    ///
    /// Returns the path to gup's global lock file (e.g., `~/.gup/.gup-lock`)
    /// used to prevent concurrent operations that could corrupt state.
    pub fn lock_file(&self) -> PathBuf {
        self.guphome.join(".gup-lock")
    }

    /// Returns the directory where executable symlinks/shims are placed
    ///
    /// Returns the path to gup's bin directory (e.g., `~/.gup/bin`) where
    /// symlinks or shims to managed executables are created. This directory
    /// should be added to the user's PATH.
    pub fn gup_bin_dir(&self) -> PathBuf {
        self.guphome.join("bin")
    }

    /// Returns the base directory for a specific managed project
    ///
    /// # Arguments
    /// * `project_unique_name` - The unique identifier for the project
    ///
    /// # Returns
    /// Path to the project's directory (e.g., `~/.gup/projects/nodejs`)
    pub fn project_dir(&self, project_unique_name: &str) -> PathBuf {
        self.guphome.join("projects").join(project_unique_name)
    }

    /// Returns the path to a project's cached metadata file
    ///
    /// # Arguments
    /// * `project_unique_name` - The unique identifier for the project
    ///
    /// # Returns
    /// Path to the project's metadata cache (e.g., `~/.gup/projects/nodejs/metadata_cache.json`)
    pub fn project_metadata_cache_file(&self, project_unique_name: &str) -> PathBuf {
        self.project_dir(project_unique_name)
            .join("metadata_cache.json")
    }

    /// Returns the path to a project's local state file
    ///
    /// # Arguments
    /// * `project_unique_name` - The unique identifier for the project
    ///
    /// # Returns
    /// Path to the project's state file (e.g., `~/.gup/projects/nodejs/project_state.json`)
    pub fn project_state_file(&self, project_unique_name: &str) -> PathBuf {
        self.project_dir(project_unique_name)
            .join("project_state.json")
    }

    // Returns the directory where different versions of a project are installed, e.g. ~/.gup/projects/<project_name>/versions
    pub fn project_versions_dir(&self, project_unique_name: &str) -> PathBuf {
        self.project_dir(project_unique_name).join("versions")
    }

    // Returns the specific directory for an installed version of a project,
    // e.g. ~/.gup/projects/<project_name>/versions/<version_path_segment>
    pub fn specific_project_version_dir(
        &self,
        project_unique_name: &str,
        version_path_segment: &str,
    ) -> PathBuf {
        self.project_versions_dir(project_unique_name)
            .join(version_path_segment)
    }

    #[cfg(feature = "selfupdate")]
    pub fn gupselfhome(&self) -> &PathBuf {
        &self.gupselfhome
    }

    #[cfg(feature = "selfupdate")]
    pub fn gupselfbin(&self) -> &PathBuf {
        &self.gupselfbin
    }

    #[cfg(feature = "selfupdate")]
    pub fn gupselfconfig_file(&self) -> PathBuf {
        self.gupselfhome.join("gupself.json")
    }
}

// Renamed from get_juliaup_home_path to get_gup_home_path
fn get_gup_home_path() -> Result<PathBuf> {
    // Changed environment variable from JULIAUP_DEPOT_PATH to GUP_DEPOT_PATH
    match std::env::var("GUP_DEPOT_PATH") {
        Ok(val) => {
            let val = val.trim();

            if val.is_empty() {
                return get_default_gup_home_path();
            } else {
                let path = PathBuf::from(val);

                if !path.is_absolute() {
                    // Updated error message for GUP_DEPOT_PATH
                    return Err(anyhow!("The current value of '{}' for the environment variable GUP_DEPOT_PATH is not an absolute path.", val));
                } else {
                    // The GUP_DEPOT_PATH itself is the parent, gup data goes into a 'gup' subdir
                    return Ok(PathBuf::from(val).join("gup"));
                }
            }
        }
        Err(_) => return get_default_gup_home_path(),
    }
}

// Renamed from get_default_juliaup_home_path to get_default_gup_home_path
// Returns ~/.gup, if such a directory can be found
fn get_default_gup_home_path() -> Result<PathBuf> {
    let home_dir = std::env::var("HOME")
        .ok()
        .map(PathBuf::from)
        .or_else(|| user_dirs::home_dir().ok())
        .ok_or_else(|| anyhow!("Could not determine the path of the user home directory."))?;

    let path = home_dir.join(".gup"); // Changed from .julia/juliaup to .gup

    if !path.is_absolute() {
        bail!(
            "The system returned an invalid home directory path `{}`.",
            path.display()
        );
    };
    Ok(path)
}

// Renamed from get_paths to get_gup_paths
pub fn get_gup_paths() -> Result<GupGlobalPaths> {
    let guphome = get_gup_home_path()?;

    #[cfg(feature = "selfupdate")]
    let my_own_path = std::env::current_exe()
        .with_context(|| "Could not determine the path of the running exe.")?;

    #[cfg(feature = "selfupdate")]
    let gupselfbin = my_own_path
        .parent()
        .ok_or_else(|| anyhow!("Could not determine parent directory of the running exe."))?
        .to_path_buf();

    #[cfg(feature = "selfupdate")]
    let gupselfhome = gupselfbin // Assuming self bin is inside self home, e.g. .../gup/self/bin
        .parent()
        .ok_or_else(|| anyhow!("Failed to get parent path of self bin directory."))?
        .to_path_buf();

    // The old global versiondb is removed. Project-specific metadata is cached elsewhere.
    // The global config file is now global_config.json.
    // The lock file is now .gup-lock.

    Ok(GupGlobalPaths {
        guphome,
        #[cfg(feature = "selfupdate")]
        gupselfhome,
        #[cfg(feature = "selfupdate")]
        gupselfbin,
    })
}
