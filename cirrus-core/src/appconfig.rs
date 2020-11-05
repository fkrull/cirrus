#[derive(Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
#[serde(rename_all = "kebab-case")]
pub struct AppConfig {
    pub restic_binary: String,
    pub daemon: Daemon,
}

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig {
            restic_binary: "restic".to_owned(),
            daemon: Default::default(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
#[serde(default)]
#[serde(rename_all = "kebab-case")]
pub struct Daemon {
    pub desktop: Desktop,
}

#[derive(Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
#[serde(default)]
#[serde(rename_all = "kebab-case")]
pub struct Desktop {
    pub status_icon: StatusIcon,
    pub notifications: DesktopNotifications,
}

#[derive(Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
#[serde(rename_all = "kebab-case")]
pub struct StatusIcon {
    pub enabled: bool,
    pub show_when_idle: bool,
}

impl Default for StatusIcon {
    fn default() -> Self {
        StatusIcon {
            enabled: true,
            show_when_idle: false,
        }
    }
}
#[derive(Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
#[serde(rename_all = "kebab-case")]
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
        let app_config: AppConfig = toml::from_str("").unwrap();

        assert_eq!(
            app_config,
            AppConfig {
                restic_binary: "restic".to_owned(),
                daemon: Daemon {
                    desktop: Desktop {
                        status_icon: StatusIcon {
                            enabled: true,
                            show_when_idle: false
                        },
                        notifications: DesktopNotifications {
                            started: false,
                            success: false,
                            failure: true
                        }
                    }
                }
            }
        );
    }

    #[test]
    fn should_parse_partial_config() {
        let app_config: AppConfig = toml::from_str(
            //language=TOML
            r#"
            restic-binary = "/opt/restic"

            [daemon.desktop.status-icon]
            enabled = false

            [daemon.desktop.notifications]
            success = true
            "#,
        )
        .unwrap();

        assert_eq!(
            app_config,
            AppConfig {
                restic_binary: "/opt/restic".to_owned(),
                daemon: Daemon {
                    desktop: Desktop {
                        status_icon: StatusIcon {
                            enabled: false,
                            show_when_idle: false
                        },
                        notifications: DesktopNotifications {
                            started: false,
                            success: true,
                            failure: true
                        }
                    }
                }
            }
        );
    }
}
