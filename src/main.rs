use clap::Parser;
use linget::{cli, product};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// Determines which mode to run based on command-line arguments
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RunMode {
    Gui,
    Tui,
    Cli,
}

fn detect_run_mode() -> RunMode {
    detect_run_mode_from(std::env::args())
}

fn detect_run_mode_from(args: impl IntoIterator<Item = impl AsRef<str>>) -> RunMode {
    let mut args = args.into_iter();
    let _binary = args.next();

    let Some(command) = args.next() else {
        return RunMode::Tui;
    };

    match command.as_ref() {
        // Explicit GUI launch
        "gui" => RunMode::Gui,
        // Explicit TUI launch
        "tui" => RunMode::Tui,
        // CLI commands
        "list" | "search" | "install" | "remove" | "update" | "info" | "sources" | "check"
        | "completions" | "cohort-report" | "help" | "--help" | "-h" | "--version" | "-V"
        | "schedule" => RunMode::Cli,
        // Unknown argument - let clap handle it (will show error or help)
        _ => RunMode::Cli,
    }
}

#[cfg(feature = "gui")]
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

fn init_logging(run_mode: RunMode) {
    let filter = EnvFilter::from_default_env()
        .add_directive("linget=info".parse().unwrap())
        .add_directive("gtk=warn".parse().unwrap());

    match run_mode {
        RunMode::Tui => {
            // Avoid emitting logs to the terminal while in alternate screen.
            tracing_subscriber::registry()
                .with(fmt::layer().with_writer(std::io::sink))
                .with(filter)
                .init();
        }
        RunMode::Gui | RunMode::Cli => {
            tracing_subscriber::registry()
                .with(fmt::layer().with_writer(std::io::stderr))
                .with(filter)
                .init();
        }
    }
}

#[cfg(feature = "gui")]
fn run_gui(runtime: tokio::runtime::Runtime) {
    tracing::info!(
        "Starting {} v{} (GUI mode with Relm4)",
        product::APP_NAME,
        product::APP_VERSION
    );

    sanitize_environment();

    let _guard = runtime.enter();

    linget::run_gui_app();
}

#[cfg(not(feature = "gui"))]
fn run_gui(runtime: tokio::runtime::Runtime) {
    drop(runtime);
    eprintln!("Error: {}", gui_unavailable_message());
    std::process::exit(2);
}

#[cfg(any(not(feature = "gui"), test))]
fn gui_unavailable_message() -> &'static str {
    "GUI support is not included in this build. Rebuild LinGet with `--features gui`, or run `linget` for the terminal interface."
}

fn run_tui(runtime: tokio::runtime::Runtime) {
    tracing::info!(
        "Starting {} v{} (TUI mode)",
        product::APP_NAME,
        product::APP_VERSION
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
        product::APP_NAME,
        product::APP_VERSION
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
    let run_mode = detect_run_mode();

    // Initialize logging
    init_logging(run_mode);

    // Create tokio runtime for async operations
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to create Tokio runtime");

    // Detect and run appropriate mode
    match run_mode {
        RunMode::Gui => run_gui(runtime),
        RunMode::Tui => run_tui(runtime),
        RunMode::Cli => run_cli(runtime),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_command_defaults_to_tui() {
        assert_eq!(detect_run_mode_from(["linget"]), RunMode::Tui);
    }

    #[test]
    fn explicit_tui_command_remains_supported() {
        assert_eq!(detect_run_mode_from(["linget", "tui"]), RunMode::Tui);
    }

    #[test]
    fn explicit_gui_command_selects_gui_dispatch() {
        assert_eq!(detect_run_mode_from(["linget", "gui"]), RunMode::Gui);
    }

    #[test]
    fn cli_commands_and_unknown_arguments_go_through_clap() {
        assert_eq!(detect_run_mode_from(["linget", "list"]), RunMode::Cli);
        assert_eq!(
            detect_run_mode_from(["linget", "cohort-report"]),
            RunMode::Cli
        );
        assert_eq!(detect_run_mode_from(["linget", "--help"]), RunMode::Cli);
        assert_eq!(detect_run_mode_from(["linget", "unknown"]), RunMode::Cli);
    }

    #[test]
    fn unavailable_gui_message_explains_both_paths() {
        let message = gui_unavailable_message();
        assert!(message.contains("--features gui"));
        assert!(message.contains("`linget`"));
    }
}
