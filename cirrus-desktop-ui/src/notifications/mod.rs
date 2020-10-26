#[cfg(windows)]
mod winrt;
#[cfg(windows)]
pub(crate) use self::winrt::Notifications;

#[cfg(not(windows))]
mod notify;
#[cfg(not(windows))]
pub(crate) use self::notify::Notifications;
