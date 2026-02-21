use crate::cli::tui::app::App;
use crate::cli::tui::state::filters::{Filter, Focus};
use crate::cli::tui::theme::{
    dim, error, loading, muted, row_cursor, row_selected, scrollbar_style, scrollbar_thumb,
    source_color, success, table_header, text, warning,
};
use crate::cli::tui::ui::{
    format_package_version, package_status_short, panel_block, truncate_middle_to_width,
    truncate_to_width, window_start,
};
use crate::models::{PackageStatus, UpdateCategory};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
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
    let title = format!(" Packages ({}/{}) ", position, app.filtered.len());
    let block = panel_block(title, focused, compact);

    if app.loading && app.filtered.is_empty() {
        let paragraph = Paragraph::new(format!("{} Loading packages...", app.spinner_frame()))
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
        let lines = if !app.search.is_empty() {
            vec![
                Line::from(Span::styled(
                    format!("No results for '{}'", app.search),
                    muted(),
                )),
                Line::from(Span::styled("Press Esc to clear search", dim())),
            ]
        } else if app.filter == Filter::Updates {
            vec![
                Line::from(Span::styled("No updates available", muted())),
                Line::from(Span::styled("All packages are current", dim())),
            ]
        } else if app.filter == Filter::Favorites {
            if app.favorites_updates_only {
                vec![
                    Line::from(Span::styled("No favorite updates available", muted())),
                    Line::from(Span::styled("Press v to show all favorites", dim())),
                ]
            } else {
                vec![
                    Line::from(Span::styled("No favorites yet", muted())),
                    Line::from(Span::styled(
                        "Press f to favorite the selected package",
                        dim(),
                    )),
                ]
            }
        } else if app.filter == Filter::SecurityUpdates {
            vec![
                Line::from(Span::styled("No security updates available", muted())),
                Line::from(Span::styled("All packages are secure", dim())),
            ]
        } else {
            vec![Line::from(Span::styled(
                "No packages match this filter",
                muted(),
            ))]
        };
        let paragraph = Paragraph::new(lines)
            .style(text())
            .alignment(ratatui::layout::Alignment::Center);

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let vertical_padding = inner.height.saturating_sub(2) / 2;
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
        let source = package.source.to_string();
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
                        truncate_middle_to_width(&package.name, if compact { 12 } else { 16 }),
                        also.join(", ")
                    )
                } else {
                    truncate_middle_to_width(&package.name, if compact { 18 } else { 24 })
                }
            } else {
                truncate_middle_to_width(&package.name, if compact { 18 } else { 24 })
            }
        } else {
            truncate_middle_to_width(&package.name, if compact { 18 } else { 24 })
        };

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
                Cell::from(if app.filter == Filter::Duplicates {
                    Line::from(vec![Span::styled(
                        display_name,
                        if is_cursor { row_cursor() } else { text() },
                    )])
                } else {
                    Line::from(Span::styled(display_name, row_style))
                }),
                Cell::from(Span::styled(
                    truncate_to_width(&version, if compact { 15 } else { 22 }),
                    row_style,
                )),
                Cell::from(Span::styled(
                    truncate_to_width(&source, 10),
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
        Row::new(vec!["", "★", "Name", "Version", "Source", "Status"]).style(table_header());
    let widths = [
        Constraint::Length(2),
        Constraint::Length(2),
        Constraint::Min(if compact { 11 } else { 20 }),
        Constraint::Min(if compact { 10 } else { 16 }),
        Constraint::Length(10),
        Constraint::Length(if compact { 5 } else { 11 }),
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
