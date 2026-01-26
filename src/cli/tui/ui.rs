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
use unicode_width::UnicodeWidthStr;

pub fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title bar
            Constraint::Min(10),   // Main content
            Constraint::Length(6), // Console panel
            Constraint::Length(1), // Commands bar
            Constraint::Length(3), // Status bar
        ])
        .split(f.area());

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

fn draw_title_bar(f: &mut Frame, app: &App, area: Rect) {
    let (title, title_color) = if app.compact {
        (" LinGet ", title_color())
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
        .map(|s| format!("{:?}", s))
        .unwrap_or_else(|| "All".to_string());

    let pkg_count = if app.compact {
        format!(" {} | {} ", app.filtered_packages.len(), source_name)
    } else if app.show_updates_only {
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
    if app.compact {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(12),
                Constraint::Min(15),
                Constraint::Length(15),
            ])
            .split(area);

        draw_sources_panel(f, app, chunks[0]);
        draw_packages_panel(f, app, chunks[1]);
        draw_details_panel(f, app, chunks[2]);
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
        draw_details_panel(f, app, chunks[2]);
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
        let empty = Paragraph::new("No recent actions")
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
    let commands = match app.mode {
        AppMode::Normal => {
            let updates_label = if app.show_updates_only {
                "all"
            } else {
                "updates"
            };
            if app.compact {
                vec![
                    Span::styled("↑↓/jk", key_hint()),
                    Span::styled("n", description()),
                    Span::styled("│", separator()),
                    Span::styled(" Tab", key_hint()),
                    Span::styled("p", description()),
                    Span::styled("│", separator()),
                    Span::styled(" /", key_hint()),
                    Span::styled("s", description()),
                    Span::styled("│", separator()),
                    Span::styled(" u", key_hint()),
                    Span::styled(updates_label, description()),
                    Span::styled("│", separator()),
                    Span::styled(" U", key_hint()),
                    Span::styled("upd", description()),
                    Span::styled("│", separator()),
                    Span::styled(" r", key_hint()),
                    Span::styled("ref", description()),
                    Span::styled("│", separator()),
                    Span::styled(" i", key_hint()),
                    Span::styled("ins", description()),
                    Span::styled("│", separator()),
                    Span::styled(" x", key_hint()),
                    Span::styled("rm", description()),
                    Span::styled("│", separator()),
                    Span::styled(" q", key_hint()),
                    Span::styled("q", description()),
                ]
            } else {
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
                    Span::styled(" update-all (filtered) ", description()),
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
        }
        AppMode::Search => {
            if app.compact {
                vec![
                    Span::styled("Enter", key_hint()),
                    Span::styled("ok", description()),
                    Span::styled("│", separator()),
                    Span::styled(" Esc", key_hint()),
                    Span::styled("ca", description()),
                ]
            } else {
                vec![
                    Span::styled("Enter", key_hint()),
                    Span::styled(" confirm ", description()),
                    Span::styled("│", separator()),
                    Span::styled(" Esc", key_hint()),
                    Span::styled(" cancel", description()),
                ]
            }
        }
        AppMode::Confirm => {
            if app.compact {
                vec![
                    Span::styled("y", key_hint()),
                    Span::styled("y", description()),
                    Span::styled("│", separator()),
                    Span::styled(" n", key_hint()),
                    Span::styled("n", description()),
                ]
            } else {
                vec![
                    Span::styled("y", key_hint()),
                    Span::styled(" yes ", description()),
                    Span::styled("│", separator()),
                    Span::styled(" n", key_hint()),
                    Span::styled(" no", description()),
                ]
            }
        }
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
