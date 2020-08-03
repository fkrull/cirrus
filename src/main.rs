#![feature(proc_macro_hygiene, decl_macro)]

use anyhow::{anyhow, Context};
use cirrus::secrets::SecretValue;
use cirrus::{
    model::{self, backup, repo},
    restic::Restic,
    secrets::Secrets,
    Cirrus,
};
use clap::{App, AppSettings, Arg, ArgMatches, ArgSettings};
use env_logger::Env;
use std::path::PathBuf;

fn default_config_path() -> anyhow::Result<PathBuf> {
    dirs::config_dir()
        .map(|dir| dir.join("cirrus").join("config.toml"))
        .ok_or_else(|| anyhow!("can't find config file"))
}

fn run_restic(app: &Cirrus, matches: &ArgMatches) -> anyhow::Result<()> {
    let cmd = matches.values_of("cmd").unwrap();
    match matches.value_of("repo") {
        Some(repo_name) => {
            let repo_name = repo::Name(repo_name.to_owned());
            let repo = app.config.repository(&repo_name)?;
            let secrets = app.secrets.get_secrets(repo)?;
            app.restic.run(repo, &secrets, cmd)?.wait()?;
        }
        None => {
            app.restic.run_raw(cmd)?.wait()?;
        }
    }

    Ok(())
}

fn run_backup(app: &Cirrus, matches: &ArgMatches) -> anyhow::Result<()> {
    let backup_name = backup::Name(matches.value_of("backup").unwrap().to_owned());
    let backup = app.config.backup(&backup_name)?;
    let repo = app.config.repository_for_backup(backup)?;
    let secrets = app.secrets.get_secrets(repo)?;
    app.restic.backup(repo, &secrets, backup)?.wait()
}

fn run_secret(app: &Cirrus, matches: &ArgMatches) -> anyhow::Result<()> {
    match matches.subcommand() {
        ("list", Some(_)) => todo!(),
        ("set", Some(matches)) => {
            let repo_name = repo::Name(matches.value_of("secret-set-repo").unwrap().to_owned());
            let secret_name = matches
                .value_of("secret-set-secret")
                .map(|s| repo::SecretName(s.to_owned()));
            let repo = app.config.repository(&repo_name)?;

            let (secret, value) = match secret_name {
                None => {
                    let prompt = format!("Password for repository '{}': ", repo_name.0);
                    let value = SecretValue::new(rpassword::read_password_from_tty(Some(&prompt))?);
                    (&repo.password, value)
                }
                Some(secret_name) => {
                    let secret = repo
                        .secrets
                        .get(&secret_name)
                        .ok_or_else(|| anyhow!("no such secret '{}'", secret_name.0))?;
                    let prompt = format!("Value for secret '{}.{}': ", repo_name.0, secret_name.0);
                    let value = SecretValue::new(rpassword::read_password_from_tty(Some(&prompt))?);
                    (secret, value)
                }
            };
            app.secrets.set_secret(secret, value)
        }
        _ => unreachable!("unexpected secret subcommand"),
    }
}

fn main() -> anyhow::Result<()> {
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
                .subcommand(App::new("list")),
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
    let cfg_data = std::fs::read_to_string(&config_path).context(format!(
        "failed to read config file '{}'",
        config_path.display()
    ))?;
    let config: model::Config = toml::from_str(&cfg_data).context(format!(
        "failed to parse config file '{}'",
        config_path.display()
    ))?;

    let app = Cirrus {
        config,
        restic: Restic::new(matches.value_of("restic-binary").unwrap()),
        secrets: Secrets,
    };

    match matches.subcommand() {
        ("restic", Some(matches)) => run_restic(&app, matches),
        ("backup", Some(matches)) => run_backup(&app, matches),
        ("secret", Some(matches)) => run_secret(&app, matches),
        _ => todo!(),
    }
}
