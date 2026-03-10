use crate::models::history::FailureCategory;
use crate::models::Package;
use crate::ui::escape_markup_text;

use chrono::Local;
use gtk4::prelude::*;
use gtk4::{self as gtk, glib};
use libadwaita as adw;
use libadwaita::prelude::*;
use relm4::prelude::*;
use std::time::Instant;

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskStatus {
    Active,
    Success,
    Error,
    Info,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub enum PackageOp {
    Install,
    Update,
    Remove,
    Downgrade,
    DowngradeTo(String),
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub enum RetrySpec {
    Package {
        package: Box<Package>,
        op: PackageOp,
    },
    BulkUpdate {
        packages: Vec<Package>,
    },
    BulkRemove {
        packages: Vec<Package>,
    },
}

#[derive(Clone, Debug)]
pub struct BatchProgress {
    pub current_step: usize,
    pub total_steps: usize,
    pub current_item: Option<String>,
    pub started_at: Instant,
}

impl BatchProgress {
    pub fn new(total_steps: usize) -> Self {
        Self {
            current_step: 0,
            total_steps,
            current_item: None,
            started_at: Instant::now(),
        }
    }

    pub fn eta_seconds(&self) -> Option<u64> {
        if self.current_step == 0 {
            return None;
        }

        let elapsed = self.started_at.elapsed().as_secs_f64();
        let avg_per_item = elapsed / self.current_step as f64;
        let remaining = self.total_steps.saturating_sub(self.current_step);
        Some((avg_per_item * remaining as f64) as u64)
    }

    pub fn eta_display(&self) -> Option<String> {
        self.eta_seconds().map(|secs| {
            if secs < 60 {
                format!("~{}s left", secs)
            } else {
                let mins = secs / 60;
                let remaining_secs = secs % 60;
                if remaining_secs > 0 {
                    format!("~{}m {}s left", mins, remaining_secs)
                } else {
                    format!("~{}m left", mins)
                }
            }
        })
    }

    pub fn fraction(&self) -> f64 {
        if self.total_steps == 0 {
            0.0
        } else {
            self.current_step as f64 / self.total_steps as f64
        }
    }

    pub fn step_display(&self) -> String {
        format!("{}/{}", self.current_step, self.total_steps)
    }
}

const MAX_LOG_LINES: usize = 120;

#[derive(Clone, Debug)]
pub struct TaskEntry {
    pub id: usize,
    pub title: String,
    pub details: String,
    pub timestamp: String,
    pub status: TaskStatus,
    pub command: Option<String>,
    pub retry_spec: Option<RetrySpec>,
    pub batch_progress: Option<BatchProgress>,
    pub log_lines: Vec<String>,
}

impl TaskEntry {
    fn subtitle(&self) -> String {
        let details = self.details.trim();
        if details.is_empty() {
            self.timestamp.clone()
        } else {
            format!("{} · {}", self.timestamp, details)
        }
    }

    fn icon_name(&self) -> &'static str {
        match self.status {
            TaskStatus::Active => "content-loading-symbolic",
            TaskStatus::Success => "emblem-ok-symbolic",
            TaskStatus::Error => "dialog-error-symbolic",
            TaskStatus::Info => "dialog-information-symbolic",
        }
    }

    fn failure_context(&self) -> Option<(FailureCategory, String)> {
        if !matches!(self.status, TaskStatus::Error) {
            return None;
        }

        let details = self.details.trim();
        if !details.is_empty() {
            return Some((FailureCategory::classify(details), details.to_owned()));
        }

        self.log_lines
            .last()
            .map(|line| (FailureCategory::classify(line), line.clone()))
    }

    fn failure_report(&self) -> Option<String> {
        let (category, error_text) = self.failure_context()?;
        let mut lines = vec![
            format!("Task: {}", self.title),
            format!("Time: {}", self.timestamp),
            format!("Failure: {} ({})", category.label(), category.code()),
        ];

        let details = self.details.trim();
        if !details.is_empty() {
            lines.push(format!("Details: {}", details));
        }

        if let Some(command) = self.command.as_deref() {
            lines.push(format!("Command: {}", command));
        }

        if !error_text.trim().is_empty() {
            lines.push(format!("Provider error: {}", error_text.trim()));
        }

        lines.push(format!("Recovery: {}", category.remediation_copy()));
        lines.push(format!("Next step: {}", category.action_hint()));

        if !self.log_lines.is_empty() {
            lines.push("Recent logs:".to_string());
            for line in self.log_lines.iter().rev().take(10).rev() {
                lines.push(format!("- {}", line));
            }
        }

        Some(lines.join("\n"))
    }
}

#[derive(Debug)]
pub struct TaskHubInit;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum TaskHubInput {
    BeginTask {
        task_id: usize,
        title: String,
        details: String,
        retry_spec: Option<RetrySpec>,
    },
    BeginBatchTask {
        task_id: usize,
        title: String,
        total_steps: usize,
        retry_spec: Option<RetrySpec>,
    },
    UpdateBatchProgress {
        task_id: usize,
        current_step: usize,
        current_item: Option<String>,
    },
    AppendTaskLog {
        task_id: usize,
        line: String,
    },
    FinishTask {
        task_id: usize,
        status: TaskStatus,
        title: String,
        details: String,
        command: Option<String>,
    },
    AddEvent {
        status: TaskStatus,
        title: String,
        details: String,
        command: Option<String>,
    },
    Clear,
    MarkRead,
    CopyCommand(usize),
    CopyFailureDetails(usize),
    Retry(usize),
}

#[derive(Debug, Clone)]
pub enum TaskHubOutput {
    RetryOperation(RetrySpec),
    CopyText(String),
    UnreadCountChanged(u32),
}

pub struct TaskHubModel {
    tasks: Vec<TaskEntry>,
    next_id: usize,
    unread_count: u32,
}

impl TaskHubModel {
    fn now_stamp() -> String {
        Local::now().format("%H:%M:%S").to_string()
    }

    fn active_tasks(&self) -> impl Iterator<Item = &TaskEntry> {
        self.tasks
            .iter()
            .filter(|t| matches!(t.status, TaskStatus::Active))
    }

    fn history_tasks(&self) -> impl Iterator<Item = &TaskEntry> {
        self.tasks
            .iter()
            .filter(|t| !matches!(t.status, TaskStatus::Active))
    }

    fn has_active(&self) -> bool {
        self.active_tasks().next().is_some()
    }

    fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }
}

#[derive(Debug)]
pub struct TaskHubWidgets {
    stack: gtk::Stack,
    active_section: gtk::Box,
    active_list: gtk::ListBox,
    history_list: gtk::ListBox,
    unread_badge: gtk::Label,
}

impl SimpleComponent for TaskHubModel {
    type Init = TaskHubInit;
    type Input = TaskHubInput;
    type Output = TaskHubOutput;
    type Root = gtk::Box;
    type Widgets = TaskHubWidgets;

    fn init_root() -> Self::Root {
        gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .width_request(320)
            .css_classes(vec!["task-hub-popover"])
            .build()
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = TaskHubModel {
            tasks: Vec::new(),
            next_id: 0,
            unread_count: 0,
        };

        let header = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(8)
            .margin_top(12)
            .margin_bottom(8)
            .margin_start(12)
            .margin_end(12)
            .build();

        let title = gtk::Label::builder()
            .label("Tasks")
            .hexpand(true)
            .xalign(0.0)
            .css_classes(vec!["title-3"])
            .build();

        let unread_badge = gtk::Label::builder()
            .label("0")
            .visible(false)
            .css_classes(vec!["badge-accent"])
            .build();

        let clear_btn = gtk::Button::builder()
            .icon_name("user-trash-symbolic")
            .tooltip_text("Clear history")
            .css_classes(vec!["flat", "circular"])
            .build();

        {
            let sender = sender.clone();
            clear_btn.connect_clicked(move |_| {
                sender.input(TaskHubInput::Clear);
            });
        }

        header.append(&title);
        header.append(&unread_badge);
        header.append(&clear_btn);

        let active_header = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(8)
            .margin_top(6)
            .margin_bottom(6)
            .margin_start(12)
            .margin_end(12)
            .build();

        let active_label = gtk::Label::builder()
            .label("Active")
            .hexpand(true)
            .xalign(0.0)
            .css_classes(vec!["caption", "dim-label"])
            .build();
        active_header.append(&active_label);

        let active_list = gtk::ListBox::builder()
            .selection_mode(gtk::SelectionMode::None)
            .css_classes(vec!["boxed-list"])
            .build();

        let active_section = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .visible(false)
            .build();
        active_section.append(&active_header);
        active_section.append(&active_list);

        let history_header = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(8)
            .margin_top(18)
            .margin_bottom(6)
            .margin_start(12)
            .margin_end(12)
            .build();

        let history_label = gtk::Label::builder()
            .label("History")
            .hexpand(true)
            .xalign(0.0)
            .css_classes(vec!["caption", "dim-label"])
            .build();
        history_header.append(&history_label);

        let history_list = gtk::ListBox::builder()
            .selection_mode(gtk::SelectionMode::None)
            .css_classes(vec!["boxed-list"])
            .build();

        let content = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();
        content.append(&active_section);
        content.append(&history_header);
        content.append(&history_list);

        let empty = adw::StatusPage::builder()
            .icon_name("format-justify-fill-symbolic")
            .title("No recent activity")
            .description("Updates, installs, and removals will appear here")
            .build();

        let stack = gtk::Stack::builder()
            .transition_type(gtk::StackTransitionType::Crossfade)
            .transition_duration(150)
            .build();
        stack.add_named(&empty, Some("empty"));
        stack.add_named(&content, Some("list"));
        stack.set_visible_child_name("empty");

        let scrolled = gtk::ScrolledWindow::builder()
            .vexpand(true)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .child(&stack)
            .min_content_height(300)
            .max_content_height(500)
            .build();

        root.append(&header);
        root.append(&scrolled);

        let widgets = TaskHubWidgets {
            stack,
            active_section,
            active_list,
            history_list,
            unread_badge,
        };

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            TaskHubInput::BeginTask {
                task_id,
                title,
                details,
                retry_spec,
            } => {
                let task = TaskEntry {
                    id: task_id,
                    title,
                    details,
                    timestamp: Self::now_stamp(),
                    status: TaskStatus::Active,
                    command: None,
                    retry_spec,
                    batch_progress: None,
                    log_lines: Vec::new(),
                };
                self.tasks.insert(0, task);
            }

            TaskHubInput::BeginBatchTask {
                task_id,
                title,
                total_steps,
                retry_spec,
            } => {
                let task = TaskEntry {
                    id: task_id,
                    title,
                    details: format!("0/{} packages", total_steps),
                    timestamp: Self::now_stamp(),
                    status: TaskStatus::Active,
                    command: None,
                    retry_spec,
                    batch_progress: Some(BatchProgress::new(total_steps)),
                    log_lines: Vec::new(),
                };
                self.tasks.insert(0, task);
            }

            TaskHubInput::UpdateBatchProgress {
                task_id,
                current_step,
                current_item,
            } => {
                if let Some(task) = self.tasks.iter_mut().find(|t| t.id == task_id) {
                    if let Some(ref mut progress) = task.batch_progress {
                        progress.current_step = current_step;
                        progress.current_item = current_item.clone();

                        let eta_str = progress.eta_display().unwrap_or_default();
                        let item_str = current_item.map(|n| format!(": {}", n)).unwrap_or_default();

                        task.details = if eta_str.is_empty() {
                            format!("{}/{}{}", current_step, progress.total_steps, item_str)
                        } else {
                            format!(
                                "{}/{}{} · {}",
                                current_step, progress.total_steps, item_str, eta_str
                            )
                        };
                    }
                }
            }

            TaskHubInput::AppendTaskLog { task_id, line } => {
                if let Some(task) = self.tasks.iter_mut().find(|t| t.id == task_id) {
                    task.log_lines.push(line);
                    if task.log_lines.len() > MAX_LOG_LINES {
                        task.log_lines.remove(0);
                    }
                }
            }

            TaskHubInput::FinishTask {
                task_id,
                status,
                title,
                details,
                command,
            } => {
                if let Some(task) = self.tasks.iter_mut().find(|t| t.id == task_id) {
                    task.status = status;
                    task.title = title;
                    task.details = details;
                    task.timestamp = Self::now_stamp();
                    task.command = command;

                    self.unread_count = self.unread_count.saturating_add(1);
                    let _ = sender.output(TaskHubOutput::UnreadCountChanged(self.unread_count));
                }
            }

            TaskHubInput::AddEvent {
                status,
                title,
                details,
                command,
            } => {
                let id = self.next_id;
                self.next_id += 1;

                let task = TaskEntry {
                    id,
                    title,
                    details,
                    timestamp: Self::now_stamp(),
                    status,
                    command,
                    retry_spec: None,
                    batch_progress: None,
                    log_lines: Vec::new(),
                };
                self.tasks.insert(0, task);

                self.unread_count = self.unread_count.saturating_add(1);
                let _ = sender.output(TaskHubOutput::UnreadCountChanged(self.unread_count));
            }

            TaskHubInput::Clear => {
                self.tasks.clear();
                self.unread_count = 0;
                let _ = sender.output(TaskHubOutput::UnreadCountChanged(0));
            }

            TaskHubInput::MarkRead => {
                self.unread_count = 0;
                let _ = sender.output(TaskHubOutput::UnreadCountChanged(0));
            }

            TaskHubInput::CopyCommand(task_id) => {
                if let Some(task) = self.tasks.iter().find(|t| t.id == task_id) {
                    if let Some(ref cmd) = task.command {
                        let _ = sender.output(TaskHubOutput::CopyText(cmd.clone()));
                    }
                }
            }

            TaskHubInput::CopyFailureDetails(task_id) => {
                if let Some(task) = self.tasks.iter().find(|t| t.id == task_id) {
                    if let Some(report) = task.failure_report() {
                        let _ = sender.output(TaskHubOutput::CopyText(report));
                    }
                }
            }

            TaskHubInput::Retry(task_id) => {
                if let Some(task) = self.tasks.iter().find(|t| t.id == task_id) {
                    if let Some(ref spec) = task.retry_spec {
                        let _ = sender.output(TaskHubOutput::RetryOperation(spec.clone()));
                    }
                }
            }
        }
    }

    fn update_view(&self, widgets: &mut Self::Widgets, sender: ComponentSender<Self>) {
        if self.is_empty() {
            widgets.stack.set_visible_child_name("empty");
        } else {
            widgets.stack.set_visible_child_name("list");
        }

        if self.unread_count > 0 {
            widgets
                .unread_badge
                .set_label(&self.unread_count.to_string());
            widgets.unread_badge.set_visible(true);
        } else {
            widgets.unread_badge.set_visible(false);
        }

        widgets.active_section.set_visible(self.has_active());

        while let Some(child) = widgets.active_list.first_child() {
            widgets.active_list.remove(&child);
        }

        for task in self.active_tasks() {
            let row = self.build_task_row(task, &sender);
            widgets.active_list.append(&row);
        }

        while let Some(child) = widgets.history_list.first_child() {
            widgets.history_list.remove(&child);
        }

        for task in self.history_tasks() {
            let row = self.build_task_row(task, &sender);
            widgets.history_list.append(&row);
        }
    }
}

impl TaskHubModel {
    fn build_task_row(&self, task: &TaskEntry, sender: &ComponentSender<Self>) -> gtk::ListBoxRow {
        let failure_context = task.failure_context();
        let show_logs = !task.log_lines.is_empty()
            && matches!(task.status, TaskStatus::Active | TaskStatus::Error);
        let use_expander = show_logs || failure_context.is_some();

        let suffix = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(6)
            .valign(gtk::Align::Center)
            .build();

        if matches!(task.status, TaskStatus::Active) {
            if let Some(ref progress) = task.batch_progress {
                let progress_box = gtk::Box::builder()
                    .orientation(gtk::Orientation::Vertical)
                    .spacing(2)
                    .valign(gtk::Align::Center)
                    .build();

                let step_label = gtk::Label::builder()
                    .label(progress.step_display())
                    .css_classes(vec!["caption", "numeric"])
                    .build();

                let mini_progress = gtk::ProgressBar::builder()
                    .fraction(progress.fraction())
                    .width_request(60)
                    .build();
                mini_progress.add_css_class("batch-progress-mini");

                progress_box.append(&step_label);
                progress_box.append(&mini_progress);
                suffix.append(&progress_box);
            } else {
                let spinner = gtk::Spinner::builder()
                    .spinning(true)
                    .valign(gtk::Align::Center)
                    .css_classes(vec!["row-spinner"])
                    .build();
                suffix.append(&spinner);
            }
        }

        if task.retry_spec.is_some() && matches!(task.status, TaskStatus::Error) {
            let retry_btn = gtk::Button::builder()
                .icon_name("view-refresh-symbolic")
                .tooltip_text("Retry")
                .css_classes(vec!["flat", "circular"])
                .build();

            let task_id = task.id;
            let sender = sender.clone();
            retry_btn.connect_clicked(move |_| {
                sender.input(TaskHubInput::Retry(task_id));
            });
            suffix.append(&retry_btn);
        }

        if task.failure_context().is_some() {
            let copy_failure_btn = gtk::Button::builder()
                .icon_name("edit-copy-symbolic")
                .tooltip_text("Copy failure details")
                .css_classes(vec!["flat", "circular"])
                .build();

            let task_id = task.id;
            let sender = sender.clone();
            copy_failure_btn.connect_clicked(move |_| {
                sender.input(TaskHubInput::CopyFailureDetails(task_id));
            });
            suffix.append(&copy_failure_btn);
        }

        if task.command.is_some() {
            let copy_btn = gtk::Button::builder()
                .icon_name("edit-copy-symbolic")
                .tooltip_text("Copy command")
                .css_classes(vec!["flat", "circular"])
                .build();

            let task_id = task.id;
            let sender = sender.clone();
            copy_btn.connect_clicked(move |_| {
                sender.input(TaskHubInput::CopyCommand(task_id));
            });
            suffix.append(&copy_btn);
        }

        let wrapper = gtk::ListBoxRow::new();
        let title = escape_markup_text(&task.title);
        let subtitle = escape_markup_text(&task.subtitle());
        if use_expander {
            let expander = adw::ExpanderRow::builder()
                .title(title.as_str())
                .subtitle(subtitle.as_str())
                .css_classes(vec!["cmd-row"])
                .show_enable_switch(false)
                .build();
            expander.set_expanded(matches!(task.status, TaskStatus::Error));

            let exp_icon = gtk::Image::from_icon_name(task.icon_name());
            exp_icon.add_css_class("dim-label");
            expander.add_prefix(&exp_icon);
            expander.add_suffix(&suffix);

            if let Some((category, error_text)) = failure_context {
                let failure_row = build_expander_detail_row(
                    "Failure",
                    &format!("{} ({})", category.label(), category.code()),
                    &["caption"],
                );
                expander.add_row(&failure_row);

                let recovery_row = build_expander_detail_row(
                    "Recovery",
                    category.remediation_copy(),
                    &["dim-label"],
                );
                expander.add_row(&recovery_row);

                let next_step_row =
                    build_expander_detail_row("Next step", category.action_hint(), &["dim-label"]);
                expander.add_row(&next_step_row);

                if !error_text.trim().is_empty() {
                    let error_row = build_expander_detail_row(
                        "Provider error",
                        &error_text,
                        &["caption", "monospace"],
                    );
                    expander.add_row(&error_row);
                }
            }

            if show_logs {
                let log_row = build_log_row(task);
                expander.add_row(&log_row);
            }

            wrapper.set_child(Some(&expander));
            return wrapper;
        }

        let row = adw::ActionRow::builder()
            .title(title.as_str())
            .subtitle(subtitle.as_str())
            .css_classes(vec!["cmd-row"])
            .build();

        let icon = gtk::Image::from_icon_name(task.icon_name());
        icon.add_css_class("dim-label");
        row.add_prefix(&icon);
        row.add_suffix(&suffix);

        wrapper.set_child(Some(&row));
        wrapper
    }
}

fn build_expander_detail_row(title: &str, body: &str, body_css_classes: &[&str]) -> adw::ActionRow {
    let row = adw::ActionRow::builder()
        .title(title)
        .activatable(false)
        .selectable(false)
        .build();

    let label = gtk::Label::builder()
        .label(body)
        .xalign(0.0)
        .wrap(true)
        .build();
    for class_name in body_css_classes {
        label.add_css_class(class_name);
    }

    row.set_child(Some(&label));
    row
}

fn build_log_row(task: &TaskEntry) -> adw::ActionRow {
    let log_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(2)
        .margin_start(12)
        .margin_end(12)
        .margin_top(4)
        .margin_bottom(8)
        .build();

    for line in &task.log_lines {
        let log_label = gtk::Label::builder()
            .label(line)
            .xalign(0.0)
            .wrap(true)
            .css_classes(vec!["caption", "monospace", "dim-label"])
            .build();
        log_box.append(&log_label);
    }

    let log_scrolled = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .min_content_height(120)
        .max_content_height(220)
        .child(&log_box)
        .build();

    let vadj = log_scrolled.vadjustment();
    glib::idle_add_local_once(move || {
        let max = (vadj.upper() - vadj.page_size()).max(vadj.lower());
        vadj.set_value(max);
    });

    let log_row = adw::ActionRow::builder()
        .title("Recent logs")
        .activatable(false)
        .selectable(false)
        .build();
    log_row.set_child(Some(&log_scrolled));
    log_row
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failure_report_includes_recovery_and_recent_logs() {
        let task = TaskEntry {
            id: 1,
            title: "APT update failed".to_string(),
            details: "network timeout while fetching package lists".to_string(),
            timestamp: "12:34:56".to_string(),
            status: TaskStatus::Error,
            command: Some("apt update".to_string()),
            retry_spec: None,
            batch_progress: None,
            log_lines: vec!["Hit:1 repo".to_string(), "Err:2 timeout".to_string()],
        };

        let report = task.failure_report().expect("expected failure report");
        assert!(report.contains("Failure: Network"));
        assert!(report.contains("Command: apt update"));
        assert!(report.contains("Recovery:"));
        assert!(report.contains("Recent logs:"));
    }
}
