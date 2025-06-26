use anyhow::Result;
use clap::builder::BoolishValueParser;
use clap::Parser;
use gup::cli::{CiContext, GupChannel}; // Renamed from JuliaupChannel, assuming gup::cli will define this

#[cfg(feature = "selfupdate")]
fn run_individual_config_wizard(
    install_choices: &mut InstallChoices,
    theme: &dyn dialoguer::theme::Theme,
) -> Result<Option<()>> {
    use std::path::PathBuf;

    use dialoguer::{Confirm, Input};
    use log::trace;

    trace!(
        "install_location pre inside the prompt function {:?}",
        install_choices.install_location
    );

    let new_install_location = Input::with_theme(theme)
        .with_prompt("Enter the folder where you want to install gup") // Renamed Juliaup to gup
        .validate_with(|input: &String| match input.parse::<PathBuf>() {
            Ok(_) => Ok(()),
            Err(_) => Err("Not a valid input".to_owned()),
        })
        .with_initial_text(install_choices.install_location.to_string_lossy().clone())
        .interact_text()?;

    let new_install_location = shellexpand::tilde(&new_install_location)
        .parse::<PathBuf>()
        .unwrap();

    let new_modifypath = match Confirm::with_theme(theme)
        .with_prompt("Do you want to add the gup bin directory to your PATH by manipulating various shell startup scripts?") // Updated prompt
        .default(install_choices.modifypath)
        .interact_opt()? {
            Some(value) => value,
            None => return Ok(None)
        };

    // Symlinks for channels are Julia-specific, this might be removed or rethought for gup.
    // For now, let's assume gup might still have a concept of its own channels for self-update,
    // but not per-project channel symlinks created by the installer itself.
    // This specific question about "channel specific symlinks" is likely removed.
    // install_choices.symlinks might be repurposed or removed.
    // For now, I'll comment out this specific dialog.
    // let new_symlinks = match Confirm::with_theme(theme)
    //     .with_prompt("Do you want to add channel specific symlinks?") // This is Julia-specific
    //     .default(install_choices.symlinks)
    //     .interact_opt()?
    // {
    //     Some(value) => value,
    //     None => return Ok(None),
    // };
    let new_symlinks = install_choices.symlinks; // Keep current value if dialog removed

    let new_startupselfupdate = Input::with_theme(theme)
        .with_prompt(
            "Enter minutes between check for new gup version at gup startup, use 0 to disable", // Updated prompt
        )
        .validate_with(|input: &String| -> Result<(), &str> {
            match input.parse::<i64>() {
                Ok(val) => {
                    if val >= 0 {
                        Ok(())
                    } else {
                        Err("Not a valid input")
                    }
                }
                Err(_) => Err("Not a valid input"),
            }
        })
        .default(install_choices.startupselfupdate.to_string())
        .interact_text()?
        .parse::<i64>()
        .unwrap();

    let new_backgroundselfupdate = Input::with_theme(theme)
        .with_prompt(
            "Enter minutes between check for new gup version by a background task, use 0 to disable", // Updated prompt
        )
        .validate_with(|input: &String| -> Result<(), &str> {
            match input.parse::<i64>() {
                Ok(val) => {
                    if val >= 0 {
                        Ok(())
                    } else {
                        Err("Not a valid input")
                    }
                }
                Err(_) => Err("Not a valid input"),
            }
        })
        .default(install_choices.backgroundselfupdate.to_string())
        .interact_text()?
        .parse::<i64>()
        .unwrap();

    install_choices.install_location = new_install_location;
    install_choices.modifypath = new_modifypath;
    install_choices.symlinks = new_symlinks;
    install_choices.startupselfupdate = new_startupselfupdate;
    install_choices.backgroundselfupdate = new_backgroundselfupdate;

    Ok(Some(()))
}

#[cfg(feature = "selfupdate")]
fn is_gup_installed() -> bool {
    // Renamed from is_juliaup_installed
    use std::process::Stdio;

    let exit_status = std::process::Command::new("gup") // Renamed from juliaup
        .args(["--version"]) // Assuming gup will have a --version flag
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .stdin(Stdio::null())
        .status();

    match exit_status {
        Ok(status) => status.success(),
        Err(_) => false, // failed to execute `gup` command
    }
}

#[derive(Parser)]
#[clap(name = "GupBootstrap", version)] // Renamed from Juliainstaller
/// The gup (Generic Updater/Installer Program) Bootstrapper
struct GupBootstrapCli {
    // Renamed from Juliainstaller
    // Default channel for a *specific project* is not set by gup installer.
    // This might be a default project to add, or removed. For now, removing.
    // /// Default channel
    // #[clap(long, default_value = "release")]
    // default_channel: String,
    /// gup's own update channel
    #[clap(long, value_enum, default_value = "release")]
    gup_channel: GupChannel, // Renamed from juliaup_channel
    /// Disable confirmation prompt
    #[clap(short = 'y', long = "yes")]
    disable_confirmation_prompt: bool,
    /// Specify alternate path for gup's home directory
    #[clap(short = 'p', long = "path")]
    alternate_gup_home_path: Option<String>, // Renamed from alternate_path
    /// Control adding the gup bin dir to the PATH
    #[clap(long = "add-to-path", value_parser = BoolishValueParser::new(), default_value = "yes")]
    add_to_path: Option<bool>,
    /// Manually specify the background self-update interval
    #[clap(long = "background-selfupdate", default_value_t = 0)]
    background_selfupdate_interval: i64,
    /// Manually specify the statup self-update interval
    #[clap(long = "startup-selfupdate", default_value_t = 1440)]
    startup_selfupdate_interval: i64,
}

#[cfg(feature = "selfupdate")]
struct InstallChoices {
    backgroundselfupdate: i64,
    startupselfupdate: i64,
    symlinks: bool,
    modifypath: bool,
    install_location: std::path::PathBuf,
    modifypath_files: Vec<std::path::PathBuf>,
}

#[cfg(feature = "selfupdate")]
fn print_install_choices(install_choices: &InstallChoices) -> Result<()> {
    use console::style;

    println!("gup will be installed into the gup home directory, located at:"); // Renamed
    println!();
    println!("  {}", install_choices.install_location.to_string_lossy());
    println!();
    println!(
        "The {} command and shims for managed tools will be added to", // Updated
        style("gup").bold()
    );
    println!("gup's bin directory, located at:"); // Renamed
    println!();
    println!(
        "  {}",
        install_choices
            .install_location
            .join("bin")
            .to_string_lossy()
    );
    println!();

    if install_choices.modifypath {
        println!(
            "This path will then be added to your {} environment variable by",
            style("PATH").bold()
        );
        println!("modifying the profile files located at:");
        println!();
        for p in &install_choices.modifypath_files {
            println!("  {}", p.to_string_lossy());
        }
        println!();
    }

    if install_choices.backgroundselfupdate > 0 {
        println!(
            "The installer will configure a CRON job/Scheduled Task that checks for updates of"
        ); // Updated
        println!(
            "gup itself. This task will run approximately every {} minutes.", // Updated
            install_choices.backgroundselfupdate // Assuming this is in minutes now
        );
        println!();
    }

    if install_choices.startupselfupdate > 0 {
        println!("gup will look for a new version of itself every {} minutes when you run a gup command.", install_choices.startupselfupdate); // Updated
        println!();
    }

    // Symlinks for channels are Julia-specific. This section might be removed or rephrased
    // if gup has a different mechanism or no per-project channel symlinks created by default.
    // if install_choices.symlinks {
    //     println!("gup might create symlinks for specific project channels if configured to do so."); // Placeholder
    //     println!();
    // }

    Ok(())
}

#[cfg(feature = "selfupdate")]
pub fn main() -> Result<()> {
    use anyhow::{anyhow, Context};
    use console::{style, Style};
    use dialoguer::{
        theme::{ColorfulTheme, SimpleTheme, Theme},
        Confirm, Select,
    };
    use gup::{
        // Renamed
        // command_add and command_default are for projects, not gup itself during bootstrap
        // command_add::run_command_add,
        // command_default::run_command_default,
        command_selfchannel::run_command_selfchannel, // For gup's own update channel
        // config_file::GupSelfConfig, // Self-update configuration for GUP
        // get_juliaup_target, // This needs to be generic for gup if used for self-update downloads
        get_own_version,
        global_paths::get_gup_paths, // Renamed
                                     // operations::{download_extract_sans_parent, find_shell_scripts_to_be_modified}, // Removed, imported at module level
                                     // utils::get_juliaserver_base_url, // gup's self-update server URL will be different
    };
    use is_terminal::IsTerminal;
    // Need a way to get gup's self-update server URL and target string.
    // For now, these will be placeholders or hardcoded temporarily.
    use gup::global_paths::GupGlobalPaths;
    use gup::operations_download::download_extract_sans_parent;
    use gup::operations_shell::find_shell_scripts_to_be_modified;
    use gup::state_config::GupGlobalConfig; // For creating initial global config
    use url::Url;

    use std::io::Seek;
    use std::path::PathBuf;

    human_panic::setup_panic!(human_panic::Metadata::new(
        "GupBootstrap", // Renamed
        env!("CARGO_PKG_VERSION")
    )
    .support("https://github.com/your-repo/gup")); // Placeholder URL

    let env = env_logger::Env::new()
        .filter("GUP_LOG") // Renamed
        .write_style("GUP_LOG_STYLE"); // Renamed
    env_logger::init_from_env(env);

    info!("Parsing command line arguments.");
    let args = GupBootstrapCli::parse(); // Renamed

    if !args.disable_confirmation_prompt && !std::io::stdin().is_terminal() {
        return Err(anyhow!(
            "To install gup in non-interactive mode pass the -y parameter." // Renamed
        ));
    }

    let theme: Box<dyn Theme> = if std::io::stdout().is_terminal() {
        Box::new(ColorfulTheme {
            values_style: Style::new().yellow().dim(),
            ..ColorfulTheme::default()
        })
    } else {
        Box::new(SimpleTheme)
    };

    // paths will be GupGlobalPaths
    let mut paths = get_gup_paths().with_context(|| "Trying to load all global paths for gup.")?; // Renamed

    use gup::{
        // Renamed
        command_config_backgroundselfupdate::run_command_config_backgroundselfupdate,
        command_config_modifypath::run_command_config_modifypath,
        command_config_startupselfupdate::run_command_config_startupselfupdate,
        // command_config_symlinks::run_command_config_symlinks, // Symlinks config might be removed/repurposed
    };
    use log::{debug, info, trace};

    // Check CI context to determine if banners should be shown
    let ci_context = CiContext::detect();
    if ci_context.should_show_banners() {
        println!("{}", style("Welcome to gup!").bold()); // Renamed
        println!();
    }

    if is_gup_installed() {
        // Renamed
        println!("It seems that gup is already installed on this system. Please remove the previous installation of gup before you try to install a new version."); // Renamed
        return Ok(());
    }

    if ci_context.should_show_banners() {
        println!("This will download and install gup, the generic updater/installer program."); // Updated
        println!();
    }

    // Check for existing global_config_file
    if paths.global_config_file().exists() {
        println!("While gup does not seem to be installed on this system, there is a"); // Renamed
        println!("gup configuration file present from a previous installation:"); // Renamed
        println!("{}", paths.global_config_file().display());

        if args.disable_confirmation_prompt {
            println!();
            println!(
                "Please remove the existing gup configuration file or use interactive mode." // Renamed
            );

            return Ok(());
        } else {
            let continue_with_setup = Confirm::with_theme(theme.as_ref())
                .with_prompt("Do you want to continue with the installation and overwrite the existing gup configuration file?") // Renamed
                .default(true)
                .interact_opt()?;

            if !continue_with_setup.unwrap_or(false) {
                return Ok(());
            }

            println!();
        }
    }

    let mut install_choices = InstallChoices {
        backgroundselfupdate: args.background_selfupdate_interval,
        startupselfupdate: args.startup_selfupdate_interval,
        symlinks: false, // Defaulting to false, as project-specific symlinks are not handled by bootstrap
        modifypath: args.add_to_path.unwrap_or(true), // Default to true for modifypath
        install_location: match args.alternate_gup_home_path {
            // Renamed
            Some(alternate_path) => PathBuf::from(shellexpand::tilde(&alternate_path).into_owned()),
            None => {
                let home_dir = std::env::var("HOME")
                    .ok()
                    .map(PathBuf::from)
                    .or_else(|| user_dirs::home_dir().ok())
                    .ok_or(anyhow!(
                        "Could not determine the path of the user home directory."
                    ))?;
                home_dir.join(".gup") // Changed to .gup
            }
        },
        modifypath_files: find_shell_scripts_to_be_modified(true)
            .with_context(|| "Failed to identify the shell scripts that need to be modified.")?,
    };

    print_install_choices(&install_choices)?;

    println!(
        "You can uninstall at any time with {} and these",
        style("gup self uninstall").bold() // Renamed
    );
    println!("changes will be reverted.");
    println!();

    if !args.disable_confirmation_prompt {
        debug!("Next running the prompt for default choices");

        let answer_default = Select::with_theme(theme.as_ref())
            .with_prompt("Do you want to install with these default configuration choices?")
            .item("Proceed with installation")
            .item("Customize installation")
            .item("Cancel installation")
            .default(0)
            .interact()?;

        trace!("choice is {:?}", answer_default);

        println!();

        if answer_default == 1 {
            debug!("Next running the individual config wizard");

            loop {
                // run_individual_config_wizard might need to be adapted if symlinks choice is removed/changed
                if run_individual_config_wizard(&mut install_choices, theme.as_ref())?.is_none() {
                    return Ok(()); // User cancelled
                }

                print_install_choices(&install_choices)?;

                let confirmcustom = Select::with_theme(theme.as_ref())
                    .with_prompt("Do you want to install with these custom configuration choices?")
                    .item("Proceed with installation")
                    .item("Customize installation")
                    .item("Cancel installation")
                    .default(0)
                    .interact()?;

                trace!("gup home dir is {:?}", install_choices.install_location); // Renamed

                if confirmcustom == 0 {
                    break;
                } else if confirmcustom == 2 {
                    return Ok(());
                }
            }
        } else if answer_default == 2 {
            return Ok(());
        }
    }

    if install_choices.install_location.exists() {
        if ci_context.should_show_banners() {
            println!("You are trying to install gup into the folder"); // Renamed
            println!("`{}`,", install_choices.install_location.display());
            println!("but that folder already exists. Please remove that folder");
            println!("and then start the setup process again.");
        }
        return Ok(());
    }

    if install_choices.modifypath {
        let paths_to_modify = find_shell_scripts_to_be_modified(true)?; // Use true to create .zshrc if needed

        let mut failed_paths: Vec<PathBuf> = Vec::<PathBuf>::new();

        for cur_path in paths_to_modify {
            let file_result = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .create(true) // Ensure file exists if we intend to modify it (e.g. .zshrc on new macOS)
                .open(&cur_path);

            if file_result.is_err() {
                failed_paths.push(cur_path.clone());
            }
        }

        if !failed_paths.is_empty() {
            // Check if vec is not empty
            if ci_context.should_show_banners() {
                println!("gup needs to modify a number of existing files on your"); // Renamed
                println!("system, but is unable to edit some of these files. Most likely");
                println!("this is caused by incorrect permissions on these files. The");
                println!("following files could not be edited:");
                for cur_path in failed_paths {
                    println!("  {}", cur_path.display());
                }
                println!("You can find more help with this problem at");
                println!(
                    "  https://github.com/your-repo/gup/wiki/Permission-problems-during-setup" // Placeholder URL
                );
                println!();
            }
            return Ok(());
        }
    }

    let gup_self_bin = install_choices.install_location.join("bin"); // Renamed

    trace!("Set gup_self_bin to `{:?}`", gup_self_bin); // Renamed

    if ci_context.should_show_banners() {
        println!("Now installing gup"); // Renamed
    }

    // This refers to the old global config path, should be paths.global_config_file()
    if paths.global_config_file().exists() {
        // Updated path
        std::fs::remove_file(paths.global_config_file()).unwrap();
    }

    std::fs::create_dir_all(&gup_self_bin) // Renamed
        .with_context(|| "Failed to create install folder for gup.")?; // Renamed

    // Get the actual target triple for this system
    let gup_target = gup::utils::get_target_triple_id()
        .with_context(|| "Failed to determine target platform for gup installation.")?;

    // Use GitHub releases for gup self-updates
    let gup_server_base_url = "https://github.com/your-org/gup/releases/download/";

    let version = get_own_version().unwrap(); // This should be gup's version from Cargo.toml

    let download_url_path = format!("v{}/gup-{}-{}.tar.gz", version, version, gup_target);

    let new_gup_url = Url::parse(gup_server_base_url)? // Use Url::parse
        .join(&download_url_path)
        .with_context(|| {
            format!(
                "Failed to construct a valid url from '{}' and '{}'.",
                gup_server_base_url, download_url_path
            )
        })?;

    download_extract_sans_parent(&new_gup_url.to_string(), &gup_self_bin, 0)?; // Renamed

    // Create initial GupGlobalConfig
    {
        let initial_gup_config = GupGlobalConfig {
            self_update_channel: Some(args.gup_channel.to_lowercase().to_string()), // Use GupChannel
            background_self_update_interval_minutes: if install_choices.backgroundselfupdate > 0 {
                Some(install_choices.backgroundselfupdate as u64)
            } else {
                None
            },
            managed_projects: Default::default(), // Empty initially
        };

        // The old JuliaupSelfConfig is removed. We write to GupGlobalConfig.
        // let self_config_path = install_choices.install_location.join("juliaupself.json");
        // This should now be paths.global_config_file() which is ~/.gup/global_config.json
        // However, the `paths` object here is the *old* paths before gup is installed.
        // We need to construct the new paths based on install_choices.install_location.
        let new_gup_home = install_choices.install_location.clone();
        let global_config_file_path = new_gup_home.join("global_config.json");

        let mut global_config_file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true) // Ensure it's truncated if it somehow exists
            .open(&global_config_file_path)
            .with_context(|| {
                format!(
                    "Failed to open gup global config file at {}.",
                    global_config_file_path.display()
                )
            })?;

        global_config_file
            .rewind()
            .with_context(|| "Failed to rewind gup global config file for write.")?;

        global_config_file.set_len(0).with_context(|| {
            "Failed to set len to 0 for gup global config file before writing new content."
        })?;

        serde_json::to_writer_pretty(&global_config_file, &initial_gup_config)
            .with_context(|| format!("Failed to write gup global configuration file."))?;

        global_config_file
            .sync_all()
            .with_context(|| "Failed to write gup global config data to disc.")?;

        // Update the `paths` object to reflect the newly chosen install_location for subsequent operations
        // This is tricky because `paths` was initialized with potentially default/old values.
        // For operations like `run_command_config_modifypath`, they need the *correct, new* paths.
        // A better way would be to re-initialize `paths` or pass `install_choices.install_location` directly.
        // For now, we'll assume subsequent commands can derive paths from `install_choices.install_location`
        // or a re-initialized GupGlobalPaths.
        // This part of the original code that mutated `paths.juliaupselfbin` etc. is problematic.
        // Let's create a new GupGlobalPaths for the installed location.
        let installed_paths = GupGlobalPaths {
            guphome: new_gup_home,
            // selfupdate fields might not be fully known here if gup was just extracted
            // but for config commands, guphome is the main one.
            #[cfg(feature = "selfupdate")]
            gupselfhome: Default::default(), // Placeholder, this part of logic needs review for self-update bootstrap
            #[cfg(feature = "selfupdate")]
            gupselfbin: Default::default(), // Placeholder
        };
        paths = installed_paths; // Overwrite paths with the new one based on install_choices
    }

    run_command_config_backgroundselfupdate(
        Some(install_choices.backgroundselfupdate),
        true,
        &paths, // Use the potentially updated paths
    )
    .unwrap();
    run_command_config_startupselfupdate(Some(install_choices.startupselfupdate), true, &paths) // Use the potentially updated paths
        .unwrap();
    if install_choices.modifypath {
        run_command_config_modifypath(Some(install_choices.modifypath), true, &paths).unwrap();
        // Use the potentially updated paths
    }
    // run_command_config_symlinks(Some(install_choices.symlinks), true, &paths).unwrap(); // Symlinks config is Julia-specific
    run_command_selfchannel(Some(args.gup_channel), &paths).unwrap(); // For gup's own channel

    // Adding a default project/channel is not part of gup's bootstrap.
    // Users will add projects via `gup project add ...` later.
    // run_command_add(&args.default_channel, &paths)
    //     .with_context(|| "Failed to run `run_command_add`.")?;
    // run_command_default(&args.default_channel, &paths)
    //     .with_context(|| "Failed to run `run_command_default`.")?;

    // The symlink `julia` to `julialauncher` is Julia-specific.
    // gup will manage its shims/symlinks differently.
    // let symlink_path = gup_self_bin.join("julia");
    // std::os::unix::fs::symlink(gup_self_bin.join("julialauncher"), &symlink_path).with_context(
    //     || {
    //         format!(
    //             "failed to create symlink `{}`.",
    //             symlink_path.to_string_lossy()
    //         )
    //     },
    // )?;

    if ci_context.should_show_banners() {
        println!("gup was successfully installed on your system."); // Renamed
    }

    if install_choices.modifypath && ci_context.should_show_banners() {
        println!();
        println!("Depending on which shell you are using, run one of the following");
        println!(
            "commands to reload the {} environment variable:",
            style("PATH").bold()
        );
        println!();
        for p in &install_choices.modifypath_files {
            println!("  . {}", p.to_string_lossy());
        }
        println!();
    }

    Ok(())
}

#[cfg(not(feature = "selfupdate"))]
pub fn main() -> Result<()> {
    panic!("This should never run.");
}
