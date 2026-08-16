//! Rendering for the reimagined TUI.
//!
//! Visual law:
//! - No boxes around panels. Whitespace and one-character rules separate.
//! - Reverse video means "cursor is here" and nothing else.
//! - Red = security/failure, amber = updates, green = healthy, dim = metadata.
//! - One accent (cyan) appears only in the ambient brand and the search field.

use crate::cli::tui_next::app::{
    App, Filter, LoadPhase, Overlay, Row, DOCK_WIDTH, MIN_HEIGHT, MIN_WIDTH,
};
use crate::cli::tui_next::palette;
use crate::models::history::{TaskQueueEntry, TaskQueueStatus};
use crate::models::{Package, PackageStatus, UpdateCategory};
use chrono::Local;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
    Frame,
};
use unicode_width::UnicodeWidthStr;

// ---------------------------------------------------------------------
// Palette (single source of truth)
// ---------------------------------------------------------------------

const FG: Color = Color::Rgb(215, 218, 224);
const DIM: Color = Color::Rgb(122, 128, 138);
const FAINT: Color = Color::Rgb(70, 74, 82);
const ACCENT: Color = Color::Rgb(94, 200, 220);
const AMBER: Color = Color::Rgb(230, 180, 80);
const RED: Color = Color::Rgb(224, 90, 90);
const GREEN: Color = Color::Rgb(120, 190, 120);

fn fg() -> Style {
    Style::default().fg(FG)
}
fn dim() -> Style {
    Style::default().fg(DIM)
}
fn faint() -> Style {
    Style::default().fg(FAINT)
}
fn accent() -> Style {
    Style::default().fg(ACCENT)
}
fn amber() -> Style {
    Style::default().fg(AMBER)
}
fn red() -> Style {
    Style::default().fg(RED)
}
fn green() -> Style {
    Style::default().fg(GREEN)
}
fn cursor_style() -> Style {
    Style::default().add_modifier(Modifier::REVERSED)
}
fn bold() -> Style {
    fg().add_modifier(Modifier::BOLD)
}

// ---------------------------------------------------------------------
// Frame
// ---------------------------------------------------------------------

pub fn draw(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
        frame.render_widget(
            Paragraph::new(vec![
                Line::from(Span::styled("terminal too small", red())),
                Line::from(Span::styled(
                    format!("need at least {MIN_WIDTH}x{MIN_HEIGHT}"),
                    dim(),
                )),
            ]),
            area,
        );
        return;
    }

    let queue_peek = queue_peek_height(app);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // ambient
            Constraint::Length(1), // rule
            Constraint::Min(4),    // list
            Constraint::Length(queue_peek),
            Constraint::Length(1), // command bar
        ])
        .split(area);

    draw_ambient(frame, app, chunks[0]);
    draw_rule(frame, chunks[1]);

    let list_area = if app.queue_open {
        let split = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
            .split(chunks[2]);
        draw_queue_drawer(frame, app, split[1]);
        split[0]
    } else {
        chunks[2]
    };

    draw_list(frame, app, list_area);

    if queue_peek > 0 {
        draw_queue_peek(frame, app, chunks[3]);
    }
    draw_command_bar(frame, app, chunks[4]);

    match &app.overlay {
        Some(Overlay::Palette { query }) => draw_palette(frame, app, query),
        Some(Overlay::Help) => draw_help(frame),
        Some(Overlay::Confirm { title, body, .. }) => {
            draw_confirm(frame, title, body);
        }
        Some(Overlay::Changelog {
            title,
            lines,
            scroll,
        }) => {
            draw_changelog(frame, title, lines, *scroll);
        }
        None => {}
    }
}

// ---------------------------------------------------------------------
// Ambient line
// ---------------------------------------------------------------------

fn draw_ambient(frame: &mut Frame, app: &App, area: Rect) {
    let (updates, security, _) = app.counts();
    let (queued, running, failed, _) = app.queue_counts();

    let mut left = vec![
        Span::styled(" ◆ ", accent().add_modifier(Modifier::BOLD)),
        Span::styled("linget", bold()),
        Span::styled(format!("  ·  {}", app.filter.label()), faint()),
        Span::styled("   ", fg()),
    ];

    if app.is_loading() || app.refreshing {
        let verb = if app.rows.is_empty() {
            "loading"
        } else {
            "refreshing"
        };
        let mut text = format!("{} {verb}…", app.spinner_frame());
        if let Some(saved_at) = &app.cached_at {
            text.push_str(&format!(" (cached {})", age_text(*saved_at)));
        }
        left.push(Span::styled(text, amber()));
    } else {
        if updates > 0 {
            left.push(Span::styled(format!("↑ {updates}"), amber()));
            left.push(Span::styled("  ", fg()));
        }
        if security > 0 {
            left.push(Span::styled(format!("⚠ {security}"), red()));
            left.push(Span::styled("  ", fg()));
        }
        if failed > 0 {
            left.push(Span::styled(format!("✗ {failed}"), red()));
            left.push(Span::styled("  ", fg()));
        }
        if queued + running > 0 {
            left.push(Span::styled(
                format!(
                    "{} queue {}/{}",
                    app.spinner_frame(),
                    running,
                    queued + running
                ),
                dim(),
            ));
        }
        if updates == 0 && security == 0 && failed == 0 {
            left.push(Span::styled("✓ up to date", green()));
        }
    }

    // Search field lives here, always visible, right-aligned.
    let width = area.width as usize;
    let search_width = 26.min(width.saturating_sub(40));
    let right: Vec<Span> = if app.search.focused {
        let text = format!("╱ {}█", app.search.text);
        vec![Span::styled(pad_left(&text, search_width), accent())]
    } else if app.search.text.is_empty() {
        vec![Span::styled(pad_left("╱ search", search_width), faint())]
    } else {
        vec![Span::styled(
            pad_left(&format!("╱ {}", app.search.text), search_width),
            dim(),
        )]
    };

    let left_width: usize = left
        .iter()
        .map(|s| UnicodeWidthStr::width(s.content.as_ref()))
        .sum();
    let gap = width.saturating_sub(left_width + search_width);
    let mut spans = left;
    spans.push(Span::raw(" ".repeat(gap)));
    spans.extend(right);
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn pad_left(text: &str, width: usize) -> String {
    let used = UnicodeWidthStr::width(text);
    if used >= width {
        text.chars().take(width).collect()
    } else {
        format!("{}{}", " ".repeat(width - used), text)
    }
}

fn draw_rule(frame: &mut Frame, area: Rect) {
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "─".repeat(area.width as usize),
            faint(),
        ))),
        area,
    );
}

// ---------------------------------------------------------------------
// The list
// ---------------------------------------------------------------------

fn draw_list(frame: &mut Frame, app: &mut App, area: Rect) {
    if area.height == 0 {
        return;
    }

    // Loading skeleton: the list's future shape, shimmering in.
    if app.is_loading() && app.rows.is_empty() {
        draw_skeleton(frame, app, area);
        return;
    }

    if app.rows.is_empty() {
        draw_empty(frame, app, area);
        return;
    }

    // Wide terminals: expansion docks to the right instead of inline.
    let dock = app.expanded.is_some() && area.width >= DOCK_WIDTH;
    let (list_area, dock_area) = if dock {
        let split = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(60), Constraint::Length(48)])
            .split(area);
        (split[0], Some(split[1]))
    } else {
        (area, None)
    };

    let height = list_area.height as usize;
    app.visible_rows = height;
    ensure_visible(app, height);

    // Adaptive columns, with a guaranteed floor for versions. Names are
    // recognizable from head+tail when middle-truncated; a version pair like
    // `3.0.13-0ubuntu3.11 → 3.0.13-0ubuntu3.12` is not. So versions claim a
    // fixed slice first and the name column shrinks into whatever remains.
    let visible_end = (app.scroll + height).min(app.rows.len());
    let name_max = app.rows[app.scroll..visible_end]
        .iter()
        .filter_map(|row| match row {
            Row::Item { id } => app
                .package(id)
                .map(|package| UnicodeWidthStr::width(package.name.as_str())),
            Row::Header { .. } => None,
        })
        .max()
        .unwrap_or(20);
    let meta_max = app.rows[app.scroll..visible_end]
        .iter()
        .filter_map(|row| match row {
            Row::Item { id } => app
                .package(id)
                .map(|package| UnicodeWidthStr::width(meta_text(package).as_str())),
            Row::Header { .. } => None,
        })
        .max()
        .unwrap_or(10);
    let meta_width = meta_max.clamp(8, 20);
    // Column budget: markers(4) + name + gap(2) + versions + gap(2) + meta.
    let fixed = 4 + 2 + meta_width + 2;
    let spare = (list_area.width as usize).saturating_sub(fixed);
    let desired_versions = 40.min(spare.saturating_sub(16));
    let name_ceiling = spare.saturating_sub(desired_versions).clamp(16, 46);
    let name_width = name_max.clamp(16, name_ceiling) as u16;
    let meta_width = meta_width as u16;

    let mut lines: Vec<Line> = Vec::with_capacity(height);
    let mut row_index = app.scroll;
    while lines.len() < height && row_index < app.rows.len() {
        let row = &app.rows[row_index];
        let is_cursor = row_index == app.cursor;
        match row {
            Row::Header { label, count, .. } => {
                lines.push(section_header(label, *count, is_cursor, list_area.width));
            }
            Row::Item { id } => {
                if let Some(package) = app.package(id) {
                    lines.push(package_row(
                        app,
                        package,
                        is_cursor,
                        list_area.width,
                        name_width,
                        meta_width,
                    ));
                    // Inline expansion: the list breathes.
                    if !dock && app.expanded.as_deref() == Some(id.as_str()) {
                        for line in expansion_lines(package, list_area.width) {
                            if lines.len() < height {
                                lines.push(line);
                            }
                        }
                    }
                }
            }
        }
        row_index += 1;
    }

    frame.render_widget(Paragraph::new(lines), list_area);

    if let (Some(dock_area), Some(expanded_id)) = (dock_area, app.expanded.clone()) {
        if let Some(package) = app.package(&expanded_id) {
            draw_dock(frame, package, dock_area);
        }
    }
}

fn ensure_visible(app: &mut App, height: usize) {
    if height == 0 {
        return;
    }
    if app.cursor < app.scroll {
        app.scroll = app.cursor;
    }
    // Count expansion lines between scroll and cursor so the cursor row is
    // never pushed off the bottom by an open expansion above it.
    let mut used = 0;
    let mut index = app.scroll;
    while index <= app.cursor && index < app.rows.len() {
        used += 1;
        if let Row::Item { id } = &app.rows[index] {
            if app.expanded.as_deref() == Some(id.as_str()) {
                used += expansion_height(app, id);
            }
        }
        index += 1;
    }
    while used > height && app.scroll < app.cursor {
        app.scroll += 1;
        used = used.saturating_sub(1);
    }
    if app.cursor >= app.scroll + height {
        app.scroll = app.cursor + 1 - height;
    }
}

fn expansion_height(app: &App, id: &str) -> usize {
    app.package(id)
        .map(|package| {
            let mut height = 4; // title, version, deps, actions
            if !package.description.trim().is_empty() {
                height += 1;
            }
            height
        })
        .unwrap_or(4)
}

fn section_header(label: &str, count: usize, is_cursor: bool, width: u16) -> Line<'static> {
    let style = match label {
        "SECURITY" => red().add_modifier(Modifier::BOLD),
        "UPDATES" => amber().add_modifier(Modifier::BOLD),
        _ => dim().add_modifier(Modifier::BOLD),
    };
    let title = format!(" {label} ");
    let count_label = format!(" {count} ");
    let rule_len = (width as usize).saturating_sub(
        UnicodeWidthStr::width(title.as_str()) + UnicodeWidthStr::width(count_label.as_str()) + 1,
    );
    // Cursor on a header reverses the label and count only — a full-width
    // reversed rule reads as a rendering error, not a cursor.
    Line::from(vec![
        Span::styled(title, if is_cursor { cursor_style() } else { style }),
        Span::styled("─".repeat(rule_len), faint()),
        Span::styled(
            count_label,
            if is_cursor { cursor_style() } else { faint() },
        ),
    ])
}

fn meta_text(package: &Package) -> String {
    let mut parts = vec![package.source.to_string().to_lowercase()];
    if package.size.is_some() {
        parts.push(package.size_display());
    }
    parts.join(" · ")
}

fn package_row(
    app: &App,
    package: &Package,
    is_cursor: bool,
    width: u16,
    name_width: u16,
    meta_width: u16,
) -> Line<'static> {
    let id = package.id();
    let selected = app.selected.contains(&id);
    let expanded = app.expanded.as_deref() == Some(id.as_str());

    let base = if is_cursor { cursor_style() } else { fg() };
    let meta = if is_cursor { cursor_style() } else { dim() };

    let marker = if is_cursor {
        "▸"
    } else if selected {
        "✓"
    } else if expanded {
        "▾"
    } else {
        " "
    };

    let (glyph, glyph_style) = match package.status {
        PackageStatus::UpdateAvailable
            if package.detect_update_category() == UpdateCategory::Security =>
        {
            ("⚠", if is_cursor { cursor_style() } else { red() })
        }
        PackageStatus::UpdateAvailable => ("↑", if is_cursor { cursor_style() } else { amber() }),
        _ if app.favorites.contains(&id) => (
            "★",
            if is_cursor {
                cursor_style()
            } else {
                amber().add_modifier(Modifier::BOLD)
            },
        ),
        _ => ("·", if is_cursor { cursor_style() } else { faint() }),
    };

    let name_width = name_width as usize;
    let meta_width = meta_width as usize;
    // Columns: marker+glyph | name | versions (the remainder) | meta (right)
    let versions_width = (width as usize).saturating_sub(4 + name_width + 2 + meta_width + 2);

    let name = truncate_middle(&package.name, name_width);
    let name_spans = highlight_matches(&name, &app.search.text, base, amber());
    let name_pad = name_width.saturating_sub(UnicodeWidthStr::width(name.as_str()));

    let mut spans = vec![
        Span::styled(marker.to_string(), base),
        Span::styled(" ".to_string(), base),
        Span::styled(glyph.to_string(), glyph_style),
        Span::styled(" ".to_string(), base),
    ];
    spans.extend(name_spans);
    spans.push(Span::styled(" ".repeat(name_pad + 2), base));

    // Versions: old → new. The new version is what the user is deciding
    // about, so it takes the larger share of the column. Equal versions mean
    // a rebuild or commit-only update — printing the same string twice would
    // read as a rendering bug.
    if package.status == PackageStatus::UpdateAvailable {
        let total = versions_width.saturating_sub(3);
        let old_w = (total * 2 / 5).max(6);
        let new_w = total.saturating_sub(old_w).max(8);
        let old = truncate_middle(&package.version, old_w);
        let new_raw = package.available_version.as_deref().unwrap_or("");
        let new = if new_raw.is_empty() {
            "?".to_string()
        } else if new_raw == package.version {
            "new build".to_string()
        } else {
            truncate_middle(new_raw, new_w)
        };
        spans.push(Span::styled(
            format!("{:>width$}", old, width = old_w),
            if is_cursor { cursor_style() } else { dim() },
        ));
        spans.push(Span::styled(" → ".to_string(), meta));
        spans.push(Span::styled(
            format!("{:<width$}", new, width = new_w),
            if is_cursor { cursor_style() } else { amber() },
        ));
    } else {
        let version = truncate_middle(&package.version, versions_width);
        spans.push(Span::styled(
            format!("{:>width$}", version, width = versions_width),
            meta,
        ));
    }

    let meta_string = truncate(&meta_text(package), meta_width);
    let meta_pad = meta_width.saturating_sub(UnicodeWidthStr::width(meta_string.as_str()));
    spans.push(Span::styled("  ".to_string(), base));
    spans.push(Span::styled(" ".repeat(meta_pad), base));
    spans.push(Span::styled(meta_string, meta));

    Line::from(spans)
}

fn expansion_lines(package: &Package, width: u16) -> Vec<Line<'static>> {
    let width = width as usize;
    let bar = "  │ ";
    let content_width = width.saturating_sub(UnicodeWidthStr::width(bar));
    let mut lines = Vec::new();

    let mut title = vec![
        Span::styled(bar.to_string(), faint()),
        Span::styled(package.name.clone(), bold()),
        Span::styled(
            format!("  ·  {}", package.source.to_string().to_lowercase()),
            dim(),
        ),
    ];
    if let Some(homepage) = &package.homepage {
        title.push(Span::styled(
            format!("  ·  {}", truncate(homepage, 40)),
            faint(),
        ));
    }
    lines.push(Line::from(title));

    if !package.description.trim().is_empty() {
        lines.push(Line::from(vec![
            Span::styled(bar.to_string(), faint()),
            Span::styled(truncate(package.description.trim(), content_width), dim()),
        ]));
    }

    let version_text = match package.status {
        PackageStatus::UpdateAvailable => {
            let new = package.available_version.as_deref().unwrap_or("?");
            if new == package.version {
                format!("{}  →  new build (same version)", package.version)
            } else {
                format!("{}  →  {}", package.version, new)
            }
        }
        _ => package.version.clone(),
    };
    lines.push(Line::from(vec![
        Span::styled(bar.to_string(), faint()),
        Span::styled(version_text, amber()),
    ]));

    let mut facts = Vec::new();
    if !package.dependencies.is_empty() {
        facts.push(format!("deps: {}", package.dependencies.len()));
    }
    if let Some(license) = &package.license {
        facts.push(format!("license: {license}"));
    }
    if let Some(date) = &package.install_date {
        facts.push(format!("installed: {}", truncate(date, 16)));
    }
    if facts.is_empty() {
        facts.push("no extra metadata".to_string());
    }
    lines.push(Line::from(vec![
        Span::styled(bar.to_string(), faint()),
        Span::styled(truncate(&facts.join("   "), content_width), faint()),
    ]));

    let action = match package.status {
        PackageStatus::UpdateAvailable => "u queue update",
        PackageStatus::Installed => ": palette → remove",
        _ => ": palette",
    };
    lines.push(Line::from(vec![
        Span::styled(bar.to_string(), faint()),
        Span::styled(action.to_string(), accent()),
        Span::styled("   ␣ select   esc close".to_string(), faint()),
    ]));
    lines.push(Line::from(Span::styled("  ╵".to_string(), faint())));
    lines
}

fn draw_dock(frame: &mut Frame, package: &Package, area: Rect) {
    let mut lines = Vec::new();
    lines.push(Line::from(Span::styled(
        format!(" {}", package.name),
        bold(),
    )));
    lines.push(Line::from(Span::styled(
        format!(" {}", package.source.to_string().to_lowercase()),
        dim(),
    )));
    lines.push(Line::from(""));
    if !package.description.trim().is_empty() {
        for chunk in wrap_text(
            package.description.trim(),
            area.width.saturating_sub(2) as usize,
        ) {
            lines.push(Line::from(Span::styled(format!(" {chunk}"), dim())));
        }
        lines.push(Line::from(""));
    }
    let version_text = match package.status {
        PackageStatus::UpdateAvailable => {
            let new = package.available_version.as_deref().unwrap_or("?");
            if new == package.version {
                format!(" {}  →  new build (same version)", package.version)
            } else {
                format!(" {}  →  {}", package.version, new)
            }
        }
        _ => format!(" {}", package.version),
    };
    lines.push(Line::from(Span::styled(version_text, amber())));

    let mut facts = Vec::new();
    if package.size.is_some() {
        facts.push(package.size_display());
    }
    if let Some(license) = package
        .license
        .as_deref()
        .filter(|license| !license.trim().is_empty())
    {
        facts.push(format!("license {license}"));
    }
    if !package.dependencies.is_empty() {
        facts.push(format!("{} dependencies", package.dependencies.len()));
    }
    if !facts.is_empty() {
        lines.push(Line::from(Span::styled(
            format!(" {}", facts.join("  ·  ")),
            faint(),
        )));
    }
    lines.push(Line::from(""));
    if !package.dependencies.is_empty() {
        lines.push(Line::from(Span::styled(
            format!(" dependencies ({})", package.dependencies.len()),
            dim(),
        )));
        for dep in package.dependencies.iter().take(8) {
            lines.push(Line::from(Span::styled(
                format!(
                    "   {}",
                    truncate(dep, area.width.saturating_sub(4) as usize)
                ),
                faint(),
            )));
        }
    }
    lines.push(Line::from(""));
    let action = match package.status {
        PackageStatus::UpdateAvailable => " u queue update",
        PackageStatus::Installed => " : palette → remove",
        _ => " : palette",
    };
    lines.push(Line::from(Span::styled(action.to_string(), accent())));

    // Thin vertical rule on the dock's left edge.
    let rule_area = Rect {
        height: area.height,
        width: 1,
        ..area
    };
    frame.render_widget(
        Paragraph::new(
            (0..area.height)
                .map(|_| Line::from(Span::styled("│", faint())))
                .collect::<Vec<_>>(),
        ),
        rule_area,
    );
    let content = Rect {
        x: area.x + 1,
        width: area.width.saturating_sub(1),
        ..area
    };
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), content);
}

fn draw_skeleton(frame: &mut Frame, app: &App, area: Rect) {
    let phase = (app.spinner / 2) % 4;
    let widths = [0.62, 0.45, 0.71, 0.38, 0.55, 0.66, 0.42, 0.58];
    let mut lines = Vec::new();
    lines.push(Line::from(vec![
        Span::styled(format!(" {} ", app.spinner_frame()), amber()),
        Span::styled(
            match app.load_phase {
                LoadPhase::Installed => "loading installed packages…".to_string(),
                LoadPhase::Updates => "checking for updates…".to_string(),
                LoadPhase::Done => String::new(),
            },
            dim(),
        ),
        Span::styled(format!("  ({} sources)", app.sources_done), faint()),
    ]));
    lines.push(Line::from(""));
    for (index, factor) in widths.iter().enumerate() {
        let len = ((area.width as f32 * factor) as usize).max(8);
        let shade = match (index + phase) % 4 {
            0 => Color::Rgb(84, 90, 102),
            1 => Color::Rgb(66, 71, 82),
            2 => Color::Rgb(54, 58, 68),
            _ => Color::Rgb(46, 49, 58),
        };
        lines.push(Line::from(Span::styled(
            " ".repeat(len),
            Style::default().bg(shade),
        )));
        if lines.len() >= area.height as usize {
            break;
        }
    }
    frame.render_widget(Paragraph::new(lines), area);
}

fn draw_empty(frame: &mut Frame, app: &App, area: Rect) {
    let lines = if !app.search.text.is_empty() {
        vec![
            Line::from(""),
            Line::from(Span::styled(
                format!("no matches for \"{}\"", app.search.text),
                dim(),
            )),
            Line::from(Span::styled("esc clears the search", faint())),
        ]
    } else if app.filter == Filter::Favorites {
        vec![
            Line::from(""),
            Line::from(Span::styled("no favorites yet", dim())),
            Line::from(Span::styled(
                "   f stars the package under the cursor",
                faint(),
            )),
        ]
    } else if app.filter == Filter::Updates && app.load_phase == LoadPhase::Done {
        vec![
            Line::from(""),
            Line::from(Span::styled(
                " ✓ system is up to date",
                green().add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled("   r re-check · 3 browse installed", faint())),
        ]
    } else {
        vec![
            Line::from(""),
            Line::from(Span::styled("nothing here", dim())),
        ]
    };
    let top = area.height.saturating_sub(lines.len() as u16) / 3;
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(top), Constraint::Min(1)])
        .split(area);
    frame.render_widget(Paragraph::new(lines), chunks[1]);
}

// ---------------------------------------------------------------------
// Queue: peek line + drawer
// ---------------------------------------------------------------------

fn queue_peek_height(app: &App) -> u16 {
    if app.queue_open || app.queue.is_empty() {
        return 0;
    }
    let (queued, running, failed, _) = app.queue_counts();
    if queued + running + failed > 0 {
        1
    } else {
        0
    }
}

fn draw_queue_peek(frame: &mut Frame, app: &App, area: Rect) {
    let (queued, running, failed, _) = app.queue_counts();
    let orphans = app.orphan_count();
    let mut spans = vec![Span::styled(" queue  ", faint())];
    if running > 0 {
        let current = app
            .queue
            .iter()
            .find(|entry| entry.status == TaskQueueStatus::Running && !App::is_orphan(entry));
        let label = current
            .map(|entry| entry.package_name.clone())
            .unwrap_or_default();
        spans.push(Span::styled(format!("{} ", app.spinner_frame()), amber()));
        spans.push(Span::styled(truncate(&label, 24), fg()));
        spans.push(Span::styled(format!("  {running} running"), dim()));
    }
    if queued > 0 {
        spans.push(Span::styled(format!("   ·  {queued} pending"), dim()));
    }
    if failed > 0 {
        spans.push(Span::styled("   ·  ".to_string(), dim()));
        spans.push(Span::styled(format!("✗ {failed} failed"), red()));
    }
    if orphans > 0 {
        spans.push(Span::styled("   ·  ".to_string(), dim()));
        spans.push(Span::styled(format!("⚠ {orphans} orphaned"), amber()));
    }
    spans.push(Span::styled("   ·  tab to open", faint()));
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn draw_queue_drawer(frame: &mut Frame, app: &App, area: Rect) {
    if area.height < 3 {
        return;
    }
    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(vec![
        Span::styled(" QUEUE ", bold()),
        Span::styled("─".repeat(area.width.saturating_sub(8) as usize), faint()),
    ]));

    let orphans: Vec<&TaskQueueEntry> = app.queue.iter().filter(|e| App::is_orphan(e)).collect();
    let running: Vec<&TaskQueueEntry> = app
        .queue
        .iter()
        .filter(|e| e.status == TaskQueueStatus::Running && !App::is_orphan(e))
        .collect();
    let pending: Vec<&TaskQueueEntry> = app
        .queue
        .iter()
        .filter(|e| e.status == TaskQueueStatus::Queued)
        .collect();
    // Failure dedup: one row per (package, error), with a repeat count.
    let mut failures: Vec<(String, String, usize, &TaskQueueEntry)> = Vec::new();
    for entry in app
        .queue
        .iter()
        .filter(|e| e.status == TaskQueueStatus::Failed)
    {
        let error = entry.error.clone().unwrap_or_default();
        if let Some(existing) = failures
            .iter_mut()
            .find(|(name, err, _, _)| *name == entry.package_name && *err == error)
        {
            existing.2 += 1;
        } else {
            failures.push((entry.package_name.clone(), error, 1, entry));
        }
    }
    let done = app
        .queue
        .iter()
        .filter(|e| {
            matches!(
                e.status,
                TaskQueueStatus::Completed | TaskQueueStatus::Cancelled
            )
        })
        .count();

    let budget = area.height.saturating_sub(2) as usize;

    for entry in running.iter().take(3) {
        lines.push(Line::from(vec![
            Span::styled(format!(" {} ", app.spinner_frame()), amber()),
            Span::styled(truncate(&entry.package_name, 30), fg()),
            Span::styled(
                format!(
                    "  {} · {}",
                    action_label(entry.action),
                    elapsed(entry.started_at)
                ),
                dim(),
            ),
        ]));
        if let Some(log) = app.live_logs.get(&entry.id) {
            lines.push(Line::from(vec![
                Span::styled("   ↳ ", faint()),
                Span::styled(
                    truncate(log.trim(), area.width.saturating_sub(8) as usize),
                    faint(),
                ),
            ]));
        }
    }
    if !pending.is_empty() {
        let names: Vec<String> = pending
            .iter()
            .take(6)
            .map(|entry| entry.package_name.clone())
            .collect();
        let suffix = if pending.len() > 6 {
            format!(" +{} more", pending.len() - 6)
        } else {
            String::new()
        };
        lines.push(Line::from(vec![
            Span::styled(" ◦ ", faint()),
            Span::styled(
                truncate(
                    &format!("pending: {}{}", names.join(", "), suffix),
                    area.width.saturating_sub(6) as usize,
                ),
                dim(),
            ),
        ]));
    }
    for (name, error, count, entry) in failures.iter().take(budget.max(2)) {
        let repeat = if *count > 1 {
            format!(" ×{count}")
        } else {
            String::new()
        };
        lines.push(Line::from(vec![
            Span::styled(" ✗ ", red()),
            Span::styled(truncate(name, 30), fg()),
            Span::styled(repeat, red()),
            Span::styled(
                format!(
                    "  {} · {}",
                    action_label(entry.action),
                    elapsed(entry.completed_at)
                ),
                dim(),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled("   ", faint()),
            Span::styled(
                truncate(error.trim(), area.width.saturating_sub(8) as usize),
                faint(),
            ),
        ]));
    }
    for orphan in orphans.iter().take(2) {
        lines.push(Line::from(vec![
            Span::styled(" ⚠ ", amber()),
            Span::styled(
                truncate(
                    &format!(
                        "orphaned: {} · \"running\" {}",
                        orphan.package_name,
                        elapsed(orphan.started_at)
                    ),
                    area.width.saturating_sub(6) as usize,
                ),
                amber(),
            ),
        ]));
    }
    if done > 0 {
        lines.push(Line::from(Span::styled(
            format!(" ✓ {done} completed",),
            green(),
        )));
    }

    lines.truncate(area.height as usize);
    frame.render_widget(Paragraph::new(lines), area);
}

// ---------------------------------------------------------------------
// Command bar
// ---------------------------------------------------------------------

fn draw_command_bar(frame: &mut Frame, app: &App, area: Rect) {
    if let Some((message, _)) = &app.status {
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(" ", fg()),
                Span::styled(message.clone(), accent()),
            ])),
            area,
        );
        return;
    }

    let mut spans = Vec::new();
    let push = |key: &'static str, label: String, spans: &mut Vec<Span<'static>>| {
        spans.push(Span::styled(format!(" {key}"), accent()));
        spans.push(Span::styled(format!(" {label}  "), dim()));
    };

    if app.queue_open {
        push("tab", "close queue".to_string(), &mut spans);
        push("r", "retry failed".to_string(), &mut spans);
        push("x", "reap orphaned".to_string(), &mut spans);
        push("esc", "back".to_string(), &mut spans);
    } else if !app.selected.is_empty() {
        push("u", "queue".to_string(), &mut spans);
        push("space", "toggle".to_string(), &mut spans);
        push("esc", "clear".to_string(), &mut spans);
        spans.push(Span::styled(
            format!("·  {} selected", app.selected.len()),
            amber(),
        ));
    } else {
        push("⏎", "expand".to_string(), &mut spans);
        push("␣", "select".to_string(), &mut spans);
        push("/", "search".to_string(), &mut spans);
        push(":", "commands".to_string(), &mut spans);
        push("tab", "queue".to_string(), &mut spans);
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

// ---------------------------------------------------------------------
// Overlays
// ---------------------------------------------------------------------

fn dim_backdrop(frame: &mut Frame) {
    let area = frame.area();
    let buffer = frame.buffer_mut();
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            if let Some(cell) = buffer.cell_mut((x, y)) {
                cell.set_fg(FAINT);
            }
        }
    }
}

fn centered(frame: &mut Frame, width: u16, height: u16) -> Rect {
    let area = frame.area();
    let width = width.min(area.width.saturating_sub(4));
    let height = height.min(area.height.saturating_sub(2));
    Rect {
        x: area.x + (area.width.saturating_sub(width)) / 2,
        y: area.y + (area.height.saturating_sub(height)) / 2,
        width,
        height,
    }
}

fn overlay_card(frame: &mut Frame, area: Rect, title: &str) -> Rect {
    use ratatui::widgets::{Block, Borders, Clear};
    frame.render_widget(Clear, area);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(faint())
        .title(format!(" {title} "))
        .title_style(bold());
    let inner = block.inner(area);
    frame.render_widget(block, area);
    inner
}

fn draw_palette(frame: &mut Frame, app: &App, query: &str) {
    dim_backdrop(frame);
    let commands = palette::filtered_for(app, query);
    let area = frame.area();
    let width = area.width.saturating_sub(6).max(30);
    let height = (commands.len() as u16 + 5).min(area.height.saturating_sub(2));
    let card = Rect {
        x: 3,
        y: 1,
        width,
        height,
    };
    let inner = overlay_card(frame, card, ": commands");

    let mut lines = vec![Line::from(vec![
        Span::styled(":".to_string(), accent()),
        Span::styled(format!(" {query}█"), fg()),
    ])];
    lines.push(Line::from(""));

    if commands.is_empty() {
        lines.push(Line::from(Span::styled(
            "no matching command · esc closes",
            dim(),
        )));
    }
    for (index, command) in commands.iter().enumerate() {
        let is_cursor = index == app.palette_cursor;
        let style = if is_cursor { cursor_style() } else { fg() };
        let number = if index < 9 {
            format!("{} ", index + 1)
        } else {
            "  ".to_string()
        };
        let mut spans = vec![
            Span::styled(
                number.clone(),
                if is_cursor { cursor_style() } else { faint() },
            ),
            Span::styled(command.title.clone(), style),
        ];
        if !command.hint.is_empty() {
            let used = UnicodeWidthStr::width(number.as_str())
                + UnicodeWidthStr::width(command.title.as_str());
            let hint_w = UnicodeWidthStr::width(command.hint);
            let pad = (inner.width as usize).saturating_sub(used + hint_w + 1);
            spans.push(Span::styled(" ".repeat(pad), style));
            spans.push(Span::styled(
                command.hint.to_string(),
                if is_cursor { cursor_style() } else { faint() },
            ));
        }
        lines.push(Line::from(spans));
    }
    frame.render_widget(Paragraph::new(lines), inner);
}

fn draw_help(frame: &mut Frame) {
    dim_backdrop(frame);
    let area = centered(frame, 62, 22);
    let inner = overlay_card(frame, area, "keys");

    let groups: &[(&str, &[(&str, &str)])] = &[
        (
            "move",
            &[
                ("↑↓ / jk", "move"),
                ("pgup/pgdn", "page"),
                ("g / G", "top / bottom"),
            ],
        ),
        (
            "act",
            &[
                ("enter", "expand / collapse"),
                ("space", "select"),
                ("u", "queue update"),
                ("a", "queue all updates"),
                ("f", "star / unstar"),
            ],
        ),
        (
            "find",
            &[
                ("/", "search — src:npm scopes to npm"),
                (":", "command palette"),
                ("1 2 3 4", "updates · security · installed · favorites"),
            ],
        ),
        (
            "rest",
            &[
                ("tab", "queue drawer"),
                ("r", "refresh · retry (in queue)"),
                ("x", "reap orphaned (in queue)"),
                ("?", "help"),
                ("q", "quit"),
            ],
        ),
    ];

    let mut lines = Vec::new();
    for (group, keys) in groups {
        lines.push(Line::from(Span::styled(group.to_string(), amber())));
        for (key, label) in *keys {
            lines.push(Line::from(vec![
                Span::styled(format!("  {key:<12}"), accent()),
                Span::styled(label.to_string(), dim()),
            ]));
        }
        lines.push(Line::from(""));
    }
    lines.push(Line::from(Span::styled(
        "everything else lives in the : palette",
        faint(),
    )));
    frame.render_widget(Paragraph::new(lines), inner);
}

fn draw_confirm(frame: &mut Frame, title: &str, body: &str) {
    dim_backdrop(frame);
    let area = centered(frame, 52, 7);
    let inner = overlay_card(frame, area, title);
    let lines = vec![
        Line::from(Span::styled(body.to_string(), fg())),
        Line::from(""),
        Line::from(vec![
            Span::styled(" y", red()),
            Span::styled(" confirm   ", faint()),
            Span::styled(" n/esc", accent()),
            Span::styled(" cancel", faint()),
        ]),
    ];
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: true }), inner);
}

fn draw_changelog(frame: &mut Frame, title: &str, lines: &[String], scroll: usize) {
    dim_backdrop(frame);
    let area = centered(frame, 76, frame.area().height.saturating_sub(6).min(30));
    let inner = overlay_card(frame, area, title);
    let height = inner.height as usize;
    let start = scroll.min(lines.len().saturating_sub(height));
    let visible: Vec<Line> = lines[start..(start + height).min(lines.len())]
        .iter()
        .map(|line| Line::from(Span::styled(line.clone(), fg())))
        .collect();
    frame.render_widget(Paragraph::new(visible), inner);
}

// ---------------------------------------------------------------------
// Text helpers
// ---------------------------------------------------------------------

fn truncate(text: &str, width: usize) -> String {
    let used = UnicodeWidthStr::width(text);
    if used <= width {
        return text.to_string();
    }
    if width <= 1 {
        return "…".to_string();
    }
    let mut out = String::new();
    let mut used = 0;
    for ch in text.chars() {
        let w = UnicodeWidthStr::width(ch.to_string().as_str());
        if used + w > width - 1 {
            break;
        }
        out.push(ch);
        used += w;
    }
    out.push('…');
    out
}

fn truncate_middle(text: &str, width: usize) -> String {
    let used = UnicodeWidthStr::width(text);
    if used <= width {
        return text.to_string();
    }
    if width <= 4 {
        return truncate(text, width);
    }
    let keep = width - 1;
    let head = keep / 2 + keep % 2;
    let tail = keep / 2;
    let head_str: String = text.chars().take(head).collect();
    let skip = text.chars().count().saturating_sub(tail);
    let tail_str: String = text.chars().skip(skip).collect();
    format!("{head_str}…{tail_str}")
}

fn highlight_matches(
    text: &str,
    needle: &str,
    base: Style,
    highlight: Style,
) -> Vec<Span<'static>> {
    let needle = needle.trim();
    if needle.is_empty() {
        return vec![Span::styled(text.to_string(), base)];
    }
    let lower = text.to_lowercase();
    let needle = needle.to_lowercase();
    let mut spans = Vec::new();
    let mut rest = lower.as_str();
    let mut offset = 0;
    while let Some(pos) = rest.find(&needle) {
        if pos > 0 {
            spans.push(Span::styled(text[offset..offset + pos].to_string(), base));
        }
        spans.push(Span::styled(
            text[offset + pos..offset + pos + needle.len()].to_string(),
            highlight.add_modifier(Modifier::BOLD),
        ));
        offset += pos + needle.len();
        rest = &rest[pos + needle.len()..];
    }
    if offset < text.len() {
        spans.push(Span::styled(text[offset..].to_string(), base));
    }
    if spans.is_empty() {
        spans.push(Span::styled(text.to_string(), base));
    }
    spans
}

fn wrap_text(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        if UnicodeWidthStr::width(current.as_str()) + word.len() + 1 > width && !current.is_empty()
        {
            lines.push(current.clone());
            current.clear();
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

fn elapsed(since: Option<chrono::DateTime<Local>>) -> String {
    let Some(since) = since else {
        return String::new();
    };
    let span = Local::now().signed_duration_since(since);
    let seconds = span.num_seconds().max(0);
    if seconds < 60 {
        format!("{seconds}s")
    } else if seconds < 3600 {
        format!("{}m", seconds / 60)
    } else if seconds < 86_400 {
        format!("{}h", seconds / 3600)
    } else {
        format!("{}d", seconds / 86_400)
    }
}

fn age_text(saved_at: chrono::DateTime<Local>) -> String {
    elapsed(Some(saved_at))
}

fn action_label(action: crate::models::history::TaskQueueAction) -> &'static str {
    match action {
        crate::models::history::TaskQueueAction::Install => "install",
        crate::models::history::TaskQueueAction::Remove => "remove",
        crate::models::history::TaskQueueAction::Update => "update",
    }
}
