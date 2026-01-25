use super::app::{ActivePanel, App, AppMode};
use super::theme::*;
use crate::models::PackageStatus;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
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
            Constraint::Length(1), // Commands bar
            Constraint::Length(3), // Status bar
        ])
        .split(f.area());

    draw_title_bar(f, app, chunks[0]);
    draw_main_content(f, app, chunks[1]);
    draw_commands_bar(f, app, chunks[2]);
    draw_status_bar(f, app, chunks[3]);

    // Draw search popup if in search mode
    if app.mode == AppMode::Search {
        draw_search_popup(f, app);
    }
}

fn draw_title_bar(f: &mut Frame, app: &App, area: Rect) {
    let (title, title_color) = if app.show_updates_only {
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
        .map(|s| format!("{:?}", s))
        .unwrap_or_else(|| "All".to_string());

    let pkg_count = if app.show_updates_only {
        format!(
            " {} updates available | Source: {} ",
            app.filtered_packages.len(),
            source_name
        )
    } else {
        format!(
            " {} packages | Source: {} ",
            app.filtered_packages.len(),
            source_name
        )
    };

    let paragraph = Paragraph::new(pkg_count)
        .block(title_block)
        .style(title_bar());

    f.render_widget(paragraph, area);
}

fn draw_main_content(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(22), Constraint::Min(40)])
        .split(area);

    draw_sources_panel(f, app, chunks[0]);
    draw_packages_panel(f, app, chunks[1]);
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

    let mut items: Vec<ListItem> = vec![ListItem::new(Line::from(vec![
        Span::styled(if app.source_index == 0 { "▶ " } else { "  " }, accent()),
        Span::styled(
            "All",
            if app.source_index == 0 {
                panel_title_active()
            } else {
                panel()
            },
        ),
    ]))];

    for (i, source) in app.available_sources.iter().enumerate() {
        let is_selected = app.source_index == i + 1;
        items.push(ListItem::new(Line::from(vec![
            Span::styled(if is_selected { "▶ " } else { "  " }, accent()),
            Span::styled(
                format!("{:?}", source),
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
        let loading = Paragraph::new("Loading...")
            .block(block)
            .style(status_loading());
        f.render_widget(loading, area);
        return;
    }

    if app.filtered_packages.is_empty() {
        let empty = Paragraph::new("No packages found")
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

            Row::new(vec![
                Span::styled(truncate_string(&pkg.name, 25), style),
                Span::styled(truncate_string(&version, 20), style),
                Span::styled(format!("{:?}", pkg.source), style),
                Span::styled(status_icon, status_style),
            ])
            .style(style)
        })
        .collect();

    let header = Row::new(vec!["Name", "Version", "Source", ""])
        .style(table_header())
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
        .row_highlight_style(selection_focused());

    f.render_widget(table, area);
}

fn draw_commands_bar(f: &mut Frame, app: &App, area: Rect) {
    let commands = match app.mode {
        AppMode::Normal => {
            let updates_label = if app.show_updates_only {
                "all"
            } else {
                "updates"
            };
            vec![
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
                Span::styled(format!(" {} ", updates_label), description()),
                Span::styled("│", separator()),
                Span::styled(" r", key_hint()),
                Span::styled(" refresh ", description()),
                Span::styled("│", separator()),
                Span::styled(" i", key_hint()),
                Span::styled(" install ", description()),
                Span::styled("│", separator()),
                Span::styled(" x", key_hint()),
                Span::styled(" remove ", description()),
                Span::styled("│", separator()),
                Span::styled(" q", key_hint()),
                Span::styled(" quit", description()),
            ]
        }
        AppMode::Search => vec![
            Span::styled("Enter", key_hint()),
            Span::styled(" confirm ", description()),
            Span::styled("│", separator()),
            Span::styled(" Esc", key_hint()),
            Span::styled(" cancel", description()),
        ],
        AppMode::Confirm => vec![
            Span::styled("y", key_hint()),
            Span::styled(" yes ", description()),
            Span::styled("│", separator()),
            Span::styled(" n", key_hint()),
            Span::styled(" no", description()),
        ],
    };

    let line = Line::from(commands);
    let paragraph = Paragraph::new(line).style(panel());
    f.render_widget(paragraph, area);
}

fn draw_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let mode_indicator = match app.mode {
        AppMode::Normal => "NORMAL",
        AppMode::Search => "SEARCH",
        AppMode::Confirm => "CONFIRM",
    };

    let status_style = match app.mode {
        AppMode::Normal => mode_normal(),
        AppMode::Search => mode_search(),
        AppMode::Confirm => mode_confirm(),
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
    let area = centered_rect(50, 3, f.area());

    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_active())
        .title(" Search ")
        .title_style(panel_title_active());

    let search_text = format!("{}▏", app.search_query);
    let paragraph = Paragraph::new(search_text).block(block).style(panel());

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
