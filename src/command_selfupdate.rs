#[cfg(feature = "selfupdate")]
use crate::cli::GupChannel;
#[cfg(feature = "selfupdate")]
use crate::constants::{
    APP_NAME, DEFAULT_UPDATE_CHANNEL, GITHUB_ORG, GITHUB_REPO, SELFUPDATE_BASE_URL,
};
#[cfg(feature = "selfupdate")]
use crate::get_own_version;
#[cfg(feature = "selfupdate")]
use crate::global_config_manager::{load_global_config, save_global_config};
use crate::global_paths::GupGlobalPaths;
#[cfg(feature = "selfupdate")]
use anyhow::{anyhow, bail, Context, Result};

#[cfg(feature = "selfupdate")]
pub fn run_command_selfupdate(channel: Option<GupChannel>, paths: &GupGlobalPaths) -> Result<()> {
    let mut global_config = load_global_config(paths)
        .with_context(|| "Failed to load global configuration for self-update.")?;

    // Determine target channel - use provided channel or current config or default to release
    let target_channel = match &channel {
        Some(ch) => ch.to_lowercase().to_string(),
        None => global_config
            .self_update_channel
            .as_deref()
            .unwrap_or(DEFAULT_UPDATE_CHANNEL)
            .to_string(),
    };

    // Update config with new channel if provided
    if channel.is_some() {
        global_config.self_update_channel = Some(target_channel.clone());
        save_global_config(&global_config, paths)
            .with_context(|| "Failed to save global configuration after channel update.")?;
    }

    let current_version =
        get_own_version().with_context(|| "Failed to determine current gup version.")?;

    eprintln!(
        "Checking for {} updates on channel '{}'...",
        APP_NAME, target_channel
    );
    eprintln!("Current {} version: {}", APP_NAME, current_version);

    // For now, implement a simple mock update check
    // In a real implementation, this would:
    // 1. Query a remote server for the latest version on the channel
    // 2. Download and verify the new binary
    // 3. Replace the current binary atomically
    // 4. Update the last_self_update timestamp

    // Mock implementation - simulate that we're always up to date
    match check_for_updates(&target_channel) {
        Ok(Some(new_version)) => {
            eprintln!("Found new {} version: {}", APP_NAME, new_version);
            eprintln!("Downloading and installing update...");

            // In a real implementation, download and install here
            perform_update(&new_version, paths)
                .with_context(|| format!("Failed to perform {} update.", APP_NAME))?;

            // Update timestamp
            global_config.last_self_update = Some(chrono::Utc::now());
            save_global_config(&global_config, paths)
                .with_context(|| "Failed to save configuration after update.")?;

            eprintln!(
                "Successfully updated {} to version {}!",
                APP_NAME, new_version
            );
            eprintln!("Note: You may need to restart your shell to use the new version.");
        }
        Ok(None) => {
            eprintln!(
                "{} is already up to date (version {}) on channel '{}'.",
                APP_NAME, current_version, target_channel
            );

            // Update timestamp even if no update was needed
            global_config.last_self_update = Some(chrono::Utc::now());
            save_global_config(&global_config, paths)
                .with_context(|| "Failed to save configuration after update check.")?;
        }
        Err(e) => {
            eprintln!("Failed to check for updates: {}", e);
            eprintln!("You can try again later or check your internet connection.");
            return Err(e.context("Update check failed"));
        }
    }

    Ok(())
}

/// Check for available updates on the specified channel
/// Returns Ok(Some(version)) if update available, Ok(None) if up to date
#[cfg(feature = "selfupdate")]
fn check_for_updates(channel: &str) -> anyhow::Result<Option<String>> {
    use crate::operations_selfupdate::check_for_gup_updates;

    let current_version = get_own_version()?;

    check_for_gup_updates(
        &current_version.to_string(),
        channel,
        GITHUB_ORG,
        GITHUB_REPO,
    )
}

/// Perform the actual update by downloading and replacing the binary
#[cfg(feature = "selfupdate")]
fn perform_update(new_version: &str, _paths: &GupGlobalPaths) -> anyhow::Result<()> {
    use crate::operations_selfupdate::download_and_install_gup_update;
    use crate::utils::get_target_triple_id;

    // Determine target architecture
    let target_triple =
        get_target_triple_id().with_context(|| "Failed to determine target platform")?;

    download_and_install_gup_update(new_version, &target_triple, SELFUPDATE_BASE_URL)
}

#[cfg(not(any(feature = "windowsstore", feature = "selfupdate")))]
pub fn run_command_selfupdate(
    _channel: Option<crate::cli::GupChannel>,
    _paths: &GupGlobalPaths,
) -> anyhow::Result<()> {
    eprintln!("Self-update is not available in this build of gup.");
    Ok(())
}
