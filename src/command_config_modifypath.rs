pub fn run_command_config_modifypath(
    value: Option<bool>,
    quiet: bool,
    paths: &crate::global_paths::GupGlobalPaths,
) -> anyhow::Result<()> {
    use crate::global_config_manager::{load_global_config, save_global_config};
    use crate::operations_shell::{
        add_binfolder_to_path_in_shell_scripts, remove_binfolder_from_path_in_shell_scripts,
    };
    use anyhow::Context;

    let mut global_config = load_global_config(paths)
        .with_context(|| "Failed to load global configuration for modify PATH setting.")?;

    match value {
        Some(v) => {
            let value_changed = global_config.modify_path_on_install != Some(v);

            global_config.modify_path_on_install = Some(v);

            if value_changed {
                save_global_config(&global_config, paths).with_context(|| {
                    "Failed to save global configuration after updating modify PATH setting."
                })?;

                // Apply the change immediately to shell scripts
                if v {
                    add_binfolder_to_path_in_shell_scripts(&paths.gup_bin_dir()).with_context(
                        || "Failed to add gup bin directory to PATH in shell scripts.",
                    )?;
                } else {
                    remove_binfolder_from_path_in_shell_scripts().with_context(|| {
                        "Failed to remove gup bin directory from PATH in shell scripts."
                    })?;
                }
            }

            if !quiet {
                if value_changed {
                    if v {
                        eprintln!("Automatic PATH modification enabled. Gup bin directory will be added to shell scripts.");
                    } else {
                        eprintln!("Automatic PATH modification disabled. Gup bin directory will be removed from shell scripts.");
                    }
                } else {
                    if v {
                        eprintln!("Automatic PATH modification is already enabled.");
                    } else {
                        eprintln!("Automatic PATH modification is already disabled.");
                    }
                }
            }
        }
        None => {
            if !quiet {
                match global_config.modify_path_on_install {
                    Some(true) => eprintln!("Automatic PATH modification is currently enabled."),
                    Some(false) => eprintln!("Automatic PATH modification is currently disabled."),
                    None => eprintln!("Automatic PATH modification setting is not configured (defaults to enabled)."),
                }
            }
        }
    }

    Ok(())
}
