mod app;
mod backend;
mod models;
mod ui;

use gtk4::prelude::*;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

fn sanitize_environment() {
    // When launching LinGet from some snapped terminals (e.g. Ghostty),
    // environment variables can point GTK's pixbuf loader to Snap-provided
    // modules built against a different glibc, causing icon-load failures.
    // Prefer the system loaders for this app.
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

    tracing::info!("Starting {} v{}", app::APP_NAME, app::APP_VERSION);

    sanitize_environment();

    // Create tokio runtime for async operations
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to create Tokio runtime");

    // Keep the runtime alive in a background thread
    let _guard = runtime.enter();

    // Initialize GTK
    let app = app::build_app();

    // Run the application
    let exit_code = app.run();

    tracing::info!("Exiting with code: {:?}", exit_code);
    std::process::exit(exit_code.into());
}
