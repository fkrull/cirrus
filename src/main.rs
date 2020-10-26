use cirrus::{commands, daemon};
use cirrus_core::appconfig::AppConfig;
use cirrus_core::{model::Config, restic::Restic, secrets::Secrets};
use clap::{App, AppSettings, Arg, ArgMatches, ArgSettings};
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

async fn load_config(matches: &ArgMatches<'_>) -> eyre::Result<(PathBuf, Config)> {
    let config_path = matches
        .value_of_os("config")
        .map(PathBuf::from)
        .map(Ok)
        .unwrap_or_else(|| default_config_path())
        .wrap_err("failed to get default path for the config file")?;
    let config_string = tokio::fs::read_to_string(&config_path)
        .await
        .wrap_err_with(|| format!("failed to read config file '{}'", config_path.display()))?;
    let config: Config = toml::from_str(&config_string)
        .wrap_err_with(|| format!("failed to parse config file '{}'", config_path.display()))?;
    Ok((config_path, config))
}

async fn load_appconfig(matches: &ArgMatches<'_>) -> eyre::Result<AppConfig> {
    let appconfig_path = matches
        .value_of_os("app-config")
        .map(PathBuf::from)
        .map(Ok)
        .unwrap_or_else(|| default_app_config_path())
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

    if let Some(restic_binary) = matches.value_of("restic-binary") {
        appconfig.restic_binary = restic_binary.to_owned();
    }

    Ok(appconfig)
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    color_eyre::install()?;
    env_logger::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let cli = App::new("cirrus")
        .arg(
            Arg::with_name("config")
                .short("c")
                .long("config")
                .value_name("FILE")
                .help("Set a custom config file")
                .env("CIRRUS_CONFIG")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("app-config")
                .long("app-config")
                .value_name("FILE")
                .help("Set a custom application config file")
                .env("CIRRUS_APP_CONFIG")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("restic-binary")
                .long("restic-binary")
                .help("Set the restic binary to use")
                .takes_value(true),
        )
        .subcommand(
            App::new("backup").arg(
                Arg::with_name("backup")
                    .help("the backup to run")
                    .required(true)
                    .takes_value(true),
            ),
        )
        .subcommand(
            App::new("secret")
                .alias("secrets")
                .setting(AppSettings::SubcommandRequiredElseHelp)
                .subcommand(
                    App::new("set")
                        .arg(
                            Arg::with_name("secret-set-repo")
                                .help("the repository of the secret")
                                .value_name("REPOSITORY")
                                .required(true)
                                .takes_value(true),
                        )
                        .arg(
                            Arg::with_name("secret-set-secret")
                                .help("the name of the secret")
                                .value_name("SECRET")
                                .takes_value(true),
                        ),
                )
                .subcommand(
                    App::new("list").arg(
                        Arg::with_name("secret-list-show-passwords")
                            .long("show-passwords")
                            .help("show passwords in clear text"),
                    ),
                ),
        )
        .subcommand(
            App::new("restic")
                .arg(
                    Arg::with_name("repo")
                        .short("r")
                        .long("repo")
                        .value_name("REPOSITORY")
                        .help("Set the cirrus repository to use")
                        .env("CIRRUS_REPOSITORY")
                        .takes_value(true),
                )
                .setting(AppSettings::TrailingVarArg)
                .arg(
                    Arg::with_name("cmd")
                        .help("command-line arguments to pass to restic")
                        .required(true)
                        .set(ArgSettings::AllowLeadingHyphen)
                        .multiple(true),
                ),
        );
    #[cfg(feature = "desktop-commands")]
    let cli = cli.subcommand(
        App::new("desktop")
            .setting(AppSettings::SubcommandRequiredElseHelp)
            .subcommand(App::new("open-config-file")),
    );

    let matches = cli.get_matches();
    let (config_path, config) = load_config(&matches).await?;
    let appconfig = load_appconfig(&matches).await?;
    let restic = Restic::new(&appconfig.restic_binary);
    let secrets = Secrets;
    let _ = config_path;

    match matches.subcommand() {
        ("restic", Some(matches)) => commands::restic(&restic, &secrets, &config, matches).await,
        ("backup", Some(matches)) => commands::backup(&restic, &secrets, &config, matches).await,
        ("secret", Some(matches)) => match matches.subcommand() {
            ("list", Some(matches)) => commands::secret::list(&secrets, &config, matches).await,
            ("set", Some(matches)) => commands::secret::set(&secrets, &config, matches).await,
            _ => unreachable!("unexpected subcommand for secret"),
        },
        #[cfg(feature = "desktop-commands")]
        ("desktop", Some(matches)) => match matches.subcommand() {
            ("open-config-file", Some(_)) => commands::desktop::open_config_file(&config_path),
            _ => unreachable!("unexpected subcommand for desktop"),
        },
        _ => daemon::run(restic, secrets, config, appconfig, &matches).await,
    }
}
