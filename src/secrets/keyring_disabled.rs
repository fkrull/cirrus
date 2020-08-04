use crate::secrets::SecretValue;
use anyhow::anyhow;

pub(super) fn get_secret(_name: &str) -> anyhow::Result<SecretValue> {
    Err(anyhow!("OS keyring support is not enabled"))
}

pub(super) fn set_secret(_name: &str, _value: SecretValue) -> anyhow::Result<()> {
    Err(anyhow!("OS keyring support is not enabled"))
}
