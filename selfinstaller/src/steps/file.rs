use crate::{Action, Destination};
use std::{
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Contents {
    Text(String),
    Binary(Vec<u8>),
}

impl From<&str> for Contents {
    fn from(string: &str) -> Self {
        Contents::Text(string.to_owned())
    }
}

impl From<String> for Contents {
    fn from(string: String) -> Self {
        Contents::Text(string)
    }
}

impl From<&[u8]> for Contents {
    fn from(bytes: &[u8]) -> Self {
        Contents::Binary(bytes.to_owned())
    }
}

impl Contents {
    fn as_bytes(&self) -> &[u8] {
        match self {
            Contents::Text(text) => text.as_bytes(),
            Contents::Binary(bytes) => bytes.as_slice(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct InstallFile {
    path: PathBuf,
    contents: Contents,
    executable: bool,
}

impl InstallFile {
    #[cfg(not(unix))]
    fn update_permissions(&self, _full_path: &Path) -> eyre::Result<()> {
        Ok(())
    }

    #[cfg(unix)]
    fn update_permissions(&self, full_path: &Path) -> eyre::Result<()> {
        use std::{fs::Permissions, os::unix::fs::PermissionsExt};
        let mode = if self.executable { 0o755 } else { 0o644 };
        std::fs::set_permissions(full_path, Permissions::from_mode(mode))?;
        Ok(())
    }
}

impl crate::InstallStep for InstallFile {
    fn install_description(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if self.executable {
            write!(f, "install executable file {}", self.path.display())
        } else {
            write!(f, "install file {}", self.path.display())
        }
    }

    fn uninstall_description(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "remove file {}", self.path.display())
    }

    fn details(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if self.executable {
            write!(f, "executable {}:", self.path.display())?;
        } else {
            write!(f, "file {}:", self.path.display())?;
        }

        match &self.contents {
            Contents::Text(text) => {
                writeln!(f)?;
                for line in text.lines() {
                    writeln!(f, "  {}", line)?;
                }
            }
            Contents::Binary(_) => {
                writeln!(f, " <binary>")?;
            }
        }

        Ok(())
    }

    fn install(&self, destination: &Destination) -> eyre::Result<Action> {
        let full_path = destination.full_path(&self.path);
        let dir = full_path.parent().ok_or_else(|| {
            eyre::eyre!(
                "could not determine parent directory for {}",
                full_path.display()
            )
        })?;
        let mut tmp = tempfile::NamedTempFile::new_in(dir)?;
        tmp.write_all(self.contents.as_bytes())?;
        tmp.persist(&full_path)?;
        self.update_permissions(&full_path)?;
        Ok(Action::Ok)
    }

    fn uninstall(&self, destination: &Destination) -> eyre::Result<Action> {
        std::fs::remove_file(destination.full_path(&self.path))?;
        Ok(Action::Ok)
    }
}

pub fn file(path: impl Into<PathBuf>, contents: impl Into<Contents>) -> InstallFile {
    let path = path.into();
    let contents = contents.into();
    InstallFile {
        path,
        contents,
        executable: false,
    }
}

pub fn executable(path: impl Into<PathBuf>, contents: impl Into<Contents>) -> InstallFile {
    let path = path.into();
    let contents = contents.into();
    InstallFile {
        path,
        contents,
        executable: true,
    }
}
