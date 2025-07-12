#[cfg(feature = "selfupdate")]
use crate::{cli::CiContext, global_paths::GupGlobalPaths};
#[cfg(feature = "selfupdate")]
use anyhow::Result; // Renamed

#[cfg(feature = "selfupdate")]
pub fn run_command_selfuninstall(paths: &GupGlobalPaths) -> Result<()> {
    use dialoguer::Confirm;
    // These command imports will need to point to the gup versions
    use crate::{
        command_config_backgroundselfupdate::run_command_config_backgroundselfupdate,
        command_config_modifypath::run_command_config_modifypath,
        command_config_startupselfupdate::run_command_config_startupselfupdate,
        // command_config_symlinks::run_command_config_symlinks, // This is Julia-specific
    };

    // Check CI context to determine if prompts should be shown
    let ci_context = CiContext::detect();
    let choice = if ci_context.should_prompt() {
        Confirm::new()
            .with_prompt("Do you really want to uninstall gup and all managed projects?") // Updated prompt
            .default(false)
            .interact()?
    } else {
        // In CI mode, require explicit confirmation via environment variable
        std::env::var("GUP_FORCE").is_ok()
    };

    if !choice {
        return Ok(());
    }

    eprint!("Removing background self update task for gup.");
    match run_command_config_backgroundselfupdate(Some(0), true, paths) {
        Ok(_) => eprintln!(" Success."),
        Err(_) => eprintln!(" Failed."),
    };

    eprint!("Removing startup self update configuration for gup.");
    match run_command_config_startupselfupdate(Some(0), true, &paths) {
        Ok(_) => eprintln!(" Success."),
        Err(_) => eprintln!(" Failed."),
    };

    eprint!("Removing gup PATH modifications in startup scripts.");
    match run_command_config_modifypath(Some(false), true, &paths) {
        Ok(_) => eprintln!(" Success."),
        Err(_) => eprintln!(" Failed."),
    };

    // Removing channel symlinks is Julia-specific. Gup's symlinks are handled by removing gup_bin_dir.
    // eprint!("Removing symlinks.");
    // match run_command_config_symlinks(Some(false), true, &paths) {
    //     Ok(_) => eprintln!(" Success."),
    //     Err(_) => eprintln!(" Failed."),
    // };

    // Delete the main gup home directory which contains everything:
    // projects, gup's own config, etc.
    eprint!("Deleting gup home folder {:?}.", paths.guphome());
    match std::fs::remove_dir_all(paths.guphome()) {
        Ok(_) => eprintln!(" Success."),
        Err(e) => eprintln!(" Failed ({}). You may need to remove it manually.", e), // Provide more error info
    };

    // The logic for deleting juliaupselfhome separately from juliauphome was complex
    // and depended on how juliaup was installed (e.g. if it was installed to a custom location
    // separate from its data directory).
    // For gup, if `paths.guphome()` is the root, removing it should be sufficient.
    // If gup's own binaries are installed elsewhere (e.g. system-wide by a package manager,
    // or in a separate `gupselfhome`), that logic would need to be preserved/adapted.
    // The current `GupGlobalPaths` has `gupselfhome` and `gupselfbin` under `cfg(feature = "selfupdate")`.
    // If `gupselfhome` is different from `guphome`, it should also be cleaned up.

    // This part assumes gup's own binaries might be in `paths.gupselfhome()` if different from `paths.guphome()`
    // This is often the case if gup is installed via the installer script to a custom location.
    #[cfg(feature = "selfupdate")]
    if paths.guphome() != paths.gupselfhome() && paths.gupselfhome().exists() {
        eprint!(
            "Deleting gup installation folder {:?}.",
            paths.gupselfhome()
        );
        match std::fs::remove_dir_all(paths.gupselfhome()) {
            Ok(_) => eprintln!(" Success."),
            Err(e) => eprintln!(" Failed ({}). You may need to remove it manually.", e),
        }
    } else if paths.guphome() == paths.gupselfhome() {
        // If they are the same, guphome removal already handled it.
        // We might still want to clean up the gup binary itself if it's in a standard system path
        // but that's beyond what this script can reliably do without knowing how it was installed.
        // The current executable deletion is very risky.
    }

    // Deleting the running executable itself is problematic and platform-dependent.
    // The original code attempted this for non-MSIX installs.
    // For now, we'll skip direct deletion of the running `gup` binary.
    // The user might need to remove it manually after running uninstall, or the OS handles it.
    /*
    if paths.guphome() != paths.gupselfhome() { // This condition needs to be re-evaluated for gup
        let gup_binfolder_path = paths.gupselfbin(); // Assuming gupselfbin is correct
        let gup_exe_path = gup_binfolder_path.join("gup"); // Assuming gup executable is named 'gup'
        // ... logic to remove gup_exe_path and then gup_binfolder_path and gupselfhome if empty ...
        // This is complex and error-prone, especially if gup is currently running from that location.
    }
    */

    eprintln!("Successfully uninstalled gup. You may need to manually remove the gup executable if it was installed to a system PATH location not managed by gup's home directory, and restart your shell for PATH changes to take full effect.");

    Ok(())
}

#[cfg(not(feature = "selfupdate"))]
use anyhow::Result; // This use was already here, but make sure it's only compiled when needed.

#[cfg(not(feature = "selfupdate"))]
pub fn run_command_selfuninstall_unavailable() -> Result<()> {
    eprintln!(
        "Self uninstall command is unavailable in this variant of gup.
This software may have been built with the intention of distributing it
through a package manager." // Updated message
    );
    Ok(())
}
