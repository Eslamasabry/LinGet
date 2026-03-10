#![allow(dead_code)]

use crate::models::history::FailureCategory;
use crate::models::{ScheduledTask, SchedulerState};
use crate::ui::escape_markup_text;
use gtk4::prelude::*;
use gtk4::{self as gtk};
use libadwaita as adw;
use libadwaita::prelude::*;

#[derive(Debug, Clone, Default)]
pub struct TaskQueueViewData {
    pub scheduler: SchedulerState,
    pub is_loading: bool,
    pub running_task_id: Option<String>,
}

#[derive(Debug, Clone)]
pub enum TaskQueueAction {
    Cancel(String),
    Refresh,
    RunNow(String),
    ClearCompleted,
    CopyFailureDetails(String),
    RetryFailed,
    Retry(String),
}

fn queue_summary_label(
    active_count: usize,
    completed_count: usize,
    failed_count: usize,
    running: bool,
) -> Option<String> {
    let total = active_count + completed_count;
    if total == 0 {
        return None;
    }

    let status = if running { "running" } else { "idle" };
    Some(if failed_count > 0 {
        format!(
            "{} active • {} completed ({} failed) • {}",
            active_count, completed_count, failed_count, status
        )
    } else {
        format!(
            "{} active • {} completed • {}",
            active_count, completed_count, status
        )
    })
}

fn retry_failed_button_label(failed_count: usize) -> String {
    if failed_count == 1 {
        "Retry Failed".to_string()
    } else {
        format!("Retry Failed ({failed_count})")
    }
}

pub(crate) fn retry_failed_task_ids(scheduler: &SchedulerState) -> Vec<String> {
    scheduler
        .tasks
        .iter()
        .filter(|task| task.completed && task.error.is_some())
        .map(|task| task.id.clone())
        .collect()
}

fn scheduled_task_failure_report(task: &ScheduledTask) -> Option<String> {
    let error = task.error.as_deref()?;
    let category = FailureCategory::classify(error);

    Some(
        [
            format!("Task: {}", task.package_name),
            format!("Operation: {}", task.operation.display_name()),
            format!("Source: {}", task.source),
            format!("Failure: {} ({})", category.label(), category.code()),
            format!("Provider error: {}", error),
            format!("Recovery: {}", category.remediation_copy()),
            format!("Next step: {}", category.action_hint()),
        ]
        .join("\n"),
    )
}

pub fn build_task_queue_view<F>(data: &TaskQueueViewData, on_action: F) -> gtk::Box
where
    F: Fn(TaskQueueAction) + Clone + 'static,
{
    let container = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(0)
        .build();

    let failed_task_ids = retry_failed_task_ids(&data.scheduler);
    let header = build_header(failed_task_ids.len(), on_action.clone());
    container.append(&header);

    let scrolled = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vexpand(true)
        .build();

    if data.is_loading {
        let spinner = gtk::Spinner::builder()
            .spinning(true)
            .width_request(32)
            .height_request(32)
            .halign(gtk::Align::Center)
            .valign(gtk::Align::Center)
            .margin_top(48)
            .build();
        scrolled.set_child(Some(&spinner));
    } else {
        let mut active_tasks: Vec<&ScheduledTask> = data.scheduler.pending_tasks();
        active_tasks.sort_by_key(|task| task.scheduled_at);

        let mut completed_tasks: Vec<&ScheduledTask> = data
            .scheduler
            .tasks
            .iter()
            .filter(|t| t.completed)
            .collect();
        completed_tasks
            .sort_by_key(|task| std::cmp::Reverse(task.completed_at.unwrap_or(task.scheduled_at)));

        if let Some(summary) = build_queue_summary(
            active_tasks.len(),
            completed_tasks.len(),
            failed_task_ids.len(),
            data.running_task_id.is_some(),
        ) {
            container.append(&summary);
        }

        if active_tasks.is_empty() && completed_tasks.is_empty() {
            let empty = build_empty_state();
            scrolled.set_child(Some(&empty));
        } else {
            let content = build_task_list(
                &active_tasks,
                &completed_tasks,
                data.running_task_id.as_deref(),
                on_action,
            );
            scrolled.set_child(Some(&content));
        }
    }

    container.append(&scrolled);
    container
}

fn build_header<F>(failed_count: usize, on_action: F) -> gtk::Box
where
    F: Fn(TaskQueueAction) + Clone + 'static,
{
    let header = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(12)
        .margin_top(16)
        .margin_bottom(16)
        .margin_start(24)
        .margin_end(24)
        .build();

    let title = gtk::Label::builder()
        .label("Manage your scheduled package operations")
        .hexpand(true)
        .xalign(0.0)
        .build();
    title.add_css_class("dim-label");

    header.append(&title);

    if failed_count > 0 {
        let retry_failed_btn = gtk::Button::builder()
            .label(retry_failed_button_label(failed_count))
            .tooltip_text("Mark failed tasks due now and retry them in queue order")
            .build();
        retry_failed_btn.add_css_class("suggested-action");

        let on_action_retry = on_action.clone();
        retry_failed_btn.connect_clicked(move |_| {
            on_action_retry(TaskQueueAction::RetryFailed);
        });

        header.append(&retry_failed_btn);
    }

    let clear_btn = gtk::Button::builder()
        .label("Clear Completed")
        .tooltip_text("Remove all completed tasks from the list")
        .build();
    clear_btn.add_css_class("flat");

    let on_action_clear = on_action.clone();
    clear_btn.connect_clicked(move |_| {
        on_action_clear(TaskQueueAction::ClearCompleted);
    });

    header.append(&clear_btn);

    let refresh_btn = gtk::Button::builder()
        .icon_name("view-refresh-symbolic")
        .tooltip_text("Refresh")
        .build();
    refresh_btn.add_css_class("flat");

    refresh_btn.connect_clicked(move |_| {
        on_action(TaskQueueAction::Refresh);
    });

    header.append(&refresh_btn);

    header
}

fn build_queue_summary(
    active_count: usize,
    completed_count: usize,
    failed_count: usize,
    running: bool,
) -> Option<gtk::Box> {
    let label_text = queue_summary_label(active_count, completed_count, failed_count, running)?;
    let total = active_count + completed_count;

    let wrapper = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(6)
        .margin_start(24)
        .margin_end(24)
        .margin_bottom(8)
        .build();

    let label = gtk::Label::builder()
        .label(&label_text)
        .xalign(0.0)
        .css_classes(vec!["caption", "dim-label"])
        .build();

    let progress = gtk::ProgressBar::builder()
        .fraction(completed_count as f64 / total as f64)
        .show_text(true)
        .build();
    progress.add_css_class("osd");
    progress.set_text(Some(&format!("{}/{} complete", completed_count, total)));

    wrapper.append(&label);
    if failed_count > 0 {
        let guidance = gtk::Label::builder()
            .label("Failed tasks need attention. Use Retry Failed to requeue them now.")
            .xalign(0.0)
            .css_classes(vec!["caption", "warning"])
            .build();
        wrapper.append(&guidance);
    }
    wrapper.append(&progress);
    Some(wrapper)
}

fn build_empty_state() -> gtk::Box {
    let container = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(12)
        .halign(gtk::Align::Center)
        .valign(gtk::Align::Center)
        .margin_top(48)
        .margin_bottom(48)
        .build();

    let icon = gtk::Image::builder()
        .icon_name("alarm-symbolic")
        .pixel_size(64)
        .build();
    icon.add_css_class("dim-label");

    let title = gtk::Label::builder().label("No Scheduled Tasks").build();
    title.add_css_class("title-2");

    let subtitle = gtk::Label::builder()
        .label("Schedule updates for later from the package details panel or selection bar.\nTasks will run automatically at the scheduled time.")
        .wrap(true)
        .max_width_chars(50)
        .justify(gtk::Justification::Center)
        .build();
    subtitle.add_css_class("dim-label");

    container.append(&icon);
    container.append(&title);
    container.append(&subtitle);

    container
}

fn build_task_list<F>(
    active: &[&ScheduledTask],
    completed: &[&ScheduledTask],
    running_task_id: Option<&str>,
    on_action: F,
) -> gtk::Box
where
    F: Fn(TaskQueueAction) + Clone + 'static,
{
    let container = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(16)
        .margin_top(8)
        .margin_bottom(24)
        .margin_start(24)
        .margin_end(24)
        .build();

    if !active.is_empty() {
        let active_count = active.len();
        let pending_group = adw::PreferencesGroup::builder()
            .title(format!("Active Queue ({active_count})"))
            .description("Queued, due, and running tasks")
            .build();

        for task in active {
            let is_running = running_task_id.is_some_and(|id| id == task.id);
            let row = build_pending_task_row(task, is_running, on_action.clone());
            pending_group.add(&row);
        }

        container.append(pending_group.upcast_ref::<gtk::Widget>());
    }

    if !completed.is_empty() {
        let completed_group = adw::PreferencesGroup::builder()
            .title("Recently Completed")
            .build();

        for task in completed.iter().take(10) {
            let row = build_completed_task_row(task, on_action.clone());
            completed_group.add(&row);
        }

        container.append(completed_group.upcast_ref::<gtk::Widget>());
    }

    container
}

fn build_pending_task_row<F>(task: &ScheduledTask, is_running: bool, on_action: F) -> adw::ActionRow
where
    F: Fn(TaskQueueAction) + Clone + 'static,
{
    let subtitle = if is_running {
        "Running...".to_string()
    } else {
        let time_until = task.time_until();
        let scheduled_time = task.scheduled_time_display();
        format!("{} \u{2022} {}", time_until, scheduled_time)
    };

    let row = adw::ActionRow::builder()
        .title(escape_markup_text(&task.package_name).as_str())
        .subtitle(escape_markup_text(&subtitle).as_str())
        .build();

    if is_running {
        let spinner = gtk::Spinner::builder()
            .spinning(true)
            .valign(gtk::Align::Center)
            .build();
        row.add_prefix(&spinner);
    } else {
        let icon = gtk::Image::builder()
            .icon_name(task.operation.icon_name())
            .build();
        icon.add_css_class("accent");
        row.add_prefix(&icon);
    }

    let source_label = gtk::Label::builder().label(task.source.to_string()).build();
    source_label.add_css_class("caption");
    source_label.add_css_class("dim-label");
    row.add_suffix(&source_label);

    if !is_running {
        let run_now_btn = gtk::Button::builder()
            .icon_name("media-playback-start-symbolic")
            .tooltip_text("Run now")
            .valign(gtk::Align::Center)
            .build();
        run_now_btn.add_css_class("flat");
        run_now_btn.add_css_class("circular");

        let task_id_run = task.id.clone();
        let on_action_run = on_action.clone();
        run_now_btn.connect_clicked(move |_| {
            on_action_run(TaskQueueAction::RunNow(task_id_run.clone()));
        });
        row.add_suffix(&run_now_btn);

        let cancel_btn = gtk::Button::builder()
            .icon_name("window-close-symbolic")
            .tooltip_text("Cancel task")
            .valign(gtk::Align::Center)
            .build();
        cancel_btn.add_css_class("flat");
        cancel_btn.add_css_class("circular");
        cancel_btn.add_css_class("destructive-action");

        let task_id_cancel = task.id.clone();
        cancel_btn.connect_clicked(move |_| {
            on_action(TaskQueueAction::Cancel(task_id_cancel.clone()));
        });
        row.add_suffix(&cancel_btn);
    }

    row
}

fn build_completed_task_row<F>(task: &ScheduledTask, on_action: F) -> adw::ActionRow
where
    F: Fn(TaskQueueAction) + Clone + 'static,
{
    let completed_time = task.completed_time_display();
    let failure_context = task
        .error
        .as_deref()
        .map(|error| (FailureCategory::classify(error), error));

    let status_text = if let Some((category, _)) = failure_context {
        format!("Failed · {} ({})", category.label(), category.code())
    } else {
        "Completed".to_string()
    };

    let subtitle = format!(
        "{} \u{2022} {} \u{2022} {}",
        task.operation.display_name(),
        completed_time,
        status_text
    );

    let row = adw::ActionRow::builder()
        .title(escape_markup_text(&task.package_name).as_str())
        .subtitle(escape_markup_text(&subtitle).as_str())
        .build();

    let (icon_name, icon_class) = if task.error.is_some() {
        ("dialog-error-symbolic", "error")
    } else {
        ("emblem-ok-symbolic", "success")
    };

    let icon = gtk::Image::builder().icon_name(icon_name).build();
    icon.add_css_class(icon_class);
    row.add_prefix(&icon);

    let meta_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(6)
        .valign(gtk::Align::Center)
        .build();

    let source_label = gtk::Label::builder().label(task.source.to_string()).build();
    source_label.add_css_class("caption");
    source_label.add_css_class("dim-label");
    meta_box.append(&source_label);

    if let Some((category, _)) = failure_context {
        let code_label = gtk::Label::builder().label(category.code()).build();
        code_label.add_css_class("caption");
        code_label.add_css_class("error");
        meta_box.append(&code_label);
    }

    row.add_suffix(&meta_box);

    if task.error.is_some() {
        if let Some(report) = scheduled_task_failure_report(task) {
            let on_action_copy = on_action.clone();
            let copy_btn = gtk::Button::builder()
                .icon_name("edit-copy-symbolic")
                .tooltip_text("Copy failure details")
                .valign(gtk::Align::Center)
                .build();
            copy_btn.add_css_class("flat");
            copy_btn.add_css_class("circular");

            copy_btn.connect_clicked(move |_| {
                on_action_copy(TaskQueueAction::CopyFailureDetails(report.clone()));
            });
            row.add_suffix(&copy_btn);
        }

        let retry_btn = gtk::Button::builder()
            .icon_name("view-refresh-symbolic")
            .tooltip_text("Retry")
            .valign(gtk::Align::Center)
            .build();
        retry_btn.add_css_class("flat");
        retry_btn.add_css_class("circular");

        let task_id = task.id.clone();
        let on_action_retry = on_action.clone();
        retry_btn.connect_clicked(move |_| {
            on_action_retry(TaskQueueAction::Retry(task_id.clone()));
        });
        row.add_suffix(&retry_btn);

        if let Some((category, error)) = failure_context {
            row.set_tooltip_text(Some(&format!(
                "{}\nRecovery: {}\nNext step: {}",
                error,
                category.remediation_copy(),
                category.action_hint()
            )));
        }
    }

    row
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{PackageSource, ScheduledOperation, ScheduledTask};
    use chrono::Utc;

    #[test]
    fn queue_summary_label_mentions_failed_tasks() {
        assert_eq!(
            queue_summary_label(1, 2, 1, true).as_deref(),
            Some("1 active • 2 completed (1 failed) • running")
        );
        assert_eq!(
            queue_summary_label(1, 2, 0, false).as_deref(),
            Some("1 active • 2 completed • idle")
        );
    }

    #[test]
    fn retry_failed_task_ids_only_returns_failed_completed_entries() {
        let mut failed = ScheduledTask::new(
            "APT:vim".to_string(),
            "vim".to_string(),
            PackageSource::Apt,
            ScheduledOperation::Update,
            Utc::now(),
        );
        failed.completed = true;
        failed.error = Some("network timeout".to_string());

        let mut completed = ScheduledTask::new(
            "APT:curl".to_string(),
            "curl".to_string(),
            PackageSource::Apt,
            ScheduledOperation::Update,
            Utc::now(),
        );
        completed.completed = true;

        let running = ScheduledTask::new(
            "APT:git".to_string(),
            "git".to_string(),
            PackageSource::Apt,
            ScheduledOperation::Update,
            Utc::now(),
        );

        let scheduler = SchedulerState {
            tasks: vec![failed.clone(), completed, running],
        };

        assert_eq!(retry_failed_task_ids(&scheduler), vec![failed.id]);
    }

    #[test]
    fn scheduled_task_failure_report_contains_recovery_guidance() {
        let mut failed = ScheduledTask::new(
            "APT:vim".to_string(),
            "vim".to_string(),
            PackageSource::Apt,
            ScheduledOperation::Update,
            Utc::now(),
        );
        failed.completed = true;
        failed.error = Some("permission denied".to_string());

        let report = scheduled_task_failure_report(&failed).expect("expected failure report");
        assert!(report.contains("Failure: Permissions"));
        assert!(report.contains("Recovery:"));
        assert!(report.contains("Next step:"));
    }
}
