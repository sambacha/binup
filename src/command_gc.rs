use crate::global_paths::GupGlobalPaths;
use crate::operations_gc::garbage_collect_versions;
// use crate::project_metadata_m::ProjectMetadata; // Unused as type, metadata is passed to operations_gc
use crate::global_config_manager::load_global_config;
use crate::operations_metadata::sync_project_metadata;
use crate::project_state_manager::{load_project_local_state, save_project_local_state};
// use crate::state_config::{GupGlobalConfig, ProjectLocalState}; // Unused as types

use anyhow::{anyhow, bail, Context, Result};

pub fn run_command_gc(
    project_name: &str,
    prune_linked_paths: bool,
    paths: &GupGlobalPaths,
) -> Result<()> {
    // 1. Load GupGlobalConfig (to ensure project is managed)
    let global_config = load_global_config(paths)?;
    if !global_config.managed_projects.contains_key(project_name) {
        bail!("Project '{}' is not managed by gup.", project_name);
    }

    // 2. Load ProjectMetadata (needed by garbage_collect_versions to understand channel resolutions)
    //    We need the source_of_truth_url from global_config for sync_project_metadata
    let project_info = global_config
        .managed_projects
        .get(project_name)
        .ok_or_else(|| {
            anyhow!(
                "Project '{}' not found in global config despite earlier check.",
                project_name
            )
        })?;
    let source_url = url::Url::parse(&project_info.source_of_truth_url).with_context(|| {
        format!(
            "Failed to parse source_of_truth_url for project '{}': '{}'",
            project_name, project_info.source_of_truth_url
        )
    })?;
    let project_metadata = sync_project_metadata(project_name, &source_url, paths)?;

    // 3. Load ProjectLocalState (this will be modified)
    let mut project_state = load_project_local_state(project_name, paths)?;

    // 4. Call the refactored operations::garbage_collect_versions
    garbage_collect_versions(
        project_name,
        prune_linked_paths,
        &project_metadata,
        &mut project_state,
        paths,
    )?;

    // 5. Save the modified ProjectLocalState
    save_project_local_state(project_name, &project_state, paths)?;

    Ok(())
}
