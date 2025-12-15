mod commands;
mod output;
pub mod tui;

use crate::app::{APP_NAME, APP_VERSION};
use crate::backend::PackageManager;
use crate::models::PackageSource;
use clap::{Parser, Subcommand, ValueEnum};
use std::sync::Arc;
use tokio::sync::Mutex;

pub use output::{OutputFormat, OutputWriter};

/// LinGet - A unified package manager for Linux
#[derive(Parser)]
#[command(name = APP_NAME)]
#[command(version = APP_VERSION)]
#[command(about = "A unified package manager for Linux", long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Output format
    #[arg(long, global = true, default_value = "human")]
    pub format: OutputFormat,

    /// Verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Quiet mode (minimal output)
    #[arg(short, long, global = true)]
    pub quiet: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// List installed packages
    List {
        /// Filter by package source
        #[arg(short, long)]
        source: Option<SourceArg>,

        /// Show only packages with updates available
        #[arg(short, long)]
        updates: bool,
    },

    /// Search for packages
    Search {
        /// Search query
        query: String,

        /// Filter by package source
        #[arg(short, long)]
        source: Option<SourceArg>,
    },

    /// Install a package
    Install {
        /// Package name
        package: String,

        /// Package source
        #[arg(short, long)]
        source: Option<SourceArg>,

        /// Skip confirmation
        #[arg(short, long)]
        yes: bool,
    },

    /// Remove a package
    Remove {
        /// Package name
        package: String,

        /// Package source
        #[arg(short, long)]
        source: Option<SourceArg>,

        /// Skip confirmation
        #[arg(short, long)]
        yes: bool,
    },

    /// Update packages
    Update {
        /// Package name (omit for all packages)
        package: Option<String>,

        /// Package source
        #[arg(short, long)]
        source: Option<SourceArg>,

        /// Update all packages
        #[arg(short, long)]
        all: bool,

        /// Skip confirmation
        #[arg(short, long)]
        yes: bool,
    },

    /// Show package information
    Info {
        /// Package name
        package: String,

        /// Package source
        #[arg(short, long)]
        source: Option<SourceArg>,
    },

    /// Manage package sources
    Sources {
        #[command(subcommand)]
        action: Option<SourcesAction>,
    },

    /// Check for available updates
    Check,

    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        shell: clap_complete::Shell,
    },

    /// Launch interactive TUI mode
    Tui,

    /// Launch graphical user interface (default when no command given)
    Gui,
}

#[derive(Subcommand)]
pub enum SourcesAction {
    /// List available sources
    List,
    /// Enable a source
    Enable {
        /// Source to enable
        source: SourceArg,
    },
    /// Disable a source
    Disable {
        /// Source to disable
        source: SourceArg,
    },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum SourceArg {
    Apt,
    Dnf,
    Pacman,
    Zypper,
    Flatpak,
    Snap,
    Npm,
    Pip,
    Pipx,
    Cargo,
    Brew,
    Aur,
    Conda,
    Mamba,
    Dart,
}

impl From<SourceArg> for PackageSource {
    fn from(arg: SourceArg) -> Self {
        match arg {
            SourceArg::Apt => PackageSource::Apt,
            SourceArg::Dnf => PackageSource::Dnf,
            SourceArg::Pacman => PackageSource::Pacman,
            SourceArg::Zypper => PackageSource::Zypper,
            SourceArg::Flatpak => PackageSource::Flatpak,
            SourceArg::Snap => PackageSource::Snap,
            SourceArg::Npm => PackageSource::Npm,
            SourceArg::Pip => PackageSource::Pip,
            SourceArg::Pipx => PackageSource::Pipx,
            SourceArg::Cargo => PackageSource::Cargo,
            SourceArg::Brew => PackageSource::Brew,
            SourceArg::Aur => PackageSource::Aur,
            SourceArg::Conda => PackageSource::Conda,
            SourceArg::Mamba => PackageSource::Mamba,
            SourceArg::Dart => PackageSource::Dart,
        }
    }
}

impl std::fmt::Display for SourceArg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            SourceArg::Apt => "apt",
            SourceArg::Dnf => "dnf",
            SourceArg::Pacman => "pacman",
            SourceArg::Zypper => "zypper",
            SourceArg::Flatpak => "flatpak",
            SourceArg::Snap => "snap",
            SourceArg::Npm => "npm",
            SourceArg::Pip => "pip",
            SourceArg::Pipx => "pipx",
            SourceArg::Cargo => "cargo",
            SourceArg::Brew => "brew",
            SourceArg::Aur => "aur",
            SourceArg::Conda => "conda",
            SourceArg::Mamba => "mamba",
            SourceArg::Dart => "dart",
        };
        write!(f, "{}", s)
    }
}

/// Run the CLI application
pub async fn run(cli: Cli) -> anyhow::Result<()> {
    let pm = Arc::new(Mutex::new(PackageManager::new()));
    let writer = OutputWriter::new(cli.format, cli.verbose, cli.quiet);

    match cli.command {
        Commands::List { source, updates } => {
            commands::list::run(pm, source.map(Into::into), updates, &writer).await
        }
        Commands::Search { query, source } => {
            commands::search::run(pm, &query, source.map(Into::into), &writer).await
        }
        Commands::Install {
            package,
            source,
            yes,
        } => commands::install::run(pm, &package, source.map(Into::into), yes, &writer).await,
        Commands::Remove {
            package,
            source,
            yes,
        } => commands::remove::run(pm, &package, source.map(Into::into), yes, &writer).await,
        Commands::Update {
            package,
            source,
            all,
            yes,
        } => {
            commands::update::run(pm, package.as_deref(), source.map(Into::into), all, yes, &writer)
                .await
        }
        Commands::Info { package, source } => {
            commands::info::run(pm, &package, source.map(Into::into), &writer).await
        }
        Commands::Sources { action } => commands::sources::run(pm, action, &writer).await,
        Commands::Check => commands::check::run(pm, &writer).await,
        Commands::Completions { shell } => {
            commands::completions::run(shell);
            Ok(())
        }
        Commands::Tui => tui::run().await,
        Commands::Gui => {
            // This is handled in main.rs - should not reach here
            unreachable!("GUI command should be handled in main.rs")
        }
    }
}
