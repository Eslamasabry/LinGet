use crate::cli::tui::app::App;
use crate::cli::tui::state::filters::{Filter, Focus, ViewMode};
use crate::cli::tui::theme::{dim, header_bar, muted, palette, tab_active};
use crate::cli::tui::ui::{compose_left_right, spans_width};
use chrono::Local;
use once_cell::sync::Lazy;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::Span,
    widgets::Paragraph,
    Frame,
};
use std::fs;
use std::process::Command;

static SYSTEM_LABEL: Lazy<String> = Lazy::new(system_label);
static KERNEL_LABEL: Lazy<String> = Lazy::new(kernel_label);

pub fn draw_filter_bar(frame: &mut Frame, app: &App, area: Rect) {
    let mut left = header_brand_spans();
    for spec in primary_tab_specs(app) {
        left.extend(render_primary_tab(&spec.label, spec.active, app.searching));
        left.push(Span::raw("  "));
    }

    let right = vec![
        Span::styled("System: ", muted()),
        Span::styled(SYSTEM_LABEL.as_str(), dim()),
        Span::styled("   Kernel: ", muted()),
        Span::styled(KERNEL_LABEL.as_str(), dim()),
        Span::styled("   Uptime: ", muted()),
        Span::styled(uptime_label(), dim()),
        Span::styled("   ", dim()),
        Span::styled(Local::now().format("%H:%M").to_string(), dim()),
    ];

    let line = compose_left_right(left, right, area.width as usize);
    frame.render_widget(Paragraph::new(line).style(header_bar()), area);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeaderAction {
    Browse,
    Updates,
    Installed,
    Sources,
    Queue,
    Health,
}

struct PrimaryTabSpec {
    label: String,
    active: bool,
    action: HeaderAction,
}

fn header_brand_spans() -> Vec<Span<'static>> {
    vec![
        Span::styled(
            " LinGet ",
            Style::default()
                .fg(palette::CYAN())
                .bg(palette::HEADER_BG())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("   ", header_bar()),
    ]
}

fn primary_tab_specs(app: &App) -> Vec<PrimaryTabSpec> {
    let (queued, running, _completed, failed, _cancelled) = app.queue_counts();
    let queue_count = queued + running;
    let health_count = failed + app.filter_counts[4];
    vec![
        PrimaryTabSpec {
            label: format!("Browse {}", compact_count(app.filter_counts[0])),
            active: app.focus == Focus::Packages
                && app.filter == Filter::All
                && !app.queue_expanded
                && app.view_mode == ViewMode::Browse,
            action: HeaderAction::Browse,
        },
        PrimaryTabSpec {
            label: format!("Upd {}", compact_count(app.filter_counts[2])),
            active: app.filter == Filter::Updates && !app.queue_expanded,
            action: HeaderAction::Updates,
        },
        PrimaryTabSpec {
            label: format!("Inst {}", compact_count(app.filter_counts[1])),
            active: app.filter == Filter::Installed && !app.queue_expanded,
            action: HeaderAction::Installed,
        },
        PrimaryTabSpec {
            label: format!("Src {}", compact_count(app.visible_sources().len() + 1)),
            active: app.focus == Focus::Sources && !app.queue_expanded,
            action: HeaderAction::Sources,
        },
        PrimaryTabSpec {
            label: format!("Q {}", compact_count(queue_count)),
            active: app.queue_expanded,
            action: HeaderAction::Queue,
        },
        PrimaryTabSpec {
            label: format!("Health {}", compact_count(health_count)),
            active: app.view_mode == ViewMode::Dashboard && !app.queue_expanded,
            action: HeaderAction::Health,
        },
    ]
}

fn render_primary_tab(label: &str, active: bool, searching: bool) -> Vec<Span<'static>> {
    if active && !searching {
        vec![
            Span::styled("[ ", tab_active()),
            Span::styled(label.to_string(), tab_active()),
            Span::styled(" ]", tab_active()),
        ]
    } else {
        vec![
            Span::styled("[ ", header_bar()),
            Span::styled(
                label.to_string(),
                Style::default()
                    .fg(palette::DARK_GRAY())
                    .bg(palette::HEADER_BG()),
            ),
            Span::styled(" ]", header_bar()),
        ]
    }
}

fn compact_count(value: usize) -> String {
    if value >= 10_000 {
        format!("{:.1}k", value as f64 / 1000.0)
    } else {
        value.to_string()
    }
}

fn system_label() -> String {
    let Ok(contents) = fs::read_to_string("/etc/os-release") else {
        return std::env::consts::OS.to_string();
    };

    for key in ["PRETTY_NAME", "NAME"] {
        if let Some(value) = contents.lines().find_map(|line| {
            let (line_key, raw) = line.split_once('=')?;
            (line_key == key).then(|| raw.trim_matches('"').to_string())
        }) {
            return value;
        }
    }

    std::env::consts::OS.to_string()
}

fn kernel_label() -> String {
    Command::new("uname")
        .arg("-r")
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "unknown".to_string())
}

fn uptime_label() -> String {
    let seconds = fs::read_to_string("/proc/uptime")
        .ok()
        .and_then(|contents| {
            contents
                .split_whitespace()
                .next()
                .and_then(|value| value.parse::<f64>().ok())
        })
        .map(|value| value as u64)
        .unwrap_or(0);

    let days = seconds / 86_400;
    let hours = (seconds % 86_400) / 3_600;
    let minutes = (seconds % 3_600) / 60;
    if days > 0 {
        format!("{days}d {hours}h")
    } else if hours > 0 {
        format!("{hours}h {minutes}m")
    } else {
        format!("{minutes}m")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueHintAction {
    Retry,
    RetrySafe,
    Remediate,
}

pub fn header_action_hit_test(
    app: &App,
    header_filter_row: Rect,
    col: u16,
    row: u16,
) -> Option<HeaderAction> {
    if header_filter_row.width == 0 || header_filter_row.height == 0 || row != header_filter_row.y {
        return None;
    }
    if col < header_filter_row.x || col >= header_filter_row.x + header_filter_row.width {
        return None;
    }

    let mut cursor = header_filter_row.x + spans_width(&header_brand_spans()) as u16;
    let tabs = primary_tab_specs(app);

    for spec in tabs {
        let width =
            spans_width(&render_primary_tab(&spec.label, spec.active, app.searching)) as u16;
        if col >= cursor && col < cursor.saturating_add(width) {
            return Some(spec.action);
        }
        cursor = cursor.saturating_add(width).saturating_add(2);
    }

    None
}
