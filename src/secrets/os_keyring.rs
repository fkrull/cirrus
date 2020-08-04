use crate::secrets::SecretValue;
use anyhow::Context;
use keyring::Keyring;
use std::{
    error::Error,
    fmt::{Debug, Display},
    sync::Mutex,
};
use thiserror::Error;

#[derive(Debug, Error)]
#[error("{}", self.0.lock().unwrap())]
struct SyncError<E: Debug + Display + Error>(Mutex<E>);

impl<E: Debug + Display + Error> SyncError<E> {
    fn new(error: E) -> Self {
        SyncError(Mutex::new(error))
    }
}

const KEYRING_SERVICE: &'static str = "io.gitlab.fkrull.cirrus";

pub(super) fn get_secret(name: &str) -> anyhow::Result<SecretValue> {
    let value = Keyring::new(KEYRING_SERVICE, name)
        .get_password()
        .map_err(SyncError::new)
        .context(format!("no stored password for key '{}'", name))?;
    Ok(SecretValue(value))
}

pub(super) fn set_secret(name: &str, value: SecretValue) -> anyhow::Result<()> {
    Keyring::new(KEYRING_SERVICE, name)
        .set_password(&value.0)
        .map_err(SyncError::new)
        .context(format!("failed to set value for key '{}'", name))
}