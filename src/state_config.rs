//! # State and Configuration Data Structures
//!
//! This module defines the core data structures used to store gup's global configuration
//! and per-project state information. These structures are serialized to JSON files
//! on disk for persistence.
//!
//! ## File Mapping
//!
//! - [`GupGlobalConfig`] → `~/.gup/global_config.json`
//! - [`ProjectLocalState`] → `~/.gup/projects/<project>/project_state.json`
//!
//! ## Key Concepts
//!
//! - **Managed Projects**: Projects that gup knows about and can install/manage
//! - **Installed Versions**: Concrete versions that are physically present on disk
//! - **Active Channels**: Dynamic version selectors that resolve to specific versions
//! - **Linked Paths**: User-defined aliases pointing to custom executables
//! - **Directory Overrides**: Per-directory version preferences
//!
//! ## Example Usage
//!
//! ```rust
//! use gup::state_config::{GupGlobalConfig, ProjectLocalState};
//! use std::collections::HashMap;
//!
//! // Create a new global config
//! let mut config = GupGlobalConfig::default();
//! config.self_update_channel = Some("stable".to_string());
//!
//! // Create project state
//! let mut state = ProjectLocalState::default();
//! state.default_version_or_channel_name = Some("1.0.0".to_string());
//! ```

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Information about a project that gup is managing
///
/// This struct contains metadata about projects that have been added to gup's management.
/// It tracks the project's identity, where to get updates, and user preferences.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ManagedProjectInfo {
    /// Unique identifier for the project (e.g., "nodejs", "python")
    pub unique_name: String,
    /// Human-readable name for display (e.g., "Node.js", "Python")
    pub display_name: String,
    /// URL where the project's metadata can be downloaded
    pub source_of_truth_url: String,
    /// When the metadata was last synchronized from the source
    pub last_metadata_sync: Option<DateTime<Utc>>,
    /// User's preferred default channel/version for this project
    pub preferred_default: Option<String>,
}

/// Global configuration for the gup installation
///
/// This structure contains settings that apply to gup itself and tracks
/// all projects that gup is managing. It is persisted as `global_config.json`.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct GupGlobalConfig {
    /// Update channel for gup itself (e.g., "stable", "beta", "dev")
    pub self_update_channel: Option<String>,
    /// How often to check for gup updates in the background (in minutes)
    pub background_self_update_interval_minutes: Option<u64>,
    /// How often to check for gup updates on startup (in minutes)
    pub startup_self_update_interval_minutes: Option<u64>,
    /// Whether to automatically modify PATH in shell scripts when installing
    pub modify_path_on_install: Option<bool>,
    /// When the last self-update check was performed
    pub last_self_update: Option<DateTime<Utc>>,
    /// Registry of all projects that gup is managing
    ///
    /// Key: project unique name (e.g., "nodejs"), Value: project metadata
    pub managed_projects: HashMap<String, ManagedProjectInfo>,
}

/// Information about a specific version that has been installed
///
/// Tracks concrete versions that are physically present on the filesystem,
/// including where they are stored and when they were installed.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InstalledVersionConcreteInfo {
    /// The concrete version string (e.g., "1.0.1", "18.14.0")
    pub version_string: String,
    /// Directory name under `~/.gup/projects/<project_name>/versions/`
    ///
    /// Examples: "mytool-1.0.1-win-x64", "1.0.1", "node-18.14.0-linux-x64"
    pub path_segment: String,
    /// Timestamp when this version was installed
    pub installed_at: DateTime<Utc>,
}

/// Information about a channel that the user is actively tracking
///
/// Channels are dynamic version selectors (like "stable", "lts", "nightly")
/// that resolve to specific concrete versions. This tracks the current resolution.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ActiveChannelInfo {
    /// The concrete version string this channel currently resolves to
    ///
    /// For example, the "stable" channel might resolve to "1.2.3"
    pub resolved_version: String,
    /// When this channel resolution was last checked/updated
    pub last_checked: DateTime<Utc>,
}

/// Information about a user-defined link to a custom executable
///
/// Links allow users to create custom aliases that point to arbitrary executables,
/// useful for development builds or alternative implementations.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LinkedProjectInfo {
    /// Absolute path to the custom executable or script
    pub command_path: String,
    /// Optional arguments to pass to the linked command when executed
    pub arguments: Option<Vec<String>>,
}

/// Local state for a specific project managed by gup
///
/// This structure tracks everything gup knows about a project's local installation:
/// installed versions, active channels, custom links, and directory overrides.
/// Each project has its own state file at `~/.gup/projects/<project>/project_state.json`.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ProjectLocalState {
    /// User's chosen default version or channel name for this project
    ///
    /// Can be either a concrete version ("1.2.3") or a channel name ("stable")
    pub default_version_or_channel_name: Option<String>,

    /// Versions that are physically installed on disk for this project
    ///
    /// Key: concrete version string (e.g., "1.0.1")
    /// Value: installation details and metadata
    pub installed_versions: HashMap<String, InstalledVersionConcreteInfo>,

    /// Channels the user is actively tracking
    ///
    /// Tracks channels the user has added or defaulted to, along with their
    /// last known resolved version. Used by `gup update <project>`.
    /// Key: channel name (e.g., "stable"), Value: current resolution info
    pub active_channels: HashMap<String, ActiveChannelInfo>,

    /// User-defined links to custom executables
    ///
    /// For linking to local builds or alternative implementations.
    /// Key: custom link name (e.g., "dev"), Value: executable info
    pub linked_paths: HashMap<String, LinkedProjectInfo>,

    /// Directory-specific version overrides for this project
    pub overrides: Vec<DirectoryOverride>,
}

/// A directory-specific version override
///
/// Allows different directories to use different versions of the same project.
/// Useful when working on multiple projects with different version requirements.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DirectoryOverride {
    /// Absolute, canonicalized path to the directory
    pub path: PathBuf,
    /// The version or channel name to use for this project in this directory
    pub version_or_channel: String,
}
