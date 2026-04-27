use crate::error::Error;
use regex::Regex;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

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

pub struct Packages {
    pub root: PathBuf,
    pub files: Vec<Package>,
}

pub struct Package {
    pub path: PathBuf,
    pub filename: String,
    pub distribution: String,
    pub version: String,
    pub size: usize,
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

            if metadata.is_file()
                && let Some((distribution, version)) = wheel_distribution(&filename)
            {
                let file = Package {
                    path: entry.path(),
                    filename,
                    distribution,
                    version,
                    size: metadata.size() as usize,
                };
                self.files.push(file);
            }
        }

        Ok(())
    }
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
