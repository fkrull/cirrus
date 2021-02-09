use clap::Clap;
use std::path::PathBuf;

/// A configuration-driven backup program based on restic.
#[derive(Clap)]
pub struct Cli {
    /// set a custom configuration file path
    #[clap(short, long, env = "CIRRUS_CONFIG_FILE")]
    pub config_file: Option<PathBuf>,

    /// set the configuration from a string
    #[clap(long, env = "CIRRUS_CONFIG")]
    pub config_string: Option<String>,

    /// set a custom app configuration file path
    #[clap(long, env = "CIRRUS_APPCONFIG")]
    pub appconfig_file: Option<PathBuf>,

    /// set the restic binary to use
    #[clap(long)]
    pub restic_binary: Option<String>,

    #[clap(subcommand)]
    pub subcommand: Option<Cmd>,
}

#[derive(Clap)]
pub enum Cmd {
    /// run a configured backup
    Backup(backup::Cli),

    /// print the active configuration
    Config,

    /// get and set secrets
    #[clap(alias = "secrets")]
    Secret(secret::Cli),

    /// run custom restic commands on configured repositories
    Restic(restic::Cli),

    /// commands specific to the desktop build
    #[cfg(feature = "desktop-commands")]
    Desktop(desktop::Cli),
}

pub mod backup {
    use clap::Clap;

    #[derive(Clap)]
    pub struct Cli {
        /// the backup to run
        #[clap(name = "BACKUP")]
        pub backup: String,
    }
}

pub mod secret {
    use clap::Clap;

    #[derive(Clap)]
    pub struct Cli {
        #[clap(subcommand)]
        pub subcommand: Cmd,
    }

    #[derive(Clap)]
    pub enum Cmd {
        Set(Set),
        List(List),
    }

    /// set a secret from the terminal
    #[derive(Clap)]
    pub struct Set {
        /// repository of the secret
        #[clap(name = "REPOSITORY")]
        pub repository: String,
        /// name of the secret
        #[clap(name = "SECRET")]
        pub secret: Option<String>,
    }

    /// list all configured secrets and whether they are currently set
    #[derive(Clap)]
    pub struct List {
        /// show passwords in clear text
        #[clap(long)]
        pub show_passwords: bool,
    }
}

pub mod restic {
    use clap::{AppSettings, ArgSettings, Clap};
    use std::ffi::OsString;

    #[derive(Clap)]
    #[clap(setting(AppSettings::TrailingVarArg))]
    pub struct Cli {
        /// the cirrus repository to use with restic
        #[clap(short, long, env = "CIRRUS_REPOSITORY")]
        pub repository: Option<String>,

        /// command-line arguments to pass to restic
        #[clap(setting(ArgSettings::AllowHyphenValues))]
        pub cmd: Vec<OsString>,
    }
}

#[cfg(feature = "desktop-commands")]
pub mod desktop {
    use clap::Clap;

    #[derive(Clap)]
    pub struct Cli {
        #[clap(subcommand)]
        pub subcommand: Cmd,
    }

    #[derive(Clap)]
    pub enum Cmd {
        /// open the config file in the default editor
        OpenConfigFile,

        /// open the app config file in the default editor
        OpenAppconfigFile,
    }
}
