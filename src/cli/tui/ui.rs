use super::app::{action_label, App, Filter, Focus, MIN_HEIGHT, MIN_WIDTH};
use super::theme::*;
use super::update_center;
use crate::models::history::{TaskQueueAction, TaskQueueEntry, TaskQueueStatus};
use crate::models::{Package, PackageStatus};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Cell, Clear, Gauge, List, ListItem, Paragraph, Row, Scrollbar,
        ScrollbarOrientation, ScrollbarState, Table, Wrap,
    },
    Frame,
};
use unicode_width::UnicodeWidthStr;

pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();
    if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
        draw_too_small(frame, area);
        return;
    }

    let queue_height = if app.should_show_queue_bar() { 1 } else { 0 };
    let constraints = vec![
        Constraint::Length(2),
        Constraint::Min(1),
        Constraint::Length(queue_height),
        Constraint::Length(1),
    ];
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    let header_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(chunks[0]);
    draw_filter_bar(frame, app, header_chunks[0]);
    draw_status_legend(frame, header_chunks[1]);
    draw_main_content(frame, app, chunks[1]);

    let footer_chunk = if queue_height == 1 {
        draw_queue_bar(frame, app, chunks[2]);
        chunks[3]
    } else {
        chunks[2]
    };
    draw_footer(frame, app, footer_chunk);

    if app.showing_help {
        draw_help_overlay(frame);
    }
}

fn draw_too_small(frame: &mut Frame, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_set(ROUNDED)
        .title(" LinGet ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines = vec![
        Line::from(Span::styled("Terminal too small", error())),
        Line::from(Span::styled(
            format!("Minimum size: {}x{}", MIN_WIDTH, MIN_HEIGHT),
            muted(),
        )),
    ];
    let paragraph = Paragraph::new(lines)
        .alignment(ratatui::layout::Alignment::Center)
        .wrap(Wrap { trim: true });
    frame.render_widget(paragraph, inner);
}

fn draw_filter_bar(frame: &mut Frame, app: &App, area: Rect) {
    let mut left: Vec<Span> = vec![
        Span::styled(
            " ◆ ",
            Style::default()
                .fg(palette::CYAN)
                .bg(palette::HEADER_BG)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "LinGet ",
            Style::default()
                .fg(palette::WHITE)
                .bg(palette::HEADER_BG)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" ", header_bar()),
    ];
    let installed_label = if app.compact { "Inst" } else { "Installed" };
    let updates_label = if app.compact { "Upd" } else { "Updates" };

    left.extend(render_filter_tab(
        "1",
        "All",
        app.filter_counts[0],
        app.filter == Filter::All,
        app.searching,
    ));
    left.push(Span::raw(" "));
    left.extend(render_filter_tab(
        "2",
        installed_label,
        app.filter_counts[1],
        app.filter == Filter::Installed,
        app.searching,
    ));
    left.push(Span::raw(" "));
    left.extend(render_filter_tab(
        "3",
        updates_label,
        app.filter_counts[2],
        app.filter == Filter::Updates,
        app.searching,
    ));

    let right = if app.searching {
        vec![Span::styled(
            format!(
                "/ {}█",
                render_search_input(&app.search, area.width as usize / 3)
            ),
            accent(),
        )]
    } else if !app.search.is_empty() {
        vec![Span::styled(format!("/ \"{}\"", app.search), muted())]
    } else if app.loading {
        vec![Span::styled(
            format!("{} refreshing", app.spinner_frame()),
            loading(),
        )]
    } else if !app.status.is_empty() {
        vec![Span::styled(app.status.clone(), italic_status())]
    } else {
        Vec::new()
    };

    let line = compose_left_right(left, right, area.width as usize);
    let paragraph = Paragraph::new(line).style(header_bar());
    frame.render_widget(paragraph, area);
}

fn draw_status_legend(frame: &mut Frame, area: Rect) {
    let legend = Line::from(vec![
        Span::styled("Status ", footer_label()),
        Span::styled(" ✓ ", badge_installed()),
        Span::styled("installed  ", muted()),
        Span::styled(" ↑ ", badge_update()),
        Span::styled("updates  ", muted()),
        Span::styled(" ⟳ ", badge_progress()),
        Span::styled("in-progress  ", muted()),
        Span::styled(" ○ ", badge_not_installed()),
        Span::styled("available", muted()),
    ]);
    frame.render_widget(Paragraph::new(legend).style(header_bar()), area);
}

fn render_filter_tab(
    key: &str,
    label: &str,
    count: usize,
    active: bool,
    searching: bool,
) -> Vec<Span<'static>> {
    if active && !searching {
        vec![
            Span::styled(" ", tab_active()),
            Span::styled(
                key.to_string(),
                Style::default()
                    .fg(palette::YELLOW)
                    .bg(palette::TAB_ACTIVE_BG)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" ", tab_active()),
            Span::styled(label.to_string(), tab_active()),
            Span::styled(" ", tab_active()),
            Span::styled(count.to_string(), tab_active()),
            Span::styled(" ", tab_active()),
        ]
    } else {
        vec![
            Span::styled(" ", header_bar()),
            Span::styled(
                key.to_string(),
                Style::default()
                    .fg(palette::DARK_GRAY)
                    .bg(palette::HEADER_BG),
            ),
            Span::styled(" ", header_bar()),
            Span::styled(
                label.to_string(),
                Style::default()
                    .fg(palette::DARK_GRAY)
                    .bg(palette::HEADER_BG),
            ),
            Span::styled(" ", header_bar()),
            Span::styled(
                count.to_string(),
                Style::default()
                    .fg(palette::DARK_GRAY)
                    .bg(palette::HEADER_BG),
            ),
            Span::styled(" ", header_bar()),
        ]
    }
}

fn compose_left_right<'a>(mut left: Vec<Span<'a>>, right: Vec<Span<'a>>, width: usize) -> Line<'a> {
    let left_width = spans_width(&left);
    let right_width = spans_width(&right);

    if right.is_empty() {
        return Line::from(left);
    }

    if left_width + right_width + 1 > width {
        return Line::from(left);
    }

    let padding = width.saturating_sub(left_width + right_width);
    left.push(Span::raw(" ".repeat(padding)));
    left.extend(right);

    Line::from(left)
}

fn spans_width(spans: &[Span<'_>]) -> usize {
    spans
        .iter()
        .map(|span| UnicodeWidthStr::width(span.content.as_ref()))
        .sum()
}

fn render_search_input(query: &str, max_width: usize) -> String {
    if UnicodeWidthStr::width(query) <= max_width {
        return query.to_string();
    }
    if max_width <= 3 {
        return "...".to_string();
    }

    let mut out = String::new();
    let mut width = 0usize;
    let target = max_width - 3;

    for ch in query.chars().rev() {
        let char_width = UnicodeWidthStr::width(ch.to_string().as_str());
        if width + char_width > target {
            break;
        }
        out.insert(0, ch);
        width += char_width;
    }

    format!("...{}", out)
}

fn draw_main_content(frame: &mut Frame, app: &App, area: Rect) {
    if app.compact {
        draw_compact_content(frame, app, area);
    } else {
        draw_full_content(frame, app, area);
    }
}

fn draw_full_content(frame: &mut Frame, app: &App, area: Rect) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(18), Constraint::Min(1)])
        .split(area);

    draw_sources_panel(frame, app, columns[0]);

    let right = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(65), Constraint::Percentage(35)])
        .split(columns[1]);

    draw_packages_panel(frame, app, right[0], false);

    if app.queue_expanded {
        draw_expanded_queue(frame, app, right[1]);
    } else {
        draw_details_panel(frame, app, right[1]);
    }
}

fn draw_compact_content(frame: &mut Frame, app: &App, area: Rect) {
    if app.queue_expanded {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(area);
        draw_packages_panel(frame, app, chunks[0], true);
        draw_expanded_queue(frame, app, chunks[1]);
        return;
    }

    if area.height < 4 {
        draw_packages_panel(frame, app, area, true);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(2)])
        .split(area);
    draw_packages_panel(frame, app, chunks[0], true);
    draw_compact_details_summary(frame, app, chunks[1]);
}

fn panel_block(title: String, focused: bool) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_set(ROUNDED)
        .border_style(if focused {
            border_focused()
        } else {
            border_unfocused()
        })
        .title(title)
        .title_style(if focused { accent() } else { text() })
}

fn draw_sources_panel(frame: &mut Frame, app: &App, area: Rect) {
    let focused = app.focus == Focus::Sources && !app.queue_expanded;
    let block = panel_block(" Sources ".to_string(), focused);
    let inner = block.inner(area);

    let visible = app.visible_sources();
    let total = visible.len() + 1;
    let selected = app.source_index();
    let visible_rows = inner.height as usize;
    let start = window_start(total, visible_rows, selected);
    let end = (start + visible_rows).min(total);

    let mut items = Vec::new();
    for idx in start..end {
        let selected_row = idx == selected;
        let (label, label_style) = if idx == 0 {
            let count_str = source_count_label(
                app.filter,
                (
                    app.filter_counts[0],
                    app.filter_counts[1],
                    app.filter_counts[2],
                ),
            );
            (
                format!("All{}", count_str),
                if selected_row { accent() } else { text() },
            )
        } else {
            let source = visible[idx - 1];
            let counts = app.source_counts.get(&source).copied().unwrap_or((0, 0, 0));
            let count_str = source_count_label(app.filter, counts);
            (
                format!("{}{}", source, count_str),
                if selected_row {
                    accent()
                } else {
                    source_color(source)
                },
            )
        };
        items.push(ListItem::new(Line::from(vec![
            Span::styled(if selected_row { "▸ " } else { "  " }, row_selected()),
            Span::styled(label, label_style),
        ])));
    }

    let list = List::new(items).block(block);
    frame.render_widget(list, area);

    if total > visible_rows {
        let mut scrollbar_state = ScrollbarState::new(total).position(selected);
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

fn source_count_label(filter: Filter, counts: (usize, usize, usize)) -> String {
    match filter {
        Filter::All => {
            if counts.2 > 0 {
                format!(" {} (+{})", counts.0, counts.2)
            } else {
                format!(" {}", counts.0)
            }
        }
        Filter::Installed => format!(" {}", counts.1),
        Filter::Updates => format!(" {}", counts.2),
    }
}

fn draw_packages_panel(frame: &mut Frame, app: &App, area: Rect, compact: bool) {
    let focused = app.focus == Focus::Packages && !app.queue_expanded;
    let position = if app.filtered.is_empty() {
        0
    } else {
        app.cursor + 1
    };
    let title = format!(" Packages ({}/{}) ", position, app.filtered.len());
    let block = panel_block(title, focused);

    if app.loading && app.filtered.is_empty() {
        let paragraph = Paragraph::new(format!("{} Loading packages...", app.spinner_frame()))
            .block(block)
            .style(loading());
        frame.render_widget(paragraph, area);
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
        } else {
            vec![Line::from(Span::styled(
                "No packages match this filter",
                muted(),
            ))]
        };
        let paragraph = Paragraph::new(lines).block(block).style(text());
        frame.render_widget(paragraph, area);
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
        let Some(package) = app.packages.get(package_index) else {
            continue;
        };

        let package_id = package.id();
        let is_cursor = row_index == app.cursor;
        let is_selected = app.selected.contains(&package_id);
        let row_style = if is_cursor { row_cursor() } else { text() };

        let marker = if is_selected { "☑" } else { " " };
        let version = format_package_version(package);
        let source = package.source.to_string();
        let status = package_status_short(package.status);

        rows.push(
            Row::new(vec![
                Cell::from(Span::styled(
                    marker,
                    if is_selected { row_selected() } else { text() },
                )),
                Cell::from(Span::styled(
                    truncate_middle_to_width(&package.name, if compact { 20 } else { 26 }),
                    row_style,
                )),
                Cell::from(Span::styled(
                    truncate_to_width(&version, if compact { 16 } else { 24 }),
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
                Cell::from(Span::styled(status.0, status.1)),
            ])
            .style(row_style),
        );
    }

    let header = Row::new(vec!["", "Name", "Version", "Source", "Status"]).style(table_header());
    let widths = [
        Constraint::Length(2),
        Constraint::Min(if compact { 12 } else { 22 }),
        Constraint::Min(if compact { 10 } else { 18 }),
        Constraint::Length(10),
        Constraint::Length(5),
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

fn draw_details_panel(frame: &mut Frame, app: &App, area: Rect) {
    let block = panel_block(" Details ".to_string(), false);

    if app.loading && app.current_package().is_none() {
        frame.render_widget(
            Paragraph::new("Loading details...")
                .block(block)
                .style(loading()),
            area,
        );
        return;
    }

    let Some(package) = app.current_package() else {
        frame.render_widget(
            Paragraph::new("Select a package").block(block).style(dim()),
            area,
        );
        return;
    };

    let mut lines = vec![
        Line::from(vec![
            Span::styled("Name: ", dim()),
            Span::styled(package.name.clone(), accent()),
        ]),
        Line::from(vec![
            Span::styled("Version: ", dim()),
            Span::styled(format_package_version(package), text()),
        ]),
        Line::from(vec![
            Span::styled("Source: ", dim()),
            Span::styled(package.source.to_string(), source_color(package.source)),
        ]),
    ];

    if package.status == PackageStatus::UpdateAvailable || package.status == PackageStatus::Updating
    {
        if let Some(priority) = update_priority_label(package) {
            lines.push(Line::from(vec![
                Span::styled("Priority: ", dim()),
                Span::styled(priority, warning()),
            ]));
        }
    }

    if matches!(
        package.status,
        PackageStatus::Installing | PackageStatus::Removing | PackageStatus::Updating
    ) {
        lines.push(Line::from(vec![
            Span::styled("Status: ", dim()),
            Span::styled("Operation in progress...", loading()),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled("Description:", dim())));

    let description_width = area.width.saturating_sub(4) as usize;
    for line in wrap_text(&package.description, description_width) {
        lines.push(Line::from(Span::styled(line, muted())));
    }

    frame.render_widget(
        Paragraph::new(lines).block(block).wrap(Wrap { trim: true }),
        area,
    );
}

fn draw_compact_details_summary(frame: &mut Frame, app: &App, area: Rect) {
    let Some(package) = app.current_package() else {
        frame.render_widget(Paragraph::new("No package selected").style(dim()), area);
        return;
    };

    let first = format!(
        "{} {} ({})",
        package.name,
        format_package_version(package),
        package.source
    );
    let second = truncate_to_width(&package.description, area.width as usize);

    let lines = vec![
        Line::from(Span::styled(first, text())),
        Line::from(Span::styled(second, muted())),
    ];
    frame.render_widget(Paragraph::new(lines), area);
}

fn draw_queue_bar(frame: &mut Frame, app: &App, area: Rect) {
    if app.tasks.is_empty() {
        return;
    }

    let (queued, running, completed, failed, cancelled) = app.queue_counts();
    let total = app.tasks.len();
    let done = completed + failed + cancelled;
    let performance_hint = queue_performance_hint(&app.tasks, total.saturating_sub(done));

    if running > 0 {
        let active_label = app
            .tasks
            .iter()
            .find(|task| task.status == TaskQueueStatus::Running)
            .map(|task| format!("{} {}", action_label(task.action), task.package_name))
            .unwrap_or_else(|| "Working".to_string());
        let progressed = (done + running).min(total);
        let ratio = if total > 0 {
            (progressed as f64 / total as f64).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let label_text = build_running_queue_label(
            &active_label,
            progressed,
            total,
            failed,
            performance_hint.as_deref(),
        );
        let gauge_style = if failed > 0 {
            gauge_failed()
        } else {
            gauge_bar()
        };
        let gauge = Gauge::default()
            .gauge_style(gauge_style)
            .label(Span::styled(
                truncate_to_width(&label_text, area.width.saturating_sub(1) as usize),
                text(),
            ))
            .ratio(ratio);
        frame.render_widget(gauge, area);
    } else {
        let (message, state) = build_idle_queue_label(
            queued,
            completed,
            cancelled,
            failed,
            done,
            total,
            performance_hint.as_deref(),
        );
        let style = match state {
            QueueBarState::Queued => muted(),
            QueueBarState::Failed => error(),
            QueueBarState::Complete => success(),
        };
        let line = truncate_to_width(&message, area.width as usize);
        frame.render_widget(Paragraph::new(line).style(style), area);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum QueueBarState {
    Queued,
    Failed,
    Complete,
}

fn build_running_queue_label(
    active_label: &str,
    progressed: usize,
    total: usize,
    failed: usize,
    performance_hint: Option<&str>,
) -> String {
    let mut text_value = if failed > 0 {
        format!(
            "▶ {} ({}/{}) · {} failed",
            active_label, progressed, total, failed
        )
    } else {
        format!("▶ {} ({}/{})", active_label, progressed, total)
    };
    if let Some(hint) = performance_hint {
        text_value.push_str(" • ");
        text_value.push_str(hint);
    }
    text_value.push_str("  [l]");
    text_value
}

fn build_idle_queue_label(
    queued: usize,
    completed: usize,
    cancelled: usize,
    failed: usize,
    done: usize,
    total: usize,
    performance_hint: Option<&str>,
) -> (String, QueueBarState) {
    if queued > 0 {
        let mut text_value = format!("◻ {} tasks queued", queued);
        if let Some(hint) = performance_hint {
            text_value.push_str(" • ");
            text_value.push_str(hint);
        }
        text_value.push_str("  [l expand]");
        return (text_value, QueueBarState::Queued);
    }

    if failed > 0 {
        let mut text_value = format!("⚠ {} done, {} failed", completed + cancelled, failed);
        if let Some(hint) = performance_hint {
            text_value.push_str(" • ");
            text_value.push_str(hint);
        }
        text_value.push_str("  [l details]");
        return (text_value, QueueBarState::Failed);
    }

    let mut text_value = format!("✓ {}/{} complete", done, total);
    if let Some(hint) = performance_hint {
        text_value.push_str(" • ");
        text_value.push_str(hint);
    }
    text_value.push_str("  [l details]");
    (text_value, QueueBarState::Complete)
}

fn queue_performance_hint(tasks: &[TaskQueueEntry], remaining: usize) -> Option<String> {
    const MAX_SAMPLES: usize = 8;
    const MIN_SAMPLES: usize = 2;

    let mut duration_secs = Vec::new();
    for task in tasks.iter().rev() {
        if task.status != TaskQueueStatus::Completed {
            continue;
        }
        let (Some(started_at), Some(completed_at)) =
            (task.started_at.as_ref(), task.completed_at.as_ref())
        else {
            continue;
        };
        let elapsed_ms = completed_at
            .signed_duration_since(*started_at)
            .num_milliseconds();
        if elapsed_ms <= 0 {
            continue;
        }
        duration_secs.push(elapsed_ms as f64 / 1000.0);
        if duration_secs.len() >= MAX_SAMPLES {
            break;
        }
    }

    if duration_secs.len() < MIN_SAMPLES {
        return None;
    }

    let avg_secs = duration_secs.iter().sum::<f64>() / duration_secs.len() as f64;
    if avg_secs <= 0.0 {
        return None;
    }

    let throughput = 60.0 / avg_secs;
    if remaining == 0 {
        return Some(format!("{:.1} t/m", throughput));
    }

    let eta_secs = (avg_secs * remaining as f64).round().max(1.0) as u64;
    Some(format!(
        "{:.1} t/m • ETA {}",
        throughput,
        format_eta_seconds(eta_secs)
    ))
}

fn format_eta_seconds(total_seconds: u64) -> String {
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;
    if hours > 0 {
        return format!("{}h{:02}m", hours, minutes);
    }
    if minutes > 0 {
        return format!("{}m{:02}s", minutes, seconds);
    }
    format!("{}s", seconds)
}

fn draw_footer(frame: &mut Frame, app: &App, area: Rect) {
    if let Some(confirming) = &app.confirming {
        let verb_style = match confirming.action {
            TaskQueueAction::Install => success(),
            TaskQueueAction::Remove => error(),
            TaskQueueAction::Update => warning(),
        };
        let line = Line::from(vec![
            Span::styled(
                confirming.label.clone(),
                verb_style.add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled("[y]", key_hint()),
            Span::styled(" yes  ", footer_label()),
            Span::styled("[n]", key_hint()),
            Span::styled(" cancel", footer_label()),
        ]);
        frame.render_widget(Paragraph::new(line), area);
        return;
    }

    let mut spans = Vec::new();
    push_hint(&mut spans, "↑↓", "nav");
    push_hint(&mut spans, "/", "search");
    push_hint(&mut spans, "Space", "sel");
    push_hint(&mut spans, "a", "all");
    push_hint(&mut spans, "i", "inst");
    push_hint(&mut spans, "x", "rm");
    push_hint(&mut spans, "u", "upd");
    push_hint(&mut spans, "q", "quit");

    let selection = if app.hidden_selected_count() > 0 {
        format!(
            "{} selected ({} hidden)",
            app.selected.len(),
            app.hidden_selected_count()
        )
    } else {
        format!("{} selected", app.selected.len())
    };

    let right = if app.compact && !app.status.is_empty() {
        vec![Span::styled(app.status.clone(), italic_status())]
    } else {
        vec![Span::styled(selection, muted())]
    };

    let line = compose_left_right(spans, right, area.width as usize);
    frame.render_widget(Paragraph::new(line), area);
}

fn push_hint(spans: &mut Vec<Span<'static>>, key: &'static str, label: &'static str) {
    if !spans.is_empty() {
        spans.push(Span::raw("  "));
    }
    spans.push(Span::styled(key, key_hint()));
    spans.push(Span::raw(" "));
    spans.push(Span::styled(label, footer_label()));
}

fn draw_expanded_queue(frame: &mut Frame, app: &App, area: Rect) {
    let focused = app.focus == Focus::Queue;
    let block = panel_block(
        format!(" Task Queue ({}) [l close] ", app.tasks.len()),
        focused,
    );

    if app.tasks.is_empty() {
        frame.render_widget(Paragraph::new("No tasks").block(block).style(dim()), area);
        return;
    }

    let inner = block.inner(area);
    let sections = if inner.height >= 8 {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(55),
                Constraint::Percentage(35),
                Constraint::Length(1),
            ])
            .split(inner)
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(inner)
    };

    let visible = sections[0].height as usize;
    let start = window_start(app.tasks.len(), visible.max(1), app.task_cursor);
    let end = (start + visible.max(1)).min(app.tasks.len());
    let task_width = sections[0].width.saturating_sub(2) as usize;

    let mut task_lines = Vec::new();
    for idx in start..end {
        let task = &app.tasks[idx];
        let selected = idx == app.task_cursor;
        task_lines.push(render_task_line(task, selected, task_width));
    }
    frame.render_widget(Paragraph::new(task_lines), sections[0]);

    let mut logs_scroll = 0usize;
    if sections.len() == 3 {
        if let Some(task) = app.tasks.get(app.task_cursor) {
            let logs = app.task_logs.get(&task.id);
            let mut lines = vec![Line::from(Span::styled("Logs:", dim()))];
            if let Some(logs) = logs {
                let max = sections[1].height.saturating_sub(1) as usize;
                let (start, end, scroll) = task_log_window(logs.len(), max, app.task_log_scroll);
                logs_scroll = scroll;
                for line in logs.iter().skip(start).take(end.saturating_sub(start)) {
                    lines.push(Line::from(Span::styled(
                        truncate_to_width(line, sections[1].width as usize),
                        muted(),
                    )));
                }
            } else {
                lines.push(Line::from(Span::styled("No logs yet", dim())));
            }
            frame.render_widget(Paragraph::new(lines), sections[1]);
        }

        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("C", key_hint()),
                Span::styled(" cancel  ", footer_label()),
                Span::styled("R", key_hint()),
                Span::styled(" retry  ", footer_label()),
                Span::styled("↑↓", key_hint()),
                Span::styled(" navigate  ", footer_label()),
                Span::styled("[ ]", key_hint()),
                Span::styled(
                    if logs_scroll == 0 {
                        " logs".to_string()
                    } else {
                        format!(" logs +{}", logs_scroll)
                    },
                    footer_label(),
                ),
            ])),
            sections[2],
        );
    } else {
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("C", key_hint()),
                Span::styled(" cancel  ", footer_label()),
                Span::styled("R", key_hint()),
                Span::styled(" retry", footer_label()),
            ])),
            sections[1],
        );
    }

    frame.render_widget(block, area);
}

fn render_task_line(task: &TaskQueueEntry, selected: bool, max_width: usize) -> Line<'static> {
    let (symbol, style, status_text) = match task.status {
        TaskQueueStatus::Queued => ("◻", warning(), "queued"),
        TaskQueueStatus::Running => ("▶", loading(), "running"),
        TaskQueueStatus::Completed => ("✓", success(), "completed"),
        TaskQueueStatus::Failed => ("✗", error(), "failed"),
        TaskQueueStatus::Cancelled => ("-", dim(), "cancelled"),
    };

    let mut text_value = format!(
        "{} {}  {}  {}",
        symbol,
        task.package_name,
        action_label(task.action).to_lowercase(),
        status_text
    );
    if task.status == TaskQueueStatus::Failed {
        if let Some(error_text) = &task.error {
            text_value.push_str(&format!(": {}", error_text));
        }
    }
    let text_value = truncate_middle_to_width(&text_value, max_width);

    Line::from(vec![
        Span::styled(if selected { "▸ " } else { "  " }, row_selected()),
        Span::styled(text_value, if selected { row_cursor() } else { style }),
    ])
}

fn draw_help_overlay(frame: &mut Frame) {
    let area = centered_rect(frame.area(), 50, 80, 40, 18);
    frame.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_set(ROUNDED)
        .border_style(border_focused())
        .title(" Help ")
        .title_style(accent());

    let lines = vec![
        Line::from(Span::styled("Navigation", section_header())),
        Line::from("  ↑↓/jk  move      g/G top/bottom"),
        Line::from("  ^d/^u  half-page  Tab switch panel"),
        Line::from(""),
        Line::from(Span::styled("Filters", section_header())),
        Line::from("  1 All    2 Installed    3 Updates"),
        Line::from(""),
        Line::from(Span::styled("Actions", section_header())),
        Line::from("  Space select   a select all"),
        Line::from("  i install      x remove      u update"),
        Line::from("  Esc clear / back"),
        Line::from(""),
        Line::from(Span::styled("Global", section_header())),
        Line::from("  / search   r refresh   l queue log"),
        Line::from("  ? help     q quit"),
        Line::from(""),
        Line::from(Span::styled("Queue (expanded)", section_header())),
        Line::from("  C cancel queued   R retry failed"),
        Line::from("  [ ] logs older/newer"),
        Line::from(""),
        Line::from("  ? or Esc to close"),
    ];

    frame.render_widget(Paragraph::new(lines).block(block).style(text()), area);
}

fn centered_rect(
    area: Rect,
    width_percent: u16,
    height_percent: u16,
    min_width: u16,
    min_height: u16,
) -> Rect {
    let width = ((area.width as u32 * width_percent as u32) / 100) as u16;
    let height = ((area.height as u32 * height_percent as u32) / 100) as u16;
    let width = width.max(min_width).min(area.width);
    let height = height.max(min_height).min(area.height);

    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;

    Rect {
        x,
        y,
        width,
        height,
    }
}

fn update_priority_label(package: &Package) -> Option<&'static str> {
    let candidate = update_center::classify_updates(std::slice::from_ref(package))
        .into_iter()
        .next()?;
    let _category = candidate.category;
    Some(candidate.lane.label())
}

fn format_package_version(package: &Package) -> String {
    match package.status {
        PackageStatus::UpdateAvailable | PackageStatus::Updating => {
            let available = package.available_version.as_deref().unwrap_or("?");
            format!("{}→{}", package.version, available)
        }
        PackageStatus::NotInstalled => package
            .available_version
            .clone()
            .unwrap_or_else(|| package.version.clone()),
        _ => package.version.clone(),
    }
}

fn package_status_short(status: PackageStatus) -> (&'static str, Style) {
    match status {
        PackageStatus::Installed => (" ✓ ", badge_installed()),
        PackageStatus::UpdateAvailable => (" ↑ ", badge_update()),
        PackageStatus::NotInstalled => (" ○ ", badge_not_installed()),
        PackageStatus::Installing => (" ⟳ ", badge_progress()),
        PackageStatus::Removing => (" ⟳ ", badge_progress()),
        PackageStatus::Updating => (" ⟳ ", badge_progress()),
    }
}

pub fn window_start(total: usize, visible: usize, selected: usize) -> usize {
    if total <= visible || visible == 0 {
        return 0;
    }
    let half = visible / 2;
    let mut start = selected.saturating_sub(half);
    if start + visible > total {
        start = total - visible;
    }
    start
}

fn task_log_window(
    total_lines: usize,
    visible_lines: usize,
    scroll: usize,
) -> (usize, usize, usize) {
    if visible_lines == 0 || total_lines == 0 {
        return (0, 0, 0);
    }

    let max_scroll = total_lines.saturating_sub(visible_lines);
    let scroll = scroll.min(max_scroll);
    let end = total_lines.saturating_sub(scroll);
    let start = end.saturating_sub(visible_lines);
    (start, end, scroll)
}

pub fn truncate_to_width(text_value: &str, max_width: usize) -> String {
    if max_width == 0 {
        return String::new();
    }
    if UnicodeWidthStr::width(text_value) <= max_width {
        return text_value.to_string();
    }
    if max_width == 1 {
        return "…".to_string();
    }

    let mut out = String::new();
    let mut width = 0usize;
    let target = max_width.saturating_sub(1);
    for ch in text_value.chars() {
        let char_width = UnicodeWidthStr::width(ch.to_string().as_str());
        if width + char_width > target {
            break;
        }
        out.push(ch);
        width += char_width;
    }
    out.push('…');
    out
}

pub fn truncate_middle_to_width(text_value: &str, max_width: usize) -> String {
    if max_width == 0 {
        return String::new();
    }
    if UnicodeWidthStr::width(text_value) <= max_width {
        return text_value.to_string();
    }
    if max_width == 1 {
        return "…".to_string();
    }

    let target = max_width.saturating_sub(1);
    let left_target = target.div_ceil(2);
    let right_target = target / 2;

    let mut left = String::new();
    let mut left_width = 0usize;
    for ch in text_value.chars() {
        let char_width = UnicodeWidthStr::width(ch.to_string().as_str());
        if left_width + char_width > left_target {
            break;
        }
        left.push(ch);
        left_width += char_width;
    }

    let mut right = String::new();
    let mut right_width = 0usize;
    for ch in text_value.chars().rev() {
        let char_width = UnicodeWidthStr::width(ch.to_string().as_str());
        if right_width + char_width > right_target {
            break;
        }
        right.insert(0, ch);
        right_width += char_width;
    }

    format!("{}…{}", left, right)
}

fn wrap_text(text_value: &str, max_width: usize) -> Vec<String> {
    if text_value.trim().is_empty() {
        return vec!["No description available".to_string()];
    }
    if max_width == 0 {
        return vec![text_value.to_string()];
    }

    let mut lines = Vec::new();
    let mut current = String::new();
    let mut current_width = 0usize;

    for word in text_value.split_whitespace() {
        let word_width = UnicodeWidthStr::width(word);

        if current.is_empty() {
            if word_width <= max_width {
                current.push_str(word);
                current_width = word_width;
            } else {
                lines.push(truncate_to_width(word, max_width));
            }
            continue;
        }

        if current_width + 1 + word_width <= max_width {
            current.push(' ');
            current.push_str(word);
            current_width += 1 + word_width;
        } else {
            lines.push(current);
            if word_width <= max_width {
                current = word.to_string();
                current_width = word_width;
            } else {
                lines.push(truncate_to_width(word, max_width));
                current = String::new();
                current_width = 0;
            }
        }
    }

    if !current.is_empty() {
        lines.push(current);
    }

    lines
}

#[cfg(test)]
mod tests {
    use super::super::app::App;
    use super::*;
    use crate::backend::PackageManager;
    use crate::models::PackageSource;
    use chrono::{Duration, Local};
    use std::sync::Arc;
    use tokio::sync::Mutex;

    #[test]
    fn truncate_middle_preserves_edges() {
        let truncated = truncate_middle_to_width("super-long-package-name", 12);
        assert_eq!(truncated, "super-…-name");
    }

    #[test]
    fn truncate_middle_handles_small_width() {
        assert_eq!(truncate_middle_to_width("abcdef", 1), "…");
        assert_eq!(truncate_middle_to_width("abcdef", 2), "a…");
    }

    #[test]
    fn task_log_window_clamps_scroll_and_ranges() {
        assert_eq!(task_log_window(10, 4, 0), (6, 10, 0));
        assert_eq!(task_log_window(10, 4, 3), (3, 7, 3));
        assert_eq!(task_log_window(3, 8, 5), (0, 3, 0));
    }

    #[test]
    fn source_count_label_snapshots() {
        assert_eq!(source_count_label(Filter::All, (42, 31, 3)), " 42 (+3)");
        assert_eq!(source_count_label(Filter::All, (42, 31, 0)), " 42");
        assert_eq!(source_count_label(Filter::Installed, (42, 31, 3)), " 31");
        assert_eq!(source_count_label(Filter::Updates, (42, 31, 3)), " 3");
    }

    #[test]
    fn queue_running_label_snapshot() {
        let line = build_running_queue_label("Update vim", 3, 8, 1, Some("2.0 t/m • ETA 2m30s"));
        assert_eq!(
            line,
            "▶ Update vim (3/8) · 1 failed • 2.0 t/m • ETA 2m30s  [l]"
        );
    }

    #[test]
    fn queue_idle_label_snapshots() {
        let (queued_line, queued_state) =
            build_idle_queue_label(4, 0, 0, 0, 0, 4, Some("2.0 t/m • ETA 2m00s"));
        assert_eq!(queued_state, QueueBarState::Queued);
        assert_eq!(
            queued_line,
            "◻ 4 tasks queued • 2.0 t/m • ETA 2m00s  [l expand]"
        );

        let (failed_line, failed_state) =
            build_idle_queue_label(0, 2, 1, 1, 4, 5, Some("1.0 t/m • ETA 1m00s"));
        assert_eq!(failed_state, QueueBarState::Failed);
        assert_eq!(
            failed_line,
            "⚠ 3 done, 1 failed • 1.0 t/m • ETA 1m00s  [l details]"
        );

        let (done_line, done_state) = build_idle_queue_label(0, 3, 0, 0, 3, 3, None);
        assert_eq!(done_state, QueueBarState::Complete);
        assert_eq!(done_line, "✓ 3/3 complete  [l details]");
    }

    #[test]
    fn queue_performance_hint_snapshot() {
        let now = Local::now();
        let make_completed = |id: &str, started_secs_ago: i64, duration_secs: i64| TaskQueueEntry {
            id: id.to_string(),
            action: TaskQueueAction::Update,
            package_id: id.to_string(),
            package_name: id.to_string(),
            package_source: PackageSource::Apt,
            status: TaskQueueStatus::Completed,
            queued_at: now - Duration::seconds(started_secs_ago + 1),
            started_at: Some(now - Duration::seconds(started_secs_ago)),
            completed_at: Some(now - Duration::seconds(started_secs_ago - duration_secs)),
            error: None,
        };

        let tasks = vec![
            make_completed("a", 120, 30),
            make_completed("b", 60, 30),
            make_completed("c", 30, 30),
        ];
        assert_eq!(
            queue_performance_hint(&tasks, 3).as_deref(),
            Some("2.0 t/m • ETA 1m30s")
        );
        assert_eq!(
            queue_performance_hint(&tasks, 0).as_deref(),
            Some("2.0 t/m")
        );
    }

    #[test]
    fn queue_performance_hint_requires_sample_size() {
        let app = App::new(
            Arc::new(Mutex::new(PackageManager::new())),
            Arc::new(Mutex::new(None)),
            None,
            None,
        );
        assert_eq!(queue_performance_hint(&app.tasks, 4), None);
    }
}
