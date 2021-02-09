#[derive(Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
#[serde(rename_all = "kebab-case")]
pub struct AppConfig {
    pub daemon: Daemon,
}

#[derive(Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
#[serde(default)]
#[serde(rename_all = "kebab-case")]
pub struct Daemon {
    pub desktop: Desktop,
}

#[derive(Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
#[serde(rename_all = "kebab-case")]
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
                daemon: Daemon {
                    desktop: Desktop {
                        status_icon: true,
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
            [daemon.desktop]
            status-icon = false

            [daemon.desktop.notifications]
            success = true
            "#,
        )
        .unwrap();

        assert_eq!(
            app_config,
            AppConfig {
                daemon: Daemon {
                    desktop: Desktop {
                        status_icon: false,
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
