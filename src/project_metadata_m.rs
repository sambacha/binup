//! # Project Metadata Format
//!
//! This module defines the data structures for project metadata files that describe
//! how to download, install, and manage versions of software projects. These metadata
//! files are the core of gup's configuration-driven approach.
//!
//! ## Overview
//!
//! Each software project managed by gup is described by a [`ProjectMetadata`] structure
//! that includes:
//! - Available versions and their download artifacts
//! - Platform-specific installation instructions  
//! - Channel definitions for dynamic version resolution
//! - Executable information for creating symlinks
//!
//! ## Metadata Structure
//!
//! ```text
//! ProjectMetadata
//! ├── Basic Info (name, description, homepage)
//! ├── Download Configuration (base URLs, install config)
//! ├── Platform Definitions (OS/arch combinations)
//! ├── Available Versions (concrete releases)
//! │   └── Per-Version Artifacts (platform-specific downloads)
//! ├── Channels (dynamic version selectors)
//! └── Executables (commands provided by the project)
//! ```
//!
//! ## Example Metadata File
//!
//! ```toml
//! project_name = "nodejs"
//! display_name = "Node.js"
//! description = "JavaScript runtime"
//! homepage_url = "https://nodejs.org"
//! base_download_url = "https://nodejs.org/dist"
//!
//! [platforms.linux-x64]
//! os = "linux"
//! arch = "x86_64"
//! target_triple_pattern = "x86_64-unknown-linux-gnu"
//!
//! [available_versions."18.14.0"]
//! release_date = "2023-02-16"
//!
//! [available_versions."18.14.0".artifacts.linux-x64]
//! url_path_suffix = "/v18.14.0/node-v18.14.0-linux-x64.tar.xz"
//! sha256 = "abc123..."
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
// Consider adding: use chrono::{DateTime, Utc}; if strict date parsing is needed.

/// Installation configuration for a project or specific version
///
/// Defines how to extract and install downloaded artifacts, including
/// archive format handling and post-installation steps.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct InstallConfig {
    /// Archive format of downloaded artifacts (e.g., "zip", "tar.gz", "tar.xz", "binary")
    pub archive_format: Option<String>,
    /// Number of leading path components to strip when extracting tarballs
    pub strip_components: Option<usize>,
    /// Subdirectory within the extracted archive that contains executables
    pub bin_subdir: Option<String>,
    /// Shell command to run after successful installation (e.g., "chmod +x bin/*")
    pub post_install_hook: Option<String>,
}

/// Platform-specific information for matching system compatibility
///
/// Defines the operating system and architecture requirements for artifacts,
/// along with patterns for matching against the system's target triple.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlatformDetail {
    /// Operating system name (e.g., "linux", "windows", "macos")
    pub os: String,
    /// CPU architecture (e.g., "x86_64", "aarch64", "i686")
    pub arch: String,
    /// Pattern for matching against Rust target triples (e.g., "x86_64-unknown-linux-gnu")
    pub target_triple_pattern: String,
}

/// Information about an executable provided by the project
///
/// Describes commands that should be made available to users through symlinks or shims.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ExecutableDetail {
    /// Command name that will be available to users (e.g., "node", "npm")
    pub name: String,
    /// Relative path from bin_subdir to the actual executable file
    pub path_in_bin_subdir: String,
}

/// Download artifact information for a specific platform and version
///
/// Contains the download URL, checksums, and platform-specific installation overrides.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ArtifactDetail {
    /// URL path to append to base_download_url for this artifact
    pub url_path_suffix: String,
    /// SHA-256 checksum for download verification (optional but recommended)
    pub sha256: Option<String>,
    /// Override the project's default archive format for this specific artifact
    pub archive_format: Option<String>,
    /// Override the project's default strip_components for this artifact
    pub strip_components: Option<usize>,
    /// Override the project's default bin_subdir for this artifact
    pub bin_subdir: Option<String>,
}

/// Detailed information about a specific version of a project
///
/// Contains release metadata and platform-specific download artifacts.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VersionDetail {
    /// Release date in ISO 8601 format (e.g., "2023-01-15T10:00:00Z")
    pub release_date: Option<String>,
    /// Platform-specific download artifacts for this version
    ///
    /// Key: platform_id (e.g., "win-x64", "linux-arm64")
    /// Value: artifact download and installation details
    pub artifacts: HashMap<String, ArtifactDetail>,
    /// Override the project's default installation configuration for this version
    pub install_config_override: Option<InstallConfig>,
}

/// Configuration for a dynamic version channel
///
/// Channels provide a way to automatically select versions based on patterns or strategies.
/// Examples include "stable" (latest stable release), "lts" (long-term support), etc.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ChannelDetail {
    /// Version prefix filter (e.g., "1.0." for 1.0.x releases, "*-beta" for beta versions)
    pub version_prefix: Option<String>,
    /// Strategy for resolving this channel to a concrete version
    ///
    /// Examples: "latest_semver", "latest_by_date", "exact_match"
    pub resolution_strategy: String,
}

use url::Url; // Added import

/// Complete metadata description for a software project
///
/// This is the main structure that describes everything gup needs to know about
/// a project: how to download it, install it, and manage its versions.
///
/// # Examples
///
/// ```rust
/// use gup::project_metadata_m::{ProjectMetadata, InstallConfig};
/// use url::Url;
/// use std::collections::HashMap;
///
/// let metadata = ProjectMetadata {
///     project_name: "nodejs".to_string(),
///     display_name: Some("Node.js".to_string()),
///     description: Some("JavaScript runtime".to_string()),
///     homepage_url: None,
///     metadata_format_version: "1.0".to_string(),
///     base_download_url: Url::parse("https://nodejs.org/dist").unwrap(),
///     platforms: HashMap::new(),
///     default_install_config: InstallConfig {
///         archive_format: None,
///         strip_components: None,
///         bin_subdir: None,
///         post_install_hook: None,
///     },
///     executables: Vec::new(),
///     default_executable_name: None,
///     available_versions: HashMap::new(),
///     channels: HashMap::new(),
/// };
/// ```
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProjectMetadata {
    /// Unique project identifier (should match unique_name from registration)
    pub project_name: String,
    /// Human-friendly display name (e.g., "Node.js" for project "nodejs")
    pub display_name: Option<String>,
    /// Brief description of what this project provides
    pub description: Option<String>,
    /// URL to the project's homepage or documentation
    pub homepage_url: Option<String>,
    /// Version of the metadata format specification
    pub metadata_format_version: String,
    /// Base URL for downloading project artifacts
    pub base_download_url: Url,
    /// Supported platforms and their system requirements
    ///
    /// Key: platform_id (e.g., "win-x64", "linux-arm64")
    /// Value: platform specification details
    pub platforms: HashMap<String, PlatformDetail>,
    /// Default installation configuration for all versions
    pub default_install_config: InstallConfig,
    /// List of all executables this project provides
    pub executables: Vec<ExecutableDetail>,
    /// Name of the primary/default executable from the executables list
    pub default_executable_name: Option<String>,
    /// All available versions of this project
    ///
    /// Key: version string (e.g., "1.0.0", "2.1.0-beta1")
    /// Value: version-specific details and artifacts
    pub available_versions: HashMap<String, VersionDetail>,
    /// Dynamic version channels for automatic selection
    ///
    /// Key: channel name (e.g., "stable", "lts", "nightly")
    /// Value: channel resolution configuration
    pub channels: HashMap<String, ChannelDetail>,
}
