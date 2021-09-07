#![windows_subsystem = "windows"]

#[tokio::main]
async fn main() -> eyre::Result<()> {
    #[cfg(windows)]
    unsafe {
        // if this fails, it likely just means we weren't run from a terminal
        winapi::um::wincon::AttachConsole(winapi::um::wincon::ATTACH_PARENT_PROCESS)
    };

    cirrus::main().await
}
