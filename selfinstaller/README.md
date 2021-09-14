This library provides tools for building an installer included directly in your binary.

When distributing programs as a single binary, system integration can become tricky. This includes things like
installing desktop launchers, autostart files, and other global configuration. A common solution is an installer script,
but this library is designed for an alternative: still distribute the program as a single binary, but include an
installer in the binary so it can be run after installing the binary itself.

Features:
* several built-in installation commands
* show installation progress
* uninstaller
* show detailed installation steps to take
* installation into a destination directory instead of the system root

## Overview
The [`InstallStep`] trait describes a single self-contained installation step, including progress messages, detailed
output and uninstallation. The [`selfinstaller::steps`][steps] module contains some included installation steps, but custom
ones can be created by implementing the [`InstallStep`] trait. Each step describes a single action taken during the
installation, like creating a file or enabling a systemd unit.

The [`SelfInstaller`] type is a container that contains several [`InstallStep`]s in a certain order. The
[`SelfInstaller`] can be defined once in the application and then used to perform installation and uninstallation as
well as show a detailed listing of all installation steps.

## Example
```
# use std::path::Path;
# let tmp = tempfile::TempDir::new()?;
# let dest_path = tmp.path();
use selfinstaller::{Destination, SelfInstaller, steps::*};

#[cfg(unix)]
let base_path = Path::new("/");
#[cfg(windows)]
let base_path = Path::new("C:\\");

let mut installer = SelfInstaller::new()
    .add_step(file(base_path.join("file1"), "file1 contents"))
    .add_step(directory(base_path.join("subdir")))
    .add_step(file(base_path.join("subdir").join("file2"), "file2 contents"));

installer.install(&Destination::DestDir(dest_path.to_owned()))?;
assert!(dest_path.join("file1").is_file());
installer.uninstall(&Destination::DestDir(dest_path.to_owned()))?;
assert!(!dest_path.join("file1").is_file());
# Ok::<(), eyre::Report>(())
```
