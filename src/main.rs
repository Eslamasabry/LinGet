mod app;
mod backend;
mod cli;
mod models;
mod ui;

use clap::Parser;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// Determines which mode to run based on command-line arguments
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RunMode {
    Gui,
    Tui,
    Cli,
}

fn detect_run_mode() -> RunMode {
    let args: Vec<String> = std::env::args().collect();

    // No arguments = GUI mode (default)
    if args.len() <= 1 {
        return RunMode::Gui;
    }

    match args[1].as_str() {
        // Explicit GUI launch
        "gui" => RunMode::Gui,
        // Explicit TUI launch
        "tui" => RunMode::Tui,
        // CLI commands
        "list" | "search" | "install" | "remove" | "update" | "info" | "sources" | "check"
        | "completions" | "help" | "--help" | "-h" | "--version" | "-V" => RunMode::Cli,
        // Unknown argument - let clap handle it (will show error or help)
        _ => RunMode::Cli,
    }
}

fn sanitize_environment() {
    // When launching LinGet from some snapped terminals (e.g. Ghostty),
    // environment variables can point GTK's pixbuf loader to Snap-provided
    // modules built against a different glibc, causing icon-load failures.
    for key in ["GDK_PIXBUF_MODULEDIR", "GDK_PIXBUF_MODULE_FILE"] {
        if let Ok(val) = std::env::var(key) {
            if val.contains("/snap/") {
                std::env::remove_var(key);
                tracing::warn!(
                    "Removed {} from environment to avoid snap pixbuf loader issues",
                    key
                );
            }
        }
    }
}

fn run_gui(runtime: tokio::runtime::Runtime) {
    tracing::info!(
        "Starting {} v{} (GUI mode)",
        app::APP_NAME,
        app::APP_VERSION
    );

    sanitize_environment();

    // Keep the runtime alive in a background thread
    let _guard = runtime.enter();

    // Initialize GTK
    use gtk4::prelude::*;
    let app = app::build_app();

    // Run the application
    let exit_code = app.run();

    tracing::info!("Exiting with code: {:?}", exit_code);
    std::process::exit(exit_code.into());
}

fn run_tui(runtime: tokio::runtime::Runtime) {
    tracing::info!(
        "Starting {} v{} (TUI mode)",
        app::APP_NAME,
        app::APP_VERSION
    );

    let result = runtime.block_on(cli::tui::run());

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run_cli(runtime: tokio::runtime::Runtime) {
    tracing::info!(
        "Starting {} v{} (CLI mode)",
        app::APP_NAME,
        app::APP_VERSION
    );

    // Parse CLI arguments
    let cli = cli::Cli::parse();

    // Handle GUI command specially (redirect to GUI mode)
    if matches!(cli.command, cli::Commands::Gui) {
        drop(cli);
        run_gui(runtime);
        return;
    }

    // Handle TUI command specially (redirect to TUI mode)
    if matches!(cli.command, cli::Commands::Tui) {
        drop(cli);
        run_tui(runtime);
        return;
    }

    // Run CLI command
    let result = runtime.block_on(cli::run(cli));

    if let Err(e) = result {
        // Log the error with tracing for debugging
        tracing::error!(error = %e, "CLI command failed");

        // The error display is already handled by the command itself
        // using the OutputWriter, so we just need to exit with error code
        std::process::exit(1);
    }
}

fn main() {
    // Initialize logging
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(
            EnvFilter::from_default_env()
                .add_directive("linget=info".parse().unwrap())
                .add_directive("gtk=warn".parse().unwrap()),
        )
        .init();

    // Create tokio runtime for async operations
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to create Tokio runtime");

    // Detect and run appropriate mode
    match detect_run_mode() {
        RunMode::Gui => run_gui(runtime),
        RunMode::Tui => run_tui(runtime),
        RunMode::Cli => run_cli(runtime),
    }
}
