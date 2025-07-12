use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProjectDefinition {
    pub unique_name: String,
    pub display_name: String,
    pub homepage_url: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SourceOfTruth {
    #[serde(rename = "type")]
    pub source_type: String, // Expected to be "remote_json" for now
    pub url: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProjectRegistration {
    // Using `serde(alias = ...)` for flexibility if the TOML key changes slightly,
    // but direct match is preferred for new definitions.
    pub project_definition: ProjectDefinition,
    pub source_of_truth: SourceOfTruth,
}
