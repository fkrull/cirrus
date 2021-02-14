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
    use log4rs::{
        append::console::ConsoleAppender,
        append::rolling_file::policy::compound::roll::delete::DeleteRoller,
        append::rolling_file::policy::compound::trigger::size::SizeTrigger,
        append::rolling_file::policy::compound::CompoundPolicy,
        append::rolling_file::RollingFileAppender, config::Appender, config::Root,
        encode::pattern::PatternEncoder, Config,
    };

    let log_file = data_dir().await?.join("cirrus.log");

    let stdout_encoder = PatternEncoder::new("[{d(%Y-%m-%d %H:%M:%S%Z)} {h({l}):>5}] {m}{n}");
    let stdout = ConsoleAppender::builder()
        .encoder(Box::new(stdout_encoder))
        .build();

    let file_encoder = PatternEncoder::new("[{d(%Y-%m-%d %H:%M:%S%Z)} {l} - {M}] {m}{n}");
    let policy = CompoundPolicy::new(
        Box::new(SizeTrigger::new(20 * 1024 * 1024)),
        Box::new(DeleteRoller::new()),
    );
    let file = RollingFileAppender::builder()
        .encoder(Box::new(file_encoder))
        .build(&log_file, Box::new(policy))?;

    let config = Config::builder()
        .appender(Appender::builder().build("stdout", Box::new(stdout)))
        .appender(Appender::builder().build("file", Box::new(file)))
        .build(
            Root::builder()
                .appender("stdout")
                .appender("file")
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
