use anyhow::{Context, Result};
use clap::Parser;
#[allow(unused_imports)] // Used when selfupdate feature is enabled
use gup::cli::ConfigSubCmd;
use gup::cli::{CiContext, GupCli, OverrideSubCmd, ProjectSubCmd, SelfSubCmd};
use gup::command_add::run_command_add; // Added
use gup::command_api::run_command_api;
use gup::command_completions::run_command_completions;
use gup::command_config::run_command_config_show;
use gup::command_config_modifypath::run_command_config_modifypath;
use gup::command_default::run_command_default;
use gup::command_gc::run_command_gc;
use gup::command_info::run_command_info;
use gup::command_link::run_command_link;
use gup::command_list::run_command_list;
use gup::command_override::{
    run_command_override_set, run_command_override_status, run_command_override_unset,
};
use gup::command_project::{
    run_command_project_add, run_command_project_list, run_command_project_remove,
    run_command_project_update_metadata,
};
use gup::command_remove::run_command_remove;
use gup::command_selfupdate::run_command_selfupdate;
use gup::command_status::run_command_status;
use gup::command_update::run_command_update;
use gup::global_paths::get_gup_paths;
use gup::multiplexer::run_multiplexed_command;

#[cfg(feature = "selfupdate")]
use gup::{
    command_config_backgroundselfupdate::run_command_config_backgroundselfupdate,
    command_config_startupselfupdate::run_command_config_startupselfupdate,
    command_selfchannel::run_command_selfchannel, command_selfuninstall::run_command_selfuninstall,
};

#[cfg(not(feature = "selfupdate"))]
use gup::command_selfuninstall::run_command_selfuninstall_unavailable;

use log::info;
use std::env;

fn main() -> Result<()> {
    human_panic::setup_panic!(human_panic::Metadata::new("gup", env!("CARGO_PKG_VERSION"))
        .support("https://github.com/your-repo/gup"));

    let env = env_logger::Env::new()
        .filter("GUP_LOG")
        .write_style("GUP_LOG_STYLE");
    env_logger::init_from_env(env);

    // Check if invoked as a multiplexer symlink
    let current_exe = env::current_exe()?;
    let exe_file_name = current_exe
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default();

    let paths = get_gup_paths().with_context(|| "Trying to load all global paths for gup.")?;

    // If the executable name is not "gup" (or "gup.exe" on Windows),
    // it means it's a symlink for a managed executable.
    if exe_file_name != "gup" && exe_file_name != "gup.exe" {
        let program_args: Vec<String> = env::args().skip(1).collect();
        match run_multiplexed_command(exe_file_name.as_ref(), &program_args, &paths) {
            Ok(exit_code) => {
                std::process::exit(exit_code);
            }
            Err(e) => {
                // Log error or print to stderr before exiting with error
                eprintln!("Error during multiplexed execution: {}", e);
                // Propagate the error to allow human-panic to handle it, or exit with a generic error code
                return Err(e.context("Multiplexed command execution failed"));
            }
        }
    }

    info!("Parsing command line arguments.");
    let args = GupCli::parse();

    // Detect CI environment and create context
    let ci_context = CiContext::detect();

    match args {
        GupCli::Default {
            project_name,
            version_or_channel,
        } => run_command_default(&project_name, &version_or_channel, &paths),
        GupCli::Add {
            project_name,
            version_or_channel,
        } => run_command_add(&project_name, &version_or_channel, &paths),
        GupCli::Remove {
            project_name,
            version_or_link_name,
        } => run_command_remove(&project_name, &version_or_link_name, &paths),
        GupCli::Status { project_name } => run_command_status(project_name.as_deref(), &paths),
        GupCli::Update {
            project_name,
            channel,
        } => run_command_update(project_name.as_deref(), channel.as_deref(), &paths),
        GupCli::Gc {
            project_name,
            prune_linked,
        } => run_command_gc(&project_name, prune_linked, &paths),
        GupCli::Link {
            project_name,
            link_name,
            file_path,
            args,
        } => run_command_link(&project_name, &link_name, &file_path, &args, &paths),
        GupCli::List { project_name } => run_command_list(project_name.as_deref(), &paths),
        GupCli::Config(subcmd) => match subcmd {
            ConfigSubCmd::Show {} => run_command_config_show(&paths),
            #[cfg(feature = "selfupdate")]
            ConfigSubCmd::BackgroundSelfupdateInterval { value } => {
                run_command_config_backgroundselfupdate(value, ci_context.quiet, &paths)
            }
            #[cfg(feature = "selfupdate")]
            ConfigSubCmd::StartupSelfupdateInterval { value } => {
                run_command_config_startupselfupdate(value, ci_context.quiet, &paths)
            }
            ConfigSubCmd::ModifyPath { value } => {
                run_command_config_modifypath(value, ci_context.quiet, &paths)
            }
            // TODO: Consider if a more specific error or handling is needed for other ConfigSubCmd variants
            // if the selfupdate feature is not enabled. For now, this will fall through if not caught by cfg.
            #[allow(unreachable_patterns)]
            // This is to suppress warning if selfupdate is always on/off
            _ => {
                anyhow::bail!("Unsupported config subcommand for this build or feature set.")
            }
        },
        GupCli::Project(subcmd) => match subcmd {
            ProjectSubCmd::Add {
                registration_source,
            } => run_command_project_add(&registration_source, &paths),
            ProjectSubCmd::Remove { project_name } => {
                run_command_project_remove(&project_name, &paths)
            }
            ProjectSubCmd::List {} => run_command_project_list(&paths),
            ProjectSubCmd::UpdateMetadata { project_name } => {
                run_command_project_update_metadata(&project_name, &paths)
            }
        },
        GupCli::Api { command } => run_command_api(&command, &paths),
        GupCli::OverrideSubCmd(subcmd) => match subcmd {
            OverrideSubCmd::Status { project_name } => {
                run_command_override_status(&project_name, &paths)
            }
            OverrideSubCmd::Set {
                project_name,
                version_or_channel,
                path,
            } => run_command_override_set(&project_name, &version_or_channel, path, &paths),
            OverrideSubCmd::Unset {
                project_name,
                nonexistent,
                path,
            } => run_command_override_unset(&project_name, nonexistent, path, &paths),
        },
        GupCli::Info { project_name: _ } => run_command_info(&paths),
        #[cfg(feature = "selfupdate")]
        GupCli::SecretSelfUpdate {} => run_command_selfupdate(None, &paths),
        GupCli::SelfSubCmd(subcmd) => match subcmd {
            SelfSubCmd::Update {} => run_command_selfupdate(None, &paths),
            #[cfg(feature = "selfupdate")]
            SelfSubCmd::Channel { channel } => run_command_selfchannel(channel, &paths),
            #[cfg(feature = "selfupdate")]
            SelfSubCmd::Uninstall {} => run_command_selfuninstall(&paths),
            #[cfg(not(feature = "selfupdate"))]
            SelfSubCmd::Uninstall {} => run_command_selfuninstall_unavailable(),
        },
        GupCli::Completions { shell } => run_command_completions(shell),
    }
}
