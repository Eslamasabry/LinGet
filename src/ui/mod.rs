pub mod alias_view;
pub mod appearance;
mod backup;
pub mod command_palette;
mod empty_state;
pub mod header;
pub mod health_dashboard;
pub mod history_view;
pub mod notifications;
pub mod package_details;
pub mod preferences;
pub mod relm_app;
pub mod sidebar;
mod skeleton;
pub mod storage_view;
pub mod task_hub;
pub mod task_queue_view;
mod tray;
pub mod widgets;

#[allow(unused_imports)]
pub use health_dashboard::{
    build_health_dashboard, HealthAction, HealthData, HealthIssueData, Severity,
};
pub use relm_app::run_relm4_app;
pub use tray::TrayHandle;

#[allow(unused_imports)]
pub(crate) use appearance::apply_appearance;
pub(crate) use backup::{show_export_dialog, show_import_dialog};
pub(crate) use empty_state::EmptyState;
pub(crate) use preferences::{apply_theme_settings, build_preferences_window};
pub(crate) use skeleton::{SkeletonGrid, SkeletonList};

use gtk4::glib;
use once_cell::sync::Lazy;
use parking_lot::RwLock;

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};

static UI_START: Lazy<Instant> = Lazy::new(Instant::now);
static UI_HEARTBEAT_AT_MS: AtomicU64 = AtomicU64::new(0);
static UI_LAST_ACTION: Lazy<RwLock<String>> = Lazy::new(|| RwLock::new(String::new()));
static UI_LAST_ACTION_AT_MS: AtomicU64 = AtomicU64::new(0);
static UI_WATCHDOG_STARTED: AtomicBool = AtomicBool::new(false);

pub(crate) fn set_ui_marker(marker: impl Into<String>) {
    *UI_LAST_ACTION.write() = marker.into();
    UI_LAST_ACTION_AT_MS.store(UI_START.elapsed().as_millis() as u64, Ordering::Relaxed);
}

pub(crate) fn start_ui_watchdog() {
    if UI_WATCHDOG_STARTED.swap(true, Ordering::Relaxed) {
        return;
    }

    UI_HEARTBEAT_AT_MS.store(UI_START.elapsed().as_millis() as u64, Ordering::Relaxed);

    glib::timeout_add_local(Duration::from_millis(200), || {
        UI_HEARTBEAT_AT_MS.store(UI_START.elapsed().as_millis() as u64, Ordering::Relaxed);
        glib::ControlFlow::Continue
    });

    if let Err(e) = thread::Builder::new()
        .name("ui-watchdog".to_string())
        .spawn(|| {
            let mut last_warn_at = 0u64;
            let threshold_ms = 1500u64;

            loop {
                thread::sleep(Duration::from_millis(250));
                let now = UI_START.elapsed().as_millis() as u64;
                let beat = UI_HEARTBEAT_AT_MS.load(Ordering::Relaxed);
                let lag = now.saturating_sub(beat);

                if lag >= threshold_ms && now.saturating_sub(last_warn_at) >= threshold_ms {
                    let last_action = UI_LAST_ACTION.read().clone();
                    let last_action_at = UI_LAST_ACTION_AT_MS.load(Ordering::Relaxed);
                    let last_action_age = now.saturating_sub(last_action_at);

                    tracing::warn!(
                        lag_ms = lag,
                        last_action_age_ms = last_action_age,
                        last_action = %last_action,
                        "GTK main loop appears blocked"
                    );

                    last_warn_at = now;
                }
            }
        })
    {
        tracing::warn!(error = %e, "Failed to start UI watchdog thread");
    }
}

/// Regex pattern to match HTML tags for stripping
static HTML_TAG_PATTERN: Lazy<regex::Regex> =
    Lazy::new(|| regex::Regex::new(r"<[^>]*>").expect("Invalid HTML tag regex"));

/// Strips HTML tags from text, returning plain text content.
/// Used to sanitize package descriptions that may contain HTML/Markdown.
pub fn strip_html_tags(text: &str) -> String {
    let stripped = HTML_TAG_PATTERN.replace_all(text, "");
    stripped.split_whitespace().collect::<Vec<_>>().join(" ")
}
