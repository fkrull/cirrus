//! Installation step that creates a directory.
use crate::{Action, Destination};
use std::path::{Path, PathBuf};

/// Implementation type for the directory step.
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

/// An installation step that creates a directory. Any missing parent
/// directories will be created as well. On Unix systems, the new directory will
/// have mode 0755 (owner-writable, world-readable).
///
/// ## Uninstallation
/// On uninstallation, the directory will be deleted *only if it's empty*. If
/// the directory isn't empty or can't be deleted for other reasons, the
/// uninstall step will return a warning instead of failing outright.
///
/// ## Example
/// ```
/// # use selfinstaller::{Destination, InstallStep, steps::directory};
/// # let tmp = tempfile::TempDir::new()?;
/// # let dir_path = tmp.path().join("subdir1").join("subdir2");
/// directory(&dir_path).install(&Destination::System)?;
/// assert!(dir_path.exists());
/// # Ok::<(), eyre::Report>(())
/// ```
pub fn directory(path: impl Into<PathBuf>) -> InstallDirectory {
    let path = path.into();
    InstallDirectory { path }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{steps::testutil, InstallStep};

    #[test]
    fn test_install_description() {
        let step = directory("/test/path");
        assert_eq!(
            &testutil::install_description(&step),
            "create directory /test/path"
        );
    }

    #[test]
    fn test_uninstall_description() {
        let step = directory("/test/path");
        assert_eq!(
            &testutil::uninstall_description(&step),
            "remove empty directory /test/path"
        );
    }

    #[test]
    fn should_create_directory() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("subdir");
        let step = directory(path.clone());

        let result = step.install(&Destination::System).unwrap();

        assert_eq!(result, Action::Ok);
        assert!(path.is_dir());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = std::fs::metadata(&path).unwrap();
            assert_eq!(metadata.permissions().mode() & 0o755, 0o755);
        }
    }

    #[test]
    fn should_create_directory_and_parent() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("1").join("2").join("3").join("4");
        let step = directory(path.clone());

        let result = step.install(&Destination::System).unwrap();

        assert_eq!(result, Action::Ok);
        assert!(path.is_dir());
    }

    #[test]
    fn should_create_directory_in_destination() {
        let tmp = tempfile::TempDir::new().unwrap();
        let step = directory("/test/path");

        let result = step
            .install(&Destination::DestDir(tmp.path().to_owned()))
            .unwrap();

        assert_eq!(result, Action::Ok);
        assert!(tmp.path().join("test/path").is_dir());
    }

    #[test]
    fn should_remove_empty_directory() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("subdir");
        std::fs::create_dir(&path).unwrap();
        let step = directory(path.clone());

        let result = step.uninstall(&Destination::System).unwrap();

        assert_eq!(result, Action::Ok);
        assert!(!path.exists());
    }

    #[test]
    fn should_remove_empty_directory_in_destination() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::create_dir(tmp.path().join("subdir")).unwrap();
        let step = directory("/subdir");

        let result = step
            .uninstall(&Destination::DestDir(tmp.path().to_owned()))
            .unwrap();

        assert_eq!(result, Action::Ok);
        assert!(!tmp.path().join("subdir").exists());
    }

    #[test]
    fn should_not_remove_not_empty_directory() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("subdir");
        std::fs::create_dir(&path).unwrap();
        std::fs::write(path.join("file"), "test").unwrap();
        let step = directory(path.clone());

        let result = step.uninstall(&Destination::System).unwrap();

        assert!(matches!(result, Action::Warn(_)));
        assert!(path.is_dir());
    }
}
