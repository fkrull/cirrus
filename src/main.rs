use anyhow::{anyhow, Context};
use cirrus::{commands, daemon};
use cirrus_core::{model::Config, restic::Restic, secrets::Secrets};
use clap::{App, AppSettings, Arg, ArgSettings};
use env_logger::Env;
use std::path::PathBuf;

fn default_config_path() -> anyhow::Result<PathBuf> {
    dirs::config_dir()
        .map(|dir| dir.join("cirrus").join("config.toml"))
        .ok_or_else(|| anyhow!("can't find config file"))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::from_env(Env::default().default_filter_or("info")).init();

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
            Arg::with_name("restic-binary")
                .long("restic-binary")
                .help("Set the restic binary to use")
                .default_value("restic")
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
    #[cfg(feature = "desktop-integration")]
    let cli = cli.subcommand(
        App::new("desktop")
            .setting(AppSettings::SubcommandRequiredElseHelp)
            .subcommand(App::new("open-config-file")),
    );

    let matches = cli.get_matches();
    let config_path = matches
        .value_of_os("config")
        .map(PathBuf::from)
        .map(Ok)
        .unwrap_or_else(|| default_config_path())
        .context("failed to get path for the default config file")?;
    let cfg_data = tokio::fs::read_to_string(&config_path)
        .await
        .context(format!(
            "failed to read config file '{}'",
            config_path.display()
        ))?;
    let config: Config = toml::from_str(&cfg_data).context(format!(
        "failed to parse config file '{}'",
        config_path.display()
    ))?;

    let restic = Restic::new(matches.value_of("restic-binary").unwrap());
    let secrets = Secrets;

    match matches.subcommand() {
        ("restic", Some(matches)) => commands::restic(&restic, &secrets, &config, matches).await,
        ("backup", Some(matches)) => commands::backup(&restic, &secrets, &config, matches).await,
        ("secret", Some(matches)) => match matches.subcommand() {
            ("list", Some(matches)) => commands::secret::list(&secrets, &config, matches).await,
            ("set", Some(matches)) => commands::secret::set(&secrets, &config, matches).await,
            _ => unreachable!("unexpected subcommand for secret"),
        },
        #[cfg(feature = "desktop-integration")]
        ("desktop", Some(matches)) => match matches.subcommand() {
            ("open-config-file", Some(_)) => commands::desktop::open_config_file(&config_path),
            _ => unreachable!("unexpected subcommand for desktop"),
        },
        _ => daemon::run(restic, secrets, config, &matches).await,
    }
}
