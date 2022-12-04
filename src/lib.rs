use crate::cli::{LogLevel, ResticArg};
use cirrus_core::{cache::Cache, config::Config, restic, secrets::Secrets};
use dirs_next as dirs;
use std::path::PathBuf;
use tracing_subscriber::{
    filter::{LevelFilter, Targets},
    fmt::{format::FmtSpan, layer, time::LocalTime},
    prelude::*,
    registry,
};

mod cli;
mod commands;

async fn load_config(args: &cli::Cli) -> eyre::Result<Config> {
    let config = if let Some(config_string) = &args.config_string {
        Config::parse(config_string)?
    } else {
        Config::parse_file(args.config_file.path()?).await?
    };
    Ok(config)
}

fn system_restic() -> restic::CommandConfig {
    restic::CommandConfig::from_path(PathBuf::from("restic"))
}

#[cfg(feature = "bundled-restic-support")]
fn bundled_restic() -> eyre::Result<restic::CommandConfig> {
    let current_exe = std::env::current_exe()?;
    let bundled_path = current_exe
        .parent()
        .ok_or_else(|| eyre::eyre!("can't determine parent directory for executable"))?
        .join("restic")
        .with_extension(std::env::consts::EXE_EXTENSION);
    Ok(restic::CommandConfig::from_path(bundled_path))
}

fn restic_config(restic_arg: ResticArg) -> eyre::Result<restic::Config> {
    let config = match restic_arg {
        ResticArg::System => restic::Config {
            primary: system_restic(),
            fallback: None,
        },
        #[cfg(feature = "bundled-restic-support")]
        ResticArg::Bundled => restic::Config {
            primary: bundled_restic()?,
            fallback: None,
        },
        #[cfg(feature = "bundled-restic-support")]
        ResticArg::SystemThenBundled => restic::Config {
            primary: system_restic(),
            fallback: bundled_restic().ok(),
        },
        ResticArg::Path(path) => restic::Config {
            primary: restic::CommandConfig::from_path(path),
            fallback: None,
        },
    };
    Ok(config)
}

fn setup_cli_logger(log_level: Option<LogLevel>) -> eyre::Result<()> {
    tracing_subscriber::registry()
        .with(
            layer()
                .without_time()
                .with_level(false)
                .with_target(false)
                .with_file(false)
                .with_line_number(false)
                .with_filter(Targets::new().with_target("cli", LevelFilter::INFO)),
        )
        .with(
            layer()
                .with_ansi(true)
                .with_target(false)
                .without_time()
                .with_filter(
                    Targets::new()
                        .with_target("cli", LevelFilter::OFF)
                        .with_default(log_level.map(Into::into)),
                ),
        )
        .try_init()?;

    Ok(())
}

fn setup_daemon_logger(log_level: LogLevel, log_file: Option<&PathBuf>) -> eyre::Result<()> {
    let builder = registry()
        .with(LevelFilter::from_level(log_level.into()))
        .with(layer().with_ansi(true).with_target(false).without_time());

    if let Some(log_file) = log_file {
        let time_format = time::macros::format_description!(
            "[year]-[month]-[day] [hour repr:24]:[minute]:[second][offset_hour sign:mandatory]:[offset_minute]"
        );

        let file = std::fs::File::options()
            .append(true)
            .create(true)
            .open(log_file)?;
        builder
            .with(
                layer()
                    .with_ansi(false)
                    .with_span_events(FmtSpan::CLOSE)
                    .with_timer(LocalTime::new(time_format))
                    .with_writer(file),
            )
            .try_init()?;
    } else {
        builder.try_init()?;
    }

    Ok(())
}

fn setup_logger(args: &cli::Cli) -> eyre::Result<()> {
    match &args.subcommand {
        cli::Cmd::Daemon(daemon_args) => setup_daemon_logger(
            args.log_level.unwrap_or(LogLevel::Info),
            daemon_args.log_file.as_ref(),
        ),
        _ => setup_cli_logger(args.log_level),
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

    use clap::Parser as _;
    let args: cli::Cli = cli::Cli::parse();
    setup_logger(&args)?;
    let maybe_config = load_config(&args).await;
    let restic = restic::Restic::new(restic_config(args.restic)?);
    let secrets = Secrets;
    let cache_dir = dirs::cache_dir()
        .ok_or_else(|| eyre::eyre!("can't determine cache directory"))?
        .join("cirrus");
    let cache = Cache::new(cache_dir);

    match args.subcommand {
        cli::Cmd::Daemon(args) => {
            commands::daemon::main(args, restic, secrets, maybe_config?, cache).await
        }
        cli::Cmd::Backup(args) => commands::backup(&restic, &secrets, &maybe_config?, args).await,
        cli::Cmd::Config => commands::config(&maybe_config?),
        cli::Cmd::Secret(args) => match args.subcommand {
            cli::secret::Cmd::Set(args) => commands::secret::set(&secrets, &maybe_config?, args),
            cli::secret::Cmd::List(args) => commands::secret::list(&secrets, &maybe_config?, args),
        },
        cli::Cmd::Restic(args) => commands::restic(&restic, &secrets, maybe_config, args).await,
        #[cfg(feature = "cirrus-self")]
        cli::Cmd::SelfCommands(args) => cirrus_self::self_action(args),
        cli::Cmd::Files(args) => {
            commands::files::main(restic, secrets, cache, maybe_config?, args).await
        }
        cli::Cmd::RepoContents(args) => {
            commands::repo_contents::repo_contents(&restic, &secrets, &maybe_config?, &cache, args)
                .await
        }
        cli::Cmd::Version => commands::version(&restic).await,
    }
}
