use super::app::{ActivePanel, App, AppMode, PendingAction};
use super::theme::*;
use super::update_center::UpdateLane;
use crate::models::history::{TaskQueueAction, TaskQueueEntry, TaskQueueStatus};
use crate::models::PackageStatus;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Row, Table},
    Frame,
};
use unicode_width::UnicodeWidthStr;

pub fn draw(f: &mut Frame, app: &App) {
    let console_height = if app.compact { 3 } else { 6 };
    let commands_height =
        if matches!(app.mode, AppMode::Normal | AppMode::UpdateCenter) && !app.compact {
            2
        } else {
            1
        };
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),                             // Title bar
            Constraint::Min(if app.compact { 8 } else { 10 }), // Main content
            Constraint::Length(console_height),                // Console panel
            Constraint::Length(commands_height),               // Commands bar
            Constraint::Length(3),                             // Status bar
        ])
        .split(f.area());

    draw_title_bar(f, app, chunks[0]);
    draw_main_content(f, app, chunks[1]);
    draw_console_panel(f, app, chunks[2]);
    draw_commands_bar(f, app, chunks[3]);
    draw_status_bar(f, app, chunks[4]);

    if app.mode == AppMode::Search {
        draw_search_popup(f, app);
    }
    if app.mode == AppMode::Confirm {
        draw_confirm_popup(f, app);
    }
}

fn is_update_context(app: &App) -> bool {
    app.mode == AppMode::UpdateCenter
        || (app.mode == AppMode::Search && app.search_return_mode == AppMode::UpdateCenter)
        || (app.mode == AppMode::Confirm && app.confirm_return_mode == AppMode::UpdateCenter)
}

fn draw_title_bar(f: &mut Frame, app: &App, area: Rect) {
    let update_context = is_update_context(app);
    let (title, title_color) = if app.compact {
        (" LinGet ", title_color())
    } else if update_context {
        (" LinGet TUI - Update Center ", accent_color())
    } else if app.show_updates_only {
        (" LinGet TUI - Updates Available ", accent_color())
    } else {
        (" LinGet TUI - Installed Packages ", title_color())
    };

    let title_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(title_color))
        .title(title)
        .title_style(title_style());

    let source_name = app
        .selected_source
        .map(|s| s.to_string())
        .unwrap_or_else(|| "All".to_string());

    let pkg_count = if update_context {
        let selected_count = app.selected_update_count();
        if app.compact {
            if selected_count > 0 {
                format!(
                    " {} upd | sec {} rec {} | {} sel ",
                    app.update_summary.total,
                    app.update_summary.security,
                    app.update_summary.recommended,
                    selected_count
                )
            } else {
                format!(
                    " {} upd | sec {} rec {} ",
                    app.update_summary.total,
                    app.update_summary.security,
                    app.update_summary.recommended
                )
            }
        } else if selected_count > 0 {
            format!(
                " {} updates | security {} | recommended {} | optional {} | risky {} | {} selected ",
                app.update_summary.total,
                app.update_summary.security,
                app.update_summary.recommended,
                app.update_summary.optional,
                app.update_summary.risky,
                selected_count
            )
        } else {
            format!(
                " {} updates | security {} | recommended {} | optional {} | risky {} ",
                app.update_summary.total,
                app.update_summary.security,
                app.update_summary.recommended,
                app.update_summary.optional,
                app.update_summary.risky
            )
        }
    } else {
        let selected_count = app.selected_count();
        if app.compact {
            if selected_count > 0 {
                format!(
                    " {} | {} | {} sel ",
                    app.filtered_packages.len(),
                    source_name,
                    selected_count
                )
            } else {
                format!(" {} | {} ", app.filtered_packages.len(), source_name)
            }
        } else if app.show_updates_only {
            if selected_count > 0 {
                format!(
                    " {} updates available | Source: {} | {} selected ",
                    app.filtered_packages.len(),
                    source_name,
                    selected_count
                )
            } else {
                format!(
                    " {} updates available | Source: {} ",
                    app.filtered_packages.len(),
                    source_name
                )
            }
        } else if selected_count > 0 {
            format!(
                " {} packages | Source: {} | {} selected ",
                app.filtered_packages.len(),
                source_name,
                selected_count
            )
        } else {
            format!(
                " {} packages | Source: {} ",
                app.filtered_packages.len(),
                source_name
            )
        }
    };

    let paragraph = Paragraph::new(pkg_count)
        .block(title_block)
        .style(title_bar());

    f.render_widget(paragraph, area);
}

fn draw_main_content(f: &mut Frame, app: &App, area: Rect) {
    if is_update_context(app) {
        draw_update_center(f, app, area);
        return;
    }

    if app.compact {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(30),
                Constraint::Percentage(40),
                Constraint::Percentage(30),
            ])
            .split(area);

        draw_sources_panel(f, app, chunks[0]);
        draw_packages_panel(f, app, chunks[1]);

        let detail_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(chunks[2]);

        draw_details_panel(f, app, detail_chunks[0]);
        draw_task_queue_panel(f, app, detail_chunks[1]);
    } else {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(22),
                Constraint::Percentage(50),
                Constraint::Min(35),
            ])
            .split(area);

        draw_sources_panel(f, app, chunks[0]);
        draw_packages_panel(f, app, chunks[1]);

        let detail_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(65), Constraint::Percentage(35)])
            .split(chunks[2]);

        draw_details_panel(f, app, detail_chunks[0]);
        draw_task_queue_panel(f, app, detail_chunks[1]);
    }
}

fn draw_update_center(f: &mut Frame, app: &App, area: Rect) {
    if app.compact {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(8),
                Constraint::Min(8),
                Constraint::Min(8),
            ])
            .split(area);
        draw_update_summary_panel(f, app, chunks[0]);
        draw_update_packages_panel(f, app, chunks[1]);
        draw_update_details_panel(f, app, chunks[2]);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(62), Constraint::Percentage(38)])
        .split(area);

    let left = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(10), Constraint::Min(10)])
        .split(chunks[0]);

    draw_update_summary_panel(f, app, left[0]);
    draw_update_packages_panel(f, app, left[1]);
    draw_update_details_panel(f, app, chunks[1]);
}

fn draw_update_summary_panel(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_active())
        .title(" Update Summary ")
        .title_style(panel_title_active());

    let mut lines = vec![
        Line::from(vec![
            Span::styled("Security: ", label()),
            Span::styled(app.update_summary.security.to_string(), status_removing()),
            Span::styled("  Recommended: ", label()),
            Span::styled(app.update_summary.recommended.to_string(), status_update()),
            Span::styled("  Optional: ", label()),
            Span::styled(app.update_summary.optional.to_string(), panel()),
            Span::styled("  Risky: ", label()),
            Span::styled(app.update_summary.risky.to_string(), status_not_installed()),
        ]),
        Line::from(vec![
            Span::styled("Total updates: ", label()),
            Span::styled(app.update_summary.total.to_string(), panel_title_active()),
            Span::styled("  Selected: ", label()),
            Span::styled(app.selected_update_count().to_string(), accent()),
        ]),
    ];

    lines.push(Line::from(vec![
        Span::styled("View: ", label()),
        Span::styled(
            if app.show_hidden_updates {
                "Hidden updates"
            } else {
                "Active updates"
            },
            if app.show_hidden_updates {
                status_not_installed()
            } else {
                panel()
            },
        ),
        Span::styled("  Failed in queue: ", label()),
        Span::styled(app.failed_update_count().to_string(), task_status_failed()),
    ]));

    if app.update_summary.by_source.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("Sources: ", label()),
            Span::styled("No updates detected", dim()),
        ]));
    } else {
        let source_text = app
            .update_summary
            .by_source
            .iter()
            .map(|(source, count)| format!("{}: {}", source, count))
            .collect::<Vec<_>>()
            .join(" | ");
        lines.push(Line::from(vec![
            Span::styled("Sources: ", label()),
            Span::styled(source_text, panel()),
        ]));
    }

    if let Some(summary) = &app.last_queue_summary {
        lines.push(Line::from(vec![
            Span::styled("Last run: ", label()),
            Span::styled(
                format!(
                    "{}/{} ok, {} failed, {} cancelled",
                    summary.succeeded, summary.total, summary.failed, summary.cancelled
                ),
                if summary.failed > 0 {
                    task_status_failed()
                } else {
                    status_installed()
                },
            ),
        ]));
    }

    let paragraph = Paragraph::new(lines).block(block).style(panel());
    f.render_widget(paragraph, area);
}

fn draw_update_packages_panel(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_active())
        .title(" Update Candidates ")
        .title_style(panel_title_active());

    if app.loading {
        let loading = Paragraph::new("Refreshing update data...")
            .block(block)
            .style(status_loading());
        f.render_widget(loading, area);
        return;
    }

    if app.update_candidates.is_empty() {
        let empty = Paragraph::new("No updates available")
            .block(block)
            .style(dim());
        f.render_widget(empty, area);
        return;
    }

    let rows: Vec<Row> = app
        .update_candidates
        .iter()
        .enumerate()
        .map(|(idx, candidate)| {
            let is_selected = idx == app.update_index;
            let is_marked = app.is_update_selected(&candidate.package);
            let base_style = if is_selected { selection() } else { panel() };
            let mark = if is_marked { "☑ " } else { "☐ " };
            let lane_tag = lane_short_label(candidate.lane);
            let lane_style = lane_style(candidate.lane);
            let hidden_tag = app
                .hidden_state_for_package(&candidate.package)
                .map(|state| {
                    if state.starts_with("Ignored") {
                        "[IGN] "
                    } else {
                        "[SNZ] "
                    }
                })
                .unwrap_or("");
            let package_name = format!("{}{}", hidden_tag, candidate.package.name);
            let version = candidate
                .package
                .available_version
                .as_ref()
                .map(|available| format!("{} -> {}", candidate.package.version, available))
                .unwrap_or_else(|| candidate.package.version.clone());

            Row::new(vec![
                Span::styled(mark, accent()),
                Span::styled(lane_tag, lane_style),
                Span::styled(truncate_string(&package_name, 26), base_style),
                Span::styled(truncate_string(&version, 20), base_style),
                Span::styled(candidate.package.source.to_string(), base_style),
            ])
            .style(base_style)
        })
        .collect();

    let header = Row::new(vec!["", "Lane", "Package", "Version", "Source"])
        .style(table_header())
        .bottom_margin(1);
    let widths = [
        Constraint::Length(3),
        Constraint::Length(6),
        Constraint::Min(22),
        Constraint::Min(18),
        Constraint::Length(10),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(block)
        .row_highlight_style(selection_focused());
    f.render_widget(table, area);
}

fn draw_update_details_panel(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_active())
        .title(" Update Details ")
        .title_style(panel_title_active());

    if app.loading {
        let loading = Paragraph::new("Loading update details...")
            .block(block)
            .style(status_loading());
        f.render_widget(loading, area);
        return;
    }

    let Some(candidate) = app.selected_update_candidate() else {
        let empty = Paragraph::new("Select an update to inspect")
            .block(block)
            .style(dim());
        f.render_widget(empty, area);
        return;
    };

    let package = &candidate.package;
    let content_width = area.width.saturating_sub(2) as usize;
    let available = package.available_version.as_deref().unwrap_or("unknown");
    let version_text = format!("{} -> {}", package.version, available);
    let lane_text = candidate.lane.label();
    let category_text = format!("{:?}", candidate.category);
    let hidden_state = app
        .hidden_state_for_package(package)
        .unwrap_or_else(|| String::from("Visible"));

    let mut lines: Vec<Line> = vec![
        labeled_line("Name: ", &package.name, panel_title_active(), content_width),
        labeled_line(
            "Source: ",
            &package.source.to_string(),
            panel(),
            content_width,
        ),
        labeled_line(
            "Lane: ",
            lane_text,
            lane_style(candidate.lane),
            content_width,
        ),
        labeled_line("Category: ", &category_text, panel(), content_width),
        labeled_line("Version: ", &version_text, panel(), content_width),
        labeled_line(
            "Visibility: ",
            &hidden_state,
            if hidden_state == "Visible" {
                status_installed()
            } else {
                status_not_installed()
            },
            content_width,
        ),
        Line::from(""),
        Line::from(vec![Span::styled("Description:", label())]),
    ];

    let wrapped = wrap_text(&package.description, content_width);
    lines.extend(
        wrapped
            .into_iter()
            .map(|line| Line::from(Span::styled(line, panel()))),
    );

    if candidate.lane == UpdateLane::Risky {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("Caution: ", status_not_installed()),
            Span::styled(
                "risky update, review before queueing all",
                status_not_installed(),
            ),
        ]));
    }

    let failed_updates = app.failed_update_names(3);
    if !failed_updates.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("Failed updates: ", label()),
            Span::styled(failed_updates.join(", "), task_status_failed()),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Press ", dim()),
            Span::styled("F", key_hint()),
            Span::styled(" to retry failed update tasks", dim()),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("Controls: ", label()),
        Span::styled("i ignore/unignore", dim()),
        Span::styled(" | ", dim()),
        Span::styled("z snooze 24h", dim()),
        Span::styled(" | ", dim()),
        Span::styled("Z clear snooze", dim()),
        Span::styled(" | ", dim()),
        Span::styled("v hidden view", dim()),
    ]));

    let paragraph = Paragraph::new(lines).block(block).style(panel());
    f.render_widget(paragraph, area);
}

fn lane_short_label(lane: UpdateLane) -> &'static str {
    match lane {
        UpdateLane::Security => "SEC",
        UpdateLane::Recommended => "REC",
        UpdateLane::Optional => "OPT",
        UpdateLane::Risky => "RSK",
    }
}

fn lane_style(lane: UpdateLane) -> Style {
    match lane {
        UpdateLane::Security => status_removing(),
        UpdateLane::Recommended => status_update(),
        UpdateLane::Optional => panel(),
        UpdateLane::Risky => status_not_installed(),
    }
}

fn draw_sources_panel(f: &mut Frame, app: &App, area: Rect) {
    let is_active = app.active_panel == ActivePanel::Sources;

    let border_style = if is_active {
        border_active()
    } else {
        border_inactive()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(" Sources ")
        .title_style(if is_active {
            panel_title_active()
        } else {
            panel_title()
        });

    let inner = block.inner(area);
    let total_sources = app.available_sources.len() + 1;
    let selected = app.source_index.min(total_sources.saturating_sub(1));
    let visible = inner.height as usize;
    let start = queue_window_start(total_sources, visible, selected);
    let end = (start + visible).min(total_sources);

    let mut items: Vec<ListItem> = Vec::new();
    for idx in start..end {
        let is_selected = idx == app.source_index;
        let label = if idx == 0 {
            String::from("All")
        } else {
            app.available_sources[idx - 1].to_string()
        };

        items.push(ListItem::new(Line::from(vec![
            Span::styled(if is_selected { "▶ " } else { "  " }, accent()),
            Span::styled(
                label,
                if is_selected {
                    panel_title_active()
                } else {
                    panel()
                },
            ),
        ])));
    }

    let list = List::new(items).block(block);
    f.render_widget(list, area);
}

fn draw_packages_panel(f: &mut Frame, app: &App, area: Rect) {
    let is_active = app.active_panel == ActivePanel::Packages;

    let border_style = if is_active {
        border_active()
    } else {
        border_inactive()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(" Packages ")
        .title_style(if is_active {
            panel_title_active()
        } else {
            panel_title()
        });

    if app.loading {
        let loading = Paragraph::new("Loading packages...")
            .block(block)
            .style(status_loading());
        f.render_widget(loading, area);
        return;
    }

    if app.filtered_packages.is_empty() {
        let empty = Paragraph::new("No packages found (use / to search)")
            .block(block)
            .style(dim());
        f.render_widget(empty, area);
        return;
    }

    let rows: Vec<Row> = app
        .filtered_packages
        .iter()
        .enumerate()
        .map(|(i, pkg)| {
            let is_selected = i == app.package_index;
            let is_marked = app.is_selected(pkg);
            let style = if is_selected { selection() } else { panel() };

            let status_style = match pkg.status {
                PackageStatus::Installed => status_installed(),
                PackageStatus::UpdateAvailable => status_update(),
                PackageStatus::NotInstalled => status_not_installed(),
                PackageStatus::Installing | PackageStatus::Updating => status_loading(),
                PackageStatus::Removing => status_removing(),
            };

            let status_icon = match pkg.status {
                PackageStatus::Installed => "✓",
                PackageStatus::UpdateAvailable => "↑",
                PackageStatus::NotInstalled => "○",
                PackageStatus::Installing | PackageStatus::Updating => "⟳",
                PackageStatus::Removing => "✗",
            };

            let version = if let Some(ref avail) = pkg.available_version {
                format!("{} → {}", pkg.version, avail)
            } else {
                pkg.version.clone()
            };

            let mark = if is_marked { "☑ " } else { "☐ " };

            Row::new(vec![
                Span::styled(mark, accent()),
                Span::styled(truncate_string(&pkg.name, 23), style),
                Span::styled(truncate_string(&version, 20), style),
                Span::styled(pkg.source.to_string(), style),
                Span::styled(status_icon, status_style),
            ])
            .style(style)
        })
        .collect();

    let header = Row::new(vec!["", "Name", "Version", "Source", ""])
        .style(table_header())
        .bottom_margin(1);

    let widths = [
        Constraint::Length(3),
        Constraint::Min(23),
        Constraint::Min(20),
        Constraint::Length(10),
        Constraint::Length(2),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(block)
        .row_highlight_style(selection_focused());

    f.render_widget(table, area);
}

fn draw_details_panel(f: &mut Frame, app: &App, area: Rect) {
    let is_active = app.active_panel == ActivePanel::Details;

    let border_style = if is_active {
        border_active()
    } else {
        border_inactive()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(" Details ")
        .title_style(if is_active {
            panel_title_active()
        } else {
            panel_title()
        });

    if app.loading {
        let loading = Paragraph::new("Loading details...")
            .block(block)
            .style(status_loading());
        f.render_widget(loading, area);
        return;
    }

    let pkg = match app.selected_package() {
        Some(p) => p,
        None => {
            let empty = Paragraph::new("Select a package to view details")
                .block(block)
                .style(dim());
            f.render_widget(empty, area);
            return;
        }
    };

    let status_text = match pkg.status {
        PackageStatus::Installed => "Installed",
        PackageStatus::UpdateAvailable => "Update Available",
        PackageStatus::NotInstalled => "Not Installed",
        PackageStatus::Installing => "Installing...",
        PackageStatus::Removing => "Removing...",
        PackageStatus::Updating => "Updating...",
    };

    let version_info = if let Some(ref avail) = pkg.available_version {
        format!("{} → {}", pkg.version, avail)
    } else {
        pkg.version.clone()
    };

    let content_width = area.width.saturating_sub(2) as usize;

    let name_line = labeled_line("Name: ", &pkg.name, panel_title_active(), content_width);
    let source_line = labeled_line(
        "Source: ",
        &format!("{}", pkg.source),
        panel(),
        content_width,
    );
    let status_line = labeled_line(
        "Status: ",
        status_text,
        status_style_for_status(pkg.status),
        content_width,
    );
    let version_line = labeled_line("Version: ", &version_info, panel(), content_width);

    let description_line = Line::from(vec![Span::styled("Description: ", label())]);

    let wrapped_description = wrap_text(&pkg.description, content_width);

    let lines: Vec<Line> = vec![
        name_line,
        source_line,
        status_line,
        version_line,
        Line::from(""),
        description_line,
    ];

    let mut description_lines: Vec<Line> = wrapped_description
        .iter()
        .map(|s| Line::from(Span::styled(s, panel())))
        .collect();

    let mut all_lines = lines;
    all_lines.append(&mut description_lines);

    let paragraph = Paragraph::new(all_lines).block(block).style(panel());

    f.render_widget(paragraph, area);
}

fn draw_task_queue_panel(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(hud_border())
        .title(" Task Queue ")
        .title_style(hud_title());

    let inner = block.inner(area);
    let content_width = inner.width as usize;
    let (active, queued, completed) = task_queue_counts(&app.queued_tasks);
    let progress = app.queue_progress();

    let mut lines: Vec<Line> = Vec::new();
    let summary = Line::from(vec![
        Span::styled("ACTIVE ", hud_label()),
        Span::styled(format!("{}", active), hud_value_active()),
        Span::styled(" | ", hud_separator()),
        Span::styled("QUEUED ", hud_label()),
        Span::styled(format!("{}", queued), hud_value_queued()),
        Span::styled(" | ", hud_separator()),
        Span::styled("COMPLETED ", hud_label()),
        Span::styled(format!("{}", completed), hud_value_completed()),
    ]);
    lines.push(summary);

    lines.push(Line::from(vec![
        Span::styled("PROGRESS ", hud_label()),
        Span::styled(
            format!("{}/{}", progress.done, progress.total),
            hud_value_completed(),
        ),
        Span::styled(" | ", hud_separator()),
        Span::styled("OK ", hud_label()),
        Span::styled(progress.succeeded.to_string(), status_installed()),
        Span::styled(" | ", hud_separator()),
        Span::styled("FAIL ", hud_label()),
        Span::styled(progress.failed.to_string(), task_status_failed()),
    ]));

    if let Some(active_task) = progress.active_task {
        let active_text = truncate_to_width(
            &format!("Active: {}", active_task),
            content_width.saturating_sub(2),
        );
        lines.push(Line::from(vec![
            Span::styled("▶ ", accent()),
            Span::styled(active_text, status_loading()),
        ]));
    }

    lines.push(Line::from(vec![
        Span::styled("Logs: ", label()),
        Span::styled(
            if app.queue_show_logs {
                "verbose (l to toggle)"
            } else {
                "quiet (l to toggle)"
            },
            if app.queue_show_logs { panel() } else { dim() },
        ),
    ]));

    if inner.height > 1 {
        lines.push(Line::from(""));
    }

    if app.queued_tasks.is_empty() {
        lines.push(Line::from(Span::styled("No queued tasks", dim())));
    } else {
        let available = inner.height.saturating_sub(lines.len() as u16) as usize;
        let total = app.queued_tasks.len();
        let selected = app.queue_index.min(total.saturating_sub(1));
        let start = queue_window_start(total, available, selected);
        let end = (start + available).min(total);

        for idx in start..end {
            let is_selected = idx == selected;
            lines.push(task_queue_line(
                &app.queued_tasks[idx],
                content_width,
                is_selected,
                app.active_panel == ActivePanel::Queue,
            ));
        }
    }

    let paragraph = Paragraph::new(lines).block(block).style(panel());
    f.render_widget(paragraph, area);
}

fn status_style_for_status(status: PackageStatus) -> Style {
    match status {
        PackageStatus::Installed => status_installed(),
        PackageStatus::UpdateAvailable => status_update(),
        PackageStatus::NotInstalled => status_not_installed(),
        PackageStatus::Installing | PackageStatus::Updating => status_loading(),
        PackageStatus::Removing => status_removing(),
    }
}

fn task_queue_counts(entries: &[TaskQueueEntry]) -> (usize, usize, usize) {
    let mut active = 0;
    let mut queued = 0;
    let mut completed = 0;

    for entry in entries {
        match entry.status {
            TaskQueueStatus::Running => active += 1,
            TaskQueueStatus::Queued => queued += 1,
            TaskQueueStatus::Completed | TaskQueueStatus::Failed | TaskQueueStatus::Cancelled => {
                completed += 1
            }
        }
    }

    (active, queued, completed)
}

fn task_queue_line(
    entry: &TaskQueueEntry,
    max_width: usize,
    is_selected: bool,
    queue_is_active: bool,
) -> Line<'static> {
    let status = entry.status;
    let status_label = task_status_label(status);
    let action_label = task_action_label(entry.action);
    let selection_prefix = if is_selected { "▶ " } else { "  " };
    let status_prefix = format!("[{}] ", status_label);
    let action_text = format!("{} ", action_label);

    let info = match status {
        TaskQueueStatus::Failed => {
            if let Some(error) = &entry.error {
                format!(
                    "{} ({:?}) - {}",
                    entry.package_name, entry.package_source, error
                )
            } else {
                format!("{} ({:?})", entry.package_name, entry.package_source)
            }
        }
        _ => format!("{} ({:?})", entry.package_name, entry.package_source),
    };

    let prefix_width = UnicodeWidthStr::width(selection_prefix)
        + UnicodeWidthStr::width(status_prefix.as_str())
        + UnicodeWidthStr::width(action_text.as_str());
    let remaining = max_width.saturating_sub(prefix_width);
    let info_text = truncate_to_width(&info, remaining);
    let info_style = if is_selected && queue_is_active {
        selection()
    } else {
        panel()
    };
    let marker_style = if is_selected { accent() } else { dim() };

    Line::from(vec![
        Span::styled(selection_prefix, marker_style),
        Span::styled(status_prefix, task_status_style(status)),
        Span::styled(action_text, hud_action()),
        Span::styled(info_text, info_style),
    ])
}

fn queue_window_start(total: usize, visible: usize, selected: usize) -> usize {
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

fn task_status_label(status: TaskQueueStatus) -> &'static str {
    match status {
        TaskQueueStatus::Queued => "QUE",
        TaskQueueStatus::Running => "RUN",
        TaskQueueStatus::Completed => "DONE",
        TaskQueueStatus::Failed => "FAIL",
        TaskQueueStatus::Cancelled => "CXL",
    }
}

fn task_action_label(action: TaskQueueAction) -> &'static str {
    match action {
        TaskQueueAction::Install => "INSTALL",
        TaskQueueAction::Remove => "REMOVE",
        TaskQueueAction::Update => "UPDATE",
    }
}

fn task_status_style(status: TaskQueueStatus) -> Style {
    match status {
        TaskQueueStatus::Queued => task_status_queued(),
        TaskQueueStatus::Running => task_status_running(),
        TaskQueueStatus::Completed => task_status_completed(),
        TaskQueueStatus::Failed => task_status_failed(),
        TaskQueueStatus::Cancelled => task_status_cancelled(),
    }
}

fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    if text.is_empty() {
        return vec![String::from("No description available")];
    }

    if max_width == 0 {
        return vec![text.to_string()];
    }

    let mut lines = Vec::new();
    let mut current_line = String::new();
    let mut current_width = 0;
    let words: Vec<&str> = text.split_whitespace().collect();
    let mut word_iter = words.into_iter();

    if let Some(first_word) = word_iter.next() {
        for chunk in split_long_word(first_word, max_width) {
            if current_line.is_empty() {
                current_line = chunk;
                current_width = UnicodeWidthStr::width(current_line.as_str());
            } else {
                lines.push(current_line);
                current_line = chunk;
                current_width = UnicodeWidthStr::width(current_line.as_str());
            }
        }
    }

    for word in word_iter {
        let word_width = UnicodeWidthStr::width(word);
        if current_line.is_empty() {
            for chunk in split_long_word(word, max_width) {
                if current_line.is_empty() {
                    current_line = chunk;
                    current_width = UnicodeWidthStr::width(current_line.as_str());
                } else {
                    lines.push(current_line);
                    current_line = chunk;
                    current_width = UnicodeWidthStr::width(current_line.as_str());
                }
            }
            continue;
        }

        let next_width = current_width + 1 + word_width;
        if next_width <= max_width {
            current_line.push(' ');
            current_line.push_str(word);
            current_width = next_width;
        } else {
            lines.push(current_line);
            current_line = String::new();
            current_width = 0;
            for chunk in split_long_word(word, max_width) {
                if current_line.is_empty() {
                    current_line = chunk;
                    current_width = UnicodeWidthStr::width(current_line.as_str());
                } else {
                    lines.push(current_line);
                    current_line = chunk;
                    current_width = UnicodeWidthStr::width(current_line.as_str());
                }
            }
        }
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }

    if lines.is_empty() {
        vec![String::from("No description available")]
    } else {
        lines
    }
}

fn labeled_line(
    label_text: &str,
    value: &str,
    value_style: Style,
    max_width: usize,
) -> Line<'static> {
    let label_width = UnicodeWidthStr::width(label_text);
    let value_width = max_width.saturating_sub(label_width);
    let value_text = truncate_to_width(value, value_width);
    let label_owned = label_text.to_string();

    Line::from(vec![
        Span::styled(label_owned, label()),
        Span::styled(value_text, value_style),
    ])
}

fn truncate_to_width(text: &str, max_width: usize) -> String {
    if max_width == 0 {
        return String::new();
    }

    if UnicodeWidthStr::width(text) <= max_width {
        return text.to_string();
    }

    if max_width == 1 {
        return String::from("…");
    }

    let mut out = String::new();
    let mut width = 0;
    let target = max_width.saturating_sub(1);

    for ch in text.chars() {
        let ch_width = UnicodeWidthStr::width(ch.to_string().as_str());
        if width + ch_width > target {
            break;
        }
        out.push(ch);
        width += ch_width;
    }

    out.push('…');
    out
}

fn split_long_word(word: &str, max_width: usize) -> Vec<String> {
    if max_width == 0 {
        return vec![String::new()];
    }

    let mut parts = Vec::new();
    let mut current = String::new();
    let mut width = 0;

    for ch in word.chars() {
        let ch_width = UnicodeWidthStr::width(ch.to_string().as_str());
        if width + ch_width > max_width && !current.is_empty() {
            parts.push(current);
            current = String::new();
            width = 0;
        }
        current.push(ch);
        width += ch_width;
    }

    if !current.is_empty() {
        parts.push(current);
    }

    parts
}

fn draw_console_panel(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_inactive())
        .title(" Console ")
        .title_style(panel_title());

    if app.console_buffer.is_empty() {
        let empty = Paragraph::new("No recent actions yet")
            .block(block)
            .style(dim());
        f.render_widget(empty, area);
        return;
    }

    let max_lines = area.height.saturating_sub(2) as usize;
    let start = app.console_buffer.len().saturating_sub(max_lines);
    let lines: Vec<Line> = app
        .console_buffer
        .iter()
        .skip(start)
        .map(|s| Line::from(Span::styled(s, panel())))
        .collect();

    let paragraph = Paragraph::new(lines).block(block);
    f.render_widget(paragraph, area);
}

fn draw_commands_bar(f: &mut Frame, app: &App, area: Rect) {
    let lines = match app.mode {
        AppMode::Normal => {
            if app.compact {
                vec![Line::from(vec![
                    Span::styled("↑↓/jk", key_hint()),
                    Span::styled(" nav ", description()),
                    Span::styled("│", separator()),
                    Span::styled(" Tab", key_hint()),
                    Span::styled(" panel ", description()),
                    Span::styled("│", separator()),
                    Span::styled(" /", key_hint()),
                    Span::styled(" search ", description()),
                    Span::styled("│", separator()),
                    Span::styled(" u", key_hint()),
                    Span::styled(" updates ", description()),
                    Span::styled("│", separator()),
                    Span::styled(" l", key_hint()),
                    Span::styled(" logs ", description()),
                    Span::styled("│", separator()),
                    Span::styled(" h", key_hint()),
                    Span::styled(" help ", description()),
                    Span::styled("│", separator()),
                    Span::styled(" q", key_hint()),
                    Span::styled(" quit", description()),
                ])]
            } else {
                vec![
                    Line::from(vec![
                        Span::styled("↑↓/jk", key_hint()),
                        Span::styled(" nav ", description()),
                        Span::styled("│", separator()),
                        Span::styled(" Tab", key_hint()),
                        Span::styled(" panel ", description()),
                        Span::styled("│", separator()),
                        Span::styled(" /", key_hint()),
                        Span::styled(" search ", description()),
                        Span::styled("│", separator()),
                        Span::styled(" Space", key_hint()),
                        Span::styled(" select ", description()),
                        Span::styled("│", separator()),
                        Span::styled(" a", key_hint()),
                        Span::styled(" select-all ", description()),
                        Span::styled("│", separator()),
                        Span::styled(" c", key_hint()),
                        Span::styled(" clear ", description()),
                        Span::styled("│", separator()),
                        Span::styled(" u", key_hint()),
                        Span::styled(" update-center ", description()),
                        Span::styled("│", separator()),
                        Span::styled(" h", key_hint()),
                        Span::styled(" help ", description()),
                        Span::styled("│", separator()),
                        Span::styled(" q", key_hint()),
                        Span::styled(" quit", description()),
                    ]),
                    Line::from(vec![
                        Span::styled(" I", key_hint()),
                        Span::styled(" queue-installs ", description()),
                        Span::styled("│", separator()),
                        Span::styled(" X", key_hint()),
                        Span::styled(" queue-removals ", description()),
                        Span::styled("│", separator()),
                        Span::styled(" C", key_hint()),
                        Span::styled(" cancel-task ", description()),
                        Span::styled("│", separator()),
                        Span::styled(" R", key_hint()),
                        Span::styled(" retry-task ", description()),
                        Span::styled("│", separator()),
                        Span::styled(" r", key_hint()),
                        Span::styled(" refresh ", description()),
                        Span::styled("│", separator()),
                        Span::styled(" l", key_hint()),
                        Span::styled(" logs ", description()),
                        Span::styled("│", separator()),
                        Span::styled(" i/x", key_hint()),
                        Span::styled(" install/remove", description()),
                    ]),
                ]
            }
        }
        AppMode::UpdateCenter => {
            if app.compact {
                vec![Line::from(vec![
                    Span::styled("↑↓/jk", key_hint()),
                    Span::styled(" nav ", description()),
                    Span::styled("│", separator()),
                    Span::styled(" Space", key_hint()),
                    Span::styled(" select ", description()),
                    Span::styled("│", separator()),
                    Span::styled(" U/B/S/A", key_hint()),
                    Span::styled(" queue ", description()),
                    Span::styled("│", separator()),
                    Span::styled(" F", key_hint()),
                    Span::styled(" retry ", description()),
                    Span::styled("│", separator()),
                    Span::styled(" l", key_hint()),
                    Span::styled(" logs ", description()),
                    Span::styled("│", separator()),
                    Span::styled(" Esc", key_hint()),
                    Span::styled(" back", description()),
                ])]
            } else {
                vec![
                    Line::from(vec![
                        Span::styled("↑↓/jk", key_hint()),
                        Span::styled(" nav ", description()),
                        Span::styled("│", separator()),
                        Span::styled(" Space", key_hint()),
                        Span::styled(" toggle ", description()),
                        Span::styled("│", separator()),
                        Span::styled(" m", key_hint()),
                        Span::styled(" mark-recommended ", description()),
                        Span::styled("│", separator()),
                        Span::styled(" a/c", key_hint()),
                        Span::styled(" select-all/clear ", description()),
                        Span::styled("│", separator()),
                        Span::styled(" i", key_hint()),
                        Span::styled(" ignore ", description()),
                        Span::styled("│", separator()),
                        Span::styled(" z/Z", key_hint()),
                        Span::styled(" snooze/clear ", description()),
                        Span::styled("│", separator()),
                        Span::styled(" v", key_hint()),
                        Span::styled(" hidden-view ", description()),
                        Span::styled("│", separator()),
                        Span::styled(" /", key_hint()),
                        Span::styled(" search ", description()),
                        Span::styled("│", separator()),
                        Span::styled(" Esc", key_hint()),
                        Span::styled(" back", description()),
                    ]),
                    Line::from(vec![
                        Span::styled(" U", key_hint()),
                        Span::styled(" queue-recommended ", description()),
                        Span::styled("│", separator()),
                        Span::styled(" B", key_hint()),
                        Span::styled(" queue-by-source ", description()),
                        Span::styled("│", separator()),
                        Span::styled(" S", key_hint()),
                        Span::styled(" queue-selected ", description()),
                        Span::styled("│", separator()),
                        Span::styled(" A", key_hint()),
                        Span::styled(" queue-all ", description()),
                        Span::styled("│", separator()),
                        Span::styled(" F", key_hint()),
                        Span::styled(" retry-failed ", description()),
                        Span::styled("│", separator()),
                        Span::styled(" r", key_hint()),
                        Span::styled(" refresh ", description()),
                        Span::styled("│", separator()),
                        Span::styled(" l", key_hint()),
                        Span::styled(" logs ", description()),
                        Span::styled("│", separator()),
                        Span::styled(" q", key_hint()),
                        Span::styled(" quit", description()),
                    ]),
                ]
            }
        }
        AppMode::Search => vec![Line::from(vec![
            Span::styled("Enter", key_hint()),
            Span::styled(" confirm ", description()),
            Span::styled("│", separator()),
            Span::styled(" Esc", key_hint()),
            Span::styled(" cancel", description()),
        ])],
        AppMode::Confirm => vec![Line::from(vec![
            Span::styled("y", key_hint()),
            Span::styled(" yes ", description()),
            Span::styled("│", separator()),
            Span::styled(" n", key_hint()),
            Span::styled(" no", description()),
        ])],
    };

    let paragraph = Paragraph::new(lines).style(panel());
    f.render_widget(paragraph, area);
}

fn draw_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let mode_indicator = match app.mode {
        AppMode::Normal => "NORMAL",
        AppMode::Search => "SEARCH",
        AppMode::Confirm => "CONFIRM",
        AppMode::UpdateCenter => "UPDATE",
    };

    let status_style = match app.mode {
        AppMode::Normal => mode_normal(),
        AppMode::Search => mode_search(),
        AppMode::Confirm => mode_confirm(),
        AppMode::UpdateCenter => mode_update_center(),
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_inactive());

    let status_text = Line::from(vec![
        Span::styled(
            format!(" [{}] ", mode_indicator),
            status_style.add_modifier(Modifier::BOLD),
        ),
        Span::raw(&app.status_message),
    ]);

    let paragraph = Paragraph::new(status_text).block(block).style(panel());
    f.render_widget(paragraph, area);
}

fn draw_search_popup(f: &mut Frame, app: &App) {
    let area = centered_rect(60, 5, f.area());

    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_active())
        .title(" Search ")
        .title_style(panel_title_active());

    let search_text = format!("{}▏", app.search_query);
    let lines = vec![
        Line::from(vec![Span::styled("Query:", label())]),
        Line::from(vec![Span::styled(search_text, panel())]),
    ];
    let paragraph = Paragraph::new(lines).block(block).style(panel());

    f.render_widget(paragraph, area);
}

fn draw_confirm_popup(f: &mut Frame, app: &App) {
    let message = if let Some(ref action) = app.pending_action {
        match action {
            PendingAction::Install(pkg) => format!("Install {}?", pkg.name),
            PendingAction::Remove(pkg) => format!("Remove {}?", pkg.name),
            PendingAction::UpdateAll(pkgs) => format!("Update {} packages?", pkgs.len()),
            PendingAction::UpdateBySource(source, pkgs) => {
                format!("Queue {} updates from {}?", pkgs.len(), source)
            }
            PendingAction::UpdateRecommended(pkgs) => {
                format!("Queue recommended updates for {} packages?", pkgs.len())
            }
            PendingAction::UpdateSelected(pkgs) => {
                format!("Queue selected updates for {} packages?", pkgs.len())
            }
            PendingAction::InstallSelected(pkgs) => {
                format!("Queue installs for {} packages?", pkgs.len())
            }
            PendingAction::RemoveSelected(pkgs) => {
                format!("Queue removals for {} packages?", pkgs.len())
            }
        }
    } else {
        app.status_message.clone()
    };

    let popup_height = if message.contains("packages") { 6 } else { 5 };
    let area = centered_rect(60, popup_height, f.area());

    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_active())
        .title(" Confirm ")
        .title_style(panel_title_active());

    let lines = vec![
        Line::from(vec![Span::styled("Action:", label())]),
        Line::from(vec![Span::styled(&message, panel())]),
        Line::from(""),
        Line::from(vec![
            Span::styled("y", key_hint()),
            Span::styled(" yes ", description()),
            Span::styled("│", separator()),
            Span::styled(" n", key_hint()),
            Span::styled(" no", description()),
        ]),
    ];

    let paragraph = Paragraph::new(lines).block(block).style(panel());
    f.render_widget(paragraph, area);
}

fn centered_rect(percent_x: u16, height: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length((r.height.saturating_sub(height)) / 2),
            Constraint::Length(height),
            Constraint::Min(0),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn truncate_string(s: &str, max_len: usize) -> String {
    let char_count = s.chars().count();
    if char_count <= max_len {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_len - 1).collect();
        format!("{}…", truncated)
    }
}
