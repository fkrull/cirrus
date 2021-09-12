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
        let metadata = std::fs::metadata(&path).unwrap();
        assert!(metadata.is_dir());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(metadata.permissions().mode(), 0o755);
        }
    }

    #[test]
    fn should_create_directory_in_destination() {
        let tmp = tempfile::TempDir::new().unwrap();
        let step = directory("/test/path");

        let result = step
            .install(&Destination::DestDir(tmp.path().to_owned()))
            .unwrap();

        assert_eq!(result, Action::Ok);
        let metadata = std::fs::metadata(tmp.path().join("test/path")).unwrap();
        assert!(metadata.is_dir());
    }

    #[test]
    fn should_remove_empty_directory() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("subdir");
        std::fs::create_dir(&path).unwrap();
        let step = directory(path.clone());

        let result = step.uninstall(&Destination::System).unwrap();

        assert_eq!(result, Action::Ok);
        assert!(std::fs::metadata(&path).is_err());
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
        assert!(std::fs::metadata(tmp.path().join("subdir")).is_err());
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
        assert!(std::fs::metadata(&path).unwrap().is_dir());
    }
}
