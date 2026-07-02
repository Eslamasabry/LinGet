use crate::cli::tui::app::App;
use crate::cli::tui::format::{
    format_package_version, package_status_short, truncate_middle_to_width, truncate_to_width,
};
use crate::cli::tui::state::filters::{Filter, Focus};
use crate::cli::tui::theme::{
    accent, dim, loading, muted, palette, row_cursor, row_selected, scrollbar_style,
    scrollbar_thumb, source_color, success, text, warning,
};
use crate::cli::tui::ui::{panel_block, window_start};
use crate::models::{Package, PackageStatus};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Cell, Paragraph, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Table},
    Frame,
};

pub fn draw_packages_panel(frame: &mut Frame, app: &App, area: Rect, _compact: bool) {
    let focused = app.focus == Focus::Packages && !app.queue_expanded;
    let block = panel_block(packages_panel_title(app), focused, true);

    if app.is_catalog_busy() && app.filtered.is_empty() {
        let paragraph = Paragraph::new(format!(
            "{} {}",
            app.spinner_frame(),
            app.catalog_loading_message()
        ))
        .style(loading())
        .alignment(ratatui::layout::Alignment::Center);

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let vertical_padding = inner.height.saturating_sub(1) / 2;
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(vertical_padding), Constraint::Min(1)])
            .split(inner);

        frame.render_widget(paragraph, chunks[1]);
        return;
    }

    if app.filtered.is_empty() {
        let lines = build_empty_state_lines(app);
        let paragraph = Paragraph::new(lines)
            .style(text())
            .alignment(ratatui::layout::Alignment::Center);

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let total_lines = 7u16; // art(3) + spacing(1) + title(1) + spacing(1) + hint(1)
        let vertical_padding = inner.height.saturating_sub(total_lines) / 2;
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(vertical_padding), Constraint::Min(1)])
            .split(inner);

        frame.render_widget(paragraph, chunks[1]);
        return;
    }

    let inner = block.inner(area);
    let sections = if inner.height >= 5 {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(2)])
            .split(inner)
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(0)])
            .split(inner)
    };

    frame.render_widget(block, area);

    let available_rows = sections[0].height.saturating_sub(2) as usize;
    let visible_rows = available_rows;
    let start = window_start(app.filtered.len(), visible_rows.max(1), app.cursor);
    let end = (start + visible_rows.max(1)).min(app.filtered.len());

    // The column set adapts to the panel width: fixed-width tables silently
    // clip their right-hand columns on narrow terminals, so we drop the
    // least important columns first instead.
    let table_width = sections[0].width as usize;
    let mode = TableMode::for_width(table_width);
    let name_width = mode.name_width(table_width);

    let mut rows = Vec::new();
    for row_index in start..end {
        let Some(package_index) = app.filtered.get(row_index).copied() else {
            continue;
        };
        let Some(package): Option<&crate::models::Package> = app.packages.get(package_index) else {
            continue;
        };

        let package_id = package.id();
        let is_cursor = row_index == app.cursor;
        let is_selected = app.selected.contains(&package_id);
        let is_favorite = app.is_favorite_id(&package_id);
        // Cursor gets the background highlight; other selected rows keep a
        // distinct tint so a bulk selection is visible at a glance.
        let row_style = if is_cursor {
            row_cursor()
        } else if is_selected {
            row_selected()
        } else {
            text()
        };
        let quiet_style = if is_cursor { row_style } else { dim() };
        let marker = if is_cursor {
            "▸"
        } else if is_selected {
            "✓"
        } else {
            " "
        };
        let favorite_marker = if is_favorite { " ★" } else { "" };
        let display_version = format_package_version(package);
        let current = if package.status == PackageStatus::UpdateAvailable {
            package.version.clone()
        } else {
            display_version.clone()
        };
        let available = package
            .available_version
            .clone()
            .unwrap_or_else(|| "-".to_string());
        let source = source_table_label(app, package);

        // In Duplicates filter, append "also: X" hint to the name
        let display_name = if app.filter == Filter::Duplicates {
            if let Some(peers) = app.duplicate_peer_sources.get(&package_id) {
                if !peers.is_empty() {
                    let also: Vec<String> = peers.iter().map(|s| s.to_string()).collect();
                    format!(
                        "{} · also: {}",
                        truncate_middle_to_width(&package.name, name_width.saturating_sub(6).max(8)),
                        also.join(", ")
                    )
                } else {
                    truncate_middle_to_width(&package.name, name_width)
                }
            } else {
                truncate_middle_to_width(&package.name, name_width)
            }
        } else {
            truncate_middle_to_width(&package.name, name_width)
        };

        let match_hint = search_match_hint(package, &app.search);
        let highlight_style = if is_cursor { warning() } else { accent() };
        let mut name_spans = if matches!(match_hint, Some("desc") | Some("desc≈")) {
            vec![Span::styled(display_name.clone(), row_style)]
        } else {
            search_highlight_spans(&display_name, &app.search, row_style, highlight_style)
        };
        if let Some(hint) = match_hint {
            name_spans.push(Span::raw(" "));
            name_spans.push(Span::styled(format!("[{}]", hint), muted()));
        }
        if !favorite_marker.is_empty() {
            name_spans.push(Span::styled(favorite_marker, warning()));
        }

        let (status_glyph, status_style) = package_status_short(package.status);
        let (released, _) = release_date_cell(package);
        let (action, action_style) = action_cell(package.status);

        let mut cells = vec![
            Cell::from(Span::styled(marker, row_style)),
            Cell::from(Span::styled(
                status_glyph,
                if is_cursor { row_style } else { status_style },
            )),
            Cell::from(Line::from(name_spans)),
        ];

        match mode {
            TableMode::Full => {
                cells.push(Cell::from(Span::styled(
                    truncate_to_width(&current, 14),
                    row_style,
                )));
                cells.push(Cell::from(Span::styled(
                    truncate_to_width(&available, 14),
                    if is_cursor {
                        row_style
                    } else {
                        Style::default().fg(palette::ORANGE())
                    },
                )));
            }
            TableMode::Medium | TableMode::Narrow => {
                cells.push(Cell::from(Span::styled(
                    truncate_to_width(&display_version, mode.version_width()),
                    if is_cursor || package.status != PackageStatus::UpdateAvailable {
                        row_style
                    } else {
                        Style::default().fg(palette::ORANGE())
                    },
                )));
            }
        }

        cells.push(Cell::from(Span::styled(
            truncate_to_width(&source, mode.source_width()),
            if is_cursor {
                row_style
            } else {
                source_color(package.source)
            },
        )));

        if mode == TableMode::Full {
            // The release date is informational — render it neutrally rather
            // than repainting the status badge background behind a date.
            cells.push(Cell::from(Span::styled(
                truncate_to_width(&released, 12),
                quiet_style,
            )));
        }

        if mode != TableMode::Narrow {
            cells.push(Cell::from(Span::styled(
                action,
                if is_cursor {
                    row_style
                } else if action == "update" {
                    quiet_style
                } else {
                    action_style
                },
            )));
        }

        rows.push(Row::new(cells).style(row_style));
    }

    let header = Row::new(mode.header_cells())
        .style(dim().add_modifier(Modifier::BOLD))
        .bottom_margin(1);
    let widths = mode.constraints();

    let table = Table::new(rows, widths).header(header).column_spacing(1);
    frame.render_widget(table, sections[0]);
    if sections[0].height > 1 {
        let separator = Paragraph::new("─".repeat(sections[0].width as usize)).style(dim());
        frame.render_widget(
            separator,
            Rect {
                x: sections[0].x,
                y: sections[0].y + 1,
                width: sections[0].width,
                height: 1,
            },
        );
    }

    if sections[1].height > 0 {
        let footer = vec![
            Line::from(Span::styled("─".repeat(sections[1].width as usize), dim())),
            package_footer_line(app, start, end, sections[1].width as usize, visible_rows),
        ];
        frame.render_widget(Paragraph::new(footer), sections[1]);
    }

    if app.filtered.len() > visible_rows {
        let mut scrollbar_state = ScrollbarState::new(app.filtered.len()).position(app.cursor);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None)
            .track_symbol(Some("║"))
            .thumb_symbol("█")
            .style(scrollbar_style())
            .thumb_style(scrollbar_thumb());
        frame.render_stateful_widget(
            scrollbar,
            Rect {
                x: sections[0].x,
                y: sections[0].y.saturating_add(2),
                width: sections[0].width,
                height: sections[0].height.saturating_sub(2),
            },
            &mut scrollbar_state,
        );
    }
}

/// Which columns fit at the current panel width. Ordered from most to least
/// spacious; columns are dropped right-to-left as the panel narrows.
#[derive(Clone, Copy, PartialEq, Eq)]
enum TableMode {
    /// Marker · Status · Package · Current · Available · Source · Released · Action
    Full,
    /// Marker · Status · Package · Version · Source · Action
    Medium,
    /// Marker · Status · Package · Version · Source
    Narrow,
}

impl TableMode {
    fn for_width(width: usize) -> Self {
        if width >= 100 {
            TableMode::Full
        } else if width >= 76 {
            TableMode::Medium
        } else {
            TableMode::Narrow
        }
    }

    fn version_width(self) -> usize {
        match self {
            TableMode::Full => 14,
            TableMode::Medium => 20,
            TableMode::Narrow => 16,
        }
    }

    fn source_width(self) -> usize {
        match self {
            TableMode::Full => 10,
            TableMode::Medium => 10,
            TableMode::Narrow => 8,
        }
    }

    /// Width left over for the Package column after fixed columns and the
    /// 1-cell spacings are accounted for.
    fn name_width(self, table_width: usize) -> usize {
        let consumed = match self {
            // 1 + 3 + 14 + 14 + 11 + 12 + 12 fixed + 7 spacings
            TableMode::Full => 74,
            // 1 + 3 + 20 + 10 + 10 fixed + 5 spacings
            TableMode::Medium => 49,
            // 1 + 3 + 16 + 8 fixed + 4 spacings
            TableMode::Narrow => 32,
        };
        table_width.saturating_sub(consumed).max(12)
    }

    fn header_cells(self) -> Vec<&'static str> {
        match self {
            TableMode::Full => vec![
                "", "", "Package", "Current", "Available", "Source", "Released", "Action",
            ],
            TableMode::Medium => vec!["", "", "Package", "Version", "Source", "Action"],
            TableMode::Narrow => vec!["", "", "Package", "Version", "Source"],
        }
    }

    fn constraints(self) -> Vec<Constraint> {
        match self {
            TableMode::Full => vec![
                Constraint::Length(1),
                Constraint::Length(3),
                Constraint::Min(24),
                Constraint::Length(14),
                Constraint::Length(14),
                Constraint::Length(11),
                Constraint::Length(12),
                Constraint::Length(12),
            ],
            TableMode::Medium => vec![
                Constraint::Length(1),
                Constraint::Length(3),
                Constraint::Min(20),
                Constraint::Length(20),
                Constraint::Length(10),
                Constraint::Length(10),
            ],
            TableMode::Narrow => vec![
                Constraint::Length(1),
                Constraint::Length(3),
                Constraint::Min(14),
                Constraint::Length(16),
                Constraint::Length(8),
            ],
        }
    }
}

fn source_table_label(app: &App, package: &Package) -> String {
    app.package_source_badge(package).to_lowercase()
}

fn packages_panel_title(app: &App) -> String {
    let label = if app.search_results.is_some() {
        app.provider_search_scope_label()
            .unwrap_or_else(|| "Provider Search".to_string())
    } else if !app.search.is_empty() {
        "Filtered Packages".to_string()
    } else {
        match app.filter {
            Filter::All => "All Packages".to_string(),
            Filter::Installed => "Installed".to_string(),
            Filter::Updates => "Updates".to_string(),
            Filter::Favorites => "Favorites".to_string(),
            Filter::SecurityUpdates => "Security".to_string(),
            Filter::Duplicates => "Duplicates".to_string(),
        }
    };

    format!(
        " {} ({} of {}) ",
        label,
        app.filtered.len(),
        total_for_filter(app)
    )
}

fn total_for_filter(app: &App) -> usize {
    match app.filter {
        Filter::All => app.filter_counts[0],
        Filter::Installed => app.filter_counts[1],
        Filter::Updates => app.filter_counts[2],
        Filter::Favorites => app.filter_counts[3],
        Filter::SecurityUpdates => app.filter_counts[4],
        Filter::Duplicates => app.filter_counts[5],
    }
}

fn release_date_cell(package: &Package) -> (String, Style) {
    let date_str = package
        .enrichment
        .as_ref()
        .and_then(|e| e.last_updated.as_ref())
        .map(|d| d.as_str())
        .unwrap_or("-");

    let formatted = if date_str == "-" {
        "-".to_string()
    } else {
        let date_part = date_str.split('T').next().unwrap_or(date_str);
        if let Ok(parsed) = chrono::NaiveDate::parse_from_str(date_part, "%Y-%m-%d") {
            parsed.format("%b %d, %Y").to_string()
        } else {
            date_part.to_string()
        }
    };
    (formatted, Style::default())
}

fn action_cell(status: PackageStatus) -> (&'static str, Style) {
    let style = Style::default().fg(palette::ORANGE());
    match status {
        PackageStatus::UpdateAvailable => ("update", style),
        PackageStatus::Installed => ("remove", style),
        PackageStatus::NotInstalled => ("install", style),
        PackageStatus::Installing => ("installing", warning()),
        PackageStatus::Updating => ("updating", warning()),
        PackageStatus::Removing => ("removing", warning()),
    }
}

fn package_footer_line(
    app: &App,
    start: usize,
    end: usize,
    width: usize,
    visible_rows: usize,
) -> Line<'static> {
    let left = if app.filtered.is_empty() {
        "Showing 0-0 of 0 packages".to_string()
    } else {
        format!(
            "Showing {}-{} of {} {}",
            start + 1,
            end,
            app.filtered.len(),
            match app.filter {
                Filter::Updates => "updates",
                Filter::Installed => "installed",
                Filter::Favorites => "favorites",
                Filter::SecurityUpdates => "security updates",
                Filter::Duplicates => "duplicates",
                Filter::All => "packages",
            }
        )
    };
    let pages = if visible_rows == 0 {
        1
    } else {
        app.filtered.len().max(1).div_ceil(visible_rows)
    };
    let current_page = if visible_rows == 0 {
        1
    } else {
        (app.cursor / visible_rows) + 1
    }
    .min(pages);
    let right = format!("Pages: {current_page}/{pages}");
    use unicode_width::UnicodeWidthStr;
    let gap = width.saturating_sub(UnicodeWidthStr::width(left.as_str()) + right.len());
    Line::from(vec![
        Span::styled(left, dim()),
        Span::raw(" ".repeat(gap)),
        Span::styled(right, dim()),
    ])
}

fn search_match_hint(package: &Package, query: &str) -> Option<&'static str> {
    let query = query.trim();
    if query.is_empty() {
        return None;
    }

    let query_lower = query.to_lowercase();
    let name_lower = package.name.to_lowercase();
    let description_lower = package.description.to_lowercase();

    if name_lower.contains(&query_lower) {
        None
    } else if description_lower.contains(&query_lower) {
        Some("desc")
    } else if !fuzzy_match_positions(&package.name, query).is_empty() {
        Some("fuzzy")
    } else if !fuzzy_match_positions(&package.description, query).is_empty() {
        Some("desc≈")
    } else {
        None
    }
}

fn search_highlight_spans(
    text: &str,
    query: &str,
    base: Style,
    highlight: Style,
) -> Vec<Span<'static>> {
    if query.trim().is_empty() {
        return vec![Span::styled(text.to_string(), base)];
    }

    if let Some((start, end)) = substring_match_range(text, query) {
        let chars: Vec<String> = text.chars().map(|ch| ch.to_string()).collect();
        let mut spans = Vec::new();
        if start > 0 {
            spans.push(Span::styled(chars[..start].concat(), base));
        }
        spans.push(Span::styled(chars[start..end].concat(), highlight));
        if end < chars.len() {
            spans.push(Span::styled(chars[end..].concat(), base));
        }
        return spans;
    }

    let fuzzy_positions = fuzzy_match_positions(text, query);
    if fuzzy_positions.is_empty() {
        return vec![Span::styled(text.to_string(), base)];
    }

    text.chars()
        .enumerate()
        .map(|(idx, ch)| {
            let style = if fuzzy_positions.contains(&idx) {
                highlight
            } else {
                base
            };
            Span::styled(ch.to_string(), style)
        })
        .collect()
}

fn substring_match_range(text: &str, query: &str) -> Option<(usize, usize)> {
    let query_lower = query.to_lowercase();
    let lower_parts: Vec<String> = text
        .chars()
        .map(|ch| ch.to_lowercase().collect::<String>())
        .collect();
    let lower_text = lower_parts.concat();
    let start_byte = lower_text.find(&query_lower)?;
    let end_byte = start_byte + query_lower.len();

    let mut consumed = 0usize;
    let mut start = None;
    let mut end = None;
    for (idx, part) in lower_parts.iter().enumerate() {
        if start.is_none() && consumed == start_byte {
            start = Some(idx);
        }
        consumed += part.len();
        if start.is_some() && consumed >= end_byte {
            end = Some(idx + 1);
            break;
        }
    }

    match (start, end) {
        (Some(start), Some(end)) => Some((start, end)),
        _ => None,
    }
}

fn fuzzy_match_positions(text: &str, query: &str) -> Vec<usize> {
    let expected: Vec<String> = query
        .chars()
        .map(|ch| ch.to_lowercase().collect::<String>())
        .collect();
    if expected.is_empty() {
        return Vec::new();
    }

    let mut positions = Vec::new();
    let mut consumed = 0usize;
    for (idx, ch) in text.chars().enumerate() {
        if consumed == expected.len() {
            break;
        }
        if ch.to_lowercase().collect::<String>() == expected[consumed] {
            positions.push(idx);
            consumed += 1;
        }
    }

    if consumed == expected.len() {
        positions
    } else {
        Vec::new()
    }
}

/// Hero-style empty state with ASCII art, contextual copy and a key-hint chip.
///
/// The art is intentionally compact (3 rows) to fit even in modest terminals;
/// the surrounding paragraph handles vertical centering.
fn build_empty_state_lines(app: &App) -> Vec<Line<'static>> {
    use crate::cli::tui::theme::key_hint;

    struct State {
        art: [&'static str; 3],
        art_style: Style,
        title: String,
        subtitle: String,
        hint_key: &'static str,
        hint_label: &'static str,
    }

    let state = if !app.search.is_empty() {
        if app.search_results.is_some() {
            State {
                art: ["  ╭─── ◯ ───╮  ", "  │  no hits │  ", "  ╰─────────╯  "],
                art_style: muted(),
                title: format!("No provider results for '{}'", app.search),
                subtitle: "Try a broader term, or clear the search.".to_string(),
                hint_key: "Esc",
                hint_label: "back to local",
            }
        } else {
            State {
                art: ["   ╭───────╮   ", "   │  ?  ? │   ", "   ╰───────╯   "],
                art_style: dim(),
                title: format!("No local matches for '{}'", app.search),
                subtitle: "Search online providers or clear the query.".to_string(),
                hint_key: "Enter",
                hint_label: "search providers",
            }
        }
    } else if app.filter == Filter::Updates {
        State {
            art: ["    ╱◉◉◉╲    ", "   ╱ all ╲   ", "   ╲ ok  ╱   "],
            art_style: success(),
            title: "No updates available".to_string(),
            subtitle: "Every installed package is on its latest version.".to_string(),
            hint_key: "2",
            hint_label: "view installed",
        }
    } else if app.filter == Filter::Favorites {
        if app.favorites_updates_only {
            State {
                art: ["    ★ ☆ ★    ", "   all clear  ", "     ✓ ✓      "],
                art_style: warning(),
                title: "No favorite has a pending update".to_string(),
                subtitle: "Your starred packages are all current.".to_string(),
                hint_key: "v",
                hint_label: "show all favorites",
            }
        } else {
            State {
                art: ["     ☆        ", "    ☆ ☆       ", "   ☆ ☆ ☆      "],
                art_style: dim(),
                title: "No favorites yet".to_string(),
                subtitle: "Star a package to pin it here and watch for updates.".to_string(),
                hint_key: "f",
                hint_label: "favorite selected",
            }
        }
    } else if app.filter == Filter::SecurityUpdates {
        State {
            art: ["   ╔═══════╗   ", "   ║ ✓ safe║   ", "   ╚═══════╝   "],
            art_style: success(),
            title: "No security updates pending".to_string(),
            subtitle: "All installed packages are secure.".to_string(),
            hint_key: "1",
            hint_label: "browse catalogue",
        }
    } else if app.filter == Filter::Duplicates {
        State {
            art: ["   ╭─╮ ╭─╮    ", "   │·│ │·│    ", "   ╰─╯ ╰─╯    "],
            art_style: muted(),
            title: "No duplicate installations".to_string(),
            subtitle: "Each package is installed from just one source.".to_string(),
            hint_key: "1",
            hint_label: "view all",
        }
    } else {
        State {
            art: ["   ┌──◆──┐   ", "   │ ... │   ", "   └─────┘   "],
            art_style: muted(),
            title: "No packages match this filter".to_string(),
            subtitle: "Try a different view or broaden your search.".to_string(),
            hint_key: "1",
            hint_label: "view all",
        }
    };

    vec![
        Line::from(Span::styled(state.art[0].to_string(), state.art_style)),
        Line::from(Span::styled(state.art[1].to_string(), state.art_style)),
        Line::from(Span::styled(state.art[2].to_string(), state.art_style)),
        Line::from(Span::raw("")),
        Line::from(Span::styled(state.title, muted())),
        Line::from(Span::raw("")),
        Line::from(vec![Span::styled(state.subtitle, dim())]),
        Line::from(Span::raw("")),
        Line::from(vec![
            Span::styled("[", dim()),
            Span::styled(state.hint_key, key_hint()),
            Span::styled("] ", dim()),
            Span::styled(state.hint_label, muted()),
        ]),
    ]
}
