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

fn is_documentation_flag(arg: &str) -> bool {
    matches!(arg, "--help" | "-h" | "--version" | "-V")
}

/// Whether this invocation is just asking what the tool is, rather than asking
/// it to do anything.
fn asks_for_documentation(args: impl IntoIterator<Item = impl AsRef<str>>) -> bool {
    args.into_iter()
        .skip(1)
        .any(|arg| is_documentation_flag(arg.as_ref()))
}

fn detect_run_mode_from(args: impl IntoIterator<Item = impl AsRef<str>>) -> RunMode {
    let mut args = args.into_iter();
    let _binary = args.next();

    let Some(command) = args.next() else {
        return RunMode::Tui;
    };

    // Asking for help or the version is always a question, never a launch —
    // even when it trails a subcommand that would otherwise open a UI.
    // `linget tui --help` used to reach the TUI, which then failed trying to
    // take over a terminal that may not even be attached.
    if args
        .into_iter()
        .any(|arg| is_documentation_flag(arg.as_ref()))
    {
        return RunMode::Cli;
    }

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

    // `--help` should print help and nothing else. A startup banner above it is
    // noise in the one output a first-time user is guaranteed to read.
    if asks_for_documentation(std::env::args()) {
        tracing_subscriber::registry()
            .with(fmt::layer().with_writer(std::io::sink))
            .with(filter)
            .init();
        return;
    }

    match run_mode {
        RunMode::Tui => {
            // Logs cannot go to the terminal while the alternate screen is up,
            // which otherwise leaves the TUI undebuggable: there is no way to
            // see why a search came back empty or a backend failed. Honour an
            // explicit log file so a session can be traced when it misbehaves.
            match std::env::var_os("LINGET_LOG_FILE").map(std::fs::File::create) {
                Some(Ok(file)) => {
                    tracing_subscriber::registry()
                        .with(fmt::layer().with_ansi(false).with_writer(file))
                        .with(filter)
                        .init();
                }
                _ => {
                    tracing_subscriber::registry()
                        .with(fmt::layer().with_writer(std::io::sink))
                        .with(filter)
                        .init();
                }
            }
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

    // This path is reached both by `linget tui --classic` and by a bare
    // `linget` invocation, which never goes through clap — so read the flag
    // directly. The reimagined TUI is the default; --classic opts out.
    let classic = std::env::args().any(|arg| arg == "--classic");
    let result = if classic {
        runtime.block_on(cli::tui::run())
    } else {
        runtime.block_on(cli::tui_next::run())
    };

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
    if matches!(cli.command, cli::Commands::Tui { .. }) {
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
    fn help_and_version_never_launch_a_ui() {
        // These used to reach the TUI, which then failed with an opaque
        // "No such device or address" when it tried to claim a terminal.
        for args in [
            ["linget", "tui", "--help"].as_slice(),
            ["linget", "tui", "-h"].as_slice(),
            ["linget", "gui", "--help"].as_slice(),
            ["linget", "tui", "--version"].as_slice(),
            ["linget", "tui", "--classic", "--help"].as_slice(),
        ] {
            assert_eq!(
                detect_run_mode_from(args.iter().copied()),
                RunMode::Cli,
                "{args:?} should be answered by the CLI, not a UI"
            );
        }
    }

    #[test]
    fn launching_a_ui_still_works_with_other_flags() {
        assert_eq!(
            detect_run_mode_from(["linget", "tui", "--classic"].iter().copied()),
            RunMode::Tui
        );
        assert_eq!(
            detect_run_mode_from(["linget", "tui"].iter().copied()),
            RunMode::Tui
        );
    }

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
