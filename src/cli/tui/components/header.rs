use crate::cli::tui::app::App;
use crate::cli::tui::state::filters::ViewMode;
use crate::cli::tui::theme::{header_bar, palette, tab_active};
use crate::cli::tui::ui::{compose_left_right, spans_width};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::Span,
    widgets::Paragraph,
    Frame,
};

pub fn draw_filter_bar(frame: &mut Frame, app: &App, area: Rect) {
    let compact = use_compact_tabs(app, area.width);
    let mut left = header_brand_spans();
    for spec in primary_tab_specs_with(app, compact) {
        left.extend(render_primary_tab(&spec.label, spec.active, app.searching));
        left.push(Span::raw("  "));
    }

    let right = Vec::new();

    let line = compose_left_right(left, right, area.width as usize);
    frame.render_widget(Paragraph::new(line).style(header_bar()), area);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeaderAction {
    Today,
    Browse,
    Queue,
}

struct PrimaryTabSpec {
    label: String,
    active: bool,
    action: HeaderAction,
}

fn header_brand_spans() -> Vec<Span<'static>> {
    vec![
        Span::styled(
            " LinGet ",
            Style::default()
                .fg(palette::CYAN())
                .bg(palette::HEADER_BG())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("   ", header_bar()),
    ]
}

/// Whether the tab row needs the short label set: either the whole UI is in
/// compact mode, or the full-word tabs simply don't fit this width.
fn use_compact_tabs(app: &App, width: u16) -> bool {
    if app.compact {
        return true;
    }
    let mut needed = spans_width(&header_brand_spans());
    for spec in primary_tab_specs_with(app, false) {
        needed += spans_width(&render_primary_tab(&spec.label, spec.active, app.searching)) + 2;
    }
    needed > width as usize
}

fn primary_tab_specs_with(app: &App, _compact: bool) -> Vec<PrimaryTabSpec> {
    let (queued, running, _completed, failed, _cancelled) = app.queue_counts();
    let queue_count = queued + running + failed;
    let (today, browse, queue) = ("F1 Today", "F2 Browse", "F3 Queue");
    vec![
        PrimaryTabSpec {
            label: today.to_string(),
            active: app.view_mode == ViewMode::Today,
            action: HeaderAction::Today,
        },
        PrimaryTabSpec {
            label: format!("{} {}", browse, compact_count(app.filter_counts[0])),
            active: app.view_mode == ViewMode::Browse,
            action: HeaderAction::Browse,
        },
        PrimaryTabSpec {
            label: format!("{} {}", queue, compact_count(queue_count)),
            active: app.view_mode == ViewMode::Queue,
            action: HeaderAction::Queue,
        },
    ]
}

fn render_primary_tab(label: &str, active: bool, searching: bool) -> Vec<Span<'static>> {
    if active && !searching {
        vec![
            Span::styled("[ ", tab_active()),
            Span::styled(label.to_string(), tab_active()),
            Span::styled(" ]", tab_active()),
        ]
    } else {
        vec![
            Span::styled("[ ", header_bar()),
            Span::styled(
                label.to_string(),
                Style::default()
                    .fg(palette::DARK_GRAY())
                    .bg(palette::HEADER_BG()),
            ),
            Span::styled(" ]", header_bar()),
        ]
    }
}

fn compact_count(value: usize) -> String {
    if value >= 10_000 {
        format!("{:.1}k", value as f64 / 1000.0)
    } else {
        value.to_string()
    }
}

pub fn header_action_hit_test(
    app: &App,
    header_filter_row: Rect,
    col: u16,
    row: u16,
) -> Option<HeaderAction> {
    if header_filter_row.width == 0 || header_filter_row.height == 0 || row != header_filter_row.y {
        return None;
    }
    if col < header_filter_row.x || col >= header_filter_row.x + header_filter_row.width {
        return None;
    }

    let mut cursor = header_filter_row.x + spans_width(&header_brand_spans()) as u16;
    let tabs = primary_tab_specs_with(app, use_compact_tabs(app, header_filter_row.width));

    for spec in tabs {
        let width =
            spans_width(&render_primary_tab(&spec.label, spec.active, app.searching)) as u16;
        if col >= cursor && col < cursor.saturating_add(width) {
            return Some(spec.action);
        }
        cursor = cursor.saturating_add(width).saturating_add(2);
    }

    None
}
