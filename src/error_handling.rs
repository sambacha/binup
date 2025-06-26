//! # Error Handling Utilities
//!
//! This module provides centralized error handling utilities for gup, focusing on
//! user-friendly error messages and helpful suggestions for common issues.
//!
//! ## Design Philosophy
//!
//! Rather than exposing raw technical errors to users, this module transforms
//! internal errors into helpful, actionable messages that guide users toward
//! solutions. Each error includes:
//!
//! - **Clear description** of what went wrong
//! - **Contextual information** about the operation that failed  
//! - **Actionable suggestions** for how to resolve the issue
//! - **Technical details** when helpful for debugging
//!
//! ## Error Categories
//!
//! - **Network Errors**: Download failures, connectivity issues
//! - **Project Errors**: Unknown projects, missing versions
//! - **URL Errors**: Malformed or invalid URLs
//! - **Installation Errors**: File system issues, permission problems
//!
//! ## Examples
//!
//! ```rust
//! use gup::error_handling::GupError;
//!
//! // Create a user-friendly project not found error
//! let error = GupError::project_not_found("nonexistent-tool");
//! println!("{}", error);
//! ```

use anyhow::{anyhow, Context, Result};
use console::style;

/// Provides user-friendly error messages and suggestions for common gup operations
///
/// This utility struct creates well-formatted error messages that help users understand
/// what went wrong and how to fix it, rather than exposing raw technical errors.
pub struct GupError;

impl GupError {
    /// Creates a user-friendly error for invalid URLs with helpful suggestions
    ///
    /// # Arguments
    /// * `url` - The invalid URL that caused the error
    /// * `context` - Description of what the URL was being used for
    ///
    /// # Returns
    /// A formatted error with suggestions for fixing the URL
    pub fn invalid_url(url: &str, context: &str) -> anyhow::Error {
        anyhow!(
            "{}\n{}\n\n{}\n  • {}\n  • {}",
            style("Error: Invalid URL").red().bold(),
            style(format!("The URL '{}' is not valid for {}", url, context)).white(),
            style("Suggestions:").yellow().bold(),
            "Ensure the URL starts with 'http://' or 'https://'",
            "Check for typos in the URL"
        )
    }

    /// Creates a user-friendly error for network issues with helpful suggestions
    ///
    /// # Arguments
    /// * `url` - The URL that failed to be accessed
    /// * `original_error` - The underlying network error that occurred
    ///
    /// # Returns
    /// A formatted error with network troubleshooting suggestions
    pub fn network_error(url: &str, original_error: &anyhow::Error) -> anyhow::Error {
        anyhow!(
            "{}\n{}\n\n{}\n  • {}\n  • {}\n  • {}\n\n{}\n{}",
            style("Error: Network request failed").red().bold(),
            style(format!("Failed to download from: {}", url)).white(),
            style("Suggestions:").yellow().bold(),
            "Check your internet connection",
            "Verify the URL is accessible in a web browser",
            "Check if you're behind a corporate firewall",
            style("Technical details:").dim(),
            style(format!("{}", original_error)).dim()
        )
    }

    /// Creates a user-friendly error for project not found with helpful suggestions
    ///
    /// # Arguments
    /// * `project_name` - The name of the project that wasn't found
    ///
    /// # Returns
    /// A formatted error with suggestions for finding or adding projects
    pub fn project_not_found(project_name: &str) -> anyhow::Error {
        anyhow!(
            "{}\n{}\n\n{}\n  • {}\n  • {}",
            style("Error: Project not found").red().bold(),
            style(format!("Project '{}' is not managed by gup", project_name)).white(),
            style("Suggestions:").yellow().bold(),
            format!("Use `gup project add <url>` to register a new project"),
            "Use `gup project list` to see all managed projects"
        )
    }

    /// Creates a user-friendly error for JSON parsing with helpful suggestions
    pub fn invalid_metadata(url: &str, json_error: &serde_json::Error) -> anyhow::Error {
        anyhow!(
            "{}\n{}\n\n{}\n  • {}\n  • {}\n\n{}\n{}",
            style("Error: Invalid project metadata").red().bold(),
            style(format!(
                "The metadata from '{}' is not valid JSON or missing required fields",
                url
            ))
            .white(),
            style("Suggestions:").yellow().bold(),
            "Verify the URL points to a valid gup project metadata file",
            "Check if the file follows the gup metadata format specification",
            style("JSON parsing error:").dim(),
            style(format!("{}", json_error)).dim()
        )
    }

    /// Creates a user-friendly error for HTTP status codes with helpful suggestions
    pub fn http_error(url: &str, status_code: u16) -> anyhow::Error {
        let suggestion = match status_code {
            404 => "The file may have been moved or deleted",
            403 => "You may not have permission to access this resource",
            500..=599 => "The server is experiencing issues, try again later",
            _ => "Check if the URL is correct and accessible",
        };

        anyhow!(
            "{}\n{}\n\n{}\n  • {}",
            style("Error: HTTP request failed").red().bold(),
            style(format!(
                "Server responded with status {} for: {}",
                status_code, url
            ))
            .white(),
            style("Suggestion:").yellow().bold(),
            suggestion
        )
    }

    /// Creates a user-friendly error for version resolution issues
    pub fn version_not_found(
        project_name: &str,
        version_or_channel: &str,
        available_versions: &[String],
    ) -> anyhow::Error {
        let available_list = if available_versions.is_empty() {
            "No versions are currently available".to_string()
        } else {
            format!("Available versions: {}", available_versions.join(", "))
        };

        anyhow!(
            "{}\n{}\n\n{}\n  • {}\n  • {}\n\n{}",
            style("Error: Version not found").red().bold(),
            style(format!(
                "Version or channel '{}' not found for project '{}'",
                version_or_channel, project_name
            ))
            .white(),
            style("Suggestions:").yellow().bold(),
            format!(
                "Use `gup list {}` to see available versions and channels",
                project_name
            ),
            "Check for typos in the version number or channel name",
            style(available_list).dim()
        )
    }

    /// Wraps any error with better context and consistent formatting
    pub fn wrap_with_context<T>(result: Result<T>, operation: &str) -> Result<T> {
        result.with_context(|| format!("{} {}", style("Failed to").red(), operation))
    }
}

/// Extension trait to add user-friendly error handling to Results
pub trait GupErrorExt<T> {
    /// Converts common error types to user-friendly messages
    fn with_user_friendly_error(self, operation: &str) -> Result<T>;
}

impl<T> GupErrorExt<T> for Result<T> {
    fn with_user_friendly_error(self, operation: &str) -> Result<T> {
        self.with_context(|| format!("{} {}", style("Operation failed:").red().bold(), operation))
    }
}
