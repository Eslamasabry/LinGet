use gtk4::gio;
use gtk4::prelude::*;

pub fn send_system_notification(title: &str, body: &str, notification_id: Option<&str>) {
    let Some(app) = gio::Application::default() else {
        tracing::debug!("No application instance for notification");
        return;
    };

    let notification = gio::Notification::new(title);
    notification.set_body(Some(body));
    notification.set_priority(gio::NotificationPriority::Normal);

    let id = notification_id.unwrap_or("linget-notification");
    app.send_notification(Some(id), &notification);
}

pub fn send_task_scheduled_notification(package_name: &str, scheduled_time: &str) {
    send_system_notification(
        "Task Scheduled",
        &format!("{} scheduled for {}", package_name, scheduled_time),
        Some("linget-task-scheduled"),
    );
}

pub fn send_task_completed_notification(package_name: &str) {
    send_system_notification(
        "Task Completed",
        &format!("Update for {} completed successfully", package_name),
        Some("linget-task-completed"),
    );
}

pub fn send_task_failed_notification(package_name: &str, error: &str) {
    let body = if error.len() > 100 {
        format!("Update for {} failed: {}...", package_name, &error[..100])
    } else {
        format!("Update for {} failed: {}", package_name, error)
    };

    send_system_notification("Task Failed", &body, Some("linget-task-failed"));
}

pub fn send_updates_available_notification(count: usize) {
    let body = if count == 1 {
        "1 package update available".to_string()
    } else {
        format!("{} package updates available", count)
    };

    send_system_notification("Updates Available", &body, Some("linget-updates"));
}
