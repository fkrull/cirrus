use crate::{Action, Destination};
use std::process::Command;

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

pub fn enable(unit: impl Into<String>) -> SystemdEnable {
    let unit = unit.into();
    SystemdEnable {
        mode: Mode::System,
        unit,
    }
}

pub fn enable_user(unit: impl Into<String>) -> SystemdEnable {
    let unit = unit.into();
    SystemdEnable {
        mode: Mode::User,
        unit,
    }
}
