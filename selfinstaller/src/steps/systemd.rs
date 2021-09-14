//! Installation steps that interact with systemd.

use crate::{Action, Destination};
use std::process::Command;

/// systemd mode
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
enum Mode {
    System,
    User,
}

impl Mode {
    fn arg(&self) -> &str {
        match self {
            Mode::System => "--system",
            Mode::User => "--user",
        }
    }
}

/// Implementation struct for enabling a systemd unit.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct SystemdEnable {
    mode: Mode,
    unit: String,
}

impl crate::InstallStep for SystemdEnable {
    fn install_description(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self.mode {
            Mode::System => write!(f, "enable systemd unit {}", self.unit),
            Mode::User => write!(f, "enable user-session systemd unit {}", self.unit),
        }
    }

    fn uninstall_description(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self.mode {
            Mode::System => write!(f, "disable systemd unit {}", self.unit),
            Mode::User => write!(f, "disable user-session systemd unit {}", self.unit),
        }
    }

    fn install(&self, destination: &Destination) -> eyre::Result<Action> {
        if destination.is_system() {
            run_systemctl(self.mode, ["daemon-reload"])?;
            run_systemctl(self.mode, ["enable", "--now", &self.unit])?;
            Ok(Action::Ok)
        } else {
            Ok(Action::Skipped("non-system destination".to_owned()))
        }
    }

    fn uninstall(&self, destination: &Destination) -> eyre::Result<Action> {
        if destination.is_system() {
            run_systemctl(self.mode, ["disable", "--now", &self.unit])?;
            Ok(Action::Ok)
        } else {
            Ok(Action::Skipped("non-system destination".to_owned()))
        }
    }
}

/// Run systemctl with the given arguments either in system or user mode.
fn run_systemctl<'a>(mode: Mode, args: impl IntoIterator<Item = &'a str>) -> eyre::Result<()> {
    let status = Command::new("systemctl")
        .arg(mode.arg())
        .args(args)
        .spawn()?
        .wait()?;
    if !status.success() {
        Err(eyre::eyre!("systemctl exited unsuccessfully"))
    } else {
        Ok(())
    }
}

/// Create an installation step to enable a system-wide systemd unit. The unit
/// will be started immediately (`--now`). If run with a non-system destination,
/// installation and uninstallation for this step will do nothing. The daemon
/// will be reloaded before enabling the unit files that were installed by a
/// preceeding step can be used.
///
/// ## Uninstallation
/// The unit will be disabled and stopped immediately.
///
/// ## Example
/// ```no_run
/// # use selfinstaller::{Destination, InstallStep, steps::systemd};
/// systemd::enable("ssh.service").install(&Destination::System)?;
/// # Ok::<(), eyre::Report>(())
pub fn enable(unit: impl Into<String>) -> SystemdEnable {
    let unit = unit.into();
    SystemdEnable {
        mode: Mode::System,
        unit,
    }
}

/// Create an installation step to enable a user-session systemd unit. The unit
/// will be started immediately (`--now`). If run with a non-system destination,
/// installation and uninstallation for this step will do nothing. The daemon
/// will be reloaded before enabling the unit files that were installed by a
/// preceeding step can be used.
///
/// ## Uninstallation
/// The user-session unit will be disabled and stopped immediately.
///
/// ## Example
/// ```no_run
/// # use selfinstaller::{Destination, InstallStep, steps::systemd};
/// systemd::enable_user("dbus.service").install(&Destination::System)?;
/// # Ok::<(), eyre::Report>(())
pub fn enable_user(unit: impl Into<String>) -> SystemdEnable {
    let unit = unit.into();
    SystemdEnable {
        mode: Mode::User,
        unit,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::steps::testutil;
    use crate::InstallStep;
    use std::path::PathBuf;

    #[test]
    fn test_mode_arg_system() {
        assert_eq!(Mode::System.arg(), "--system");
    }

    #[test]
    fn test_mode_arg_user() {
        assert_eq!(Mode::User.arg(), "--user");
    }

    #[test]
    fn test_systemd_enable() {
        let step = enable("ssh.service");
        assert_eq!(step.mode, Mode::System);
    }

    #[test]
    fn test_systemd_enable_user() {
        let step = enable_user("bob.socket");
        assert_eq!(step.mode, Mode::User);
    }

    #[test]
    fn test_system_install_description() {
        let step = enable("unit.service");
        assert_eq!(
            &testutil::install_description(&step),
            "enable systemd unit unit.service"
        );
    }

    #[test]
    fn test_user_install_description() {
        let step = enable_user("session.service");
        assert_eq!(
            &testutil::install_description(&step),
            "enable user-session systemd unit session.service"
        );
    }

    #[test]
    fn test_system_uninstall_description() {
        let step = enable("unit.service");
        assert_eq!(
            &testutil::uninstall_description(&step),
            "disable systemd unit unit.service"
        );
    }

    #[test]
    fn test_user_uninstall_description() {
        let step = enable_user("session.service");
        assert_eq!(
            &testutil::uninstall_description(&step),
            "disable user-session systemd unit session.service"
        );
    }

    #[test]
    fn should_install_nothing_for_non_system_destination() {
        let step = enable("unit.service");

        let result = step.install(&Destination::DestDir(PathBuf::new())).unwrap();

        assert!(matches!(result, Action::Skipped(_)));
    }

    #[test]
    fn should_uninstall_nothing_for_non_system_destination() {
        let step = enable("unit.service");

        let result = step
            .uninstall(&Destination::DestDir(PathBuf::new()))
            .unwrap();

        assert!(matches!(result, Action::Skipped(_)));
    }
}
