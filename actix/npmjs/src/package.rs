use crate::error::Error;
use flate2::read::GzDecoder;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::time::SystemTime;
use tar::Archive;

static PKG_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?x)
        (?P<name>[a-z0-9-]+)
        -
        (?P<version>
            (?P<major>[0-9]+)
            \.
            (?P<minor>[0-9]+)
            \.
            (?P<patch>[0-9]+)
            (?:
                -
                (?P<pre>
                    [A-Za-z0-9-]+
                    (
                        \.
                        [A-Za-z0-9-]+
                    )*
                )
            )?
            (?:
                \+
                (?P<build>
                    [A-Za-z0-9-]+
                    (
                        \.
                        [A-Za-z0-9-]+
                    )*
                )
            )?
        )
        \.tgz
        ",
    )
    .unwrap()
});

pub struct Packages {
    pub root: PathBuf,
    pub files: Vec<Package>,
}

pub struct Package {
    pub path: PathBuf,
    pub filename: String,
    pub name: String,
    pub full_version: String,
    pub version: (u16, u16, u16),
    pub metadata: PackageJson,
    pub created_at: Option<SystemTime>,
    pub updated_at: Option<SystemTime>,
    pub shasum: String,
}

impl PartialOrd for Package {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Package {
    fn cmp(&self, other: &Self) -> Ordering {
        self.version.cmp(&other.version)
    }
}

impl PartialEq for Package {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.full_version == other.full_version
    }
}

impl Eq for Package {}

impl Packages {
    pub fn new(root: &Path) -> Self {
        Packages {
            root: root.to_path_buf(),
            files: vec![],
        }
    }

    pub fn collect(&mut self) -> Result<(), Error> {
        let read_dir = self.root.read_dir()?;
        for entry in read_dir {
            let entry = entry?;
            let filename = entry.file_name().to_owned().to_str().unwrap().to_string();
            let metadata = entry.metadata()?;

            if metadata.is_file()
                && let Some((name, full_version, version)) = is_pkg(&filename)
                && let Some(pkgjson) = read_metadata(&entry.path())?
            {
                let file = Package {
                    path: entry.path(),
                    filename,
                    name,
                    full_version,
                    version,
                    metadata: pkgjson,
                    created_at: metadata.created().ok(),
                    updated_at: metadata.modified().ok(),
                    shasum: "".to_string(),
                };
                self.files.push(file);
            }
        }

        Ok(())
    }
}

fn is_pkg(filename: &str) -> Option<(String, String, (u16, u16, u16))> {
    if let Some(m) = PKG_PATTERN.captures(filename) {
        let name = m.name("name").map(|g| g.as_str().to_string()).unwrap();
        let version = m.name("version").map(|g| g.as_str().to_string()).unwrap();
        let major = m
            .name("major")
            .and_then(|g| g.as_str().parse().ok())
            .unwrap();
        let minor = m
            .name("minor")
            .and_then(|g| g.as_str().parse().ok())
            .unwrap();
        let patch = m
            .name("patch")
            .and_then(|g| g.as_str().parse().ok())
            .unwrap();

        return Some((name, version, (major, minor, patch)));
    }

    None
}

fn read_metadata(path: &Path) -> Result<Option<PackageJson>, Error> {
    let file = File::open(path)?;
    let gzip = GzDecoder::new(file);
    let mut tgz = Archive::new(gzip);

    for entry in tgz.entries()? {
        let entry = entry?;
        if is_metadata(&entry.path()?)
            && let Ok(metadata) = serde_json::from_reader(entry)
        {
            return Ok(Some(metadata));
        }
    }

    Ok(None)
}

fn is_metadata(path: &Path) -> bool {
    path.file_name()
        .and_then(|f| f.to_str())
        .map(|f| f == "package.json")
        .unwrap_or_default()
}

#[derive(Default, Deserialize, Serialize)]
pub struct PackageJson {
    pub name: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keywords: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub homepage: Option<String>,
    // TODO: bugs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<PackageJsonPeople>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contributors: Option<Vec<PackageJsonPeople>>,
    // TODO: funding ... directories
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository: Option<PackageJsonRepository>,
    // TODO: scripts ... config
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dependencies: Option<HashMap<String, String>>,
    #[serde(rename = "devDependencies", skip_serializing_if = "Option::is_none")]
    pub dev_dependencies: Option<HashMap<String, String>>,
    // TODO: peerDependencies ...
}

#[derive(Deserialize, Serialize)]
#[serde(untagged)]
pub enum PackageJsonPeople {
    Line(String),
    Object(PackageJsonPeopleObject),
}

#[derive(Default, Deserialize, Serialize)]
pub struct PackageJsonPeopleObject {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

#[derive(Deserialize, Serialize)]
#[serde(untagged)]
pub enum PackageJsonRepository {
    Line(String),
    Object(PackageJsonRepositoryObject),
}

#[derive(Default, Deserialize, Serialize)]
pub struct PackageJsonRepositoryObject {
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub ty: Option<String>,
    pub url: String,
}
