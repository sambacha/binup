#[cfg(feature = "selfupdate")]
pub fn run_command_config_startupselfupdate(
    value: Option<i64>,
    quiet: bool,
    paths: &crate::global_paths::GupGlobalPaths,
) -> anyhow::Result<()> {
    use crate::global_config_manager::{load_global_config, save_global_config};
    use anyhow::Context;

    let mut global_config = load_global_config(paths)
        .with_context(|| "Failed to load global configuration for startup self-update setting.")?;

    match value {
        Some(v) => {
            let new_interval = if v > 0 { Some(v as u64) } else { None };
            let value_changed = global_config.startup_self_update_interval_minutes != new_interval;

            global_config.startup_self_update_interval_minutes = new_interval;

            if value_changed {
                save_global_config(&global_config, paths)
                    .with_context(|| "Failed to save global configuration after updating startup self-update interval.")?;
            }

            if !quiet {
                if value_changed {
                    if v > 0 {
                        eprintln!("Startup self-update interval set to {} minutes.", v);
                    } else {
                        eprintln!("Startup self-update interval disabled.");
                    }
                } else {
                    if v > 0 {
                        eprintln!(
                            "Startup self-update interval is already set to {} minutes.",
                            v
                        );
                    } else {
                        eprintln!("Startup self-update interval is already disabled.");
                    }
                }
            }
        }
        None => {
            if !quiet {
                match global_config.startup_self_update_interval_minutes {
                    Some(interval) => eprintln!(
                        "Startup self-update interval is currently {} minutes.",
                        interval
                    ),
                    None => eprintln!("Startup self-update interval is currently disabled."),
                }
            }
        }
    }

    Ok(())
}
