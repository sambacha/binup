use anyhow::{anyhow, bail, Context, Result};
use std::path::PathBuf;

pub fn get_bin_dir() -> Result<PathBuf> {
    let entry_sep = if std::env::consts::OS == "windows" {
        ';'
    } else {
        ':'
    };

    let path = match std::env::var("GUP_BIN_DIR") {
        Ok(val) => {
            let path = PathBuf::from(val.split(entry_sep).next().unwrap()); // We can unwrap here because even when we split an empty string we should get a first element.

            if !path.is_absolute() {
                bail!("The `GUP_BIN_DIR` environment variable contains a value that resolves to an an invalid path `{}`.", path.display());
            };

            path
        }
        Err(_) => {
            let mut path = std::env::current_exe()
                .with_context(|| "Could not determine the path of the running exe.")?
                .parent()
                .ok_or_else(|| anyhow!("Could not determine parent."))?
                .to_path_buf();

            let home_dir = std::env::var("HOME")
                .ok()
                .map(PathBuf::from)
                .or_else(|| user_dirs::home_dir().ok());

            if let Some(home_dir) = home_dir {
                if !path.starts_with(&home_dir) {
                    path = home_dir.join(".local").join("bin");

                    if !path.is_absolute() {
                        bail!(
                            "The system returned an invalid home directory path `{}`.",
                            path.display()
                        );
                    };
                }
            }

            path
        }
    };

    Ok(path)
}

pub fn get_arch() -> Result<String> {
    if std::env::consts::ARCH == "x86" {
        return Ok("x86".to_string());
    } else if std::env::consts::ARCH == "x86_64" {
        return Ok("x86_64".to_string());
    } else if std::env::consts::ARCH == "aarch64" {
        return Ok("aarch64".to_string());
    }

    bail!("Running on an unknown arch: {}.", std::env::consts::ARCH)
}

/// Returns a platform identifier string like "x86_64-pc-windows-msvc" or "aarch64-apple-darwin".
///
/// This attempts to be specific for common variations like MUSL libc on Linux or GNU toolchain on Windows.
/// The exact identifiers used will need to align with what `installer-metadata.json` provides.
pub fn get_target_triple_id() -> Result<String> {
    let arch = match std::env::consts::ARCH {
        "x86" => "i686",
        "x86_64" => "x86_64",
        "arm" => "arm", // Note: Further distinction for ARM variants (e.g., armv7, armhf) might be needed if projects require it.
        "aarch64" => "aarch64",
        other => bail!("Unsupported architecture: {}", other),
    };

    let os_env = match std::env::consts::OS {
        "linux" => {
            if cfg!(target_env = "musl") {
                "unknown-linux-musl"
            } else {
                // Default to gnu for linux if not musl.
                // Other envs like android (ndk) could be added if needed.
                "unknown-linux-gnu"
            }
        }
        "macos" => "apple-darwin",
        "windows" => {
            if cfg!(target_env = "gnu") {
                "pc-windows-gnu"
            } else {
                // Default to msvc for windows if not gnu.
                "pc-windows-msvc"
            }
        }
        "freebsd" => "unknown-freebsd",
        other => bail!("Unsupported OS: {}", other),
    };

    Ok(format!("{}-{}", arch, os_env))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_target_triple_id() {
        // This test is environment dependent, but we can check if it produces a non-empty string
        // and matches a general pattern based on compile-time cfgs.
        let triple_result = get_target_triple_id();
        assert!(triple_result.is_ok());
        let triple_str = triple_result.unwrap();
        assert!(!triple_str.is_empty());
        println!("Detected target triple: {}", triple_str); // Useful for seeing what it detects

        // Example checks based on common platforms and environments.
        // Note: Testing all variants (e.g., linux-musl vs linux-gnu) in a single test run
        // is tricky as `target_env` is a compile-time configuration.
        // These checks verify the logic for the environment the tests are compiled in.
        if cfg!(all(
            target_arch = "x86_64",
            target_os = "linux",
            target_env = "gnu"
        )) {
            assert_eq!(triple_str, "x86_64-unknown-linux-gnu");
        } else if cfg!(all(
            target_arch = "x86_64",
            target_os = "linux",
            target_env = "musl"
        )) {
            assert_eq!(triple_str, "x86_64-unknown-linux-musl");
        } else if cfg!(all(target_arch = "aarch64", target_os = "macos")) {
            // macos typically implies darwin env
            assert_eq!(triple_str, "aarch64-apple-darwin");
        } else if cfg!(all(
            target_arch = "x86_64",
            target_os = "windows",
            target_env = "msvc"
        )) {
            assert_eq!(triple_str, "x86_64-pc-windows-msvc");
        } else if cfg!(all(
            target_arch = "x86_64",
            target_os = "windows",
            target_env = "gnu"
        )) {
            assert_eq!(triple_str, "x86_64-pc-windows-gnu");
        }
        // Add more specific checks if running tests in diverse environments.
    }
}
