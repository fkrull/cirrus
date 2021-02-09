use clap::Clap;
use std::path::PathBuf;

/// A configuration-driven backup program based on restic.
#[derive(Clap)]
pub struct Cli {
    /// Sets a custom configuration file path
    #[clap(short, long, env = "CIRRUS_CONFIG_FILE")]
    pub config_file: Option<PathBuf>,

    /// Sets the configuration from a string
    #[clap(long, env = "CIRRUS_CONFIG")]
    pub config_string: Option<String>,

    /// Sets the restic binary to use
    #[clap(long, default_value = "restic")]
    pub restic_binary: PathBuf,

    #[clap(subcommand)]
    pub subcommand: Option<Cmd>,
}

#[derive(Clap)]
pub enum Cmd {
    /// Runs a configured backup
    Backup(backup::Cli),

    /// Prints the active configuration
    Config,

    /// Gets and sets secrets
    #[clap(alias = "secrets")]
    Secret(secret::Cli),

    /// Runs custom restic commands on configured repositories
    Restic(restic::Cli),

    /// Commands specific to the desktop build
    #[cfg(feature = "desktop-commands")]
    Desktop(desktop::Cli),
}

pub mod backup {
    use clap::Clap;

    #[derive(Clap)]
    pub struct Cli {
        /// The backup to run
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
        /// Sets a secret from the terminal
        Set(Set),

        /// Lists all configured secrets and whether they are currently set
        List(List),
    }

    #[derive(Clap)]
    pub struct Set {
        /// Repository of the secret
        #[clap(name = "REPOSITORY")]
        pub repository: String,
        /// Name of the secret, or the repository password if not set
        #[clap(name = "SECRET")]
        pub secret: Option<String>,
    }

    #[derive(Clap)]
    pub struct List {
        /// Shows passwords in clear text
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
        /// The cirrus repository to use with restic
        #[clap(short, long, env = "CIRRUS_REPOSITORY")]
        pub repository: Option<String>,

        /// Command-line arguments to pass to restic
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
        /// Opens the config file in the default editor
        OpenConfigFile,
    }
}
