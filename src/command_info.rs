use crate::global_config_manager::load_global_config;
use crate::global_paths::GupGlobalPaths;
use anyhow::{Context, Result};
use cli_table::{
    format::{Border, Separator}, // Corrected path
    print_stdout,
    Table,
    WithTitle,
};
use human_sort::compare;
use itertools::Itertools;

#[derive(Table)]
struct ManagedProjectInfoRow {
    #[table(title = "Project Name")]
    name: String,
    #[table(title = "Display Name")]
    display_name: String,
    #[table(title = "Metadata Source URL")]
    source_url: String,
}

pub fn run_command_info(paths: &GupGlobalPaths) -> Result<()> {
    println!("gup version: {}", crate::get_own_version()?);
    println!(
        "Platform triplet: {}",
        crate::utils::get_target_triple_id()?
    );
    println!(
        "Global configuration file: {}",
        paths.global_config_file().display()
    );
    println!("gup home directory: {}", paths.guphome().display());

    println!("\n--- Managed Projects ---");

    let global_config =
        load_global_config(paths).with_context(|| "Failed to load global gup configuration.")?;

    if global_config.managed_projects.is_empty() {
        println!("No projects are currently managed by gup.");
        println!("Use `gup project add <registration_file_or_url>` to add one.");
    } else {
        let project_rows: Vec<_> = global_config
            .managed_projects
            .values()
            .map(|p_info| ManagedProjectInfoRow {
                name: p_info.unique_name.clone(),
                display_name: p_info.display_name.clone(),
                source_url: p_info.source_of_truth_url.clone(),
            })
            .sorted_by(|a, b| compare(&a.name, &b.name))
            .collect();

        let border = Border::builder().build();
        let separator = Separator::builder().build();

        print_stdout(
            project_rows
                .with_title()
                .border(border)
                .separator(separator),
        )?;
    }

    Ok(())
}
