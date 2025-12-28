#![allow(dead_code)]

use gtk4::prelude::*;
use gtk4::{self as gtk};
use libadwaita as adw;
use libadwaita::prelude::*;

#[derive(Debug, Clone, Default)]
pub struct HealthData {
    pub score: u8,
    pub issues: Vec<HealthIssueData>,
    pub is_loading: bool,
}

#[derive(Debug, Clone)]
pub struct HealthIssueData {
    pub icon: &'static str,
    pub title: String,
    pub subtitle: String,
    pub severity: Severity,
    pub action_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Critical,
    Warning,
    Good,
}

#[derive(Debug, Clone)]
pub enum HealthAction {
    FixSecurityUpdates,
    FixPendingUpdates,
    CleanupSpace,
    RemoveOrphans,
    Refresh,
    ViewLockProcess(String),
}

pub fn build_health_dashboard<F>(data: &HealthData, on_action: F) -> gtk::Box
where
    F: Fn(HealthAction) + Clone + 'static,
{
    let container = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(24)
        .margin_top(24)
        .margin_bottom(24)
        .margin_start(24)
        .margin_end(24)
        .build();

    let score_widget = build_score_widget(data.score, data.is_loading);
    container.append(&score_widget);

    let issues_group = build_issues_group(&data.issues, on_action.clone());
    container.append(issues_group.upcast_ref::<gtk::Widget>());

    let refresh_group = build_refresh_section(on_action);
    container.append(refresh_group.upcast_ref::<gtk::Widget>());

    container
}

fn build_score_widget(score: u8, is_loading: bool) -> gtk::Box {
    let container = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(8)
        .halign(gtk::Align::Center)
        .margin_bottom(8)
        .build();

    let score_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .halign(gtk::Align::Center)
        .valign(gtk::Align::Center)
        .width_request(120)
        .height_request(120)
        .build();
    score_box.add_css_class("card");

    if is_loading {
        let spinner = gtk::Spinner::builder()
            .spinning(true)
            .width_request(48)
            .height_request(48)
            .halign(gtk::Align::Center)
            .valign(gtk::Align::Center)
            .margin_top(36)
            .build();
        score_box.append(&spinner);
    } else {
        let score_label = gtk::Label::builder()
            .label(format!("{}", score))
            .halign(gtk::Align::Center)
            .valign(gtk::Align::Center)
            .margin_top(24)
            .build();
        score_label.add_css_class("title-1");
        score_label.add_css_class(score_color_class(score));

        let percent_label = gtk::Label::builder()
            .label("%")
            .halign(gtk::Align::Center)
            .build();
        percent_label.add_css_class("dim-label");

        score_box.append(&score_label);
        score_box.append(&percent_label);
    }

    container.append(&score_box);

    let title_label = gtk::Label::builder()
        .label("System Health")
        .halign(gtk::Align::Center)
        .build();
    title_label.add_css_class("title-2");
    container.append(&title_label);

    let status_label = gtk::Label::builder()
        .label(if is_loading {
            "Analyzing..."
        } else {
            score_status_text(score)
        })
        .halign(gtk::Align::Center)
        .build();
    status_label.add_css_class("dim-label");
    if !is_loading {
        status_label.add_css_class(score_color_class(score));
    }
    container.append(&status_label);

    container
}

fn build_issues_group<F>(issues: &[HealthIssueData], on_action: F) -> adw::PreferencesGroup
where
    F: Fn(HealthAction) + Clone + 'static,
{
    let group = adw::PreferencesGroup::builder()
        .title("Health Status")
        .description("Issues and recommendations for your system")
        .build();

    if issues.is_empty() {
        let all_good_row = adw::ActionRow::builder()
            .title("Everything looks good!")
            .subtitle("No issues detected")
            .build();

        let check_icon = gtk::Image::builder()
            .icon_name("emblem-ok-symbolic")
            .build();
        check_icon.add_css_class("success");
        all_good_row.add_prefix(&check_icon);

        group.add(&all_good_row);
        return group;
    }

    let critical_count = issues
        .iter()
        .filter(|i| i.severity == Severity::Critical)
        .count();
    let warning_count = issues
        .iter()
        .filter(|i| i.severity == Severity::Warning)
        .count();
    let good_count = issues
        .iter()
        .filter(|i| i.severity == Severity::Good)
        .count();

    for issue in issues {
        let row = adw::ActionRow::builder()
            .title(&issue.title)
            .subtitle(&issue.subtitle)
            .build();

        let icon = gtk::Image::builder().icon_name(issue.icon).build();
        icon.add_css_class(severity_color_class(issue.severity));
        row.add_prefix(&icon);

        if issue.severity != Severity::Good {
            let action_btn = gtk::Button::builder().valign(gtk::Align::Center).build();

            let (label, action) = if issue.action_id.starts_with("lock_process:") {
                let holder = issue
                    .action_id
                    .strip_prefix("lock_process:")
                    .unwrap_or("")
                    .to_string();
                ("View Process", HealthAction::ViewLockProcess(holder))
            } else {
                match issue.action_id.as_str() {
                    "security_updates" => ("Update Now", HealthAction::FixSecurityUpdates),
                    "pending_updates" => ("Update", HealthAction::FixPendingUpdates),
                    "cleanup_space" => ("Clean Up", HealthAction::CleanupSpace),
                    "remove_orphans" => ("Remove", HealthAction::RemoveOrphans),
                    _ => ("Fix", HealthAction::Refresh),
                }
            };

            action_btn.set_label(label);

            if issue.severity == Severity::Critical {
                action_btn.add_css_class("destructive-action");
            } else {
                action_btn.add_css_class("suggested-action");
            }
            action_btn.add_css_class("pill");

            let on_action_clone = on_action.clone();
            action_btn.connect_clicked(move |_| {
                on_action_clone(action.clone());
            });

            row.add_suffix(&action_btn);
        } else {
            let check_icon = gtk::Image::builder()
                .icon_name("emblem-ok-symbolic")
                .valign(gtk::Align::Center)
                .build();
            check_icon.add_css_class("success");
            row.add_suffix(&check_icon);
        }

        group.add(&row);
    }

    if critical_count > 0 || warning_count > 0 || good_count > 0 {
        let summary_parts: Vec<String> = [
            (critical_count, "critical"),
            (warning_count, "warning"),
            (good_count, "good"),
        ]
        .iter()
        .filter(|(count, _)| *count > 0)
        .map(|(count, label)| format!("{} {}", count, label))
        .collect();

        if !summary_parts.is_empty() {
            group.set_description(Some(&format!("{} items", summary_parts.join(", "))));
        }
    }

    group
}

fn build_refresh_section<F>(on_action: F) -> adw::PreferencesGroup
where
    F: Fn(HealthAction) + Clone + 'static,
{
    let group = adw::PreferencesGroup::builder().build();

    let refresh_row = adw::ActionRow::builder()
        .title("Refresh Health Check")
        .subtitle("Re-scan your system for issues")
        .activatable(true)
        .build();

    let refresh_icon = gtk::Image::builder()
        .icon_name("view-refresh-symbolic")
        .build();
    refresh_icon.add_css_class("dim-label");
    refresh_row.add_prefix(&refresh_icon);

    let arrow_icon = gtk::Image::builder()
        .icon_name("go-next-symbolic")
        .valign(gtk::Align::Center)
        .build();
    arrow_icon.add_css_class("dim-label");
    refresh_row.add_suffix(&arrow_icon);

    refresh_row.connect_activated(move |_| {
        on_action(HealthAction::Refresh);
    });

    group.add(&refresh_row);

    group
}

fn score_color_class(score: u8) -> &'static str {
    match score {
        80..=100 => "success",
        50..=79 => "warning",
        _ => "error",
    }
}

fn severity_color_class(severity: Severity) -> &'static str {
    match severity {
        Severity::Critical => "error",
        Severity::Warning => "warning",
        Severity::Good => "success",
    }
}

fn score_status_text(score: u8) -> &'static str {
    match score {
        90..=100 => "Excellent",
        80..=89 => "Good",
        60..=79 => "Fair",
        40..=59 => "Needs Attention",
        _ => "Critical",
    }
}
