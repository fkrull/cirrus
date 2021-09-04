use cirrus_core::{model::Config, restic, secrets::Secrets};
use std::path::PathBuf;

mod cli;
mod commands;

async fn load_config(args: &cli::Cli) -> eyre::Result<Config> {
    let config = if let Some(config_string) = &args.config_string {
        Config::from_str(config_string)?
    } else {
        Config::from_file(args.config_file.path()?).await?
    };
    Ok(config)
}

fn current_exe_dir() -> Option<PathBuf> {
    let current_exe = std::env::current_exe().ok()?;
    let dir = current_exe.parent()?.to_owned();
    Some(dir)
}

fn restic_binary_config(restic_binary_arg: Option<PathBuf>) -> restic::BinaryConfig {
    if let Some(path) = restic_binary_arg {
        restic::BinaryConfig {
            path,
            fallback: None,
        }
    } else {
        let system = PathBuf::from("restic");
        let fallback = current_exe_dir().map(|d| {
            d.join("restic")
                .with_extension(std::env::consts::EXE_EXTENSION)
        });
        restic::BinaryConfig {
            path: system,
            fallback,
        }
    }
}

pub async fn main() -> eyre::Result<()> {
    color_eyre::install()?;

    // exit on thread panic
    let panic_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        panic_hook(panic_info);
        std::process::exit(1);
    }));

    use clap::Clap as _;
    let args: cli::Cli = cli::Cli::parse();
    let maybe_config = load_config(&args).await;
    let restic = restic::Restic::new(restic_binary_config(args.restic_binary));
    let secrets = Secrets;

    match args.subcommand {
        cli::Cmd::Daemon => commands::daemon::run(restic, secrets, maybe_config?).await,
        cli::Cmd::Backup(args) => commands::backup(&restic, &secrets, &maybe_config?, args).await,
        cli::Cmd::Config => commands::config(&maybe_config?),
        cli::Cmd::Secret(args) => match args.subcommand {
            cli::secret::Cmd::Set(args) => commands::secret::set(&secrets, &maybe_config?, args),
            cli::secret::Cmd::List(args) => commands::secret::list(&secrets, &maybe_config?, args),
        },
        cli::Cmd::Restic(args) => commands::restic(&restic, &secrets, &maybe_config?, args).await,
        #[cfg(feature = "selfinstaller")]
        cli::Cmd::SelfCommands(args) => cirrus_self::run_self_action(args),
        cli::Cmd::Internal(args) => match args.subcommand {
            #[cfg(feature = "daemon-supervisor")]
            cli::internal::Cmd::DaemonSupervisor => commands::internal::daemon_supervisor().await,
        },
        cli::Cmd::Version => commands::version(&restic).await,
    }
}
