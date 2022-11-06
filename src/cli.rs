use clap::builder::TypedValueParser;
use dirs_next as dirs;
use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone)]
pub struct ConfigFile(Option<PathBuf>);

impl ConfigFile {
    pub fn path(&self) -> eyre::Result<&Path> {
        self.0
            .as_deref()
            .ok_or_else(|| eyre::eyre!("failed to get default config file path"))
    }
}

impl Default for ConfigFile {
    fn default() -> Self {
        let default_path = dirs::config_dir().map(|dir| dir.join("cirrus").join("backups.toml"));
        ConfigFile(default_path)
    }
}

#[derive(Debug, Clone)]
struct ConfigFileParser;

impl TypedValueParser for ConfigFileParser {
    type Value = ConfigFile;

    fn parse_ref(
        &self,
        _cmd: &clap::Command,
        _arg: Option<&clap::Arg>,
        value: &OsStr,
    ) -> Result<Self::Value, clap::Error> {
        Ok(ConfigFile(Some(PathBuf::from(value))))
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

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
struct ResticArgParser;

impl TypedValueParser for ResticArgParser {
    type Value = ResticArg;

    fn parse_ref(
        &self,
        _cmd: &clap::Command,
        _arg: Option<&clap::Arg>,
        value: &OsStr,
    ) -> Result<Self::Value, clap::Error> {
        let restic_arg = match value.to_str() {
            Some("system") => ResticArg::System,
            #[cfg(feature = "bundled-restic-support")]
            Some("bundled") => ResticArg::Bundled,
            #[cfg(feature = "bundled-restic-support")]
            Some("system-then-bundled") => ResticArg::SystemThenBundled,
            _ => ResticArg::Path(PathBuf::from(value)),
        };
        Ok(restic_arg)
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

#[derive(Debug, Copy, Clone, clap::ValueEnum)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl Default for LogLevel {
    fn default() -> Self {
        LogLevel::Info
    }
}

impl From<LogLevel> for tracing::Level {
    fn from(level: LogLevel) -> Self {
        match level {
            LogLevel::Trace => tracing::Level::TRACE,
            LogLevel::Debug => tracing::Level::DEBUG,
            LogLevel::Info => tracing::Level::INFO,
            LogLevel::Warn => tracing::Level::WARN,
            LogLevel::Error => tracing::Level::ERROR,
        }
    }
}

/// A configuration-driven backup program based on restic.
#[derive(clap::Parser)]
#[command(disable_version_flag = true)]
pub struct Cli {
    /// Sets a custom configuration file path
    #[arg(short, long, env = "CIRRUS_CONFIG_FILE", default_value_t, value_parser = ConfigFileParser)]
    pub config_file: ConfigFile,

    /// Sets the configuration from a string
    #[arg(long, env = "CIRRUS_CONFIG")]
    pub config_string: Option<String>,

    /// Set the restic binary to use.
    /// Possible values:
    /// "system": use the system restic;
    /// <PATH>: use a specific restic binary
    #[cfg(not(feature = "bundled-restic-support"))]
    #[arg(long, default_value_t, value_name = "special value or PATH", value_parser = ResticArgParser)]
    pub restic: ResticArg,

    /// Set the restic binary to use.
    /// Possible values:
    /// "system": use the system restic;
    /// "bundled": use the bundled restic build;
    /// "system-then-bundled": first try the system restic, then the bundled restic;
    /// <PATH>: use a specific restic binary
    #[cfg(feature = "bundled-restic-support")]
    #[arg(long, default_value_t, value_name = "special value or PATH", value_parser = ResticArgParser)]
    pub restic: ResticArg,

    #[command(subcommand)]
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
    #[command(alias = "secrets")]
    Secret(secret::Cli),

    /// Runs custom restic commands on configured repositories
    Restic(restic::Cli),

    /// Runs self management tasks
    #[cfg(feature = "cirrus-self")]
    #[command(name = "self")]
    SelfCommands(cirrus_self::Cli),

    /// List and search repository contents
    RepoContents(repo_contents::Cli),

    /// Prints version information
    Version,
}

pub mod daemon {
    use crate::cli::LogLevel;
    use std::path::PathBuf;

    #[derive(clap::Parser)]
    pub struct Cli {
        /// Run the daemon under the built-in supervisor
        #[arg(long)]
        pub supervisor: bool,

        /// Set the log level
        #[arg(long, value_enum, default_value_t)]
        pub log_level: LogLevel,

        /// Send all output to the given log file
        #[arg(long)]
        pub log_file: Option<PathBuf>,
    }
}

pub mod backup {
    #[derive(clap::Parser)]
    pub struct Cli {
        /// The backup to run
        #[arg(value_name = "BACKUP")]
        pub backup: String,
    }
}

pub mod secret {
    #[derive(clap::Parser)]
    pub struct Cli {
        #[command(subcommand)]
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
        #[arg(value_name = "REPOSITORY")]
        pub repository: String,
        /// Name of the secret, or the repository password if not set
        #[arg(value_name = "SECRET")]
        pub secret: Option<String>,
    }

    #[derive(clap::Parser)]
    pub struct List {
        /// Shows passwords in clear text
        #[arg(long)]
        pub show_passwords: bool,
    }
}

pub mod restic {
    use std::ffi::OsString;

    #[derive(clap::Parser)]
    pub struct Cli {
        /// The cirrus repository to use with restic
        #[arg(short, long, env = "CIRRUS_REPOSITORY")]
        pub repository: Option<String>,

        /// Command-line arguments to pass to restic
        #[arg(trailing_var_arg = true)]
        pub cmd: Vec<OsString>,
    }
}

pub mod repo_contents {
    #[derive(clap::Parser)]
    pub struct Cli {
        /// The repository to use
        #[arg(short, long, env = "CIRRUS_REPOSITORY")]
        pub repository: String,

        #[command(subcommand)]
        pub subcommand: Cmd,
    }

    #[derive(clap::Parser)]
    pub enum Cmd {
        /// Create and populate the contents index for the repository
        Index(Index),

        /// List the contents of the repository
        Ls(Ls),
    }

    #[derive(clap::Parser)]
    pub struct Index {
        /// Set the number of unindexed snapshots to index
        #[arg(short, long, default_value = "20")]
        pub snapshots_count: u32,
    }

    #[derive(clap::Parser)]
    pub struct Ls {
        /// Path to list
        #[arg()]
        pub path: String,
    }
}
