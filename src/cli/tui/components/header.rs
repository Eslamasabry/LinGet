use crate::cli::tui::app::App;
use crate::cli::tui::state::filters::Filter;
use crate::cli::tui::theme::{
    accent, header_bar, italic_status, loading, muted, palette, tab_active, warning,
};
use crate::cli::tui::ui::{compose_left_right, spans_width};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::Span,
    widgets::Paragraph,
    Frame,
};
use unicode_width::UnicodeWidthStr;

pub fn draw_filter_bar(frame: &mut Frame, app: &App, area: Rect) {
    let mut left: Vec<Span> = vec![
        Span::styled(
            " ❖ ",
            Style::default()
                .fg(palette::CYAN())
                .bg(palette::HEADER_BG())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "LinGet",
            Style::default()
                .fg(palette::WHITE())
                .bg(palette::HEADER_BG())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("  ·  {}", crate::cli::tui::theme::current_theme_name()),
            Style::default()
                .fg(palette::DARK_GRAY())
                .bg(palette::HEADER_BG())
                .add_modifier(Modifier::ITALIC),
        ),
        Span::styled(
            format!("  ·  {} pkgs", app.filter_counts[0]),
            Style::default()
                .fg(palette::DARK_GRAY())
                .bg(palette::HEADER_BG()),
        ),
    ];

    let mut right: Vec<Span> = Vec::new();

    right.push(Span::styled(
        format!("[{}] ", app.tui_mode_label()),
        mode_badge_style(app),
    ));

    if let Some(activity) = app.catalog_activity_label() {
        right.push(Span::styled(
            format!("{} {} ", app.spinner_frame(), activity),
            loading(),
        ));
    }

    if !app.status.is_empty() {
        right.push(Span::styled(
            format!("{} ", app.status.clone()),
            italic_status(),
        ));
    }

    // Reserve a trailing space so the right edge doesn't touch the border.
    left.push(Span::styled(" ", header_bar()));

    let line = compose_left_right(left, right, area.width as usize);
    let paragraph = Paragraph::new(line).style(header_bar());
    frame.render_widget(paragraph, area);
}

fn mode_badge_style(app: &App) -> Style {
    if app.showing_palette
        || app.showing_changelog
        || app.showing_help
        || app.confirming.is_some()
        || app.showing_import_preview
    {
        accent()
    } else if app.searching || app.search_results.is_some() || !app.search.is_empty() {
        warning()
    } else if app.queue_expanded && app.focus == crate::cli::tui::state::filters::Focus::Queue {
        loading()
    } else {
        muted()
    }
}

fn render_filter_tab(
    key: &str,
    label: &str,
    count: Option<usize>,
    active: bool,
    searching: bool,
) -> Vec<Span<'static>> {
    if active && !searching {
        let mut spans = vec![
            Span::styled(" ", tab_active()),
            Span::styled(
                key.to_string(),
                Style::default()
                    .fg(palette::YELLOW())
                    .bg(palette::TAB_ACTIVE_BG())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" ", tab_active()),
            Span::styled(label.to_string(), tab_active()),
        ];
        if let Some(count) = count {
            spans.push(Span::styled(" ", tab_active()));
            spans.push(Span::styled(count.to_string(), tab_active()));
        }
        spans.push(Span::styled(" ", tab_active()));
        spans
    } else {
        let mut spans = vec![
            Span::styled(" ", header_bar()),
            Span::styled(
                key.to_string(),
                Style::default()
                    .fg(palette::DARK_GRAY())
                    .bg(palette::HEADER_BG()),
            ),
            Span::styled(" ", header_bar()),
            Span::styled(
                label.to_string(),
                Style::default()
                    .fg(palette::DARK_GRAY())
                    .bg(palette::HEADER_BG()),
            ),
        ];
        if let Some(count) = count {
            spans.push(Span::styled(" ", header_bar()));
            spans.push(Span::styled(
                count.to_string(),
                Style::default()
                    .fg(palette::DARK_GRAY())
                    .bg(palette::HEADER_BG()),
            ));
        }
        spans.push(Span::styled(" ", header_bar()));
        spans
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
            (app.filter == Filter::All).then_some(app.filter_counts[0]),
            app.filter == Filter::All,
            Filter::All,
        ),
        (
            "2",
            installed_label,
            (app.filter == Filter::Installed).then_some(app.filter_counts[1]),
            app.filter == Filter::Installed,
            Filter::Installed,
        ),
        (
            "3",
            updates_label,
            (app.filter == Filter::Updates).then_some(app.filter_counts[2]),
            app.filter == Filter::Updates,
            Filter::Updates,
        ),
        (
            "4",
            favorites_label,
            (app.filter == Filter::Favorites).then_some(app.filter_counts[3]),
            app.filter == Filter::Favorites,
            Filter::Favorites,
        ),
        (
            "5",
            "Security",
            (app.filter == Filter::SecurityUpdates).then_some(app.filter_counts[4]),
            app.filter == Filter::SecurityUpdates,
            Filter::SecurityUpdates,
        ),
        (
            "6",
            dupes_label,
            (app.filter == Filter::Duplicates).then_some(app.filter_counts[5]),
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
