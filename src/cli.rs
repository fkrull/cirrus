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

#[derive(Debug)]
pub enum ResticArg {
    SystemThenBundled,
    Bundled,
    Path(PathBuf),
}

impl Default for ResticArg {
    fn default() -> Self {
        ResticArg::SystemThenBundled
    }
}

impl From<&OsStr> for ResticArg {
    fn from(s: &OsStr) -> Self {
        if s == OsStr::new("system-then-bundled") {
            ResticArg::SystemThenBundled
        } else if s == OsStr::new("bundled") {
            ResticArg::Bundled
        } else {
            ResticArg::Path(PathBuf::from(s))
        }
    }
}

impl std::fmt::Display for ResticArg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            ResticArg::SystemThenBundled => write!(f, "system-then-bundled"),
            ResticArg::Bundled => write!(f, "bundled"),
            ResticArg::Path(path) => write!(f, "{}", path.display()),
        }
    }
}

/// A configuration-driven backup program based on restic.
#[derive(clap::Clap)]
#[clap(global_setting(clap::AppSettings::NoAutoVersion))]
#[clap(global_setting(clap::AppSettings::DisableVersionForSubcommands))]
pub struct Cli {
    /// Sets a custom configuration file path
    #[clap(
        short,
        long,
        env = "CIRRUS_CONFIG_FILE",
        default_value_t,
        parse(from_os_str)
    )]
    pub config_file: ConfigFile,

    /// Sets the configuration from a string
    #[clap(long, env = "CIRRUS_CONFIG")]
    pub config_string: Option<String>,

    /// Use the specific restic implementation
    #[clap(
        long,
        possible_values(&["system-then-bundled", "bundled", "<PATH>"]),
        default_value_t,
        parse(from_os_str)
    )]
    pub restic: ResticArg,

    #[clap(subcommand)]
    pub subcommand: Cmd,
}

#[derive(clap::Clap)]
pub enum Cmd {
    /// Runs the cirrus daemon
    Daemon(daemon::Cli),

    /// Runs a configured backup
    Backup(backup::Cli),

    /// Prints the active configuration
    Config,

    /// Gets and sets secrets
    #[clap(alias = "secrets")]
    Secret(secret::Cli),

    /// Runs custom restic commands on configured repositories
    Restic(restic::Cli),

    /// Runs self management tasks
    #[cfg(feature = "cirrus-self")]
    #[clap(name = "self")]
    SelfCommands(cirrus_self::Cli),

    /// Prints version information
    Version,
}

pub mod daemon {
    #[derive(clap::Clap)]
    pub struct Cli {
        /// Run the daemon under the built-in supervisor
        #[clap(long)]
        pub supervisor: bool,
    }
}

pub mod backup {
    #[derive(clap::Clap)]
    pub struct Cli {
        /// The backup to run
        #[clap(name = "BACKUP")]
        pub backup: String,
    }
}

pub mod secret {
    #[derive(clap::Clap)]
    pub struct Cli {
        #[clap(subcommand)]
        pub subcommand: Cmd,
    }

    #[derive(clap::Clap)]
    pub enum Cmd {
        /// Sets a secret from the terminal
        Set(Set),

        /// Lists all configured secrets and whether they are currently set
        List(List),
    }

    #[derive(clap::Clap)]
    pub struct Set {
        /// Repository of the secret
        #[clap(name = "REPOSITORY")]
        pub repository: String,
        /// Name of the secret, or the repository password if not set
        #[clap(name = "SECRET")]
        pub secret: Option<String>,
    }

    #[derive(clap::Clap)]
    pub struct List {
        /// Shows passwords in clear text
        #[clap(long)]
        pub show_passwords: bool,
    }
}

pub mod restic {
    use std::ffi::OsString;

    #[derive(clap::Clap)]
    #[clap(setting(clap::AppSettings::TrailingVarArg))]
    pub struct Cli {
        /// The cirrus repository to use with restic
        #[clap(short, long, env = "CIRRUS_REPOSITORY")]
        pub repository: Option<String>,

        /// Command-line arguments to pass to restic
        pub cmd: Vec<OsString>,
    }
}
