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
    System,
    Path(PathBuf),
    #[cfg(feature = "bundled-restic-support")]
    Bundled,
    #[cfg(feature = "bundled-restic-support")]
    SystemThenBundled,
}

impl Default for ResticArg {
    #[cfg(feature = "bundled-restic-support")]
    fn default() -> Self {
        ResticArg::SystemThenBundled
    }

    #[cfg(not(feature = "bundled-restic-support"))]
    fn default() -> Self {
        ResticArg::System
    }
}

impl From<&OsStr> for ResticArg {
    fn from(s: &OsStr) -> Self {
        match s.to_str() {
            Some("system") => ResticArg::System,
            #[cfg(feature = "bundled-restic-support")]
            Some("bundled") => ResticArg::Bundled,
            #[cfg(feature = "bundled-restic-support")]
            Some("system-then-bundled") => ResticArg::SystemThenBundled,
            _ => ResticArg::Path(PathBuf::from(s)),
        }
    }
}

impl std::fmt::Display for ResticArg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            ResticArg::System => write!(f, "system"),
            ResticArg::Path(path) => write!(f, "{}", path.display()),
            #[cfg(feature = "bundled-restic-support")]
            ResticArg::Bundled => write!(f, "bundled"),
            #[cfg(feature = "bundled-restic-support")]
            ResticArg::SystemThenBundled => write!(f, "system-then-bundled"),
        }
    }
}

/// A configuration-driven backup program based on restic.
#[derive(clap::Parser)]
#[clap(global_setting(clap::AppSettings::NoAutoVersion))]
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

    /// Set the restic binary to use.
    /// Possible values:
    /// "system": use the system restic;
    /// <PATH>: use a specific restic binary
    #[cfg(not(feature = "bundled-restic-support"))]
    #[clap(
        long,
        default_value_t,
        value_name = "special value or PATH",
        parse(from_os_str)
    )]
    pub restic: ResticArg,

    /// Set the restic binary to use.
    /// Possible values:
    /// "system": use the system restic;
    /// "bundled": use the bundled restic build;
    /// "system-then-bundled": first try the system restic, then the bundled restic;
    /// <PATH>: use a specific restic binary
    #[cfg(feature = "bundled-restic-support")]
    #[clap(
        long,
        default_value_t,
        value_name = "special value or PATH",
        parse(from_os_str)
    )]
    pub restic: ResticArg,

    #[clap(subcommand)]
    pub subcommand: Cmd,
}

#[derive(clap::Parser)]
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
    #[derive(clap::Parser)]
    pub struct Cli {
        /// Run the daemon under the built-in supervisor
        #[clap(long)]
        pub supervisor: bool,
    }
}

pub mod backup {
    #[derive(clap::Parser)]
    pub struct Cli {
        /// The backup to run
        #[clap(name = "BACKUP")]
        pub backup: String,
    }
}

pub mod secret {
    #[derive(clap::Parser)]
    pub struct Cli {
        #[clap(subcommand)]
        pub subcommand: Cmd,
    }

    #[derive(clap::Parser)]
    pub enum Cmd {
        /// Sets a secret from the terminal
        Set(Set),

        /// Lists all configured secrets and whether they are currently set
        List(List),
    }

    #[derive(clap::Parser)]
    pub struct Set {
        /// Repository of the secret
        #[clap(name = "REPOSITORY")]
        pub repository: String,
        /// Name of the secret, or the repository password if not set
        #[clap(name = "SECRET")]
        pub secret: Option<String>,
    }

    #[derive(clap::Parser)]
    pub struct List {
        /// Shows passwords in clear text
        #[clap(long)]
        pub show_passwords: bool,
    }
}

pub mod restic {
    use std::ffi::OsString;

    #[derive(clap::Parser)]
    #[clap(setting(clap::AppSettings::TrailingVarArg))]
    pub struct Cli {
        /// The cirrus repository to use with restic
        #[clap(short, long, env = "CIRRUS_REPOSITORY")]
        pub repository: Option<String>,

        /// Command-line arguments to pass to restic
        pub cmd: Vec<OsString>,
    }
}
