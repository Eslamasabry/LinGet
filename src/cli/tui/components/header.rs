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

pub fn draw_filter_bar(frame: &mut Frame, app: &App, area: Rect) {
    let compact_tabs = compact_tabs(app, area.width);
    let mut left = header_brand_spans();
    for spec in filter_tab_specs(app, compact_tabs) {
        left.extend(render_filter_tab(
            spec.key,
            spec.label,
            Some(spec.count),
            spec.active,
            app.searching,
        ));
        left.push(Span::styled(" ", header_bar()));
    }

    let mut right: Vec<Span> = Vec::new();

    right.push(Span::styled(
        format!("[{}] ", app.tui_mode_label()),
        mode_badge_style(app),
    ));
    right.push(Span::styled(
        format!("{} ", crate::cli::tui::theme::current_theme_name()),
        Style::default()
            .fg(palette::DARK_GRAY())
            .bg(palette::HEADER_BG())
            .add_modifier(Modifier::ITALIC),
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

struct FilterTabSpec {
    key: &'static str,
    label: &'static str,
    count: usize,
    active: bool,
    filter: Filter,
}

fn header_brand_spans() -> Vec<Span<'static>> {
    vec![
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
        Span::styled("  ", header_bar()),
    ]
}

fn compact_tabs(app: &App, width: u16) -> bool {
    app.compact || width < 112
}

fn filter_tab_specs(app: &App, compact: bool) -> [FilterTabSpec; 6] {
    let installed_label = if compact { "Inst" } else { "Installed" };
    let updates_label = if compact { "Upd" } else { "Updates" };
    let favorites_label = if compact { "Fav" } else { "Favorites" };
    let security_label = if compact { "Sec" } else { "Security" };
    let dupes_label = if compact { "Dup" } else { "Duplicates" };

    [
        FilterTabSpec {
            key: "1",
            label: "All",
            count: app.filter_counts[0],
            active: app.filter == Filter::All,
            filter: Filter::All,
        },
        FilterTabSpec {
            key: "2",
            label: installed_label,
            count: app.filter_counts[1],
            active: app.filter == Filter::Installed,
            filter: Filter::Installed,
        },
        FilterTabSpec {
            key: "3",
            label: updates_label,
            count: app.filter_counts[2],
            active: app.filter == Filter::Updates,
            filter: Filter::Updates,
        },
        FilterTabSpec {
            key: "4",
            label: favorites_label,
            count: app.filter_counts[3],
            active: app.filter == Filter::Favorites,
            filter: Filter::Favorites,
        },
        FilterTabSpec {
            key: "5",
            label: security_label,
            count: app.filter_counts[4],
            active: app.filter == Filter::SecurityUpdates,
            filter: Filter::SecurityUpdates,
        },
        FilterTabSpec {
            key: "6",
            label: dupes_label,
            count: app.filter_counts[5],
            active: app.filter == Filter::Duplicates,
            filter: Filter::Duplicates,
        },
    ]
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

    let mut cursor = header_filter_row.x + spans_width(&header_brand_spans()) as u16;
    let tabs = filter_tab_specs(app, compact_tabs(app, header_filter_row.width));

    for spec in tabs {
        let width = spans_width(&render_filter_tab(
            spec.key,
            spec.label,
            Some(spec.count),
            spec.active,
            app.searching,
        )) as u16;
        if col >= cursor && col < cursor.saturating_add(width) {
            return Some(spec.filter);
        }
        cursor = cursor.saturating_add(width);
        cursor = cursor.saturating_add(1);
    }

    None
}
