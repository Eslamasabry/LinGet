#![allow(dead_code)]

use crate::models::{ScheduledTask, SchedulerState};
use chrono::Local;
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
    Retry(String),
}

pub fn build_task_queue_view<F>(data: &TaskQueueViewData, on_action: F) -> gtk::Box
where
    F: Fn(TaskQueueAction) + Clone + 'static,
{
    let container = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(0)
        .build();

    let header = build_header(on_action.clone());
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
        let pending_tasks: Vec<&ScheduledTask> = data.scheduler.pending_tasks();
        let completed_tasks: Vec<&ScheduledTask> = data
            .scheduler
            .tasks
            .iter()
            .filter(|t| t.completed)
            .collect();

        if pending_tasks.is_empty() && completed_tasks.is_empty() {
            let empty = build_empty_state();
            scrolled.set_child(Some(&empty));
        } else {
            let content = build_task_list(
                &pending_tasks,
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

fn build_header<F>(on_action: F) -> gtk::Box
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
    pending: &[&ScheduledTask],
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

    if !pending.is_empty() {
        let pending_count = pending.len();
        let pending_group = adw::PreferencesGroup::builder()
            .title(format!("Pending ({pending_count})"))
            .description("Tasks waiting to run at their scheduled time")
            .build();

        for task in pending {
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
        .title(&task.package_name)
        .subtitle(&subtitle)
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
    let completed_time = task
        .scheduled_at
        .with_timezone(&Local)
        .format("%b %d, %I:%M %p")
        .to_string();

    let status_text = if task.error.is_some() {
        "Failed"
    } else {
        "Completed"
    };

    let subtitle = format!(
        "{} \u{2022} {} \u{2022} {}",
        task.operation.display_name(),
        completed_time,
        status_text
    );

    let row = adw::ActionRow::builder()
        .title(&task.package_name)
        .subtitle(&subtitle)
        .build();

    let (icon_name, icon_class) = if task.error.is_some() {
        ("dialog-error-symbolic", "error")
    } else {
        ("emblem-ok-symbolic", "success")
    };

    let icon = gtk::Image::builder().icon_name(icon_name).build();
    icon.add_css_class(icon_class);
    row.add_prefix(&icon);

    let source_label = gtk::Label::builder().label(task.source.to_string()).build();
    source_label.add_css_class("caption");
    source_label.add_css_class("dim-label");
    row.add_suffix(&source_label);

    if task.error.is_some() {
        let retry_btn = gtk::Button::builder()
            .icon_name("view-refresh-symbolic")
            .tooltip_text("Retry")
            .valign(gtk::Align::Center)
            .build();
        retry_btn.add_css_class("flat");
        retry_btn.add_css_class("circular");

        let task_id = task.id.clone();
        retry_btn.connect_clicked(move |_| {
            on_action(TaskQueueAction::Retry(task_id.clone()));
        });
        row.add_suffix(&retry_btn);

        if let Some(ref error) = task.error {
            row.set_tooltip_text(Some(error));
        }
    }

    row
}
