#[cfg(feature = "selfupdate")]
use crate::cli::GupChannel;
#[cfg(feature = "selfupdate")]
use crate::global_config_manager::{load_global_config, save_global_config};
#[cfg(feature = "selfupdate")]
use crate::global_paths::GupGlobalPaths;
#[cfg(feature = "selfupdate")]
use anyhow::{Context, Result};

#[cfg(feature = "selfupdate")]
pub fn run_command_selfchannel(
    channel_name_opt: Option<GupChannel>,
    paths: &GupGlobalPaths,
) -> Result<()> {
    let mut global_config = load_global_config(paths)
        .with_context(|| "`self channel` command failed to load gup configuration data.")?;

    match channel_name_opt {
        Some(new_channel) => {
            let new_channel_str = new_channel.to_lowercase().to_string();
            global_config.self_update_channel = Some(new_channel_str.clone());
            save_global_config(&global_config, paths).with_context(|| {
                "Failed to save configuration after setting self-update channel."
            })?;
            eprintln!("gup self-update channel set to '{}'.", new_channel_str);
        }
        None => {
            let current_channel = global_config
                .self_update_channel
                .as_deref()
                .unwrap_or("release (default)");
            println!(
                "gup self-update channel is currently '{}'.",
                current_channel
            );
            println!("Run `gup self channel <CHANNEL_NAME>` to change it. Available: release, releasepreview, dev.");
        }
    }

    Ok(())
}
