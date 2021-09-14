#![doc = include_str!("../README.md")]

use owo_colors::OwoColorize;
use std::{
    fmt::Display,
    path::{Component, Path, PathBuf},
};

pub mod steps;

/// The action that was taken by an install or uninstall step.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Outcome {
    /// The action completed successfully.
    Ok,

    /// The action was skipped with the given reason string. This indicates that
    /// no changes were made for a benign reason: the desired state was already
    /// reached or the action simply didn't make sense for the given parameters
    /// (for example, enabling a systemd unit while installing into a specific
    /// directory).
    Skipped(String),

    /// The action was completed with a warning. This is used for actions that
    /// failed in some way, but where the failure wasn't severe enough to
    /// justify an error. For example, deleting a directory that was created in
    /// an install step may fail because it's not empty and return a warning.
    Warn(String),
}

/// Trait that implements a single step of an installer process, including informational output and
/// uninstallation.
pub trait InstallStep {
    /// Prints a one-line description of the installation part of this step. This line is used for
    /// the progress output during installation. It should contain the most important information
    /// about what this step does, but also needs to be relatively succinct. Per convention, this
    /// should be a sentence starting with an imperative verb (lowercase) and no trailing period,
    /// for example: "install file <target file path>".
    fn install_description(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result;

    /// Prints a one-line description of the uninstallation part of this step. This line is used for
    /// the progress output during uninstallation. See
    /// [`install_description`][InstallStep::install_description] for the suggested format.
    fn uninstall_description(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result;

    /// Prints a detailed description of what this step does when installed. This may be multiple
    /// lines and include all parameters that were passed to the step and can be reasonably output
    /// to the console. For example, for a step that installs a file,
    /// [`install_description`][InstallStep::install_description] may only show the file name, but
    /// [`details`][InstallStep::details] would also show the full file contents.
    ///
    /// If not overridden, this simply calls [`install_description`][InstallStep::install_description].
    fn details(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.install_description(f)
    }

    /// Runs the installation actions for this step with the given [`Destination`]. This function
    /// should not print any output of its own; instead it should use its return value to
    /// communicate any info to the user.
    ///
    /// If the action can't be fully contained to a destination directory, it should be skipped if
    /// not installing to the [`System`][Destination::System] destination. In general, when a
    /// destination directory is specified, the method may not perform any changes outside of that
    /// directory.
    ///
    /// ## Return Value
    /// If the action can be considered successful, the function should return `Ok` with an
    /// appropriate [`Outcome`]; otherwise, an `Err`. If in doubt, this function should
    /// likely prefer errors over using [`Outcome::Warn`] because errors will stop the installation.
    fn install(&self, destination: &Destination) -> eyre::Result<Outcome>;

    /// Runs the uninstallation actions for this step with the given [`Destination`]. This function
    /// should not print any output of its own; instead it should use its return value to
    /// communicate any info to the user.
    ///
    /// If the action can't be fully contained to a destination directory, it should be skipped if
    /// not installing to the [`System`][Destination::System] destination. In general, when a
    /// destination directory is specified, the method may not perform any changes outside of that
    /// directory.
    ///
    /// ## Return Value
    /// If the action can be considered successful, the function should return `Ok` with an
    /// appropriate [`Outcome`]; otherwise, an `Err`.
    fn uninstall(&self, destination: &Destination) -> eyre::Result<Outcome>;
}

struct InstallDescription<'a>(&'a dyn InstallStep);

impl<'a> Display for InstallDescription<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.0.install_description(f)
    }
}

struct UninstallDescription<'a>(&'a dyn InstallStep);

impl<'a> Display for UninstallDescription<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.0.uninstall_description(f)
    }
}

/// An error in an installation step that caused the installation process to
/// stop.
#[derive(Debug, thiserror::Error)]
#[error("in [step #{}] {description}", index + 1)]
pub struct SingleError {
    /// A readable description of the error.
    pub description: String,

    /// The position of the failed step in the installation.
    pub index: usize,

    /// The source error that caused the failure.
    #[source]
    pub error: eyre::Report,
}

/// Multiple installation errors collected into one.
#[derive(Debug, thiserror::Error)]
pub struct MultiErrors(Vec<SingleError>);

impl Display for MultiErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for error in &self.0 {
            writeln!(f, "{}: {}", error, error.error)?;
        }
        Ok(())
    }
}

/// Error returned from [`SelfInstaller::install()`].
#[derive(Debug, thiserror::Error)]
#[error("during installation into destination '{destination}'")]
pub struct InstallError {
    pub destination: Destination,
    #[source]
    pub error: SingleError,
}

/// All errors returned from [`SelfInstaller::uninstall()`].
#[derive(Debug, thiserror::Error)]
#[error("during uninstall from destination '{destination}'")]
pub struct UninstallError {
    pub destination: Destination,
    #[source]
    pub errors: MultiErrors,
}

/// Specify whether to install normally or into a specified destination directory. Some commands
/// may be skipped when installing into a specific destination directory if they can't support it.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Destination {
    /// Install everything directly to the paths specified.
    System,

    /// Install into the given destination directory. All paths specified in the installer are
    /// appended to the destination directory path.
    DestDir(PathBuf),
}

impl Display for Destination {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Destination::System => write!(f, "system"),
            Destination::DestDir(path) => write!(f, "{}", path.display()),
        }
    }
}

impl From<Option<PathBuf>> for Destination {
    fn from(path: Option<PathBuf>) -> Self {
        path.map(Destination::DestDir)
            .unwrap_or(Destination::System)
    }
}

impl Destination {
    /// Returns the full path for the given path in the destination. For the
    /// [`System`][Destination::System] destination, this returns the path unchanged. For other
    /// destinations, the given path is appended to the destination path.
    pub fn full_path(&self, path: impl AsRef<Path>) -> PathBuf {
        let path = path.as_ref();
        match self {
            Destination::System => path.to_owned(),
            Destination::DestDir(destdir) => {
                let mut full_path = destdir.clone();
                path.components()
                    .skip_while(|c| matches!(c, Component::Prefix(_) | Component::RootDir))
                    .for_each(|c| full_path.push(c.as_os_str()));
                full_path
            }
        }
    }

    /// Returns true if this is the system destination.
    pub fn is_system(&self) -> bool {
        matches!(self, Destination::System)
    }
}

/// A [`SelfInstaller`] holds several [`InstallStep`]s and can install and uninstall them with
/// progress output, as well as print a detailed summary of steps to be taken.
#[derive(Default)]
pub struct SelfInstaller {
    steps: Vec<Box<dyn InstallStep + Send>>,
}

impl SelfInstaller {
    /// Creates a new empty installer.
    pub fn new() -> Self {
        SelfInstaller::default()
    }

    /// Adds an [`InstallStep`] to this installer. This does not run the step's installation or
    /// uninstallation. Steps are arranged in the order they are added.
    pub fn add_step<T: InstallStep + Send + 'static>(mut self, step: T) -> Self {
        self.steps.push(Box::new(step));
        self
    }

    /// Returns an object that displays detailed steps that this installer will take when installed.
    ///
    /// ```
    /// # let installer = selfinstaller::SelfInstaller::default();
    /// print!("{}", installer.details());
    /// ```
    pub fn details(&self) -> DisplayAsDetails {
        DisplayAsDetails(self)
    }

    /// Runs all installation steps in this installer to the given [`Destination`] and print
    /// progress output. This runs all steps in the order they were added and stops on the first
    /// error.
    pub fn install(&mut self, destination: &Destination) -> Result<(), InstallError> {
        if !destination.is_system() {
            println!(
                "[{}] installing into non-system destination {}",
                "info".blue(),
                destination
            );
        }
        for (index, step) in self.steps.iter().enumerate() {
            let description = InstallDescription(&**step);
            self.show_step_result(&description, step.install(destination))
                .map_err(|error| InstallError {
                    destination: destination.clone(),
                    error: SingleError {
                        description: description.to_string(),
                        index,
                        error,
                    },
                })?;
        }
        Ok(())
    }

    /// Runs all uninstallation steps in this installer to the given [`Destination`] and print
    /// progress output. This runs all steps in the reverse order. All steps will be run and the
    /// errors collected and returned at the end.
    pub fn uninstall(&mut self, destination: &Destination) -> Result<(), UninstallError> {
        let mut errors = Vec::new();
        for (index, step) in self.steps.iter().enumerate().rev() {
            let description = UninstallDescription(&**step);
            if let Err(error) = self.show_step_result(&description, step.uninstall(destination)) {
                errors.push(SingleError {
                    description: description.to_string(),
                    index,
                    error,
                });
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(UninstallError {
                destination: destination.clone(),
                errors: MultiErrors(errors),
            })
        }
    }

    fn show_step_result<T: Display>(
        &self,
        description: T,
        result: eyre::Result<Outcome>,
    ) -> eyre::Result<()> {
        match &result {
            Ok(Outcome::Ok) => println!("[{}] {}", " ok ".green(), description),
            Ok(Outcome::Skipped(reason)) => {
                println!("[{}] {}: {}", "skip".cyan(), description, reason)
            }
            Ok(Outcome::Warn(reason)) => {
                println!("[{}] {}: {}", "warn".yellow(), description, reason)
            }
            Err(error) => {
                println!("[{}] {}: {}", "fail".red(), description, error)
            }
        }
        result.map(|_| ())
    }
}

/// Implementation struct for displaying all the steps of an installer.
pub struct DisplayAsDetails<'a>(&'a SelfInstaller);

impl<'a> Display for DisplayAsDetails<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        for (index, step) in self.0.steps.iter().enumerate() {
            write!(f, "[{}] ", format!("step #{}", index + 1).blue())?;
            step.details(f)?;
            writeln!(f)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod errors {
        use super::*;

        #[test]
        fn should_format_error() {
            let error = SingleError {
                description: "error description".to_owned(),
                index: 4,
                error: eyre::eyre!("source error"),
            };

            assert_eq!(&format!("{}", error), "in [step #5] error description");
        }

        #[test]
        fn should_format_multi_error() {
            let error1 = SingleError {
                description: "something went wrong".to_owned(),
                index: 0,
                error: eyre::eyre!("source error 1"),
            };
            let error2 = SingleError {
                description: "something went more wrong".to_owned(),
                index: 1,
                error: eyre::eyre!("source error 2"),
            };
            let error = MultiErrors(vec![error1, error2]);

            assert_eq!(
                &format!("{}", error),
                "in [step #1] something went wrong: source error 1\nin [step #2] something went more wrong: source error 2\n"
            );
        }
    }

    mod destination {
        use super::*;

        #[test]
        fn system_should_return_path_unchanged() {
            let p = Path::new("/super/path");
            assert_eq!(&Destination::System.full_path(p), p);
        }

        #[test]
        fn destdir_should_join_relative_path() {
            let p = Path::new("relative/path");
            let destdir = Path::new("/super/dir");
            assert_eq!(
                &Destination::DestDir(destdir.to_owned()).full_path(p),
                Path::new("/super/dir/relative/path")
            );
        }

        #[test]
        fn destdir_should_join_path_with_root() {
            let p = Path::new("/root/path");
            let destdir = Path::new("/super/dir");
            assert_eq!(
                &Destination::DestDir(destdir.to_owned()).full_path(p),
                Path::new("/super/dir/root/path")
            );
        }

        #[test]
        #[cfg(windows)]
        fn destdir_should_join_path_with_prefix() {
            let p = Path::new("C:temp");
            let destdir = Path::new("/super/dir");
            assert_eq!(
                &Destination::DestDir(destdir.to_owned()).full_path(p),
                Path::new("/super/dir/temp")
            );
        }

        #[test]
        #[cfg(windows)]
        fn destdir_should_join_path_with_prefix_and_root() {
            let p = Path::new("C:/Windows/Temp");
            let destdir = Path::new("/super/dir");
            assert_eq!(
                &Destination::DestDir(destdir.to_owned()).full_path(p),
                Path::new("/super/dir/Windows/Temp")
            );
        }

        #[test]
        fn test_is_system() {
            assert!(Destination::System.is_system());
            assert!(!Destination::DestDir(PathBuf::from("/")).is_system());
        }
    }
}
