use crate::cli::tui::app::App;
use crate::cli::tui::format::{
    format_package_version, package_status_short, truncate_middle_to_width, truncate_to_width,
};
use crate::cli::tui::state::filters::{Filter, Focus};
use crate::cli::tui::theme::{
    accent, dim, error, loading, muted, palette, row_cursor, scrollbar_style, scrollbar_thumb,
    source_color, success, text, warning,
};
use crate::cli::tui::ui::{panel_block, window_start};
use crate::models::{Package, PackageStatus, UpdateCategory};
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
        let marker = if is_cursor {
            ">"
        } else if is_selected {
            "x"
        } else {
            " "
        };
        let favorite_marker = if is_favorite { " ★" } else { "" };
        let display_version = format_package_version(package);
        let current = if package.status == PackageStatus::UpdateAvailable {
            package.version.clone()
        } else {
            display_version
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
                        truncate_middle_to_width(&package.name, 26),
                        also.join(", ")
                    )
                } else {
                    truncate_middle_to_width(&package.name, 32)
                }
            } else {
                truncate_middle_to_width(&package.name, 32)
            }
        } else {
            truncate_middle_to_width(&package.name, 32)
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

        let (_, status_style) = package_status_short(package.status);
        let (risk, risk_style) = risk_cell(package);
        let (action, action_style) = action_cell(package.status);

        rows.push(
            Row::new(vec![
                Cell::from(Span::styled(marker, row_style)),
                Cell::from(Line::from(name_spans)),
                Cell::from(Span::styled(truncate_to_width(&current, 14), row_style)),
                Cell::from(Span::styled(
                    truncate_to_width(&available, 14),
                    if is_cursor {
                        row_style
                    } else {
                        Style::default().fg(palette::ORANGE())
                    },
                )),
                Cell::from(Span::styled(
                    truncate_to_width(&source, 10),
                    if is_cursor {
                        row_style
                    } else {
                        source_color(package.source)
                    },
                )),
                Cell::from(Span::styled(
                    risk,
                    if is_cursor {
                        row_style
                    } else if package.status == PackageStatus::UpdateAvailable {
                        risk_style
                    } else {
                        status_style
                    },
                )),
                Cell::from(Span::styled(
                    action,
                    if is_cursor { row_style } else { action_style },
                )),
            ])
            .style(row_style),
        );
    }

    let header = Row::new(vec![
        "",
        "Package",
        "Current",
        "Available",
        "Source",
        "Risk",
        "Action",
    ])
    .style(dim().add_modifier(Modifier::BOLD))
    .bottom_margin(1);
    let widths = [
        Constraint::Length(1),
        Constraint::Min(24),
        Constraint::Length(14),
        Constraint::Length(14),
        Constraint::Length(11),
        Constraint::Length(12),
        Constraint::Length(12),
    ];

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

fn risk_cell(package: &Package) -> (&'static str, Style) {
    let category = package
        .update_category
        .unwrap_or_else(|| package.detect_update_category());
    match category {
        UpdateCategory::Security => ("security", error()),
        _ if package.status == PackageStatus::UpdateAvailable => ("routine", success()),
        _ => ("routine", success()),
    }
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
    let gap = width.saturating_sub(left.len() + right.len());
    Line::from(vec![
        Span::styled(left, dim().add_modifier(Modifier::ITALIC)),
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
