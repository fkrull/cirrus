#[derive(Debug, PartialEq, Eq, Default)]
pub struct DaemonConfig {
    pub desktop: Desktop,
    pub versions: Versions,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Desktop {
    pub status_icon: bool,
    pub notifications: DesktopNotifications,
}

impl Default for Desktop {
    fn default() -> Self {
        Desktop {
            status_icon: true,
            notifications: Default::default(),
        }
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Versions {
    pub restic_version: String,
}

#[derive(Debug, PartialEq, Eq)]
pub struct DesktopNotifications {
    pub started: bool,
    pub success: bool,
    pub failure: bool,
}

impl Default for DesktopNotifications {
    fn default() -> Self {
        DesktopNotifications {
            started: false,
            success: false,
            failure: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_get_default_config() {
        let daemon_config = DaemonConfig::default();

        assert_eq!(
            daemon_config,
            DaemonConfig {
                desktop: Desktop {
                    status_icon: true,
                    notifications: DesktopNotifications {
                        started: false,
                        success: false,
                        failure: true
                    }
                },
                versions: Versions {
                    restic_version: String::new()
                }
            }
        );
    }
}
