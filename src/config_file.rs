use anyhow::{anyhow, bail, Context, Result};
use chrono::{DateTime, Utc};
use cluFlock::{ExclusiveFlock, FlockLock, SharedFlock};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{BufReader, ErrorKind, Seek, SeekFrom};

use crate::global_paths::GupGlobalPaths; // Changed GlobalPaths to GupGlobalPaths

fn is_default<T: Default + PartialEq>(t: &T) -> bool {
    t == &T::default()
}

fn default_versionsdb_update_interval() -> i64 {
    1440
}

fn is_default_versionsdb_update_interval(i: &i64) -> bool {
    *i == default_versionsdb_update_interval()
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
/// Configuration for a specific installed version.
pub struct GupConfigVersion {
    #[serde(rename = "Path")]
    pub path: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
#[serde(untagged)]
/// Configuration for different types of channels (direct download, system, linked).
pub enum GupConfigChannel {
    DirectDownloadChannel {
        #[serde(rename = "Path")]
        path: String,
        #[serde(rename = "Url")]
        url: String,
        #[serde(rename = "LocalETag")]
        local_etag: String,
        #[serde(rename = "ServerETag")]
        server_etag: String,
        #[serde(rename = "Version")]
        version: String,
    },
    SystemChannel {
        #[serde(rename = "Version")]
        version: String,
    },
    LinkedChannel {
        #[serde(rename = "Command")]
        command: String,
        #[serde(rename = "Args")]
        args: Option<Vec<String>>,
    },
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
/// Global settings for GUP configuration.
pub struct GupConfigSettings {
    #[serde(
        rename = "CreateChannelSymlinks",
        default,
        skip_serializing_if = "is_default"
    )]
    pub create_channel_symlinks: bool,
    #[serde(
        rename = "VersionsDbUpdateInterval",
        default = "default_versionsdb_update_interval",
        skip_serializing_if = "is_default_versionsdb_update_interval"
    )]
    pub versionsdb_update_interval: i64,
}

impl Default for GupConfigSettings {
    fn default() -> Self {
        GupConfigSettings {
            create_channel_symlinks: false,
            versionsdb_update_interval: default_versionsdb_update_interval(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
/// Directory-specific version override.
pub struct GupOverride {
    #[serde(rename = "Path")]
    pub path: String,
    #[serde(rename = "Channel")]
    pub channel: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
/// Main GUP configuration structure.
pub struct GupConfig {
    #[serde(rename = "Default")]
    pub default: Option<String>,
    #[serde(rename = "InstalledVersions")]
    pub installed_versions: HashMap<String, GupConfigVersion>,
    #[serde(rename = "InstalledChannels")]
    pub installed_channels: HashMap<String, GupConfigChannel>,
    #[serde(rename = "Settings", default)]
    pub settings: GupConfigSettings,
    #[serde(rename = "Overrides", default)]
    pub overrides: Vec<GupOverride>,
    #[serde(
        rename = "LastVersionDbUpdate",
        skip_serializing_if = "Option::is_none"
    )]
    pub last_version_db_update: Option<DateTime<Utc>>,
}

#[cfg(feature = "selfupdate")]
#[derive(Serialize, Deserialize, Clone)]
/// Self-update configuration for GUP.
pub struct GupSelfConfig {
    #[serde(
        rename = "BackgroundSelfUpdateInterval",
        skip_serializing_if = "Option::is_none"
    )]
    pub background_selfupdate_interval: Option<i64>,
    #[serde(
        rename = "StartupSelfUpdateInterval",
        skip_serializing_if = "Option::is_none"
    )]
    pub startup_selfupdate_interval: Option<i64>,
    #[serde(rename = "ModifyPath", default, skip_serializing_if = "is_default")]
    pub modify_path: bool,
    #[serde(rename = "GupChannel", skip_serializing_if = "Option::is_none")]
    pub gup_channel: Option<String>,
    #[serde(rename = "LastSelfUpdate", skip_serializing_if = "Option::is_none")]
    pub last_selfupdate: Option<DateTime<Utc>>,
}

/// Writable configuration file handle with locking.
pub struct GupConfigFile {
    pub file: File,
    pub lock: FlockLock<File>,
    pub data: GupConfig,
    #[cfg(feature = "selfupdate")]
    pub self_file: File,
    #[cfg(feature = "selfupdate")]
    pub self_data: GupSelfConfig,
}

/// Read-only configuration file handle.
pub struct GupReadonlyConfigFile {
    pub data: GupConfig,
    #[cfg(feature = "selfupdate")]
    pub self_data: GupSelfConfig,
}

pub fn get_read_lock(paths: &GupGlobalPaths) -> Result<FlockLock<File>> {
    std::fs::create_dir_all(paths.guphome()) // Use method
        .with_context(|| "Could not create gup home folder.")?;

    let lock_file = match OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(paths.lock_file()) // Use method
    {
        Ok(file) => file,
        Err(e) => return Err(anyhow!("Could not create lockfile: {}.", e)),
    };

    let file_lock = match SharedFlock::try_lock(lock_file) {
        Ok(lock) => lock,
        Err(e) => {
            eprintln!(
                "GUP configuration is locked by another process, waiting for it to unlock."
            );

            SharedFlock::wait_lock(e.into()).unwrap()
        }
    };

    return Ok(file_lock);
}

pub fn load_config_db(
    paths: &GupGlobalPaths,
    existing_lock: Option<&FlockLock<File>>,
) -> Result<GupReadonlyConfigFile> {
    let mut file_lock: Option<FlockLock<File>> = None;

    if existing_lock.is_none() {
        file_lock = Some(get_read_lock(paths)?);
    }

    let v = match std::fs::OpenOptions::new()
        .read(true)
        .open(paths.global_config_file()) // Use method
    {
        Ok(file) => {
            let reader = BufReader::new(&file);

            serde_json::from_reader(reader).with_context(|| {
                format!(
                    "Failed to parse configuration file '{:?}' for reading.",
                    paths.global_config_file() // Use method
                )
            })?
        }
        Err(error) => match error.kind() {
            ErrorKind::NotFound => GupConfig {
                default: None,
                installed_versions: HashMap::new(),
                installed_channels: HashMap::new(),
                overrides: Vec::new(),
                settings: GupConfigSettings {
                    create_channel_symlinks: false,
                    versionsdb_update_interval: default_versionsdb_update_interval(),
                },
                last_version_db_update: None,
            },
            other_error => {
                bail!(
                    "Problem opening the file {:?}: {:?}",
                    paths.global_config_file(), // Use method
                    other_error
                )
            }
        },
    };

    #[cfg(feature = "selfupdate")]
    let selfconfig: GupSelfConfig;
    #[cfg(feature = "selfupdate")]
    {
        selfconfig = match std::fs::OpenOptions::new()
            .read(true)
            .open(paths.gupselfconfig_file()) // Use method
        {
            Ok(file) => {
                let reader = BufReader::new(&file);

                serde_json::from_reader(reader).with_context(|| {
                    format!(
                        "Failed to parse self configuration file '{:?}' for reading.",
                        paths.gupselfconfig_file() // Use method
                    )
                })?
            }
            Err(error) => bail!(
                "Could not open self configuration file {:?}: {:?}",
                paths.gupselfconfig_file(), // Use method
                error
            ),
        };
    }

    if let Some(file_lock) = file_lock {
        file_lock
            .unlock()
            .with_context(|| "Failed to unlock configuration file.")?;
    }

    Ok(GupReadonlyConfigFile {
        data: v,
        #[cfg(feature = "selfupdate")]
        self_data: selfconfig,
    })
}

/// Load a mutable configuration database with exclusive locking.
pub fn load_mut_config_db(paths: &GupGlobalPaths) -> Result<GupConfigFile> {
    std::fs::create_dir_all(paths.guphome()) // Use method
        .with_context(|| "Could not create gup home folder.")?;

    let lock_file = match OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(paths.lock_file()) // Use method
    {
        Ok(file) => file,
        Err(e) => return Err(anyhow!("Could not create lockfile: {}.", e)),
    };

    let file_lock = match ExclusiveFlock::try_lock(lock_file) {
        Ok(lock) => lock,
        Err(e) => {
            eprintln!(
                "GUP configuration is locked by another process, waiting for it to unlock."
            );

            ExclusiveFlock::wait_lock(e.into()).unwrap()
        }
    };

    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(paths.global_config_file()) // Use method
        .with_context(|| "Failed to open gup config file.")?;

    let stream_len = file
        .seek(SeekFrom::End(0))
        .with_context(|| "Failed to determine the length of the configuration file.")?;

    let data = match stream_len {
        0 => {
            let new_config = GupConfig {
                default: None,
                installed_versions: HashMap::new(),
                installed_channels: HashMap::new(),
                overrides: Vec::new(),
                settings: GupConfigSettings {
                    create_channel_symlinks: false,
                    versionsdb_update_interval: default_versionsdb_update_interval(),
                },
                last_version_db_update: None,
            };

            serde_json::to_writer_pretty(&file, &new_config)
                .with_context(|| "Failed to write configuration file.")?;

            file.sync_all()
                .with_context(|| "Failed to write configuration data to disc.")?;

            file.rewind()
                .with_context(|| "Failed to rewind config file after initial write of data.")?;

            new_config
        }
        _ => {
            file.rewind()
                .with_context(|| "Failed to rewind existing config file.")?;

            let reader = BufReader::new(&file);

            serde_json::from_reader(reader)
                .with_context(|| "Failed to parse configuration file.")?
        }
    };

    #[cfg(feature = "selfupdate")]
    let mut self_file: File; // Made self_file mutable here
    #[cfg(feature = "selfupdate")]
    let self_data: GupSelfConfig;
    #[cfg(feature = "selfupdate")]
    {
        self_file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true) // Ensure self_config_file is created if it doesn't exist
            .open(paths.gupselfconfig_file()) // Use method
            .with_context(|| "Failed to open gup self-config file.")?;

        // Handle empty or new self_config_file
        let self_stream_len = self_file
            .seek(SeekFrom::End(0))
            .with_context(|| "Failed to determine length of self-config file.")?;

        self_data = if self_stream_len == 0 {
            let new_self_config = GupSelfConfig {
                // Provide default values
                background_selfupdate_interval: None,
                startup_selfupdate_interval: None,
                modify_path: false,    // Default for modify_path
                gup_channel: None,
                last_selfupdate: None,
            };
            serde_json::to_writer_pretty(&self_file, &new_self_config)
                .with_context(|| "Failed to write initial self-config file.")?;
            self_file
                .sync_all()
                .with_context(|| "Failed to sync initial self-config data.")?;
            self_file
                .rewind()
                .with_context(|| "Failed to rewind self-config after initial write.")?;
            new_self_config
        } else {
            self_file
                .rewind()
                .with_context(|| "Failed to rewind existing self-config file.")?;
            let reader = BufReader::new(&self_file);
            serde_json::from_reader(reader).with_context(|| {
                format!(
                    "Failed to parse self configuration file '{:?}' for reading.",
                    paths.gupselfconfig_file() // Use method
                )
            })?
        };
    }

    let result = GupConfigFile {
        file,
        lock: file_lock,
        data,
        #[cfg(feature = "selfupdate")]
        self_file: self_file,
        #[cfg(feature = "selfupdate")]
        self_data: self_data,
    };

    Ok(result)
}

/// Save the configuration database to disk.
pub fn save_config_db(gup_config_file: &mut GupConfigFile) -> Result<()> {
    gup_config_file
        .file
        .rewind()
        .with_context(|| "Failed to rewind config file for write.")?;

    gup_config_file
        .file
        .set_len(0)
        .with_context(|| "Failed to set len to 0 for config file before writing new content.")?;

    serde_json::to_writer_pretty(&gup_config_file.file, &gup_config_file.data)
        .with_context(|| "Failed to write configuration file.")?;

    gup_config_file
        .file
        .sync_all()
        .with_context(|| "Failed to write config data to disc.")?;

    #[cfg(feature = "selfupdate")]
    {
        gup_config_file
            .self_file
            .rewind()
            .with_context(|| "Failed to rewind self config file for write.")?;

        gup_config_file.self_file.set_len(0).with_context(|| {
            "Failed to set len to 0 for self config file before writing new content."
        })?;

        serde_json::to_writer_pretty(
            &gup_config_file.self_file,
            &gup_config_file.self_data,
        )
        .with_context(|| format!("Failed to write self configuration file."))?;

        gup_config_file
            .self_file
            .sync_all()
            .with_context(|| "Failed to write config data to disc.")?;
    }

    Ok(())
}
