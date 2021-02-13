use cirrus::{cli, commands, daemon};
use cirrus_core::{model::Config, restic::Restic, secrets::Secrets};

fn setup_logger() -> eyre::Result<()> {
    use log4rs::{
        append::console::ConsoleAppender,
        config::{Appender, Root},
        encode::pattern::PatternEncoder,
        Config,
    };

    let encoder = PatternEncoder::new("[{d(%Y-%m-%d %H:%M:%S %Z)} {h({l}):>5}] {m}\n");
    let stdout = ConsoleAppender::builder()
        .encoder(Box::new(encoder))
        .build();

    let config = Config::builder()
        .appender(Appender::builder().build("stdout", Box::new(stdout)))
        .build(
            Root::builder()
                .appender("stdout")
                .build(log::LevelFilter::Info),
        )?;

    let _ = log4rs::init_config(config)?;

    Ok(())
}

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
    setup_logger()?;

    // exit on thread panic
    let panic_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        panic_hook(panic_info);
        std::process::exit(1);
    }));

    use clap::Clap as _;
    let args: cli::Cli = cli::Cli::parse();

    let config = load_config(&args).await?;
    let restic = Restic::new(&args.restic_binary.clone());
    let secrets = Secrets;

    match args.subcommand {
        Some(cli::Cmd::Backup(args)) => commands::backup(&restic, &secrets, &config, args).await,
        Some(cli::Cmd::Config) => commands::config(&config),
        Some(cli::Cmd::Secret(args)) => match args.subcommand {
            cli::secret::Cmd::Set(args) => commands::secret::set(&secrets, &config, args),
            cli::secret::Cmd::List(args) => commands::secret::list(&secrets, &config, args),
        },
        Some(cli::Cmd::Restic(args)) => commands::restic(&restic, &secrets, &config, args).await,
        #[cfg(feature = "desktop-commands")]
        Some(cli::Cmd::Desktop(args)) => match args.subcommand {
            cli::desktop::Cmd::OpenConfigFile => {
                commands::desktop::open_config_file(config.source.as_ref().map(|o| o.as_path()))
            }
        },
        None => daemon::run(restic, secrets, config).await,
    }
}
