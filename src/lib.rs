use crate::cli::ResticArg;
use cirrus_core::{cache::Cache, config::Config, restic, secrets::Secrets};
use dirs_next as dirs;
use std::path::PathBuf;

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
    let maybe_config = load_config(&args).await;
    let restic = restic::Restic::new(restic_config(args.restic)?);
    let secrets = Secrets;
    let cache_dir = dirs::cache_dir()
        .ok_or_else(|| eyre::eyre!("can't determine cache directory"))?
        .join("cirrus");
    let cache = Cache::new(cache_dir);

    match args.subcommand {
        cli::Cmd::Daemon(args) => {
            commands::daemon::run(args, restic, secrets, maybe_config?, cache).await
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
        cli::Cmd::RepoContents(args) => {
            commands::repo_contents::repo_contents(&restic, &secrets, &maybe_config?, &cache, args)
                .await
        }
        cli::Cmd::Version => commands::version(&restic).await,
    }
}
