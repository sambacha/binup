use std::{fs::File, io::BufReader};

// use std::fs::File;
// use std::io::BufReader;
// use crate::utils::get_juliaup_home_path;
// Removed get_bundled_dbversion as it's no longer used
use crate::{global_paths::GupGlobalPaths, jsonstructs_versionsdb::ProjectVersionsDB};
use anyhow::{Context, Result};
// use semver::Version; // Version comparison logic removed

// load_vendored_db() is removed as it's not suitable for generic gup.
// Each project should sync its own metadata.

// This function's role needs re-evaluation for generic gup.
// It currently loads a single version DB, which might have been for Juliaup's main DB.
// For gup, version info should primarily come from ProjectMetadata.
// This function might be adapted to load a specific project's cached metadata DB if needed,
// or removed if ProjectMetadata objects are always loaded directly.
// For now, it will attempt to load from paths.versiondb and error if not found/parsable.
pub fn load_versions_db(paths: &GupGlobalPaths) -> Result<ProjectVersionsDB> {
    let version_db_path = &paths.versiondb; // This path itself needs to be project-specific in gup

    let file = File::open(version_db_path).with_context(|| {
        format!(
            "Failed to open version database file at '{}'. This might indicate the project metadata has not been synced yet.",
            version_db_path.display()
        )
    })?;

    let reader = BufReader::new(&file);
    let db: ProjectVersionsDB = serde_json::from_reader(reader).with_context(|| {
        format!(
            "Failed to parse version database file at '{}'.",
            version_db_path.display()
        )
    })?;

    Ok(db)
}
