mod model;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub(crate) use windows::StatusIcon;

#[cfg(target_family = "unix")]
mod xdg;
#[cfg(target_family = "unix")]
pub(crate) use xdg::StatusIcon;
