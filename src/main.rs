#![feature(proc_macro_hygiene, decl_macro)]

use anyhow::{anyhow, Context};
use cirrus::{
    commands,
    jobs::{repo::JobsRepo, runner::JobsRunner},
    model::Config,
    restic::Restic,
    secrets::Secrets,
};
use clap::{App, AppSettings, Arg, ArgSettings};
use env_logger::Env;
use std::{path::PathBuf, sync::Arc};

fn default_config_path() -> anyhow::Result<PathBuf> {
    dirs::config_dir()
        .map(|dir| dir.join("cirrus").join("config.toml"))
        .ok_or_else(|| anyhow!("can't find config file"))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::from_env(Env::default().default_filter_or("info")).init();

    /*let app = Arc::new(App {
        pause_state: PauseState::default(),
        jobs: JobsRepo::new(cfg.backups.0),
        repositories: cfg.repositories,
    });

    // TODO: handle panics in scheduler thread
    scheduler::start_scheduler(app)?;

    #[cfg(feature = "desktop-integration")]
    if let Err(err) = webbrowser::open("http://localhost:8000") {
        log::error!("failed to open web browser: {:?}", err);
    }

    rocket::ignite().mount("/", routes![index]).launch();*/

    let matches = App::new("cirrus")
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
        )
        .get_matches();

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
        _ => {
            let _config = Arc::new(config);
            let restic = Arc::new(restic);
            let secrets = Arc::new(secrets);
            let jobs_repo = Arc::new(JobsRepo::new());
            let (mut runner, _sender) =
                JobsRunner::new(restic.clone(), secrets.clone(), jobs_repo.clone());
            tokio::spawn(async move { runner.run_jobs().await });
            todo!()
        }
    }
}
