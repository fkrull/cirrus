use owo_colors::OwoColorize;
use std::{
    fmt::Display,
    path::{Component, Path, PathBuf},
};

pub mod steps;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Action {
    Ok,
    Skipped(String),
    Warn(String),
}

pub trait InstallStep {
    fn install_description(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result;

    fn uninstall_description(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result;

    fn details(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.install_description(f)
    }

    fn install(&self, destination: &Destination) -> eyre::Result<Action>;

    fn uninstall(&self, destination: &Destination) -> eyre::Result<Action>;
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

#[derive(Debug, thiserror::Error)]
#[error("in [step #{}] {description}", index + 1)]
pub struct StepError {
    pub description: String,
    pub index: usize,
    #[source]
    pub error: eyre::Report,
}

#[derive(Debug, thiserror::Error)]
pub struct StepErrors(Vec<StepError>);

impl Display for StepErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for error in &self.0 {
            writeln!(f, "{}: {}", error, error.error)?;
        }
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
#[error("during installation into destination '{destination}'")]
pub struct InstallError {
    pub destination: Destination,
    #[source]
    pub error: StepError,
}

#[derive(Debug, thiserror::Error)]
#[error("during uninstall from destination '{destination}'")]
pub struct UninstallError {
    pub destination: Destination,
    #[source]
    pub errors: StepErrors,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Destination {
    System,
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

    pub fn is_system(&self) -> bool {
        matches!(self, Destination::System)
    }
}

#[derive(Default)]
pub struct SelfInstaller {
    steps: Vec<Box<dyn InstallStep>>,
}

impl SelfInstaller {
    pub fn new() -> Self {
        SelfInstaller::default()
    }

    pub fn add_step<T: InstallStep + 'static>(mut self, step: T) -> Self {
        self.steps.push(Box::new(step));
        self
    }

    pub fn plan(&self) -> DisplayAsPlan {
        DisplayAsPlan(self)
    }

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
                    error: StepError {
                        description: description.to_string(),
                        index,
                        error,
                    },
                })?;
        }
        Ok(())
    }

    pub fn uninstall(&mut self, destination: &Destination) -> Result<(), UninstallError> {
        let mut errors = Vec::new();
        for (index, step) in self.steps.iter().enumerate().rev() {
            let description = UninstallDescription(&**step);
            if let Err(error) = self.show_step_result(&description, step.uninstall(destination)) {
                errors.push(StepError {
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
                errors: StepErrors(errors),
            })
        }
    }

    fn show_step_result<T: Display>(
        &self,
        description: T,
        result: eyre::Result<Action>,
    ) -> eyre::Result<()> {
        match &result {
            Ok(Action::Ok) => println!("[{}] {}", " ok ".green(), description),
            Ok(Action::Skipped(reason)) => {
                println!("[{}] {}: {}", "skip".cyan(), description, reason)
            }
            Ok(Action::Warn(reason)) => {
                println!("[{}] {}: {}", "warn".yellow(), description, reason)
            }
            Err(error) => {
                println!("[{}] {}: {}", "fail".red(), description, error)
            }
        }
        result.map(|_| ())
    }
}

pub struct DisplayAsPlan<'a>(&'a SelfInstaller);

impl<'a> Display for DisplayAsPlan<'a> {
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
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
