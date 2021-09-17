use selfinstaller::{steps::*, Destination, SelfInstaller};
use std::path::Path;

#[test]
fn should_install_and_uninstall_files() {
    let tmp = tempfile::TempDir::new().unwrap();
    let mut installer = SelfInstaller::new()
        .add_step(directory("subdir"))
        .add_step(directory(Path::new("subdir").join("subdir2")))
        .add_step(file(
            Path::new("subdir").join("subdir2").join("file.txt"),
            "file contents",
        ));

    installer.install(&Destination::from(tmp.path())).unwrap();
    let file_contents =
        std::fs::read_to_string(tmp.path().join("subdir").join("subdir2").join("file.txt"))
            .unwrap();
    assert_eq!(&file_contents, "file contents");

    installer.uninstall(&Destination::from(tmp.path())).unwrap();
    assert_eq!(std::fs::read_dir(tmp.path()).unwrap().count(), 0);
}

#[test]
fn should_fail_on_install_but_uninstall_completely() {
    let tmp = tempfile::TempDir::new().unwrap();
    let mut installer = SelfInstaller::new()
        .add_step(file("file.txt", "file contents"))
        .add_step(file(
            Path::new("missing-subdir").join("error-file.txt"),
            "error file",
        ))
        .add_step(file("file2.txt", "file2 contents"));

    let result = installer.install(&Destination::from(tmp.path()));
    assert!(result.is_err());
    assert!(!tmp.path().join("file2.txt").exists());

    let result = installer.uninstall(&Destination::from(tmp.path()));
    assert!(result.is_err());
    assert_eq!(std::fs::read_dir(tmp.path()).unwrap().count(), 0);
}
