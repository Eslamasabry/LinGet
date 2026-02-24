use crate::cli::tui::app::App;
use crate::cli::tui::state::filters::Filter;
use crate::cli::tui::theme::{accent, badge_installed, badge_not_installed, badge_progress, badge_update, footer_label, header_bar, loading, muted, palette, tab_active, italic_status};
use crate::cli::tui::ui::{compose_left_right, spans_width};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
    Frame,
};
use unicode_width::UnicodeWidthStr;

pub fn draw_filter_bar(frame: &mut Frame, app: &App, area: Rect) {
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
    let favorites_label = if app.compact { "Fav" } else { "Favorites" };

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
    left.push(Span::raw(" "));
    left.extend(render_filter_tab(
        "4",
        favorites_label,
        app.filter_counts[3],
        app.filter == Filter::Favorites,
        app.searching,
    ));
    if !app.compact {
        left.push(Span::raw(" "));
        left.extend(render_filter_tab(
            "5",
            "Security",
            app.filter_counts[4],
            app.filter == Filter::SecurityUpdates,
            app.searching,
        ));
    }
    left.push(Span::raw(" "));
    let dupes_label = if app.compact { "Dup" } else { "Dupes" };
    left.extend(render_filter_tab(
        "6",
        dupes_label,
        app.filter_counts[5],
        app.filter == Filter::Duplicates,
        app.searching,
    ));

    let mut right = Vec::new();

    if app.loading {
        right.push(Span::styled(format!("{} ", app.spinner_frame()), loading()));
    }

    if app.searching {
        right.push(Span::styled(
            format!(
                "/ {}█ ",
                render_search_input(&app.search, area.width as usize / 3)
            ),
            accent(),
        ));
    } else if !app.search.is_empty() {
        right.push(Span::styled(format!("/ \"{}\" ", app.search), muted()));
    } else if app.filter == Filter::Favorites && app.favorites_updates_only {
        right.push(Span::styled("Favorites: updates only [v] ", muted()));
    }

    if !app.status.is_empty() && (!app.searching || area.width > 80) {
        right.push(Span::styled(app.status.clone(), italic_status()));
    }

    let line = compose_left_right(left, right, area.width as usize);
    let paragraph = Paragraph::new(line).style(header_bar());
    frame.render_widget(paragraph, area);
}


pub fn draw_status_legend(frame: &mut Frame, area: Rect) {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueHintAction {
    Retry,
    RetrySafe,
    Remediate,
}


pub fn header_filter_hit_test(
    app: &App,
    header_filter_row: Rect,
    col: u16,
    row: u16,
) -> Option<Filter> {
    if header_filter_row.width == 0 || header_filter_row.height == 0 || row != header_filter_row.y {
        return None;
    }
    if col < header_filter_row.x || col >= header_filter_row.x + header_filter_row.width {
        return None;
    }

    let installed_label = if app.compact { "Inst" } else { "Installed" };
    let updates_label = if app.compact { "Upd" } else { "Updates" };
    let favorites_label = if app.compact { "Fav" } else { "Favorites" };
    let dupes_label = if app.compact { "Dup" } else { "Dupes" };

    let tabs = [
        (
            "1",
            "All",
            app.filter_counts[0],
            app.filter == Filter::All,
            Filter::All,
        ),
        (
            "2",
            installed_label,
            app.filter_counts[1],
            app.filter == Filter::Installed,
            Filter::Installed,
        ),
        (
            "3",
            updates_label,
            app.filter_counts[2],
            app.filter == Filter::Updates,
            Filter::Updates,
        ),
        (
            "4",
            favorites_label,
            app.filter_counts[3],
            app.filter == Filter::Favorites,
            Filter::Favorites,
        ),
        (
            "5",
            "Security",
            app.filter_counts[4],
            app.filter == Filter::SecurityUpdates,
            Filter::SecurityUpdates,
        ),
        (
            "6",
            dupes_label,
            app.filter_counts[5],
            app.filter == Filter::Duplicates,
            Filter::Duplicates,
        ),
    ];

    let mut cursor = header_filter_row.x
        + UnicodeWidthStr::width(" ◆ ") as u16
        + UnicodeWidthStr::width("LinGet ") as u16
        + 1;

    for (index, (key, label, count, active, filter)) in tabs.iter().enumerate() {
        let width = spans_width(&render_filter_tab(
            key,
            label,
            *count,
            *active,
            app.searching,
        )) as u16;
        if col >= cursor && col < cursor.saturating_add(width) {
            return Some(*filter);
        }
        cursor = cursor.saturating_add(width);
        if index < tabs.len() - 1 {
            cursor = cursor.saturating_add(1);
        }
    }

    None
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


