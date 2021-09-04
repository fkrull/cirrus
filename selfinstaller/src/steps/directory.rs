use crate::{Action, Destination};
use std::path::{Path, PathBuf};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct InstallDirectory {
    path: PathBuf,
}

impl InstallDirectory {
    #[cfg(not(unix))]
    fn update_permissions(&self, _full_path: &Path) -> eyre::Result<()> {
        Ok(())
    }

    #[cfg(unix)]
    fn update_permissions(&self, full_path: &Path) -> eyre::Result<()> {
        use std::{fs::Permissions, os::unix::fs::PermissionsExt};
        std::fs::set_permissions(full_path, Permissions::from_mode(0o755))?;
        Ok(())
    }
}

impl crate::InstallStep for InstallDirectory {
    fn install_description(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "create directory {}", self.path.display())
    }

    fn uninstall_description(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "remove empty directory {}", self.path.display())
    }

    fn install(&self, destination: &Destination) -> eyre::Result<Action> {
        let full_path = destination.full_path(&self.path);
        std::fs::create_dir_all(&full_path)?;
        self.update_permissions(&full_path)?;
        Ok(Action::Ok)
    }

    fn uninstall(&self, destination: &Destination) -> eyre::Result<Action> {
        match std::fs::remove_dir(destination.full_path(&self.path)) {
            Ok(_) => Ok(Action::Ok),
            Err(error) => Ok(Action::Warn(format!(
                "could not delete directory: {}",
                error
            ))),
        }
    }
}

pub fn directory(path: impl Into<PathBuf>) -> InstallDirectory {
    let path = path.into();
    InstallDirectory { path }
}
