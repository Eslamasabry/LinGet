//! Dense package workspace.
//!
//! The main TUI is intentionally one operator surface: a compact status/search
//! strip, source rail, package table, and selected-package inspector are all
//! visible at once.

use super::packages::draw_packages_panel;
use super::source_rail;
use crate::cli::tui::app::App;
use crate::cli::tui::format::truncate_to_width;
use crate::cli::tui::state::filters::{Filter, Focus};
use crate::cli::tui::theme::{
    accent, border_focused, border_unfocused, dim, error, muted, palette, row_cursor, success,
    text, warning, ROUNDED,
};
use crate::models::{Package, PackageStatus, UpdateCategory};
use ratatui::{
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

pub const FILTER_PANEL_HEIGHT: u16 = 4;
pub const WORKSPACE_GAP: u16 = 0;

pub fn draw(frame: &mut Frame, app: &mut App, area: Rect) {
    if app.focus == Focus::Sources {
        if let Some(source) = app.source {
            app.load_repositories(source);
        }
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(FILTER_PANEL_HEIGHT),
            Constraint::Length(WORKSPACE_GAP),
            Constraint::Min(8),
        ])
        .split(area);

    draw_filter_panel(frame, app, chunks[0]);
    draw_main_columns(frame, app, chunks[2]);
}

pub fn filter_panel_hit_test(
    app: &App,
    filter_panel_rect: Rect,
    col: u16,
    row: u16,
) -> Option<Filter> {
    if filter_panel_rect.width <= 2 || filter_panel_rect.height <= 2 {
        return None;
    }

    let inner = filter_panel_rect.inner(Margin {
        vertical: 1,
        horizontal: 1,
    });
    let controls_row = inner.y.saturating_add(1);
    if row != controls_row || col < inner.x || col >= inner.x + inner.width {
        return None;
    }

    let mut cursor = inner.x.saturating_add(1);
    for spec in filter_tab_specs(app) {
        let width = spans_width(&filter_tab_spans(
            spec.label,
            spec.count,
            app.filter == spec.filter,
            app.searching,
        )) as u16;
        if col >= cursor && col < cursor.saturating_add(width) {
            return Some(spec.filter);
        }
        cursor = cursor.saturating_add(width).saturating_add(2);
    }

    None
}

pub fn filter_panel_search_hit_test(filter_panel_rect: Rect, col: u16, row: u16) -> bool {
    if filter_panel_rect.width <= 2 || filter_panel_rect.height <= 2 {
        return false;
    }

    let inner = filter_panel_rect.inner(Margin {
        vertical: 1,
        horizontal: 1,
    });
    row == inner.y.saturating_add(1) && col >= inner.x && col < inner.x + inner.width
}

fn draw_filter_panel(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_set(ROUNDED)
        .border_style(border_unfocused());
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(inner);

    frame.render_widget(
        Paragraph::new(top_status_line(app, inner.width as usize)),
        rows[0],
    );

    let mut control_spans = vec![Span::raw(" ")];
    for (index, spec) in filter_tab_specs(app).iter().enumerate() {
        if index > 0 {
            control_spans.push(Span::raw("  "));
        }
        control_spans.extend(filter_tab_spans(
            spec.label,
            spec.count,
            app.filter == spec.filter,
            app.searching,
        ));
    }
    control_spans.push(Span::styled("   |   ", dim()));
    if app.searching {
        control_spans.push(Span::styled("/ ", accent()));
        control_spans.push(Span::styled(app.search.clone(), text()));
        control_spans.push(Span::styled("▌", accent()));
    } else if app.search.is_empty() {
        control_spans.push(Span::styled(
            "/ Search packages (regex supported)",
            dim().add_modifier(Modifier::ITALIC),
        ));
    } else {
        control_spans.push(Span::styled("/ ", dim()));
        control_spans.push(Span::styled(app.search.clone(), text()));
        control_spans.push(Span::styled("  Esc clears", dim()));
    }
    frame.render_widget(Paragraph::new(Line::from(control_spans)), rows[1]);
}

fn top_status_line(app: &App, width: usize) -> Line<'static> {
    let security = app.filter_counts[4];
    let safe_updates = app.filter_counts[2].saturating_sub(security);
    let (_, _, _, failed, _) = app.queue_counts();
    let (tail_label, tail_value, tail_style) = top_status_tail(app, width);
    Line::from(vec![
        Span::styled(" Today: ", accent()),
        Span::styled("🔒", error()),
        Span::styled(" Security ", muted()),
        Span::styled(security.to_string(), error()),
        Span::styled("   |   ", dim()),
        Span::styled("↻", Style::default().fg(palette::ORANGE())),
        Span::styled(" Safe updates ", muted()),
        Span::styled(
            safe_updates.to_string(),
            Style::default().fg(palette::ORANGE()),
        ),
        Span::styled("   |   ", dim()),
        Span::styled("✖", error()),
        Span::styled(" Failed tasks ", muted()),
        Span::styled(failed.to_string(), error()),
        Span::styled("   |   ", dim()),
        Span::styled(tail_label, tail_style.add_modifier(Modifier::BOLD)),
        Span::styled(tail_value, text()),
    ])
}

fn top_status_tail(app: &App, width: usize) -> (&'static str, String, Style) {
    if app.is_catalog_busy() {
        return (
            "Loading: ",
            truncate_to_width(&app.catalog_loading_message(), width.saturating_sub(82)),
            accent(),
        );
    }

    ("Recommended: ", recommended_text(app, width), success())
}

fn recommended_text(app: &App, width: usize) -> String {
    let label = if let Some(package) = app.current_package() {
        if package.status == PackageStatus::UpdateAvailable {
            format!("Review {} update", package.name)
        } else {
            app.recommended_action_label()
        }
    } else {
        app.recommended_action_label()
    };
    truncate_to_width(&label, width.saturating_sub(86))
}

fn draw_main_columns(frame: &mut Frame, app: &App, area: Rect) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    let source_width = if area.width >= 92 {
        source_rail::RAIL_WIDTH
    } else {
        0
    };
    let inspector_width = if area.width >= 142 {
        49
    } else if area.width >= 118 {
        42
    } else {
        0
    };

    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(source_width),
            Constraint::Length(if source_width > 0 { 1 } else { 0 }),
            Constraint::Min(36),
            Constraint::Length(if inspector_width > 0 { 1 } else { 0 }),
            Constraint::Length(inspector_width),
        ])
        .split(area);

    if source_width > 0 {
        source_rail::draw(frame, app, columns[0]);
    }

    draw_packages_panel(frame, app, columns[2], true);

    if inspector_width > 0 {
        draw_inspector(frame, app, columns[4]);
    }
}

fn draw_inspector(frame: &mut Frame, app: &App, area: Rect) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    let focused = app.focus == Focus::Packages && !app.queue_expanded;
    let block = Block::default()
        .borders(Borders::ALL)
        .border_set(ROUNDED)
        .border_style(if focused {
            border_focused()
        } else {
            border_unfocused()
        })
        .title(" Package Inspector ")
        .title_style(accent());
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let chunks = if inner.height >= 4 {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(inner)
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(0)])
            .split(inner)
    };

    let Some(package) = app.current_package() else {
        let lines = vec![
            Line::from(Span::styled("No package selected", muted())),
            Line::from(Span::styled(
                "Move through the table to preview actions here.",
                dim(),
            )),
        ];
        frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: true }), chunks[0]);
        return;
    };

    let content_width = chunks[0].width as usize;
    let mut lines = Vec::new();
    lines.push(Line::from(Span::styled(
        truncate_to_width(&package.name, content_width),
        text().add_modifier(Modifier::BOLD),
    )));
    lines.push(separator_line(content_width));
    push_fact_line(&mut lines, "Source", &source_detail_label(package), muted());
    lines.push(version_line(package));
    push_fact_line(
        &mut lines,
        "Status",
        status_label(package.status),
        warning(),
    );
    let (risk, risk_style) = risk_label(package);
    push_fact_line(&mut lines, "Risk", risk, risk_style);
    push_fact_line(
        &mut lines,
        "Installed",
        package.install_date.as_deref().unwrap_or("unknown"),
        muted(),
    );
    push_fact_line(&mut lines, "Size", &package.size_display(), muted());
    push_fact_line(
        &mut lines,
        "License",
        package.license.as_deref().unwrap_or("unknown"),
        muted(),
    );

    lines.push(separator_line(content_width));
    lines.push(Line::from(Span::styled("Recommended action", success())));
    lines.push(Line::from(Span::styled(
        package_action_label(package.status),
        text(),
    )));
    lines.push(Line::from(Span::styled(
        recommendation_detail(package),
        success(),
    )));

    lines.push(separator_line(content_width));
    lines.push(Line::from(Span::styled(
        release_notes_title(package),
        accent(),
    )));
    lines.push(Line::from(vec![
        Span::styled("• ", dim()),
        Span::styled("See full changelog online.", muted()),
    ]));

    let summary = package_summary(package);
    if !summary.is_empty() {
        lines.push(separator_line(content_width));
        lines.push(Line::from(Span::styled("Summary", accent())));
        lines.push(Line::from(Span::styled(
            truncate_to_width(&summary, content_width),
            muted(),
        )));
    }

    lines.push(separator_line(content_width));
    lines.push(Line::from(Span::styled("Dependencies", accent())));
    if package.dependencies.is_empty() {
        lines.push(Line::from(Span::styled("• none listed", dim())));
    } else {
        let remaining = chunks[0]
            .height
            .saturating_sub(u16::try_from(lines.len()).unwrap_or(u16::MAX))
            .saturating_sub(1) as usize;
        for dep in package.dependencies.iter().take(remaining.clamp(3, 8)) {
            lines.push(Line::from(vec![
                Span::styled("• ", dim()),
                Span::styled(
                    truncate_to_width(dep, content_width.saturating_sub(2)),
                    muted(),
                ),
            ]));
        }
    }

    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: true }), chunks[0]);

    if chunks[1].height > 0 {
        let mut buttons = Vec::new();
        inspector_button(&mut buttons, "Enter", package_action_label(package.status));
        buttons.push(Span::raw("  "));
        inspector_button(&mut buttons, "C", "Notes");
        buttons.push(Span::raw("  "));
        inspector_button(&mut buttons, "Space", "Select");
        frame.render_widget(Paragraph::new(Line::from(buttons)), chunks[1]);
    }
}

fn package_summary(package: &Package) -> String {
    package
        .enrichment
        .as_ref()
        .and_then(|enrichment| enrichment.summary.as_deref())
        .filter(|summary| !summary.trim().is_empty())
        .unwrap_or(&package.description)
        .trim()
        .to_string()
}

struct FilterTabSpec {
    label: &'static str,
    count: usize,
    filter: Filter,
}

fn filter_tab_specs(app: &App) -> [FilterTabSpec; 3] {
    [
        FilterTabSpec {
            label: "Favorites",
            count: app.filter_counts[3],
            filter: Filter::Favorites,
        },
        FilterTabSpec {
            label: "Security",
            count: app.filter_counts[4],
            filter: Filter::SecurityUpdates,
        },
        FilterTabSpec {
            label: "Duplicates",
            count: app.filter_counts[5],
            filter: Filter::Duplicates,
        },
    ]
}

fn filter_tab_spans(
    label: &str,
    count: usize,
    active: bool,
    searching: bool,
) -> Vec<Span<'static>> {
    let style = if active && !searching {
        row_cursor()
    } else {
        dim()
    };
    vec![
        Span::styled("[ ", style),
        Span::styled(label.to_string(), style),
        Span::styled(" ", style),
        Span::styled(count.to_string(), style),
        Span::styled(" ]", style),
    ]
}

fn spans_width(spans: &[Span<'_>]) -> usize {
    spans.iter().map(|span| span.content.chars().count()).sum()
}

fn separator_line(width: usize) -> Line<'static> {
    Line::from(Span::styled(
        "─".repeat(width),
        Style::default().fg(palette::DARK_GRAY()),
    ))
}

fn push_fact_line(lines: &mut Vec<Line<'static>>, label: &str, value: &str, style: Style) {
    lines.push(Line::from(vec![
        Span::styled(format!("{:<13}", label), dim()),
        Span::styled(value.to_string(), style),
    ]));
}

fn version_line(package: &Package) -> Line<'static> {
    let available = package.available_version.as_deref().unwrap_or("-");
    let version_width = 14;
    Line::from(vec![
        Span::styled(format!("{:<13}", "Version"), dim()),
        Span::styled(truncate_to_width(&package.version, version_width), muted()),
        Span::styled("  →  ", dim()),
        Span::styled(
            truncate_to_width(available, version_width),
            Style::default().fg(palette::ORANGE()),
        ),
    ])
}

fn status_label(status: PackageStatus) -> &'static str {
    match status {
        PackageStatus::UpdateAvailable => "update available",
        PackageStatus::Installed => "installed",
        PackageStatus::NotInstalled => "available",
        PackageStatus::Installing => "installing",
        PackageStatus::Updating => "updating",
        PackageStatus::Removing => "removing",
    }
}

fn source_detail_label(package: &Package) -> String {
    package.source.to_string().to_lowercase()
}

fn risk_label(package: &Package) -> (&'static str, Style) {
    match package
        .update_category
        .unwrap_or_else(|| package.detect_update_category())
    {
        UpdateCategory::Security => ("security review", warning()),
        _ if package.status == PackageStatus::UpdateAvailable => ("routine update", success()),
        _ => ("routine", success()),
    }
}

fn package_action_label(status: PackageStatus) -> &'static str {
    match status {
        PackageStatus::UpdateAvailable => "Queue update",
        PackageStatus::Installed => "Remove",
        PackageStatus::NotInstalled => "Install",
        PackageStatus::Installing => "Installing",
        PackageStatus::Updating => "Updating",
        PackageStatus::Removing => "Removing",
    }
}

fn recommendation_detail(package: &Package) -> &'static str {
    if package.status == PackageStatus::UpdateAvailable {
        "This update is safe to apply."
    } else {
        "No update action is required."
    }
}

fn release_notes_title(package: &Package) -> String {
    match package.available_version.as_deref() {
        Some(version) if package.status == PackageStatus::UpdateAvailable => {
            truncate_to_width(&format!("Release notes (v{version})"), 46)
        }
        _ => "Release notes".to_string(),
    }
}

fn inspector_button(spans: &mut Vec<Span<'static>>, key: &'static str, label: &'static str) {
    spans.push(Span::styled(
        format!("[{}]", key),
        Style::default()
            .fg(palette::WHITE())
            .bg(palette::TAB_ACTIVE_BG())
            .add_modifier(Modifier::BOLD),
    ));
    spans.push(Span::raw(" "));
    spans.push(Span::styled(label.to_string(), muted()));
}
