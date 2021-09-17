use selfinstaller::SelfInstaller;
use std::path::PathBuf;

/// A sample scheduled job that can install its own systemd units.
#[derive(argh::FromArgs)]
struct Args {
    #[argh(subcommand)]
    cmd: Subcommand,
}

#[derive(argh::FromArgs)]
#[argh(subcommand)]
enum Subcommand {
    Run(Run),
    Install(Install),
    Uninstall(Uninstall),
    Details(Details),
}

/// Run the job.
#[derive(argh::FromArgs)]
#[argh(subcommand, name = "run")]
struct Run {
    /// what to print
    #[argh(positional)]
    message: String,
}

/// Install the systemd unit for the job.
#[derive(argh::FromArgs)]
#[argh(subcommand, name = "install")]
struct Install {
    /// install into this directory instead of the system root
    #[argh(option)]
    destdir: Option<PathBuf>,
}

/// Uninstall the systemd unit.
#[derive(argh::FromArgs)]
#[argh(subcommand, name = "uninstall")]
struct Uninstall {
    /// uninstall from this directory instead of the system root
    #[argh(option)]
    destdir: Option<PathBuf>,
}

/// Show installation steps.
#[derive(argh::FromArgs)]
#[argh(subcommand, name = "details")]
struct Details {}

fn installer() -> eyre::Result<SelfInstaller> {
    use selfinstaller::steps::*;

    static SERVICE: &'static str = include_str!("scheduledjob.service");
    static TIMER: &'static str = include_str!("scheduledjob.timer");
    let exe = std::env::current_exe()?
        .to_str()
        .ok_or_else(|| eyre::eyre!("executable name is not valid UTF-8"))?
        .to_owned();

    Ok(SelfInstaller::new()
        .add_step(directory("/etc/systemd/system"))
        .add_step(file(
            "/etc/systemd/system/scheduledjob.service",
            SERVICE.replace("{{executable}}", &exe),
        ))
        .add_step(file("/etc/systemd/system/scheduledjob.timer", TIMER))
        .add_step(systemd::enable("scheduledjob.timer")))
}

fn main() -> eyre::Result<()> {
    let args: Args = argh::from_env();
    match args.cmd {
        Subcommand::Run(args) => {
            println!("{}", args.message);
        }
        Subcommand::Install(args) => {
            installer()?.install(&args.destdir.into())?;
        }
        Subcommand::Uninstall(args) => {
            installer()?.uninstall(&args.destdir.into())?;
        }
        Subcommand::Details(_) => {
            print!("{}", installer()?.details());
        }
    }
    Ok(())
}
