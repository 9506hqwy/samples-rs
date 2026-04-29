use regex::Regex;
use std::cmp::Ordering;
use std::sync::LazyLock;

static VERSION_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?x)
        v?
        (?:
            (?:(?P<epoch>[0-9]+)!)?
            (?P<release>[0-9]+(?:\.[0-9]+)*)
            (?P<pre>
                [-_\.]?
                (?P<pre_phase>alpha|a|beta|b|preview|pre|c|rc)
                [-_\.]?
                (?P<pre_num>[0-9]+)?
            )?
            (?P<post>
                (?:-(?P<post_num1>[0-9]+))
                |
                (?:
                    [-_\.]?
                    (?P<post_phase>post|rev|r)
                    [-_\.]?
                    (?P<post_num2>[0-9]+)?
                )
            )?
            (?P<dev>
                [-_\.]?
                (?P<dev_phase>dev)
                [-_\.]?
                (?P<dev_num>[0-9]+)?
            )?
        )
        (?:\+(?P<local>[a-z0-9]+(?:[-_\.][a-z0-9]+)*))?
        ",
    )
    .unwrap()
});

#[allow(clippy::derived_hash_with_manual_eq)]
#[derive(Debug, Clone, Eq, Hash)]
pub struct Version {
    raw: String,
    pub identifier: VersionIdentifier,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VersionIdentifier {
    pub epoch: u16,
    pub release: Vec<u16>,
    pub pre: (VersionPrePhase, u16),
    pub post: Option<(Option<String>, u16)>,
    pub dev: (VersionDevPhase, u16),
    pub local: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum VersionPrePhase {
    Alpha,
    Beta,
    Preview,
    Release, // for ordering only
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum VersionDevPhase {
    Dev,
    Release, // for ordering only
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialOrd for VersionIdentifier {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        self.identifier.cmp(&other.identifier)
    }
}

impl Ord for VersionIdentifier {
    fn cmp(&self, other: &Self) -> Ordering {
        let cmp = self.epoch.cmp(&other.epoch);
        if cmp != Ordering::Equal {
            return cmp;
        }

        let cmp = self.release.cmp(&other.release);
        if cmp != Ordering::Equal {
            return cmp;
        }

        let cmp = self.pre.cmp(&other.pre);
        if cmp != Ordering::Equal {
            if !self.is_pre() && self.post.is_none() && self.is_dev() {
                // "1.dev1" vs "1.a1" vs "1.post1.dev1"
                return Ordering::Less;
            }

            return cmp;
        }

        let cmp = self.post.cmp(&other.post);
        if cmp != Ordering::Equal {
            return cmp;
        }

        let cmp = self.dev.cmp(&other.dev);
        if cmp != Ordering::Equal {
            return cmp;
        }

        Ordering::Equal
    }
}

impl PartialEq for Version {
    fn eq(&self, other: &Self) -> bool {
        self.identifier == other.identifier
    }
}

impl Version {
    pub fn new(version: &str) -> Option<Self> {
        if let Some(m) = VERSION_PATTERN.captures(version) {
            let epoch = m
                .name("epoch")
                .map(|f| f.as_str().parse().unwrap())
                .unwrap_or_default();

            let release = m
                .name("release")
                .map(|f| f.as_str().split(".").map(|f| f.parse().unwrap()).collect())
                .unwrap_or_default();

            let pre = if let Some(pre_phase) = m.name("pre_phase").map(|f| f.as_str().to_owned()) {
                let pre_phase = match pre_phase.as_str() {
                    "alpha" | "a" => VersionPrePhase::Alpha,
                    "beta" | "b" => VersionPrePhase::Beta,
                    "preview" | "pre" | "c" | "rc" => VersionPrePhase::Preview,
                    _ => panic!("{pre_phase}"),
                };
                let pre_num = m
                    .name("pre_num")
                    .map(|f| f.as_str().parse().unwrap())
                    .unwrap();
                (pre_phase, pre_num)
            } else {
                (VersionPrePhase::Release, 0)
            };

            let post = if let Some(post_phase) = m.name("post_phase").map(|f| f.as_str().to_owned())
            {
                let post_num2 = m
                    .name("post_num2")
                    .map(|f| f.as_str().parse().unwrap())
                    .unwrap();
                Some((Some(post_phase), post_num2))
            } else {
                m.name("post_num1")
                    .map(|f| f.as_str().parse().unwrap())
                    .map(|post_num1| (None, post_num1))
            };

            let dev = if let Some(dev_phase) = m.name("dev_phase").map(|f| f.as_str().to_owned()) {
                let dev_phase = match dev_phase.as_str() {
                    "dev" => VersionDevPhase::Dev,
                    _ => panic!("{dev_phase}"),
                };

                let dev_num = m
                    .name("dev_num")
                    .map(|f| f.as_str().parse().unwrap())
                    .unwrap();
                (dev_phase, dev_num)
            } else {
                (VersionDevPhase::Release, 0)
            };

            let local = m.name("local").map(|f| f.as_str().to_owned());

            Some(Version {
                raw: version.to_owned(),
                identifier: VersionIdentifier {
                    epoch,
                    release,
                    pre,
                    post,
                    dev,
                    local,
                },
            })
        } else {
            None
        }
    }

    pub fn as_str(&self) -> &str {
        &self.raw
    }
}

impl VersionIdentifier {
    fn is_pre(&self) -> bool {
        self.pre.0 != VersionPrePhase::Release
    }

    fn is_dev(&self) -> bool {
        self.dev.0 != VersionDevPhase::Release
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sort_epoch() {
        let mut versions = vec![Version::new("2!1"), Version::new("3"), Version::new("1!2")];

        versions.sort();

        let expected = vec![Version::new("3"), Version::new("1!2"), Version::new("2!1")];

        assert_eq!(expected, versions);
    }

    #[test]
    fn test_sort_release() {
        let mut versions = vec![
            Version::new("2.0"),
            Version::new("1.0.1"),
            Version::new("2.0.0"),
            Version::new("1.0"),
            Version::new("1.1"),
        ];

        versions.sort();

        let expected = vec![
            Version::new("1.0"),
            Version::new("1.0.1"),
            Version::new("1.1"),
            Version::new("2.0"),
            Version::new("2.0.0"),
        ];

        assert_eq!(expected, versions);
    }

    #[test]
    fn test_sort_pre() {
        let mut versions = vec![
            Version::new("1.1a1"),
            Version::new("1"),
            Version::new("1.1"),
            Version::new("1rc0"),
            Version::new("1a1"),
            Version::new("1a10"),
            Version::new("1b2"),
        ];

        versions.sort();

        let expected = vec![
            Version::new("1a1"),
            Version::new("1a10"),
            Version::new("1b2"),
            Version::new("1rc0"),
            Version::new("1"),
            Version::new("1.1a1"),
            Version::new("1.1"),
        ];

        assert_eq!(expected, versions);
    }

    #[test]
    fn test_sort_post() {
        let mut versions = vec![
            Version::new("1.1"),
            Version::new("1.post1"),
            Version::new("1"),
            Version::new("1.post10"),
            Version::new("1-1"),
            Version::new("1.1a1.post2"),
            Version::new("1.1a1.post1"),
            Version::new("1.1a2"),
            Version::new("1.1a1"),
        ];

        versions.sort();

        let expected = vec![
            Version::new("1"),
            Version::new("1-1"),
            Version::new("1.post1"),
            Version::new("1.post10"),
            Version::new("1.1a1"),
            Version::new("1.1a1.post1"),
            Version::new("1.1a1.post2"),
            Version::new("1.1a2"),
            Version::new("1.1"),
        ];

        assert_eq!(expected, versions);
    }

    #[test]
    fn test_sort_dev() {
        let mut versions = vec![
            Version::new("1.0a1"),
            Version::new("1.dev0"),
            Version::new("1.0a2.dev456"),
            Version::new("1.0.dev456"),
            Version::new("1.0b1.dev456"),
            Version::new("1.0a12.dev456"),
            Version::new("1.0a12"),
            Version::new("1.0b2"),
            Version::new("1.0rc1.dev456"),
            Version::new("1.0b2.post345.dev456"),
            Version::new("1.0b2.post345"),
            Version::new("1.0rc1"),
            Version::new("1.1.dev1"),
            Version::new("1.0"),
            Version::new("1.0.post456"),
            Version::new("1.0.15"),
            Version::new("1.0.post456.dev34"),
        ];

        versions.sort();

        let expected = vec![
            Version::new("1.dev0"),
            Version::new("1.0.dev456"),
            Version::new("1.0a1"),
            Version::new("1.0a2.dev456"),
            Version::new("1.0a12.dev456"),
            Version::new("1.0a12"),
            Version::new("1.0b1.dev456"),
            Version::new("1.0b2"),
            Version::new("1.0b2.post345.dev456"),
            Version::new("1.0b2.post345"),
            Version::new("1.0rc1.dev456"),
            Version::new("1.0rc1"),
            Version::new("1.0"),
            Version::new("1.0.post456.dev34"),
            Version::new("1.0.post456"),
            Version::new("1.0.15"),
            Version::new("1.1.dev1"),
        ];

        assert_eq!(expected, versions);
    }
}
