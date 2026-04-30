use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Default, Deserialize, Serialize)]
pub struct Package {
    #[serde(rename = "dist-tags")]
    pub dist_tags: HashMap<String, String>,
    pub name: String,
    pub time: HashMap<String, String>,
    pub users: HashMap<String, String>,
    pub versions: HashMap<String, PackageVersion>,
    // package.json from the latest version.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<PackageAuthor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub readme: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository: Option<PackageRepository>,
}

#[derive(Default, Deserialize, Serialize)]
pub struct PackageVersion {
    pub name: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dependencies: Option<HashMap<String, String>>,
    #[serde(rename = "devDependencies", skip_serializing_if = "Option::is_none")]
    pub dev_dependencies: Option<HashMap<String, String>>,
    pub dist: PackageDist,
}

#[derive(Default, Deserialize, Serialize)]
pub struct PackageAuthor {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

#[derive(Default, Deserialize, Serialize)]
pub struct PackageRepository {
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub ty: Option<String>,
    pub url: String,
}

#[derive(Default, Deserialize, Serialize)]
pub struct PackageDist {
    pub shasum: String,
    pub tarball: String,
}
