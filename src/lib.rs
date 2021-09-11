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

fn restic_config(restic_binary_arg: Option<PathBuf>) -> restic::Config {
    if let Some(path) = restic_binary_arg {
        restic::Config {
            primary: restic::CommandConfig::from_path(path),
            fallback: None,
        }
    } else {
        let system = restic::CommandConfig {
            path: PathBuf::from("restic"),
            env_var: None,
        };
        let bundled = current_exe_dir().map(|d| {
            let path = d
                .join("restic")
                .with_extension(std::env::consts::EXE_EXTENSION);
            restic::CommandConfig::from_path(path)
        });
        restic::Config {
            primary: system,
            fallback: bundled,
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
    let restic = restic::Restic::new(restic_config(args.restic));
    let secrets = Secrets;

    match args.subcommand {
        cli::Cmd::Daemon(args) => commands::daemon::run(args, restic, secrets, maybe_config?).await,
        cli::Cmd::Backup(args) => commands::backup(&restic, &secrets, &maybe_config?, args).await,
        cli::Cmd::Config => commands::config(&maybe_config?),
        cli::Cmd::Secret(args) => match args.subcommand {
            cli::secret::Cmd::Set(args) => commands::secret::set(&secrets, &maybe_config?, args),
            cli::secret::Cmd::List(args) => commands::secret::list(&secrets, &maybe_config?, args),
        },
        cli::Cmd::Restic(args) => commands::restic(&restic, &secrets, maybe_config, args).await,
        #[cfg(feature = "cirrus-self")]
        cli::Cmd::SelfCommands(args) => cirrus_self::self_action(args),
        cli::Cmd::Version => commands::version(&restic).await,
    }
}
