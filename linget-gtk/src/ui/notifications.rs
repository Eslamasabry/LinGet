use gtk4::gio;
use gtk4::prelude::*;
use parking_lot::Mutex;
use std::sync::LazyLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationNavRequest {
    ViewTasks,
    ViewUpdates,
}

static PENDING_NAV: LazyLock<Mutex<Option<NotificationNavRequest>>> =
    LazyLock::new(|| Mutex::new(None));

pub fn set_pending_nav(request: NotificationNavRequest) {
    *PENDING_NAV.lock() = Some(request);
}

pub fn take_pending_nav() -> Option<NotificationNavRequest> {
    PENDING_NAV.lock().take()
}

pub fn send_system_notification_with_action(
    title: &str,
    body: &str,
    notification_id: Option<&str>,
    action: Option<(&str, &str)>,
) {
    let Some(app) = gio::Application::default() else {
        tracing::debug!("No application instance for notification");
        return;
    };

    let notification = gio::Notification::new(title);
    notification.set_body(Some(body));
    notification.set_priority(gio::NotificationPriority::Normal);

    if let Some((label, action_name)) = action {
        notification.add_button(label, action_name);
    }

    let id = notification_id.unwrap_or("linget-notification");
    app.send_notification(Some(id), &notification);
}

pub fn send_system_notification(title: &str, body: &str, notification_id: Option<&str>) {
    send_system_notification_with_action(title, body, notification_id, None);
}

pub fn send_task_scheduled_notification(package_name: &str, scheduled_time: &str) {
    send_system_notification_with_action(
        "Task Scheduled",
        &format!("{} scheduled for {}", package_name, scheduled_time),
        Some("linget-task-scheduled"),
        Some(("View Queue", "app.view-tasks")),
    );
}

pub fn send_task_completed_notification(package_name: &str) {
    send_system_notification_with_action(
        "Task Completed",
        &format!("Update for {} completed successfully", package_name),
        Some("linget-task-completed"),
        Some(("View Queue", "app.view-tasks")),
    );
}

pub fn send_task_failed_notification(package_name: &str, error: &str) {
    let body = if error.len() > 100 {
        format!("Update for {} failed: {}...", package_name, &error[..100])
    } else {
        format!("Update for {} failed: {}", package_name, error)
    };

    send_system_notification_with_action(
        "Task Failed",
        &body,
        Some("linget-task-failed"),
        Some(("View Queue", "app.view-tasks")),
    );
}

pub fn send_updates_available_notification(count: usize) {
    let body = if count == 1 {
        "1 package update available".to_string()
    } else {
        format!("{} package updates available", count)
    };

    send_system_notification_with_action(
        "Updates Available",
        &body,
        Some("linget-updates"),
        Some(("View Updates", "app.view-updates")),
    );
}
