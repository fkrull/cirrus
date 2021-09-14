//! Windows-specific installation steps.

use crate::{Action, Destination};
use std::path::PathBuf;

/// Implementation struct for the Windows shortcut step.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Shortcut {
    target: PathBuf,
    path: PathBuf,
    args: Option<String>,
}

impl crate::InstallStep for Shortcut {
    fn install_description(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "create Windows shortcut {}", self.path.display(),)
    }

    fn uninstall_description(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "remove Windows shortcut {}", self.path.display())
    }

    fn details(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        writeln!(f, "create Windows shortcut {}", self.path.display())?;
        writeln!(f, "  target: {}", self.target.display())?;
        if let Some(args) = &self.args {
            writeln!(f, "  arguments: {}", args)?;
        }
        Ok(())
    }

    fn install(&self, destination: &Destination) -> eyre::Result<Action> {
        let full_path = destination.full_path(&self.path);
        let mut link = mslnk::ShellLink::new(&self.target)?;
        link.set_arguments(self.args.clone());
        link.create_lnk(&full_path)?;
        Ok(Action::Ok)
    }

    fn uninstall(&self, destination: &Destination) -> eyre::Result<Action> {
        std::fs::remove_file(destination.full_path(&self.path))?;
        Ok(Action::Ok)
    }
}

/// Installation step to create a Windows shell shortcut (a `.lnk` file). The
/// `.lnk` extension should be included in the file name.
///
/// ## Uninstallation
/// The file at `path` will be removed.
///
/// ## Example
/// ```
/// # use selfinstaller::{Destination, InstallStep, steps::windows};
/// # let tmp = tempfile::TempDir::new()?;
/// # let link_path = tmp.path().join("test.lnk");
/// windows::shortcut(&link_path, "C:/Windows/system32/notepad.exe", None)
///   .install(&Destination::System)?;
/// assert!(link_path.is_file());
/// # Ok::<(), eyre::Report>(())
/// ```
pub fn shortcut(
    path: impl Into<PathBuf>,
    target: impl Into<PathBuf>,
    args: Option<&str>,
) -> Shortcut {
    let path = path.into();
    let target = target.into();
    let args = args.map(|s| s.to_owned());
    Shortcut { path, target, args }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{steps::testutil, InstallStep};

    #[test]
    fn test_install_description() {
        let step = shortcut("C:/shortcut.lnk", "C:/target.exe", Some("a b c"));
        assert_eq!(
            &testutil::install_description(&step),
            "create Windows shortcut C:/shortcut.lnk"
        );
    }

    #[test]
    fn test_uninstall_description() {
        let step = shortcut("C:/shortcut.lnk", "C:/target.exe", Some("a b c"));
        assert_eq!(
            &testutil::uninstall_description(&step),
            "remove Windows shortcut C:/shortcut.lnk"
        );
    }

    #[test]
    fn test_details_no_args() {
        let step = shortcut("C:/shortcut.lnk", "C:/target.exe", None);
        assert_eq!(
            &testutil::details(&step),
            "create Windows shortcut C:/shortcut.lnk\n  target: C:/target.exe\n"
        );
    }

    #[test]
    fn test_details_with_args() {
        let step = shortcut("C:/shortcut.lnk", "C:/target.exe", Some("a b c"));
        assert_eq!(
            &testutil::details(&step),
            "create Windows shortcut C:/shortcut.lnk\n  target: C:/target.exe\n  arguments: a b c\n"
        );
    }

    #[test]
    fn should_create_link() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("target.lnk");
        let step = shortcut(path.clone(), tmp.path(), None);

        let result = step.install(&Destination::System).unwrap();

        assert_eq!(result, Action::Ok);
        let metadata = std::fs::metadata(&path).unwrap();
        assert!(metadata.is_file());
    }

    #[test]
    fn should_create_link_in_destination() {
        let tmp = tempfile::TempDir::new().unwrap();
        let step = shortcut("test.lnk", tmp.path(), None);

        let result = step
            .install(&Destination::DestDir(tmp.path().to_owned()))
            .unwrap();

        assert_eq!(result, Action::Ok);
        let metadata = std::fs::metadata(tmp.path().join("test.lnk")).unwrap();
        assert!(metadata.is_file());
    }

    #[test]
    fn should_remove_link() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("target.lnk");
        std::fs::write(&path, "dummy").unwrap();
        let step = shortcut(path.clone(), "target", None);

        let result = step.uninstall(&Destination::System).unwrap();

        assert_eq!(result, Action::Ok);
        assert!(!path.exists());
    }

    #[test]
    fn should_remove_link_in_destination() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("test.lnk");
        std::fs::write(&path, "dummy").unwrap();
        let step = shortcut("test.lnk", tmp.path(), None);

        let result = step
            .uninstall(&Destination::DestDir(tmp.path().to_owned()))
            .unwrap();

        assert_eq!(result, Action::Ok);
        assert!(!path.exists());
    }
}
