//! Single-view workspace layout.
//!
//! Replaces the legacy F1/F2/F3 mode dispatch with one attention-first view:
//!
//! ```text
//! ┌ header band ──────────────────────────────────────┐
//! │ attention blocks (hero cards)                     │
//! │ ─────────────────────────────────── APT 48 ─┐    │
//! │   /search or :command                       │    │
//! │   package list rows…                        │    │
//! │ ────────────────────────────────────────────┘    │
//! │ details one-liner (i to expand)                   │
//! │ footer keys (6)                                   │
//! └───────────────────────────────────────────────────┘
//! ```
//!
//! Overlays (palette, help, preflight, changelog, import, queue-expanded)
//! are drawn on top in `ui::draw` and are unchanged by this module.

use super::attention;
use super::packages::draw_packages_panel;
use super::source_rail;
use crate::cli::tui::app::App;
use crate::cli::tui::format::{format_package_version, truncate_to_width};
use crate::cli::tui::state::filters::Focus;
use crate::cli::tui::theme::{
    accent, dim, key_hint, muted, palette, source_color, success, text, warning,
};
use crate::models::PackageStatus;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

pub fn draw(frame: &mut Frame, app: &mut App, area: Rect) {
    // Split vertically: attention · list(with right-rail) · details · footer
    //
    // The attention section is sized dynamically (up to ~40% of area) based
    // on the number of blocks. Footer is always 1 line, details always 2.
    let blocks = attention::build_blocks(app);
    let attention_height = attention::desired_height(&blocks)
        .min(area.height.saturating_sub(8)) // leave room for list+details+footer
        .min(area.height / 2);

    let details_height: u16 = 2;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(attention_height),
            Constraint::Min(5),
            Constraint::Length(details_height),
        ])
        .split(area);

    if attention_height > 0 {
        attention::draw(frame, &blocks, chunks[0]);
    }

    draw_list_region(frame, app, chunks[1]);
    draw_details_strip(frame, app, chunks[2]);
}

fn draw_list_region(frame: &mut Frame, app: &mut App, area: Rect) {
    if app.focus == Focus::Sources {
        if let Some(source) = app.source {
            app.load_repositories(source);
        }
    }

    // Split horizontally: source rail · list · inspector.
    let rail_width = if area.width > 58 {
        source_rail::RAIL_WIDTH
    } else {
        0
    };
    let inspector_width = if area.width > 118 { 34 } else { 0 };
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(rail_width),
            Constraint::Length(if rail_width > 0 { 1 } else { 0 }), // gutter
            Constraint::Min(20),
            Constraint::Length(if inspector_width > 0 { 1 } else { 0 }), // gutter
            Constraint::Length(inspector_width),
        ])
        .split(area);

    // Above the list: the search/command combo input (one line) + divider.
    let list_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // search/command bar
            Constraint::Length(1), // divider
            Constraint::Min(1),    // table
        ])
        .split(columns[2]);

    draw_search_combo(frame, app, list_chunks[0]);
    draw_divider(frame, app, list_chunks[1]);
    // Re-use existing packages panel (will be refined in a later phase).
    draw_packages_panel(frame, app, list_chunks[2], true);

    if rail_width > 0 {
        source_rail::draw(frame, app, columns[0]);
    }
    if inspector_width > 0 {
        draw_inspector(frame, app, columns[4]);
    }
}

fn draw_search_combo(frame: &mut Frame, app: &App, area: Rect) {
    let mut spans: Vec<Span> = Vec::new();
    let placeholder_style = dim().add_modifier(Modifier::ITALIC);
    spans.push(Span::styled(" ", text()));
    if app.searching {
        spans.push(Span::styled("/ ", accent().add_modifier(Modifier::BOLD)));
        spans.push(Span::styled(app.search.clone(), text()));
        spans.push(Span::styled("▌", accent().add_modifier(Modifier::BOLD)));
    } else if !app.search.is_empty() {
        spans.push(Span::styled("/ ", muted()));
        spans.push(Span::styled(app.search.clone(), text()));
        spans.push(Span::styled("  (Esc clears)", dim()));
    } else {
        spans.push(Span::styled("/ search", placeholder_style));
        spans.push(Span::styled("   ", dim()));
        spans.push(Span::styled(": command", placeholder_style));
        spans.push(Span::styled("   ", dim()));
        spans.push(Span::styled("? help", placeholder_style));
    }
    let left = Line::from(spans);
    frame.render_widget(Paragraph::new(left), area);
}

fn draw_divider(frame: &mut Frame, _app: &App, area: Rect) {
    if area.width == 0 {
        return;
    }
    let line: String = "─".repeat(area.width as usize);
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            line,
            Style::default().fg(palette::DARK_GRAY()),
        ))),
        area,
    );
}

fn draw_details_strip(frame: &mut Frame, app: &App, area: Rect) {
    if area.height == 0 {
        return;
    }
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(area);
    // Top row: divider
    let line: String = "─".repeat(area.width as usize);
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            line,
            Style::default().fg(palette::DARK_GRAY()),
        ))),
        rows[0],
    );
    draw_status_strip(frame, app, rows[1]);
}

fn draw_inspector(frame: &mut Frame, app: &App, area: Rect) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    let mut lines = Vec::new();
    lines.push(Line::from(vec![
        Span::styled("Inspector", text().add_modifier(Modifier::BOLD)),
        Span::styled("  package context", dim()),
    ]));
    lines.push(Line::from(Span::styled(
        "─".repeat(area.width as usize),
        Style::default().fg(palette::DARK_GRAY()),
    )));

    let Some(package) = app.current_package() else {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled("No package selected", dim())));
        lines.push(Line::from(Span::styled(
            "Move through the table to preview actions here.",
            muted(),
        )));
        frame.render_widget(Paragraph::new(lines), area);
        return;
    };

    let name_width = area.width.saturating_sub(2) as usize;
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        truncate_to_width(&package.name, name_width),
        accent().add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(vec![
        Span::styled(package.source.to_string(), source_color(package.source)),
        Span::styled("  ", dim()),
        Span::styled(format_package_version(package), muted()),
    ]));
    lines.push(Line::from(""));

    let status = match package.status {
        PackageStatus::UpdateAvailable => ("Update available", warning()),
        PackageStatus::Installed => ("Installed", success()),
        PackageStatus::NotInstalled => ("Available", muted()),
        PackageStatus::Installing => ("Installing", warning()),
        PackageStatus::Updating => ("Updating", warning()),
        PackageStatus::Removing => ("Removing", warning()),
    };
    lines.push(fact_line("Status", status.0, status.1));
    lines.push(fact_line(
        "Action",
        package_action_label(package.status),
        key_hint(),
    ));
    if !package.dependencies.is_empty() {
        lines.push(fact_line(
            "Deps",
            &format!("{} known", package.dependencies.len()),
            muted(),
        ));
    }
    if let Some(homepage) = package.homepage.as_deref() {
        lines.push(fact_line(
            "Home",
            &truncate_to_width(homepage, name_width.saturating_sub(6)),
            muted(),
        ));
    }

    let description = package.description.trim();
    if !description.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled("Summary", dim())));
        lines.push(Line::from(Span::styled(
            truncate_to_width(description, name_width),
            text(),
        )));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("[Enter]", key_hint()),
        Span::styled(" act  ", muted()),
        Span::styled("[c]", key_hint()),
        Span::styled(" notes  ", muted()),
        Span::styled("[Space]", key_hint()),
        Span::styled(" select", muted()),
    ]));

    frame.render_widget(Paragraph::new(lines), area);
}

fn draw_status_strip(frame: &mut Frame, app: &App, area: Rect) {
    let Some(package) = app.current_package() else {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled("  No package selected", dim()))),
            area,
        );
        return;
    };

    let text_value = format!(
        "  {} · {} · {} · {} selected",
        package.name,
        package.source,
        package_action_label(package.status),
        app.selected.len()
    );
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            truncate_to_width(&text_value, area.width as usize),
            muted(),
        ))),
        area,
    );
}

fn fact_line(label: &str, value: &str, style: Style) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{:<7}", label), dim()),
        Span::styled(value.to_string(), style),
    ])
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
