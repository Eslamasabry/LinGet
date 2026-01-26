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

const NARROW_WIDTH_THRESHOLD: u16 = 90;
const NARROW_HEIGHT_THRESHOLD: u16 = 24;

pub fn draw(f: &mut Frame, app: &App) {
    let area = f.area();
    let chunks = draw_main_layout(area);

    draw_title_bar(f, app, chunks[0]);
    draw_main_content(f, app, chunks[1]);
    draw_console_panel(f, app, chunks[2]);
    draw_commands_bar(f, app, chunks[3]);
    draw_status_bar(f, app, chunks[4]);

    // Draw search popup if in search mode
    if app.mode == AppMode::Search {
        draw_search_popup(f, app);
    }
}

fn draw_main_layout(area: Rect) -> Vec<Rect> {
    let total_height = area.height;

    if total_height <= NARROW_HEIGHT_THRESHOLD {
        draw_main_layout_narrow(area)
    } else {
        draw_main_layout_wide(area)
    }
}

fn draw_main_layout_wide(area: Rect) -> Vec<Rect> {
    Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(6),
            Constraint::Length(1),
            Constraint::Length(3),
        ])
        .split(area)
        .to_vec()
}

fn draw_main_layout_narrow(area: Rect) -> Vec<Rect> {
    Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Length(3),
        ])
        .split(area)
        .to_vec()
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
    if area.width <= NARROW_WIDTH_THRESHOLD {
        draw_main_content_narrow(f, app, area);
    } else {
        draw_main_content_wide(f, app, area);
    }
}

fn draw_main_content_wide(f: &mut Frame, app: &App, area: Rect) {
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
    draw_details_panel(f, app, chunks[2]);
}

fn draw_main_content_narrow(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(18),
            Constraint::Min(20),
            Constraint::Min(25),
        ])
        .split(area);

    draw_sources_panel(f, app, chunks[0]);
    draw_packages_panel(f, app, chunks[1]);
    draw_details_panel(f, app, chunks[2]);
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
        let loading = Paragraph::new("Loading...")
            .block(block)
            .style(status_loading());
        f.render_widget(loading, area);
        return;
    }

    let selected_package = app.selected_package();

    if selected_package.is_none() {
        let empty = Paragraph::new("Select a package to view details")
            .block(block)
            .style(dim());
        f.render_widget(empty, area);
        return;
    }

    let pkg = selected_package.unwrap();

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

    let name_line = Line::from(vec![
        Span::styled("Name: ", label()),
        Span::styled(&pkg.name, panel_title_active()),
    ]);

    let source_line = Line::from(vec![
        Span::styled("Source: ", label()),
        Span::styled(format!("{}", pkg.source), panel()),
    ]);

    let status_line = Line::from(vec![
        Span::styled("Status: ", label()),
        Span::styled(status_text, status_style_for_status(pkg.status)),
    ]);

    let version_line = Line::from(vec![
        Span::styled("Version: ", label()),
        Span::styled(version_info, panel()),
    ]);

    let description_line = Line::from(vec![Span::styled("Description: ", label())]);

    let wrapped_description = wrap_text(&pkg.description, area.width.saturating_sub(2) as usize);

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

fn status_style_for_status(status: PackageStatus) -> Style {
    match status {
        PackageStatus::Installed => status_installed(),
        PackageStatus::UpdateAvailable => status_update(),
        PackageStatus::NotInstalled => status_not_installed(),
        PackageStatus::Installing | PackageStatus::Updating => status_loading(),
        PackageStatus::Removing => status_removing(),
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
    let words: Vec<&str> = text.split_whitespace().collect();
    let mut word_iter = words.into_iter();

    if let Some(first_word) = word_iter.next() {
        current_line.push_str(first_word);
    }

    for word in word_iter {
        let potential = if current_line.is_empty() {
            word.to_string()
        } else {
            format!("{} {}", current_line, word)
        };

        if potential.len() <= max_width {
            current_line = potential;
        } else {
            if !current_line.is_empty() {
                lines.push(current_line.clone());
            }
            current_line = word.to_string();
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

fn draw_console_panel(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_inactive())
        .title(" Console ")
        .title_style(panel_title());

    if app.console_buffer.is_empty() {
        let empty = Paragraph::new("No recent actions")
            .block(block)
            .style(dim());
        f.render_widget(empty, area);
        return;
    }

    let available_height = area.height.saturating_sub(2) as usize;
    let total_lines = app.console_buffer.len();
    let start_index = total_lines.saturating_sub(available_height);

    let lines: Vec<Line> = app
        .console_buffer
        .iter()
        .skip(start_index)
        .take(available_height)
        .map(|s| Line::from(Span::styled(s, panel())))
        .collect();

    let paragraph = Paragraph::new(lines).block(block);
    f.render_widget(paragraph, area);
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
                Span::styled(" U", key_hint()),
                Span::styled(" update-all ", description()),
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
