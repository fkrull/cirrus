#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub(crate) use self::windows::Notifications;

#[cfg(target_family = "unix")]
mod xdg;
#[cfg(target_family = "unix")]
pub(crate) use self::xdg::Notifications;
