use crate::global_config_manager::load_global_config;
use crate::global_paths::GupGlobalPaths;
use anyhow::{Context, Result};

pub fn run_command_config_show(paths: &GupGlobalPaths) -> Result<()> {
    let global_config =
        load_global_config(paths).with_context(|| "Failed to load global configuration.")?;

    println!("Gup Configuration:");
    println!("==================");
    println!();

    // PATH modification setting
    match global_config.modify_path_on_install {
        Some(true) => println!("PATH modification:           enabled"),
        Some(false) => println!("PATH modification:           disabled"),
        None => println!("PATH modification:           enabled (default)"),
    }

    // Self-update settings (only show if selfupdate feature is enabled)
    #[cfg(feature = "selfupdate")]
    {
        match global_config.startup_selfupdate_interval {
            Some(minutes) if minutes > 0 => {
                println!("Startup self-update check:   every {} minutes", minutes)
            }
            Some(0) => println!("Startup self-update check:   disabled"),
            Some(minutes) => println!("Startup self-update check:   invalid ({} minutes)", minutes),
            None => println!("Startup self-update check:   disabled (default)"),
        }

        match global_config.background_selfupdate_interval {
            Some(minutes) if minutes > 0 => {
                println!("Background self-update check: every {} minutes", minutes)
            }
            Some(0) => println!("Background self-update check: disabled"),
            Some(minutes) => println!(
                "Background self-update check: invalid ({} minutes)",
                minutes
            ),
            None => println!("Background self-update check: disabled (default)"),
        }
    }

    println!();

    // Directory information
    println!("Directories:");
    println!("------------");
    println!("Gup home:                    {}", paths.guphome().display());
    println!(
        "Bin directory:               {}",
        paths.gup_bin_dir().display()
    );
    println!(
        "Configuration file:          {}",
        paths.global_config_file().display()
    );

    println!();

    // Project information
    let project_count = global_config.managed_projects.len();
    if project_count == 0 {
        println!("Managed projects:            none");
    } else if project_count == 1 {
        println!("Managed projects:            1 project");
    } else {
        println!("Managed projects:            {} projects", project_count);
    }

    if project_count > 0 {
        println!();
        println!("Projects:");
        for project_name in global_config.managed_projects.keys() {
            println!("  - {}", project_name);
        }
    }

    println!();
    println!("Use 'gup config <setting> [value]' to modify configuration settings.");
    println!("Use 'gup config <setting>' to view the current value of a specific setting.");

    Ok(())
}
