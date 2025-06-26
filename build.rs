extern crate itertools;
extern crate semver;
extern crate serde;
extern crate serde_json;
#[cfg(windows)]
extern crate winres;

use anyhow::Result;
// Removed unused imports:
// use std::env;
// use std::path::Path;
// use std::path::PathBuf;

fn main() -> Result<()> {
    // This file previously contained Julia-specific build logic,
    // such as copying version databases and embedding Julia-specific constants.
    // For `gup`, these are no longer needed as project metadata is synced dynamically.

    // The only remaining generic build-time information is handled by the `built` crate.
    built::write_built_file().expect("Failed to acquire build-time information");

    #[cfg(windows)]
    {
        let mut res = winres::WindowsResource::new();
        // Icon can be added here if needed:
        // res.set_icon("src/gup.ico");

        // The winpkgidentityext feature and app.manifest were specific to Juliaup's MSIX packaging.
        // This is removed for a generic updater.
        // #[cfg(feature = "winpkgidentityext")]
        // res.set_manifest_file("deploy/winpkgidentityext/app.manifest");

        res.compile().unwrap();
    }

    Ok(())
}
