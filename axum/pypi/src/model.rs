use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// -----------------------------------------------------------------------------------------------

#[derive(Default, Deserialize, Serialize)]
pub struct SimpleResponse {
    pub meta: SimpleMetadata,
    pub projects: Vec<SimpleProject>,
}

#[derive(Default, Deserialize, Serialize)]
pub struct SimpleMetadata {
    #[serde(rename = "api-versoin")]
    pub api_version: String,
}

#[derive(Default, Deserialize, Serialize)]
pub struct SimpleProject {
    pub name: String,
}

// -----------------------------------------------------------------------------------------------

#[derive(Default, Deserialize, Serialize)]
pub struct ProjectResponse {
    pub meta: ProjectMetadata,
    #[serde(rename = "project-status", skip_serializing_if = "Option::is_none")]
    pub project_status: Option<ProjectStatus>,
    pub files: Vec<ProjectFile>,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub versions: Option<Vec<String>>,
}

#[derive(Default, Deserialize, Serialize)]
pub struct ProjectMetadata {
    #[serde(rename = "api-versoin")]
    pub api_version: String,
}

#[derive(Default, Deserialize, Serialize)]
pub struct ProjectStatus {
    pub status: String,
    pub reason: String,
}

#[derive(Default, Deserialize, Serialize)]
pub struct ProjectFile {
    pub filename: String,
    pub url: String,
    pub hashes: HashMap<String, String>,
    #[serde(rename = "requires-python", skip_serializing_if = "Option::is_none")]
    pub requires_python: Option<String>,
    #[serde(rename = "gpg-sig", skip_serializing_if = "Option::is_none")]
    pub gpg_sig: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub yanked: Option<bool>,
    pub size: usize,
    #[serde(rename = "upload-time", skip_serializing_if = "Option::is_none")]
    pub upload_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provenance: Option<String>,
}
