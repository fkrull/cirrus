use cirrus::{cli, commands, daemon};
use cirrus_core::{appconfig::AppConfig, model::Config, restic::Restic, secrets::Secrets};
use eyre::{eyre, WrapErr};
use std::path::PathBuf;

fn default_config_path() -> eyre::Result<PathBuf> {
    dirs::config_dir()
        .map(|dir| dir.join("cirrus").join("backups.toml"))
        .ok_or_else(|| eyre!("can't find config file"))
}

fn default_app_config_path() -> eyre::Result<PathBuf> {
    dirs::config_dir()
        .map(|dir| dir.join("cirrus").join("application.toml"))
        .ok_or_else(|| eyre!("can't find application config file"))
}

async fn load_config(args: &cli::Cli) -> eyre::Result<Config> {
    if let Some(config_string) = &args.config_string {
        let config: Config = toml::from_str(config_string)
            .wrap_err_with(|| format!("failed to parse config string"))?;
        Ok(config)
    } else {
        let config_path = args
            .config_file
            .clone()
            .map(Ok)
            .unwrap_or_else(default_config_path)
            .wrap_err("failed to get default path for the config file")?;
        let config_string = tokio::fs::read_to_string(&config_path)
            .await
            .wrap_err_with(|| format!("failed to read config file '{}'", config_path.display()))?;
        let mut config: Config = toml::from_str(&config_string)
            .wrap_err_with(|| format!("failed to parse config file '{}'", config_path.display()))?;
        config.source = Some(config_path);
        Ok(config)
    }
}

async fn load_appconfig(args: &cli::Cli) -> eyre::Result<(PathBuf, AppConfig)> {
    let appconfig_path = args
        .appconfig_file
        .clone()
        .map(Ok)
        .unwrap_or_else(default_app_config_path)
        .wrap_err("failed to get default path for the application config file")?;
    let mut appconfig: AppConfig = tokio::fs::read_to_string(&appconfig_path)
        .await
        .ok()
        .map(|s| toml::from_str(&s))
        .transpose()
        .wrap_err_with(|| {
            format!(
                "failed to parse application config file '{}'",
                appconfig_path.display()
            )
        })?
        .unwrap_or_default();

    if let Some(restic_binary) = &args.restic_binary {
        appconfig.restic_binary = restic_binary.clone();
    }

    Ok((appconfig_path, appconfig))
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    color_eyre::install()?;
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

    // exit on thread panic
    let panic_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        panic_hook(panic_info);
        std::process::exit(1);
    }));

    use clap::Clap as _;
    let args: cli::Cli = cli::Cli::parse();

    let config = load_config(&args).await?;
    #[allow(unused_variables)]
    let (appconfig_path, appconfig) = load_appconfig(&args).await?;
    let restic = Restic::new(&appconfig.restic_binary);
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
            cli::desktop::Cmd::OpenAppconfigFile => {
                commands::desktop::open_appconfig_file(&appconfig_path).await
            }
        },
        None => daemon::run(restic, secrets, config, appconfig).await,
    }
}
