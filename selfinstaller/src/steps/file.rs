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

impl<const N: usize> From<&[u8; N]> for Contents {
    fn from(bytes: &[u8; N]) -> Self {
        Contents::Binary(bytes.to_vec())
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{steps::testutil, InstallStep};

    #[test]
    fn should_create_contents_from_owned_string() {
        let contents = Contents::from("string".to_owned());
        assert_eq!(contents, Contents::Text("string".to_owned()));
    }

    #[test]
    fn should_create_contents_from_borrowed_str() {
        let contents = Contents::from("string");
        assert_eq!(contents, Contents::Text("string".to_owned()));
    }

    #[test]
    fn should_create_contents_from_byte_slice() {
        let contents = Contents::from(&b"test"[..]);
        assert_eq!(contents, Contents::Binary(b"test".to_vec()));
    }

    #[test]
    fn should_create_contents_from_byte_array() {
        let contents = Contents::from(b"test");
        assert_eq!(contents, Contents::Binary(b"test".to_vec()));
    }

    #[test]
    fn test_install_description_file() {
        let step = file("/test/path", "contents");
        assert_eq!(
            &testutil::install_description(&step),
            "install file /test/path"
        );
    }

    #[test]
    fn test_install_description_executable() {
        let step = executable("/test/path.sh", "#!/bin/sh");
        assert_eq!(
            &testutil::install_description(&step),
            "install executable file /test/path.sh"
        );
    }

    #[test]
    fn test_uninstall_description() {
        let step = file("/test/path", "contents");
        assert_eq!(
            &testutil::uninstall_description(&step),
            "remove file /test/path"
        );
    }

    #[test]
    fn test_details_text_file() {
        let step = file("/test/file.txt", "text contents");
        assert_eq!(
            &testutil::details(&step),
            "file /test/file.txt:\n  text contents\n"
        );
    }

    #[test]
    fn test_details_binary_file() {
        let step = file("/test/file.bin", b"bytes");
        assert_eq!(&testutil::details(&step), "file /test/file.bin: <binary>\n");
    }

    #[test]
    fn test_details_executable() {
        let step = executable("/file.exe", "#!/bin/sh");
        assert_eq!(
            &testutil::details(&step),
            "executable /file.exe:\n  #!/bin/sh\n"
        );
    }

    #[test]
    fn should_create_text_file() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("test.txt");
        let step = file(&path, "text contents");

        let result = step.install(&Destination::System).unwrap();

        assert_eq!(result, Action::Ok);
        assert_eq!(&std::fs::read_to_string(&path).unwrap(), "text contents");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = std::fs::metadata(&path).unwrap();
            assert_eq!(metadata.permissions().mode() & 0o644, 0o644);
        }
    }

    #[test]
    fn should_create_binary_file() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("test");
        let step = file(&path, b"binary contents");

        let result = step.install(&Destination::System).unwrap();

        assert_eq!(result, Action::Ok);
        assert_eq!(&std::fs::read(&path).unwrap(), b"binary contents");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = std::fs::metadata(&path).unwrap();
            assert_eq!(metadata.permissions().mode() & 0o644, 0o644);
        }
    }

    #[test]
    fn should_create_executable() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("bin.sh");
        let step = executable(&path, "echo test");

        let result = step.install(&Destination::System).unwrap();

        assert_eq!(result, Action::Ok);
        assert_eq!(&std::fs::read_to_string(&path).unwrap(), "echo test");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = std::fs::metadata(&path).unwrap();
            assert_eq!(metadata.permissions().mode() & 0o755, 0o755);
        }
    }

    #[test]
    fn should_create_file_in_destination() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("file.txt");
        let step = file("/file.txt", "test file");

        let result = step
            .install(&Destination::DestDir(tmp.path().to_owned()))
            .unwrap();

        assert_eq!(result, Action::Ok);
        assert_eq!(&std::fs::read_to_string(&path).unwrap(), "test file");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = std::fs::metadata(&path).unwrap();
            assert_eq!(metadata.permissions().mode() & 0o644, 0o644);
        }
    }

    #[test]
    fn should_remove_file() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("test.txt");
        std::fs::write(&path, "text contents").unwrap();
        let step = file(&path, "text contents");

        let result = step.uninstall(&Destination::System).unwrap();

        assert_eq!(result, Action::Ok);
        assert!(!path.exists());
    }

    #[test]
    fn should_remove_file_in_destination() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("test.txt");
        std::fs::write(&path, "text contents").unwrap();
        let step = file("test.txt", "text contents");

        let result = step
            .uninstall(&Destination::DestDir(tmp.path().to_owned()))
            .unwrap();

        assert_eq!(result, Action::Ok);
        assert!(!path.exists());
    }

    #[test]
    fn should_not_remove_directory() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("test");
        std::fs::create_dir(&path).unwrap();
        let step = file("test", "text contents");

        let result = step.uninstall(&Destination::DestDir(tmp.path().to_owned()));

        assert!(result.is_err());
    }
}
