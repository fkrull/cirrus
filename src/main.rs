use cirrus::{cli, commands};
use cirrus_core::{model::Config, restic::Restic, secrets::Secrets};

async fn load_config(args: &cli::Cli) -> eyre::Result<Config> {
    let config = if let Some(config_string) = &args.config_string {
        Config::from_str(config_string)?
    } else {
        Config::from_file(args.config_file.path()?).await?
    };
    Ok(config)
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
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

    let restic = Restic::new(args.restic_binary);
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
        cli::Cmd::Generate(args) => match args.subcommand {
            cli::generate::Cmd::SystemdUnit => commands::generate::systemd_unit(),
            cli::generate::Cmd::BashCompletions => commands::generate::bash_completions(),
        },
        #[cfg(feature = "desktop-commands")]
        cli::Cmd::Desktop(args) => match args.subcommand {
            cli::desktop::Cmd::OpenConfigFile => commands::desktop::open_config_file(
                maybe_config?.source.as_ref().map(|o| o.as_path()),
            ),
        },
        cli::Cmd::Version => commands::version(&restic).await,
    }
}
