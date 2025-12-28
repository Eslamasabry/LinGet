use crate::models::{init_icon_cache, load_cache as load_enrichment_cache};
use crate::ui::TrayHandle;
use gtk4::prelude::ObjectExt;
use std::cell::RefCell;

#[allow(dead_code)]
pub const APP_ID: &str = "io.github.linget";
pub const APP_NAME: &str = "LinGet";
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

thread_local! {
    static TRAY_HANDLE: RefCell<Option<TrayHandle>> = const { RefCell::new(None) };
}

pub fn load_css_internal() {
    let provider = gtk4::CssProvider::new();
    provider.load_from_data(include_str!("../resources/style.css"));

    gtk4::style_context_add_provider_for_display(
        &gtk4::gdk::Display::default().expect("Could not get default display"),
        &provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

pub fn load_icons_internal() {
    let Some(display) = gtk4::gdk::Display::default() else {
        tracing::warn!("No display available for loading icons");
        return;
    };
    let icon_theme = gtk4::IconTheme::for_display(&display);

    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(parent) = exe_path.parent() {
            let icons_path = parent.join("../share/icons/hicolor");
            if icons_path.exists() {
                icon_theme.add_search_path(&icons_path);
            }
        }
    }

    icon_theme.add_search_path("data/icons/hicolor");
    icon_theme.add_search_path("/var/lib/flatpak/exports/share/icons/hicolor");
    icon_theme.add_search_path("/snap");

    if let Some(data_dir) = dirs::data_dir() {
        icon_theme.add_search_path(data_dir.join("icons/hicolor"));
    }
}

pub fn init_startup() {
    load_css_internal();
    load_icons_internal();

    if let Some(settings) = gtk4::Settings::default() {
        settings.set_property("gtk-decoration-layout", ":minimize,maximize,close");
    }

    init_icon_cache();
    load_enrichment_cache();

    if let Some(tray) = TrayHandle::start() {
        TRAY_HANDLE.with(|cell| {
            *cell.borrow_mut() = Some(tray);
        });
    }
}

#[allow(dead_code)]
pub fn with_tray<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&TrayHandle) -> R,
{
    TRAY_HANDLE.with(|cell| cell.borrow().as_ref().map(f))
}
