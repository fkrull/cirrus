use cirrus::{cli, commands, daemon};
use cirrus_core::{model::Config, restic::Restic, secrets::Secrets};
use dirs_next as dirs;
use std::path::PathBuf;

async fn data_dir() -> eyre::Result<PathBuf> {
    let data_dir = dirs::data_dir()
        .ok_or_else(|| eyre::eyre!("failed to get data dir path"))?
        .join("cirrus");
    tokio::fs::create_dir_all(&data_dir).await?;
    Ok(data_dir)
}

async fn setup_logger() -> eyre::Result<()> {
    use tracing::Level;
    use tracing_subscriber::{
        filter::LevelFilter,
        fmt::{format::FmtSpan, layer, time::ChronoLocal},
        layer::SubscriberExt,
        util::SubscriberInitExt,
        Registry,
    };

    const TIME_FORMAT: &str = "%Y-%m-%d %H:%M:%S%Z";

    let stdout_layer = layer()
        .with_ansi(true)
        .with_target(false)
        .with_timer(ChronoLocal::with_format(String::from(TIME_FORMAT)));

    let data_dir = data_dir().await?;
    let file_layer = layer()
        .with_ansi(false)
        .with_span_events(FmtSpan::CLOSE)
        .with_timer(ChronoLocal::with_format(String::from(TIME_FORMAT)))
        .with_writer(move || tracing_appender::rolling::daily(&data_dir, "cirrus.log"));

    Registry::default()
        .with(LevelFilter::from(Level::INFO))
        .with(stdout_layer)
        .with(file_layer)
        .try_init()?;

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
    setup_logger().await?;

    // exit on thread panic
    let panic_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        panic_hook(panic_info);
        std::process::exit(1);
    }));

    use clap::Clap as _;
    let args: cli::Cli = cli::Cli::parse();

    let config = load_config(&args).await?;

    #[cfg(feature = "bundled-restic")]
    let (restic_binary, _bundled_restic) = if let Some(restic_binary) = args.restic_binary {
        (restic_binary, None)
    } else {
        let bundled_restic = bundled_restic::bundled_restic()?;
        (bundled_restic.path().to_owned(), Some(bundled_restic))
    };

    #[cfg(not(feature = "bundled-restic"))]
    let restic_binary = args
        .restic_binary
        .unwrap_or_else(|| PathBuf::from("restic"));

    let restic = Restic::new(restic_binary);
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
