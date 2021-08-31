use tokio::process::Command;

pub async fn daemon_supervisor() -> eyre::Result<()> {
    let cirrus_exe = std::env::current_exe()?;
    loop {
        let exit_status = Command::new(&cirrus_exe)
            .arg("daemon")
            .spawn()?
            .wait()
            .await;
        match exit_status {
            Ok(s) if s.success() => break,
            _ => continue,
        }
    }
    Ok(())
}
