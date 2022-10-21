pub mod config;
pub mod index;
pub mod restic;
pub mod schedule;
pub mod secrets;

const fn filter_empty(s: Option<&str>) -> Option<&str> {
    match s {
        Some(v) if !v.is_empty() => Some(v),
        _ => None,
    }
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct Version {
    pub version: &'static str,
    pub build_string: Option<&'static str>,
    pub target: Option<&'static str>,
}

impl Version {
    const fn new(
        version: Option<&'static str>,
        build_string: Option<&'static str>,
        target: Option<&'static str>,
    ) -> Option<Self> {
        match version {
            Some(version) => Some(Version {
                version,
                build_string,
                target,
            }),
            _ => None,
        }
    }

    const fn from_env() -> Option<Self> {
        let version = filter_empty(option_env!("CIRRUS_VERSION"));
        let build_string = filter_empty(option_env!("CIRRUS_BUILD_STRING"));
        let target = filter_empty(option_env!("CIRRUS_TARGET"));
        Version::new(version, build_string, target)
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.version)?;
        if let Some(build_string) = &self.build_string {
            write!(f, " (build {})", build_string)?;
        }
        if let Some(target) = &self.target {
            write!(f, " on {}", target)?;
        }
        Ok(())
    }
}

pub const VERSION: Option<Version> = Version::from_env();

#[cfg(test)]
mod tests {
    use super::*;

    mod filter_empty {
        use super::*;

        #[test]
        fn test_empty() {
            let result = filter_empty(Some(""));
            assert_eq!(result, None)
        }

        #[test]
        fn test_non_empty() {
            let result = filter_empty(Some("non"));
            assert_eq!(result, Some("non"));
        }
    }

    mod version {
        use super::*;

        #[test]
        fn no_version() {
            let version = Version::new(None, Some("build"), Some("tgt"));
            assert_eq!(version, None);
        }

        #[test]
        fn only_version() {
            let version = Version::new(Some("version"), None, None);
            assert_eq!(
                version.unwrap(),
                Version {
                    version: "version",
                    build_string: None,
                    target: None
                }
            )
        }

        #[test]
        fn version_with_build_string() {
            let version = Version::new(Some("version"), Some("build"), None);
            assert_eq!(
                version.unwrap(),
                Version {
                    version: "version",
                    build_string: Some("build"),
                    target: None
                }
            )
        }

        #[test]
        fn version_with_build_string_and_target() {
            let version = Version::new(Some("version"), Some("build"), Some("tgt"));
            assert_eq!(
                version.unwrap(),
                Version {
                    version: "version",
                    build_string: Some("build"),
                    target: Some("tgt")
                }
            )
        }

        #[test]
        fn should_format_version() {
            let version = Version {
                version: "11.3.4",
                build_string: None,
                target: None,
            };
            assert_eq!(&format!("{}", version), "11.3.4");
        }

        #[test]
        fn should_format_version_and_build_string() {
            let version = Version {
                version: "11.3.4",
                build_string: Some("2021.09.19"),
                target: None,
            };
            assert_eq!(&format!("{}", version), "11.3.4 (build 2021.09.19)");
        }

        #[test]
        fn should_format_version_and_target() {
            let version = Version {
                version: "11.3.4",
                build_string: None,
                target: Some("aarch64-unknown-linux-gnu"),
            };
            assert_eq!(
                &format!("{}", version),
                "11.3.4 on aarch64-unknown-linux-gnu"
            );
        }

        #[test]
        fn should_format_version_and_build_string_and_target() {
            let version = Version {
                version: "11.3.4",
                build_string: Some("2021.09.19"),
                target: Some("aarch64-unknown-linux-gnu"),
            };
            assert_eq!(
                &format!("{}", version),
                "11.3.4 (build 2021.09.19) on aarch64-unknown-linux-gnu"
            );
        }
    }
}
