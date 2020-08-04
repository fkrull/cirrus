use crate::secrets::SecretValue;

pub(super) fn get_secret(name: &str) -> anyhow::Result<SecretValue> {
    Err(anyhow!("OS keyring support is not enabled"))
}

pub(super) fn set_secret(name: &str, value: SecretValue) -> anyhow::Result<()> {
    Err(anyhow!("OS keyring support is not enabled"))
}
