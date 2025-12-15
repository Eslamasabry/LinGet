use notify_rust::{Notification, Timeout};

use crate::app::{APP_ID, APP_NAME};

/// Send a desktop notification for available updates
pub fn notify_updates_available(count: usize) {
    if count == 0 {
        return;
    }

    let summary = format!("{} Updates Available", count);
    let body = if count == 1 {
        "1 package can be updated".to_string()
    } else {
        format!("{} packages can be updated", count)
    };

    send_notification(&summary, &body, "software-update-available");
}

/// Send a desktop notification for a completed operation
#[allow(dead_code)]
pub fn notify_operation_complete(operation: &str, package_name: &str, success: bool) {
    let (summary, icon) = if success {
        (
            format!("{} completed", operation),
            "emblem-ok-symbolic",
        )
    } else {
        (
            format!("{} failed", operation),
            "dialog-error-symbolic",
        )
    };

    send_notification(&summary, package_name, icon);
}

/// Send a generic notification
fn send_notification(summary: &str, body: &str, icon: &str) {
    let result = Notification::new()
        .appname(APP_NAME)
        .summary(summary)
        .body(body)
        .icon(icon)
        .hint(notify_rust::Hint::DesktopEntry(APP_ID.to_string()))
        .timeout(Timeout::Milliseconds(5000))
        .show();

    if let Err(e) = result {
        tracing::debug!("Failed to send notification: {}", e);
    }
}
