use selfinstaller::{Destination, SelfInstaller};
use std::path::PathBuf;

#[derive(clap::Clap)]
pub struct Cli {
    #[clap(subcommand)]
    pub command: Command,
}

#[derive(clap::Clap)]
pub enum Command {
    /// Installs the daemon autostart files
    Install(Install),

    /// Uninstalls the daemon autostart files
    Uninstall,
}

#[derive(clap::Clap)]
pub struct Install {
    /// Shows the installation plan instead of running it
    #[clap(long)]
    plan: bool,

    /// Installs into PATH instead of the filesystem
    #[clap(long, value_name = "PATH")]
    destdir: Option<PathBuf>,
}

mod resources {
    pub static CIRRUS_SERVICE: &str = include_str!("resources/cirrus.service");
    pub static CIRRUS_DAEMON_VBS: &str = include_str!("resources/cirrus-daemon.vbs");
}

pub fn contents(template: &str, executable: &str) -> String {
    template.replace("{{executable}}", executable)
}

fn current_exe() -> eyre::Result<String> {
    std::env::current_exe()?
        .into_os_string()
        .into_string()
        .map_err(|p| eyre::eyre!("executable path was not valid UTF-8: {:?}", p))
}

#[cfg(windows)]
fn self_installer() -> eyre::Result<SelfInstaller> {
    use selfinstaller::steps::*;

    let executable = current_exe()?;
    let startup_dir = windirs::known_folder_path(windirs::FolderId::Startup)?;
    Ok(SelfInstaller::new()
        .add_step(directory(&startup_dir))
        .add_step(file(
            startup_dir.join("cirrus-daemon.vbs"),
            contents(resources::CIRRUS_DAEMON_VBS, &executable),
        )))
}

#[cfg(not(windows))]
fn self_installer() -> eyre::Result<SelfInstaller> {
    todo!()
}

fn install(installer: &mut SelfInstaller, args: Install) -> eyre::Result<()> {
    if args.plan {
        print!("{}", installer.plan());
    } else {
        installer.install(&Destination::from(args.destdir))?;
    }
    Ok(())
}

pub fn run_self_action(args: Cli) -> eyre::Result<()> {
    let mut installer = self_installer()?;
    match args.command {
        Command::Install(args) => install(&mut installer, args),
        Command::Uninstall => installer.uninstall(&Destination::System),
    }
}
