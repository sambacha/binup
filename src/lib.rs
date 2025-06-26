//! # Gup - Generic Updater/Installer Program
//!
//! Gup is a configuration-driven version manager capable of managing arbitrary software projects.
//! It evolved from juliaup (a Julia-specific version manager) into a generic tool that can manage
//! any software project with appropriate metadata.
//!
//! ## Architecture Overview
//!
//! Gup follows a modular architecture with these key components:
//!
//! ### Core Data Structures
//! - [`state_config`]: Global configuration and per-project state management
//! - [`project_metadata_m`]: Project metadata format and version/channel definitions
//! - [`cli`]: Command-line interface definitions and parsing
//! - [`constants`]: Central constants for the application
//!
//! ### Operations Layer
//! - [`operations_metadata`]: Metadata synchronization and resolution
//! - [`operations_install`]: Version installation and management
//! - [`operations_download`]: Artifact downloading and extraction
//! - [`operations_symlink`]: Executable symlink/shim management
//!
//! ### Command Layer
//! - `command_*` modules: Individual CLI command implementations
//! - [`multiplexer`]: Handles execution when gup is invoked via symlinks
//!
//! ### Utility Layer
//! - [`global_paths`]: Cross-platform path management
//! - [`error_handling`]: Centralized error handling utilities
//! - [`utils`]: General utility functions
//!
//! ## Directory Structure
//!
//! Gup organizes its data in `~/.gup/`:
//! ```text
//! ~/.gup/
//! ├── global_config.json            # Global configuration
//! ├── bin/                          # Symlinks/shims for executables  
//! └── projects/                     # Per-project data
//!     └── <project_name>/
//!         ├── metadata_cache.json   # Cached project metadata
//!         ├── project_state.json    # Local project state
//!         └── versions/             # Installed versions
//!             └── <version_dir>/    # Individual version installations
//! ```
//!
//! ## Key Concepts
//!
//! - **Projects**: Software packages managed by gup (e.g., Julia, Node.js, Python)
//! - **Versions**: Specific releases of projects (e.g., "1.9.3", "18.0.0")
//! - **Channels**: Dynamic version selectors (e.g., "stable", "lts", "nightly")
//! - **Links**: User-defined aliases pointing to custom builds
//! - **Overrides**: Directory-specific version selections
//!
//! ## Usage Examples
//!
//! ```text
//! gup project add https://example.com/mytool.toml  # Add a new project
//! gup add mytool stable                            # Install stable channel
//! gup default mytool 1.2.3                        # Set default version
//! gup override set mytool nightly                  # Set directory override
//! gup link mytool dev /path/to/custom/build        # Link custom build
//! ```

use anyhow::Context;

pub mod cli;
pub mod command_add;
pub mod command_api;
pub mod command_completions;
pub mod command_config;
#[cfg(feature = "selfupdate")]
pub mod command_config_backgroundselfupdate;
pub mod command_config_modifypath;
#[cfg(feature = "selfupdate")]
pub mod command_config_startupselfupdate;
pub mod command_default;
pub mod command_gc;
pub mod command_info;
pub mod command_link;
pub mod command_list;
pub mod command_override;
pub mod command_project;
pub mod command_remove;
#[cfg(feature = "selfupdate")]
pub mod command_selfchannel;
pub mod command_selfuninstall;
pub mod command_selfupdate;
pub mod command_status;
pub mod command_update;
pub mod constants;

// pub mod config_file; // Removed - replaced by global_config_manager and project_state_manager
pub mod global_paths;
pub mod operations;
pub mod operations_download;
pub mod operations_gc;
pub mod operations_install;
pub mod operations_metadata;
pub mod operations_selfupdate;
pub mod operations_shell;
pub mod operations_symlink;
pub mod utils;

// New modules for gup
pub mod error_handling;
pub mod global_config_manager;
pub mod multiplexer;
pub mod project_metadata_m;
pub mod project_registration;
pub mod project_state_manager;
pub mod state_config;

include!(concat!(env!("OUT_DIR"), "/built.rs"));

/// Returns the current version of the gup binary itself.
///
/// This function parses the version string embedded at build time and returns
/// it as a semantic version object for version comparisons and display.
///
/// # Returns
///
/// * `Ok(semver::Version)` - The parsed version of the current gup binary
/// * `Err(anyhow::Error)` - If the embedded version string cannot be parsed
///
/// # Examples
///
/// ```
/// # use gup::get_own_version;
/// let version = get_own_version().expect("Valid version");
/// println!("Gup version: {}", version);
/// ```
pub fn get_own_version() -> anyhow::Result<semver::Version> {
    use semver::Version;

    let version =
        Version::parse(PKG_VERSION).with_context(|| "Failed to parse our own version.")?;

    Ok(version)
}
