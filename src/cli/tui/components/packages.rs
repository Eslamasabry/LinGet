use crate::cli::tui::app::App;
use crate::cli::tui::format::{
    format_package_version, package_status_short, truncate_middle_to_width, truncate_to_width,
};
use crate::cli::tui::state::filters::{Filter, Focus};
use crate::cli::tui::theme::{
    accent, dim, error, loading, muted, row_cursor, row_selected, scrollbar_style, scrollbar_thumb,
    source_color, success, table_header_band, text, warning,
};
use crate::cli::tui::ui::{panel_block, window_start};
use crate::models::{Package, PackageStatus, UpdateCategory};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Cell, Paragraph, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Table},
    Frame,
};

pub fn draw_packages_panel(frame: &mut Frame, app: &App, area: Rect, compact: bool) {
    let focused = app.focus == Focus::Packages && !app.queue_expanded;
    let position = if app.filtered.is_empty() {
        0
    } else {
        app.cursor + 1
    };
    let version_width = if compact { 13 } else { 16 };
    let source_width = if compact { 8 } else { 9 };
    let status_width = if compact { 7 } else { 10 };
    let title = packages_panel_title(app, position);
    let block = panel_block(title, focused, compact);

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
    let visible_rows = inner.height.saturating_sub(2) as usize;
    let start = window_start(app.filtered.len(), visible_rows.max(1), app.cursor);
    let end = (start + visible_rows.max(1)).min(app.filtered.len());

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
        let row_style = if is_cursor { row_cursor() } else { text() };

        let marker = if is_selected { "☑" } else { " " };
        let favorite_marker = if is_favorite { "★" } else { " " };
        let version = format_package_version(package);
        let source = app.package_source_badge(package);
        let status = package_status_short(package.status);
        let status_cell = if !compact && package.status == PackageStatus::UpdateAvailable {
            let category = package.detect_update_category();
            let badge_spans: Vec<Span> = match category {
                UpdateCategory::Security => vec![
                    Span::styled(status.0, status.1),
                    Span::raw(" "),
                    Span::styled("[sec]", error()),
                ],
                UpdateCategory::Bugfix => vec![
                    Span::styled(status.0, status.1),
                    Span::raw(" "),
                    Span::styled("[fix]", warning()),
                ],
                UpdateCategory::Feature => vec![
                    Span::styled(status.0, status.1),
                    Span::raw(" "),
                    Span::styled("[new]", success()),
                ],
                UpdateCategory::Minor => vec![Span::styled(status.0, status.1)],
            };
            Cell::from(Line::from(badge_spans))
        } else {
            Cell::from(Span::styled(status.0, status.1))
        };

        // In Duplicates filter, append "also: X" hint to the name
        let display_name = if app.filter == Filter::Duplicates {
            if let Some(peers) = app.duplicate_peer_sources.get(&package_id) {
                if !peers.is_empty() {
                    let also: Vec<String> = peers.iter().map(|s| s.to_string()).collect();
                    format!(
                        "{} · also: {}",
                        truncate_middle_to_width(&package.name, if compact { 14 } else { 24 }),
                        also.join(", ")
                    )
                } else {
                    truncate_middle_to_width(&package.name, if compact { 20 } else { 32 })
                }
            } else {
                truncate_middle_to_width(&package.name, if compact { 20 } else { 32 })
            }
        } else {
            truncate_middle_to_width(&package.name, if compact { 20 } else { 32 })
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

        rows.push(
            Row::new(vec![
                Cell::from(Span::styled(
                    marker,
                    if is_selected { row_selected() } else { text() },
                )),
                Cell::from(Span::styled(
                    favorite_marker,
                    if is_favorite {
                        if is_cursor {
                            row_style
                        } else {
                            warning()
                        }
                    } else {
                        row_style
                    },
                )),
                Cell::from(Line::from(name_spans)),
                Cell::from(Span::styled(
                    truncate_to_width(&version, version_width),
                    row_style,
                )),
                Cell::from(Span::styled(
                    truncate_to_width(&source, source_width),
                    if is_cursor {
                        row_style
                    } else {
                        source_color(package.source)
                    },
                )),
                status_cell,
            ])
            .style(row_style),
        );
    }

    let header =
        Row::new(vec!["", "★", "Name", "Version", "Source", "Status"]).style(table_header_band());
    let widths = [
        Constraint::Length(2),
        Constraint::Length(2),
        Constraint::Min(if compact { 12 } else { 24 }),
        Constraint::Length(version_width as u16),
        Constraint::Length(source_width as u16),
        Constraint::Length(status_width as u16),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(block)
        .column_spacing(1);
    frame.render_widget(table, area);

    if app.filtered.len() > visible_rows {
        let mut scrollbar_state = ScrollbarState::new(app.filtered.len()).position(app.cursor);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .style(scrollbar_style())
            .thumb_style(scrollbar_thumb());
        frame.render_stateful_widget(
            scrollbar,
            area.inner(ratatui::layout::Margin {
                vertical: 1,
                horizontal: 0,
            }),
            &mut scrollbar_state,
        );
    }
}

fn packages_panel_title(app: &App, position: usize) -> String {
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

    if app.filtered.is_empty() {
        format!(" {} ", label)
    } else {
        format!(" {} · {}/{} ", label, position, app.filtered.len())
    }
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
                title: "No favourite has a pending update".to_string(),
                subtitle: "Your starred packages are all current.".to_string(),
                hint_key: "v",
                hint_label: "show all favourites",
            }
        } else {
            State {
                art: ["     ☆        ", "    ☆ ☆       ", "   ☆ ☆ ☆      "],
                art_style: dim(),
                title: "No favourites yet".to_string(),
                subtitle: "Star a package to pin it here and watch for updates.".to_string(),
                hint_key: "f",
                hint_label: "favourite selected",
            }
        }
    } else if app.filter == Filter::SecurityUpdates {
        State {
            art: ["   ╔═══════╗   ", "   ║  🛡  safe║   ", "   ╚═══════╝   "],
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
