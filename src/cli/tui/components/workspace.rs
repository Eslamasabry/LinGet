//! Screenshot-matched package workspace.
//!
//! The main TUI is intentionally one dense operator surface: a Today strip,
//! filter/search tabs, source rail, package table, and selected-package
//! inspector are all visible at once.

use super::attention;
use super::packages::draw_packages_panel;
use super::source_rail;
use crate::cli::tui::app::App;
use crate::cli::tui::format::truncate_to_width;
use crate::cli::tui::state::filters::{Filter, Focus};
use crate::cli::tui::theme::{
    accent, border_focused, border_unfocused, dim, muted, palette, row_cursor, success, text,
    warning, ROUNDED,
};
use crate::models::{Package, PackageStatus, UpdateCategory};
use ratatui::{
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

pub const ATTENTION_HEIGHT: u16 = 3;
pub const FILTER_PANEL_HEIGHT: u16 = 6;
pub const WORKSPACE_GAP: u16 = 1;

pub fn draw(frame: &mut Frame, app: &mut App, area: Rect) {
    if app.focus == Focus::Sources {
        if let Some(source) = app.source {
            app.load_repositories(source);
        }
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(ATTENTION_HEIGHT),
            Constraint::Length(WORKSPACE_GAP),
            Constraint::Length(FILTER_PANEL_HEIGHT),
            Constraint::Length(WORKSPACE_GAP),
            Constraint::Min(8),
        ])
        .split(area);

    attention::draw(frame, app, chunks[0]);
    draw_filter_panel(frame, app, chunks[2]);
    draw_main_columns(frame, app, chunks[4]);
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
    if row != inner.y || col < inner.x || col >= inner.x + inner.width {
        return None;
    }

    let mut cursor = inner.x;
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
        .constraints([Constraint::Length(1), Constraint::Length(3)])
        .split(inner);

    let mut tab_spans = Vec::new();
    for (index, spec) in filter_tab_specs(app).iter().enumerate() {
        if index > 0 {
            tab_spans.push(Span::raw("  "));
        }
        tab_spans.extend(filter_tab_spans(
            spec.label,
            spec.count,
            app.filter == spec.filter,
            app.searching,
        ));
    }
    frame.render_widget(Paragraph::new(Line::from(tab_spans)), rows[0]);

    let search_block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_unfocused());
    let search_inner = search_block.inner(rows[1]);
    frame.render_widget(search_block, rows[1]);

    let mut search_spans = Vec::new();
    if app.searching {
        search_spans.push(Span::styled("/ ", accent()));
        search_spans.push(Span::styled(app.search.clone(), text()));
        search_spans.push(Span::styled("▌", accent()));
    } else if app.search.is_empty() {
        search_spans.push(Span::styled(
            "/ Search packages (regex supported)",
            dim().add_modifier(Modifier::ITALIC),
        ));
    } else {
        search_spans.push(Span::styled("/ ", dim()));
        search_spans.push(Span::styled(app.search.clone(), text()));
        search_spans.push(Span::styled("  Esc clears", dim()));
    }
    frame.render_widget(Paragraph::new(Line::from(search_spans)), search_inner);
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
    push_fact_line(&mut lines, "Source", &package.source.to_string(), muted());
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

    lines.push(separator_line(content_width));
    lines.push(Line::from(Span::styled("Dependencies", accent())));
    if package.dependencies.is_empty() {
        lines.push(Line::from(Span::styled("• none listed", dim())));
    } else {
        for dep in package.dependencies.iter().take(3) {
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

struct FilterTabSpec {
    label: &'static str,
    count: usize,
    filter: Filter,
}

fn filter_tab_specs(app: &App) -> [FilterTabSpec; 6] {
    [
        FilterTabSpec {
            label: "All",
            count: app.filter_counts[0],
            filter: Filter::All,
        },
        FilterTabSpec {
            label: "Installed",
            count: app.filter_counts[1],
            filter: Filter::Installed,
        },
        FilterTabSpec {
            label: "Updates",
            count: app.filter_counts[2],
            filter: Filter::Updates,
        },
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
