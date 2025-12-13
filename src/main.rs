mod app;
mod backend;
mod models;
mod ui;

use gtk4::prelude::*;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

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
