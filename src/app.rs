use crate::models::init_icon_cache;
use crate::ui::LinGetWindow;
use gtk4::prelude::*;
use gtk4::gio;
use libadwaita as adw;

pub const APP_ID: &str = "io.github.linget";
pub const APP_NAME: &str = "LinGet";
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn build_app() -> adw::Application {
    let app = adw::Application::builder()
        .application_id(APP_ID)
        .flags(gio::ApplicationFlags::FLAGS_NONE)
        .build();

    app.connect_startup(|_| {
        load_css();
        load_icons();
        // Initialize icon cache in background
        init_icon_cache();
    });

    app.connect_activate(build_ui);

    app
}

fn load_icons() {
    // Add custom icon path
    let icon_theme = gtk4::IconTheme::for_display(&gtk4::gdk::Display::default().unwrap());

    // Add application icons from data directory
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(parent) = exe_path.parent() {
            let icons_path = parent.join("../share/icons/hicolor");
            if icons_path.exists() {
                icon_theme.add_search_path(&icons_path);
            }
        }
    }

    // Also try local data directory
    icon_theme.add_search_path("data/icons/hicolor");

    // Add Flatpak exported icons
    icon_theme.add_search_path("/var/lib/flatpak/exports/share/icons/hicolor");

    // Add Snap icons
    icon_theme.add_search_path("/snap");

    // Add user icons
    if let Some(data_dir) = dirs::data_dir() {
        icon_theme.add_search_path(data_dir.join("icons/hicolor"));
    }
}

fn load_css() {
    let provider = gtk4::CssProvider::new();
    provider.load_from_data(include_str!("../resources/style.css"));

    gtk4::style_context_add_provider_for_display(
        &gtk4::gdk::Display::default().expect("Could not get default display"),
        &provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

fn build_ui(app: &adw::Application) {
    tracing::info!("Building UI...");
    let window = LinGetWindow::new(app);
    window.present();
    tracing::info!("UI built and presented.");
}

