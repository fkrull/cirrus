use crate::{Action, Destination};
use std::path::PathBuf;

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

pub fn shortcut(
    path: impl Into<PathBuf>,
    target: impl Into<PathBuf>,
    args: Option<impl Into<String>>,
) -> Shortcut {
    let path = path.into();
    let target = target.into();
    let args = args.map(|s| s.into());
    Shortcut { path, target, args }
}
