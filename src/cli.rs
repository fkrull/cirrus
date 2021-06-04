use clap::Clap;
use dirs_next as dirs;
use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub struct ConfigFile(Option<PathBuf>);

impl ConfigFile {
    pub fn path(&self) -> eyre::Result<&Path> {
        self.0
            .as_ref()
            .map(|p| p.as_path())
            .ok_or_else(|| eyre::eyre!("failed to get default config file path"))
    }
}

impl Default for ConfigFile {
    fn default() -> Self {
        let default_path = dirs::config_dir().map(|dir| dir.join("cirrus").join("backups.toml"));
        ConfigFile(default_path)
    }
}

impl From<&OsStr> for ConfigFile {
    fn from(s: &OsStr) -> Self {
        ConfigFile(Some(PathBuf::from(s)))
    }
}

impl std::fmt::Display for ConfigFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            Some(path) => write!(f, "{}", path.display()),
            None => write!(f, "<none>"),
        }
    }
}

/// A configuration-driven backup program based on restic.
#[derive(Clap)]
#[clap(global_setting(clap::AppSettings::NoAutoVersion))]
#[clap(global_setting(clap::AppSettings::VersionlessSubcommands))]
pub struct Cli {
    /// Sets a custom configuration file path
    #[clap(
        short,
        long,
        env = "CIRRUS_CONFIG_FILE",
        default_value,
        parse(from_os_str)
    )]
    pub config_file: ConfigFile,

    /// Sets the configuration from a string
    #[clap(long, env = "CIRRUS_CONFIG")]
    pub config_string: Option<String>,

    /// Sets the restic binary to use
    #[clap(long)]
    pub restic_binary: Option<PathBuf>,

    #[clap(subcommand)]
    pub subcommand: Cmd,
}

#[derive(Clap)]
pub enum Cmd {
    /// Runs the cirrus daemon
    Daemon,

    /// Runs a configured backup
    Backup(backup::Cli),

    /// Prints the active configuration
    Config,

    /// Gets and sets secrets
    #[clap(alias = "secrets")]
    Secret(secret::Cli),

    /// Runs custom restic commands on configured repositories
    Restic(restic::Cli),

    /// Generates various support files
    Generate(generate::Cli),

    /// Commands specific to the desktop build
    #[cfg(feature = "desktop-commands")]
    Desktop(desktop::Cli),

    /// Prints version information
    Version,
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

pub mod generate {
    use clap::Clap;

    #[derive(Clap)]
    pub struct Cli {
        #[clap(subcommand)]
        pub subcommand: Cmd,
    }

    #[derive(Clap)]
    pub enum Cmd {
        /// Generate a systemd unit file
        SystemdUnit,

        /// Generate bash completions
        BashCompletions,
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
