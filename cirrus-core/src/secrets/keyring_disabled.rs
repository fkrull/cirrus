use crate::secrets::SecretValue;

pub(super) fn get_secret(_name: &str) -> eyre::Result<SecretValue> {
    Err(eyre::eyre!("OS keyring support is not enabled"))
}

pub(super) fn set_secret(_name: &str, _value: SecretValue) -> eyre::Result<()> {
    Err(eyre::eyre!("OS keyring support is not enabled"))
}
