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

fn replace_vars(template: &str, executable: &str) -> String {
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

    static CIRRUS_DAEMON_VBS: &str = include_str!("resources/cirrus-daemon.vbs");
    let executable = current_exe()?;
    let startup_dir = windirs::known_folder_path(windirs::FolderId::Startup)?;
    Ok(SelfInstaller::new()
        .add_step(directory(&startup_dir))
        .add_step(file(
            startup_dir.join("cirrus-daemon.vbs"),
            replace_vars(CIRRUS_DAEMON_VBS, &executable),
        )))
}

#[cfg(not(windows))]
fn self_installer() -> eyre::Result<SelfInstaller> {
    use selfinstaller::steps::*;

    static CIRRUS_SERVICE: &str = include_str!("resources/cirrus.service");
    let executable = current_exe()?;
    let systemd_dir = dirs_next::home_dir()
        .ok_or_else(|| eyre::eyre!("failed to get user home"))?
        .join(".config")
        .join("systemd")
        .join("user");
    Ok(SelfInstaller::new()
        .add_step(directory(&systemd_dir))
        .add_step(file(
            systemd_dir.join("cirrus.service"),
            replace_vars(CIRRUS_SERVICE, &executable),
        ))
        .add_step(systemd::enable_user("cirrus.service")))
}

fn install(installer: &mut SelfInstaller, args: Install) -> eyre::Result<()> {
    if args.plan {
        print!("{}", installer.plan());
    } else {
        installer.install(&Destination::from(args.destdir))?;
    }
    Ok(())
}

pub fn self_action(args: Cli) -> eyre::Result<()> {
    let mut installer = self_installer()?;
    match args.command {
        Command::Install(args) => install(&mut installer, args)?,
        Command::Uninstall => installer.uninstall(&Destination::System)?,
    };
    Ok(())
}
