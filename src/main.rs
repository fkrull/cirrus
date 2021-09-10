#![windows_subsystem = "windows"]

#[tokio::main]
async fn main() -> eyre::Result<()> {
    #[cfg(windows)]
    unsafe {
        // if this fails, it likely just means we weren't run from a terminal
        winapi::um::wincon::AttachConsole(winapi::um::wincon::ATTACH_PARENT_PROCESS)
    };

    #[cfg(feature = "restigo")]
    if std::env::var_os("__CIRRUS_INTERNAL_MODE_BUNDLED_RESTIC").is_some() {
        restigo::restic_main();
    }

    cirrus::main().await
}
