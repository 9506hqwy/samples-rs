use crate::error::Error;
use crate::version::Version;
use regex::Regex;
use std::collections::HashMap;
use std::fs::Metadata;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::time::SystemTime;

static SDIST_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?x)
        (?P<distribution>[^-]+)
        -(?P<version>[^-]+)
        \.tar\.gz
        ",
    )
    .unwrap()
});

static WHEEL_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?x)
        (?P<distribution>[^-]+)
        -(?P<version>[^-]+)
        (-(?P<build_tag>\d.*))?
        -(?P<python_tag>[^-]+)
        -(?P<abi_tag>[^-]+)
        -(?P<platform_tag>[^-]+)
        \.whl
        ",
    )
    .unwrap()
});

pub enum PackageType {
    Sdist,
    Wheel,
}

pub struct Packages {
    pub root: PathBuf,
    pub files: Vec<Package>,
}

pub struct Package {
    pub path: PathBuf,
    pub filename: String,
    pub distribution: String,
    pub version: Version,
    pub size: usize,
    pub created_at: Option<SystemTime>,
    pub updated_at: Option<SystemTime>,
    pub hashes: HashMap<String, String>,
    pub ty: PackageType,
}

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

            if metadata.is_file() {
                if let Some((distribution, version)) = wheel_distribution(&filename) {
                    let file = Package::new(
                        &entry.path(),
                        &filename,
                        &distribution,
                        &version,
                        &metadata,
                        PackageType::Wheel,
                    );
                    self.files.push(file);
                } else if let Some((distribution, version)) = sdist_distribution(&filename) {
                    let file = Package::new(
                        &entry.path(),
                        &filename,
                        &distribution,
                        &version,
                        &metadata,
                        PackageType::Sdist,
                    );
                    self.files.push(file);
                }
            }
        }

        Ok(())
    }
}

impl Package {
    fn new(
        path: &Path,
        filename: &str,
        distribution: &str,
        version: &str,
        metadata: &Metadata,
        ty: PackageType,
    ) -> Self {
        Package {
            path: path.to_path_buf(),
            filename: filename.to_owned(),
            distribution: distribution.to_owned(),
            version: Version::new(version).unwrap(),
            size: metadata.size() as usize,
            created_at: metadata.created().ok(),
            updated_at: metadata.modified().ok(),
            hashes: HashMap::new(),
            ty,
        }
    }
}

fn sdist_distribution(filename: &str) -> Option<(String, String)> {
    if let Some(m) = SDIST_PATTERN.captures(filename) {
        let distribution = m
            .name("distribution")
            .map(|g| g.as_str().to_string())
            .unwrap();
        let version = m.name("version").map(|g| g.as_str().to_string()).unwrap();
        return Some((distribution, version));
    }

    None
}

fn wheel_distribution(filename: &str) -> Option<(String, String)> {
    if let Some(m) = WHEEL_PATTERN.captures(filename) {
        let distribution = m
            .name("distribution")
            .map(|g| g.as_str().to_string())
            .unwrap();
        let version = m.name("version").map(|g| g.as_str().to_string()).unwrap();
        return Some((distribution, version));
    }

    None
}
