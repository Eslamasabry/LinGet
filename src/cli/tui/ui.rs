use super::app::{ActivePanel, App, AppMode};
use crate::models::PackageStatus;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Row, Table},
    Frame,
};

pub fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title bar
            Constraint::Min(10),   // Main content
            Constraint::Length(3), // Status bar
        ])
        .split(f.area());

    draw_title_bar(f, app, chunks[0]);
    draw_main_content(f, app, chunks[1]);
    draw_status_bar(f, app, chunks[2]);

    // Draw search popup if in search mode
    if app.mode == AppMode::Search {
        draw_search_popup(f, app);
    }
}

fn draw_title_bar(f: &mut Frame, app: &App, area: Rect) {
    let title = if app.show_updates_only {
        " LinGet TUI - Updates "
    } else {
        " LinGet TUI - Installed Packages "
    };

    let title_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(title)
        .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));

    let pkg_count = format!(
        " {} packages | Source: {} ",
        app.filtered_packages.len(),
        app.selected_source
            .map(|s| format!("{:?}", s))
            .unwrap_or_else(|| "All".to_string())
    );

    let paragraph = Paragraph::new(pkg_count)
        .block(title_block)
        .style(Style::default().fg(Color::White));

    f.render_widget(paragraph, area);
}

fn draw_main_content(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(20), Constraint::Min(40)])
        .split(area);

    draw_sources_panel(f, app, chunks[0]);
    draw_packages_panel(f, app, chunks[1]);
}

fn draw_sources_panel(f: &mut Frame, app: &App, area: Rect) {
    let is_active = app.active_panel == ActivePanel::Sources;

    let border_style = if is_active {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(" Sources ")
        .title_style(if is_active {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        });

    // Build source list items
    let mut items: Vec<ListItem> = vec![ListItem::new(Line::from(vec![
        Span::styled(
            if app.source_index == 0 { "▶ " } else { "  " },
            Style::default().fg(Color::Yellow),
        ),
        Span::styled(
            "All",
            if app.source_index == 0 {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            },
        ),
    ]))];

    for (i, source) in app.available_sources.iter().enumerate() {
        let is_selected = app.source_index == i + 1;
        items.push(ListItem::new(Line::from(vec![
            Span::styled(
                if is_selected { "▶ " } else { "  " },
                Style::default().fg(Color::Yellow),
            ),
            Span::styled(
                format!("{:?}", source),
                if is_selected {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
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
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(" Packages ")
        .title_style(if is_active {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        });

    if app.loading {
        let loading = Paragraph::new("Loading...")
            .block(block)
            .style(Style::default().fg(Color::Yellow));
        f.render_widget(loading, area);
        return;
    }

    if app.filtered_packages.is_empty() {
        let empty = Paragraph::new("No packages found")
            .block(block)
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(empty, area);
        return;
    }

    // Create table rows
    let rows: Vec<Row> = app
        .filtered_packages
        .iter()
        .enumerate()
        .map(|(i, pkg)| {
            let is_selected = i == app.package_index;
            let style = if is_selected {
                Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let status_style = match pkg.status {
                PackageStatus::Installed => Style::default().fg(Color::Green),
                PackageStatus::UpdateAvailable => Style::default().fg(Color::Yellow),
                PackageStatus::NotInstalled => Style::default().fg(Color::DarkGray),
                PackageStatus::Installing | PackageStatus::Updating => {
                    Style::default().fg(Color::Cyan)
                }
                PackageStatus::Removing => Style::default().fg(Color::Red),
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

            Row::new(vec![
                Span::styled(
                    truncate_string(&pkg.name, 25),
                    style,
                ),
                Span::styled(
                    truncate_string(&version, 20),
                    style,
                ),
                Span::styled(
                    format!("{:?}", pkg.source),
                    style,
                ),
                Span::styled(status_icon, status_style),
            ])
            .style(style)
        })
        .collect();

    let header = Row::new(vec!["Name", "Version", "Source", ""])
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .bottom_margin(1);

    let widths = [
        Constraint::Min(25),
        Constraint::Min(20),
        Constraint::Length(10),
        Constraint::Length(2),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(block)
        .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    f.render_widget(table, area);
}

fn draw_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let mode_indicator = match app.mode {
        AppMode::Normal => "NORMAL",
        AppMode::Search => "SEARCH",
        AppMode::Confirm => "CONFIRM",
    };

    let status_style = match app.mode {
        AppMode::Normal => Style::default().fg(Color::Green),
        AppMode::Search => Style::default().fg(Color::Yellow),
        AppMode::Confirm => Style::default().fg(Color::Red),
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let status_text = Line::from(vec![
        Span::styled(
            format!(" [{}] ", mode_indicator),
            status_style.add_modifier(Modifier::BOLD),
        ),
        Span::raw(&app.status_message),
    ]);

    let paragraph = Paragraph::new(status_text).block(block);
    f.render_widget(paragraph, area);
}

fn draw_search_popup(f: &mut Frame, app: &App) {
    let area = centered_rect(50, 3, f.area());

    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .title(" Search ")
        .title_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));

    let search_text = format!("{}▏", app.search_query);
    let paragraph = Paragraph::new(search_text)
        .block(block)
        .style(Style::default().fg(Color::White));

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
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}…", &s[..max_len - 1])
    }
}
