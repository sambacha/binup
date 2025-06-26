use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize)]
pub struct VersionInfo {
    #[serde(rename = "UrlPath")]
    pub url_path: String,
}

#[derive(Serialize, Deserialize)]
pub struct ChannelInfo {
    #[serde(rename = "Version")]
    pub version: String,
}

#[derive(Serialize, Deserialize)]
pub struct ProjectVersionsDB {
    #[serde(rename = "AvailableVersions")]
    pub available_versions: HashMap<String, VersionInfo>,
    #[serde(rename = "AvailableChannels")]
    pub available_channels: HashMap<String, ChannelInfo>,
    #[serde(rename = "Version")] // This serde rename refers to the JSON field name
    pub db_format_version: String, // Renamed field in Rust struct
}
