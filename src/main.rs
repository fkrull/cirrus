use cirrus::{cli, commands, daemon};
use cirrus_core::{model::Config, restic::Restic, secrets::Secrets};
use std::path::PathBuf;

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

    let config = load_config(&args).await?;

    let restic = Restic::new(
        args.restic_binary
            .unwrap_or_else(|| PathBuf::from("restic")),
    );
    let secrets = Secrets;

    match args.subcommand {
        Some(cli::Cmd::Daemon(_)) => daemon::run(restic, secrets, config).await,
        Some(cli::Cmd::Backup(args)) => commands::backup(&restic, &secrets, &config, args).await,
        Some(cli::Cmd::Config) => commands::config(&config),
        Some(cli::Cmd::Secret(args)) => match args.subcommand {
            cli::secret::Cmd::Set(args) => commands::secret::set(&secrets, &config, args),
            cli::secret::Cmd::List(args) => commands::secret::list(&secrets, &config, args),
        },
        Some(cli::Cmd::Restic(args)) => commands::restic(&restic, &secrets, &config, args).await,
        Some(cli::Cmd::Generate(args)) => match args.subcommand {
            cli::generate::Cmd::SystemdUnit => commands::generate::systemd_unit(),
            cli::generate::Cmd::BashCompletions => commands::generate::bash_completions(),
        },
        #[cfg(feature = "desktop-commands")]
        Some(cli::Cmd::Desktop(args)) => match args.subcommand {
            cli::desktop::Cmd::OpenConfigFile => {
                commands::desktop::open_config_file(config.source.as_ref().map(|o| o.as_path()))
            }
        },
        None if args.version => commands::version(&restic).await,
        None => unimplemented!(),
    }
}
