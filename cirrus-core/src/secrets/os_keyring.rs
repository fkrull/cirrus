use crate::secrets::SecretValue;
use eyre::WrapErr;
use std::{
    error::Error,
    fmt::{Debug, Display},
    sync::Mutex,
};

#[derive(Debug, thiserror::Error)]
#[error("{}", self.0.lock().unwrap())]
struct SyncError<E: Debug + Display + Error>(Mutex<E>);

impl<E: Debug + Display + Error> SyncError<E> {
    fn new(error: E) -> Self {
        SyncError(Mutex::new(error))
    }
}

const KEYRING_SERVICE: &str = "io.gitlab.fkrull.cirrus";

pub(super) fn get_secret(name: &str) -> eyre::Result<SecretValue> {
    let value = keyring::Entry::new(KEYRING_SERVICE, name)
        .get_password()
        .map_err(SyncError::new)
        .wrap_err_with(|| format!("no stored password for key '{}'", name))?;
    Ok(SecretValue(value))
}

pub(super) fn set_secret(name: &str, value: SecretValue) -> eyre::Result<()> {
    keyring::Entry::new(KEYRING_SERVICE, name)
        .set_password(&value.0)
        .map_err(SyncError::new)
        .wrap_err_with(|| format!("failed to set value for key '{}'", name))
}
