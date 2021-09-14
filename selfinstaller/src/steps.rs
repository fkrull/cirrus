//! Default installation steps.

pub mod directory;
pub use directory::directory;
pub mod file;
pub use file::{executable, file};
pub mod systemd;
#[cfg(windows)]
pub mod windows;

#[cfg(test)]
mod testutil {
    use crate::{InstallDescription, InstallStep, UninstallDescription};

    struct Details<'a, T>(&'a T);

    impl<'a, T: InstallStep> std::fmt::Display for Details<'a, T> {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            self.0.details(f)
        }
    }

    pub(super) fn install_description<T: InstallStep>(step: &T) -> String {
        let desc = InstallDescription(&*step);
        format!("{}", desc)
    }

    pub(super) fn uninstall_description<T: InstallStep>(step: &T) -> String {
        let desc = UninstallDescription(&*step);
        format!("{}", desc)
    }

    pub(super) fn details<T: InstallStep>(step: &T) -> String {
        let desc = Details(step);
        format!("{}", desc)
    }
}
