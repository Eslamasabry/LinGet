use crate::cli::tui::app::{App, MIN_HEIGHT, MIN_WIDTH};
use crate::cli::tui::ui::{palette_overlay_rect, preflight_overlay_rect, source_count_label};
use ratatui::layout::{Constraint, Direction, Layout, Margin, Rect};
use unicode_width::UnicodeWidthStr;

/// Regions of the TUI layout for mouse hit-testing.
#[derive(Debug, Default)]
pub struct LayoutRegions {
    pub header_filter_row: Rect,
    pub sources: Rect,
    pub packages: Rect,
    pub details: Rect,
    pub queue_bar: Rect,
    pub expanded_queue: Rect,
    pub expanded_queue_tasks: Rect,
    pub expanded_queue_logs: Rect,
    pub expanded_queue_hints: Rect,
    pub palette: Rect,
    pub preflight_modal: Rect,
}

/// Compute layout regions matching the draw logic, for mouse hit-testing.
pub fn compute_layout(app: &App, area: Rect) -> LayoutRegions {
    if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
        return LayoutRegions::default();
    }

    let queue_height = app.queue_bar_height();
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

    let header = chunks[0];
    let header_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(header);

    let main_area = chunks[1];
    let (queue_bar, _footer) = if queue_height > 0 {
        (chunks[2], chunks[3])
    } else {
        (Rect::default(), chunks[2])
    };

    let (sources, packages, details, expanded_queue) = if app.compact {
        compute_compact_regions(app, main_area)
    } else {
        compute_full_regions(app, main_area)
    };

    let (expanded_queue_tasks, expanded_queue_logs, expanded_queue_hints) =
        expanded_queue_sections(expanded_queue);

    LayoutRegions {
        header_filter_row: header_chunks[0],
        sources,
        packages,
        details,
        queue_bar,
        expanded_queue,
        expanded_queue_tasks,
        expanded_queue_logs,
        expanded_queue_hints,
        palette: if app.showing_palette {
            palette_overlay_rect(area)
        } else {
            Rect::default()
        },
        preflight_modal: if app.confirming.is_some() {
            preflight_overlay_rect(area)
        } else {
            Rect::default()
        },
    }
}

pub fn compute_full_regions(app: &App, area: Rect) -> (Rect, Rect, Rect, Rect) {
    let source_width = if app.show_sidebar { sources_panel_width(app, area.width) } else { 0 };
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(source_width), Constraint::Min(1)])
        .split(area);

    let sources = columns[0];
    let right = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(65), Constraint::Percentage(35)])
        .split(columns[1]);

    let packages = right[0];
    if app.queue_expanded {
        (sources, packages, Rect::default(), right[1])
    } else {
        (sources, packages, right[1], Rect::default())
    }
}

pub fn compute_compact_regions(app: &App, area: Rect) -> (Rect, Rect, Rect, Rect) {
    let sources = Rect::default();
    if app.queue_expanded {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(area);
        (sources, chunks[0], Rect::default(), chunks[1])
    } else if area.height < 4 {
        (sources, area, Rect::default(), Rect::default())
    } else {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(2)])
            .split(area);
        (sources, chunks[0], chunks[1], Rect::default())
    }
}

pub fn sources_panel_width(app: &App, area_width: u16) -> u16 {
    const SOURCES_MIN_WIDTH: u16 = 18;
    const SOURCES_MAX_WIDTH: u16 = 36;

    let content_width = u16::try_from(sources_panel_content_width(app)).unwrap_or(u16::MAX);
    let desired = content_width
        .saturating_add(4)
        .clamp(SOURCES_MIN_WIDTH, SOURCES_MAX_WIDTH);

    desired.min(area_width.saturating_sub(1).max(1))
}

pub fn sources_panel_content_width(app: &App) -> usize {
    let all_counts = [
        app.filter_counts[0],
        app.filter_counts[1],
        app.filter_counts[2],
        app.filter_counts[3],
        app.filter_counts[4],
    ];
    let all_label = format!("All{}", source_count_label(app.filter, all_counts));
    let mut max_width = UnicodeWidthStr::width(all_label.as_str());

    for source in app.visible_sources() {
        let counts = app
            .source_counts
            .get(&source)
            .copied()
            .unwrap_or([0, 0, 0, 0, 0]);
        let label = format!("{}{}", source, source_count_label(app.filter, counts));
        max_width = max_width.max(UnicodeWidthStr::width(label.as_str()));
    }

    max_width
}

pub fn expanded_queue_sections(area: Rect) -> (Rect, Rect, Rect) {
    if area.width <= 2 || area.height <= 2 {
        return (Rect::default(), Rect::default(), Rect::default());
    }

    let inner = area.inner(Margin {
        vertical: 1,
        horizontal: 1,
    });
    if inner.width == 0 || inner.height == 0 {
        return (Rect::default(), Rect::default(), Rect::default());
    }

    if inner.height >= 8 {
        let sections = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(55),
                Constraint::Percentage(35),
                Constraint::Length(1),
            ])
            .split(inner);
        (sections[0], sections[1], sections[2])
    } else {
        let sections = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(inner);
        (sections[0], Rect::default(), sections[1])
    }
}
