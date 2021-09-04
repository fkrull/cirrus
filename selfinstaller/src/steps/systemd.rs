use crate::{Action, Destination};
use std::process::Command;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct SystemdEnable {
    unit: String,
}

impl crate::InstallStep for SystemdEnable {
    fn install_description(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "enable systemd unit {}", self.unit)
    }

    fn uninstall_description(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "disable systemd unit {}", self.unit)
    }

    fn install(&self, destination: &Destination) -> eyre::Result<Action> {
        if destination.is_system() {
            run_cmd(Command::new("systemctl").arg("daemon-reload"))?;
            run_cmd(
                Command::new("systemctl")
                    .arg("enable")
                    .arg("--now")
                    .arg(&self.unit),
            )?;
            Ok(Action::Ok)
        } else {
            Ok(Action::Skipped("non-system destination".to_owned()))
        }
    }

    fn uninstall(&self, destination: &Destination) -> eyre::Result<Action> {
        if destination.is_system() {
            run_cmd(
                Command::new("systemctl")
                    .arg("disable")
                    .arg("--now")
                    .arg(&self.unit),
            )?;
            Ok(Action::Ok)
        } else {
            Ok(Action::Skipped("non-system destination".to_owned()))
        }
    }
}

fn run_cmd(cmd: &mut Command) -> eyre::Result<()> {
    let status = cmd.spawn()?.wait()?;
    if !status.success() {
        Err(eyre::eyre!("systemctl exited unsuccessfully"))
    } else {
        Ok(())
    }
}

pub fn enable(unit: impl Into<String>) -> SystemdEnable {
    let unit = unit.into();
    SystemdEnable { unit }
}
