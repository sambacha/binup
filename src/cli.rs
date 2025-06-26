//! # Command Line Interface Definitions
//!
//! This module defines the complete command-line interface for gup using the clap crate.
//! It includes all main commands, subcommands, and their associated arguments.
//!
//! ## Command Structure
//!
//! The CLI is organized around these primary concepts:
//! - **Project Management**: Add/remove software projects from gup's management
//! - **Version Management**: Install, update, and switch between versions
//! - **Configuration**: Global settings and per-project overrides  
//! - **Utilities**: Status reporting, garbage collection, shell completion
//!
//! ## Examples
//!
//! ```text
//! # Project management
//! gup project add https://example.com/nodejs.toml
//! gup project list
//!
//! # Version management  
//! gup add nodejs 18.0.0
//! gup default nodejs stable
//! gup update nodejs
//!
//! # Directory overrides
//! gup override set nodejs 16.0.0 --path /my/old/project
//! ```

use clap::{Parser, ValueEnum};

#[derive(Parser)]
#[clap(name = "gup", version)]
#[command(
    after_help = "To launch a specific version of a managed tool, ensure it's the default or use overrides if supported.
Gup manages tools defined by project configurations."
)]
/// The Generic Updater/Installer Program
///
/// Gup is a configuration-driven version manager that can manage arbitrary software projects.
/// Each project is defined by a metadata file that describes available versions, download URLs,
/// and installation procedures.
pub enum GupCli {
    /// Set the default version for a managed project
    Default {
        project_name: String,
        version_or_channel: String,
    },
    /// Add a specific version or channel of a managed project to your system
    Add {
        project_name: String,
        version_or_channel: String,
    },
    /// Link an existing binary to a custom name for a managed project
    Link {
        project_name: String,
        link_name: String, // e.g., "dev", "localbuild"
        file_path: String, // Path to the executable/script
        args: Vec<String>, // Optional arguments for the linked command
    },
    /// List all installed versions and channels for a project, or all managed projects
    #[clap(alias = "ls")]
    List { project_name: Option<String> },
    #[clap(subcommand, name = "override")]
    /// Manage directory-specific version overrides for a project
    OverrideSubCmd(OverrideSubCmd), // Will need project_name context from parent or arg
    #[clap(alias = "up")]
    /// Update a specific project to its latest channel versions, or gup itself
    Update {
        // This command might need to be split or clarified (update project vs update gup)
        project_name: Option<String>, // If None, could imply updating all projects or gup itself
        channel: Option<String>,      // Specific channel within a project
    },
    #[clap(alias = "rm")]
    /// Remove a version or linked path from a managed project
    Remove {
        project_name: String,
        version_or_link_name: String,
    },
    #[clap(alias = "st")]
    /// Show status for a managed project or all projects
    Status { project_name: Option<String> },
    /// Garbage collect uninstalled versions for a project
    Gc {
        project_name: String,
        #[clap(long)]
        prune_linked: bool,
    },
    #[clap(subcommand, name = "config")]
    /// Gup configuration
    Config(ConfigSubCmd), // For gup's own settings
    #[clap(subcommand, name = "project")]
    /// Manage software projects known to gup
    Project(ProjectSubCmd), // Changed from ProjectSubCmdArgs
    #[clap(hide = true)] // Kept for potential internal/advanced use
    Api { command: String },
    #[clap(name = "info", hide = true)] // Could be a generic info command
    Info { project_name: Option<String> },
    #[clap(subcommand, name = "self")]
    /// Manage this gup installation
    SelfSubCmd(SelfSubCmd),
    /// Generate tab-completion scripts for your shell
    Completions { shell: clap_complete::Shell },
    #[cfg(feature = "selfupdate")]
    #[clap(name = "a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6", hide = true)]
    SecretSelfUpdate {},
}

#[derive(Parser)]
/// Manage directory-specific version overrides for a project
///
/// Overrides allow you to use different versions of a project in different directories.
/// This is useful for working on multiple projects that require different versions of the same tool.
pub enum OverrideSubCmd {
    /// Show current override settings for a project in the current directory
    Status {
        /// Name of the project to check overrides for
        project_name: String,
    },
    /// Set a version override for a project in a specific directory
    Set {
        /// Name of the project to set override for
        project_name: String,
        /// Version or channel to use in this directory
        version_or_channel: String,
        /// Directory path (defaults to current directory)
        #[clap(long, short)]
        path: Option<String>,
    },
    /// Remove a version override for a project
    Unset {
        /// Name of the project to remove override for
        project_name: String,
        /// Remove override even if directory doesn't exist
        #[clap(long, short)]
        nonexistent: bool,
        /// Directory path (defaults to current directory)
        #[clap(long, short)]
        path: Option<String>,
    },
}

#[derive(clap::Subcommand)]
/// Manage software projects known to gup
///
/// Projects are software packages that gup can manage (like Node.js, Python, etc.).
/// Each project must be defined by a metadata file that describes its versions and installation.
pub enum ProjectSubCmd {
    /// Add a new project to gup from a registration file or URL
    ///
    /// The registration source should be a TOML file defining the project metadata,
    /// including available versions, download URLs, and installation instructions.
    Add {
        /// Path or URL to the project registration TOML file
        registration_source: String,
    },
    /// Remove a project from gup's management
    ///
    /// This removes the project from gup's registry but does not uninstall existing versions.
    /// Use `gup gc <project>` first if you want to clean up installed versions.
    Remove {
        /// Name of the project to remove
        project_name: String,
    },
    /// List all projects managed by gup
    ///
    /// Shows all projects that have been added to gup, along with their current status.
    List {},
    /// Force an update of a project's metadata from its source
    ///
    /// Refreshes the cached metadata for a project by re-downloading it from the source.
    /// This is useful when new versions have been released.
    UpdateMetadata {
        /// Name of the project to update metadata for
        project_name: String,
    },
}

#[derive(Debug, ValueEnum, Clone)]
/// Release channels for gup's self-update mechanism
///
/// These channels determine which version of gup itself to use for updates.
/// Different channels provide different stability guarantees.
pub enum GupChannel {
    /// Stable release channel - thoroughly tested versions
    #[clap(name = "release")]
    Release,
    /// Release preview channel - pre-release versions for testing
    #[clap(name = "releasepreview")]
    ReleasePreview,
    /// Development channel - bleeding edge, potentially unstable
    #[clap(name = "dev")]
    Dev,
}

// Removed ProjectSubCmdArgs struct as it's not needed with direct enum use
// The definition of ProjectSubCmdArgs was here and is now removed.

impl GupChannel {
    /// Returns the lowercase string representation of the channel
    ///
    /// Used for serialization and API calls to gup's update servers.
    ///
    /// # Examples
    ///
    /// ```
    /// # use gup::cli::GupChannel;
    /// assert_eq!(GupChannel::Release.to_lowercase(), "release");
    /// assert_eq!(GupChannel::Dev.to_lowercase(), "dev");
    /// ```
    pub fn to_lowercase(&self) -> &str {
        match self {
            GupChannel::Release => "release",
            GupChannel::ReleasePreview => "releasepreview",
            GupChannel::Dev => "dev",
        }
    }
}

#[derive(Parser)]
/// Manage this gup installation
pub enum SelfSubCmd {
    // Update command for gup itself
    #[clap(alias = "up")]
    /// Update gup itself to the latest version from its channel
    Update {},
    #[cfg(feature = "selfupdate")]
    /// Configure the channel to use for gup updates. Leave CHANNEL blank to see current channel.
    Channel {
        #[arg(value_enum)]
        channel: Option<GupChannel>,
    },
    #[cfg(feature = "selfupdate")]
    /// Uninstall gup from the system
    Uninstall {},
    #[cfg(not(feature = "selfupdate"))]
    /// Uninstall gup from the system (UNAVAILABLE)
    Uninstall {},
}

#[derive(Parser)]
/// Gup configuration options
pub enum ConfigSubCmd {
    /// Show all current configuration settings
    Show {},
    #[cfg(feature = "selfupdate")]
    #[clap(name = "backgroundselfupdateinterval")]
    /// The time between automatic background updates of gup in minutes, use 0 to disable.
    BackgroundSelfupdateInterval {
        /// New value
        value: Option<i64>,
    },
    #[cfg(feature = "selfupdate")]
    #[clap(name = "startupselfupdateinterval")]
    /// The time between automatic updates of gup at startup in minutes, use 0 to disable.
    StartupSelfupdateInterval {
        /// New value
        value: Option<i64>,
    },
    #[clap(name = "modifypath")]
    /// Add the gup bin directory to your PATH by manipulating various shell startup scripts.
    ModifyPath {
        /// New value
        value: Option<bool>,
    },
    // /// The time between automatic updates of the versions database in minutes, use 0 to disable. // Removed, metadata update is per-project
    // #[clap(name = "versionsdbupdateinterval")]
    // VersionsDbUpdateInterval {
    //     /// New value
    //     value: Option<i64>,
    // },
}

/// Context for CI/non-interactive mode detection and control
pub struct CiContext {
    pub ci_mode: bool,
    pub quiet: bool,
}

impl CiContext {
    /// Detect CI environment and create appropriate context
    pub fn detect() -> Self {
        let ci_detected = std::env::var("CI").is_ok()
            || std::env::var("GITHUB_ACTIONS").is_ok()
            || std::env::var("GITLAB_CI").is_ok()
            || std::env::var("JENKINS_URL").is_ok()
            || std::env::var("BUILDKITE").is_ok()
            || std::env::var("GUP_CI").is_ok();

        Self {
            ci_mode: ci_detected,
            quiet: ci_detected || std::env::var("GUP_QUIET").is_ok(),
        }
    }

    /// Whether interactive prompts should be shown
    pub fn should_prompt(&self) -> bool {
        !self.ci_mode
    }

    /// Whether progress bars and visual elements should be shown
    pub fn should_show_progress(&self) -> bool {
        !self.ci_mode && !self.quiet
    }

    /// Whether banners and welcome messages should be shown
    pub fn should_show_banners(&self) -> bool {
        !self.ci_mode && !self.quiet
    }
}
