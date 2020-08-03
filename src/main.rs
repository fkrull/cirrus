#![feature(proc_macro_hygiene, decl_macro)]

use crate::model::{backup, repo};
use anyhow::{anyhow, Context};
//use cirrus::{jobs::JobsRepo, model, pause::PauseState, scheduler, App};
use env_logger::Env;
//use rocket::{get, routes};
use cirrus::model;
use cirrus::model::Config;
use cirrus::restic::Restic;
use cirrus::secrets::get_secrets;
use clap::{App, AppSettings, Arg, ArgMatches, ArgSettings};
use std::path::PathBuf;

fn default_config_path() -> anyhow::Result<PathBuf> {
    dirs::config_dir()
        .map(|dir| dir.join("cirrus").join("config.toml"))
        .ok_or_else(|| anyhow!("can't find config file"))
}

fn run_restic(config: &Config, restic: &Restic, matches: &ArgMatches) -> anyhow::Result<()> {
    let cmd = matches.values_of("cmd").unwrap();
    match matches.value_of("repo") {
        Some(repo_name) => {
            let repo_name = repo::Name(repo_name.to_owned());
            let repo = config.repository(&repo_name)?;
            let secrets = get_secrets(repo)?;
            restic.run(repo, &secrets, cmd)?.wait()?;
        }
        None => {
            restic.run_raw(cmd)?.wait()?;
        }
    }

    Ok(())
}

fn run_backup(config: &Config, restic: &Restic, matches: &ArgMatches) -> anyhow::Result<()> {
    let backup_name = backup::Name(matches.value_of("backup").unwrap().to_owned());
    let backup = config.backup(&backup_name)?;
    let repo = config.repository_for_backup(backup)?;
    let secrets = get_secrets(repo)?;
    restic.backup(repo, &secrets, backup)?.wait()
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

    let restic = Restic::new(matches.value_of("restic-binary").unwrap());

    let config_path = matches
        .value_of_os("config")
        .map(PathBuf::from)
        .map(Ok)
        .unwrap_or_else(|| default_config_path())
        .context("failed to get path for the default config file")?;
    let cfg_data =
        std::fs::read_to_string(config_path).context("failed to read the config file")?;
    let config: model::Config =
        toml::from_str(&cfg_data).context("failed to parse the config file")?;

    match matches.subcommand() {
        ("restic", Some(matches)) => run_restic(&config, &restic, matches),
        ("backup", Some(matches)) => run_backup(&config, &restic, matches),
        _ => todo!(),
    }
}
