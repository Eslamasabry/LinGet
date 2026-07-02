use crate::cli::tui::app::{App, MIN_HEIGHT, MIN_WIDTH};
use crate::cli::tui::components::{source_rail, workspace};
use crate::cli::tui::state::filters::ViewMode;
use crate::cli::tui::ui::{palette_overlay_rect, preflight_overlay_rect, source_count_label};
use ratatui::layout::{Constraint, Direction, Layout, Margin, Rect};
use unicode_width::UnicodeWidthStr;

/// Regions of the TUI layout for mouse hit-testing.
#[derive(Debug, Default)]
pub struct LayoutRegions {
    pub header_filter_row: Rect,
    pub filter_panel: Rect,
    pub sources: Rect,
    pub packages: Rect,
    pub details: Rect,
    pub queue_bar: Rect,
    pub expanded_queue: Rect,
    pub expanded_queue_tasks: Rect,
    pub expanded_queue_logs: Rect,
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
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(1),
        Constraint::Length(queue_height),
        Constraint::Length(3),
    ];
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    let main_area = chunks[2];
    let (queue_bar, _footer) = if queue_height > 0 {
        (chunks[3], chunks[4])
    } else {
        (Rect::default(), chunks[4])
    };

    // The Health dashboard shifts the workspace down; hit-testing must use
    // the same shifted area as the draw path or every click lands offset.
    let workspace_area = match dashboard_split(app, main_area) {
        Some((_, rest)) => rest,
        None => main_area,
    };

    let (filter_panel, sources, packages, details, expanded_queue) = if app.compact {
        compute_compact_regions(app, workspace_area)
    } else {
        compute_full_regions(app, workspace_area)
    };

    let (expanded_queue_tasks, expanded_queue_logs) = expanded_queue_sections(expanded_queue);

    LayoutRegions {
        header_filter_row: chunks[0],
        filter_panel,
        sources,
        packages,
        details,
        queue_bar,
        expanded_queue,
        expanded_queue_tasks,
        expanded_queue_logs,
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

/// The vertical split for the Health view: dashboard strip above, workspace
/// below. `None` when the dashboard isn't shown (other views, expanded
/// queue, or not enough height). Shared by the draw path and `compute_layout`
/// so rendering and mouse hit-testing can never disagree.
pub fn dashboard_split(app: &App, area: Rect) -> Option<(Rect, Rect)> {
    if app.view_mode != ViewMode::Dashboard || app.queue_expanded {
        return None;
    }
    let dashboard_height = (app.visible_sources().len() as u16 + 6)
        .min(area.height / 2)
        .max(6);
    if area.height <= dashboard_height + 8 {
        return None;
    }
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(dashboard_height), Constraint::Min(8)])
        .split(area);
    Some((chunks[0], chunks[1]))
}

pub fn compute_full_regions(app: &App, area: Rect) -> (Rect, Rect, Rect, Rect, Rect) {
    if app.queue_expanded {
        return (
            Rect::default(),
            Rect::default(),
            Rect::default(),
            Rect::default(),
            area,
        );
    }

    let vertical = workspace_vertical_regions(area);
    let filter_panel = vertical.0;
    let main = vertical.1;

    let source_width = if app.show_sidebar && main.width >= 92 {
        let _adaptive_source_width = sources_panel_width(app, main.width);
        source_rail::RAIL_WIDTH
    } else {
        0
    };
    let inspector_width = if main.width >= 142 {
        49
    } else if main.width >= 118 {
        42
    } else {
        0
    };
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(source_width),
            Constraint::Length(if source_width > 0 { 1 } else { 0 }),
            Constraint::Min(36),
            Constraint::Length(if inspector_width > 0 { 1 } else { 0 }),
            Constraint::Length(inspector_width),
        ])
        .split(main);

    (
        filter_panel,
        columns[0],
        columns[2],
        columns[4],
        Rect::default(),
    )
}

pub fn compute_compact_regions(app: &App, area: Rect) -> (Rect, Rect, Rect, Rect, Rect) {
    let sources = Rect::default();
    if app.queue_expanded {
        (
            Rect::default(),
            sources,
            Rect::default(),
            Rect::default(),
            area,
        )
    } else if area.height < 4 {
        (
            Rect::default(),
            sources,
            area,
            Rect::default(),
            Rect::default(),
        )
    } else {
        let vertical = workspace_vertical_regions(area);
        (
            vertical.0,
            sources,
            vertical.1,
            Rect::default(),
            Rect::default(),
        )
    }
}

fn workspace_vertical_regions(area: Rect) -> (Rect, Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(workspace::FILTER_PANEL_HEIGHT),
            Constraint::Length(workspace::WORKSPACE_GAP),
            Constraint::Min(8),
        ])
        .split(area);
    (chunks[0], chunks[2])
}

pub fn sources_panel_width(app: &App, area_width: u16) -> u16 {
    const SOURCES_MIN_WIDTH: u16 = 16;
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
        app.filter_counts[5],
    ];
    let all_label = format!("All{}", source_count_label(app.filter, all_counts));
    let mut max_width = UnicodeWidthStr::width(all_label.as_str());

    for source in app.visible_sources() {
        let counts = app
            .source_counts
            .get(&source)
            .copied()
            .unwrap_or([0, 0, 0, 0, 0, 0]);
        let label = format!("{}{}", source, source_count_label(app.filter, counts));
        max_width = max_width.max(UnicodeWidthStr::width(label.as_str()));
    }

    max_width
}

/// Mouse regions of the expanded queue: (lanes, details strip). Derived from
/// the kanban board's own geometry (`queue_board::board_regions`) so click
/// mapping always matches what is drawn.
pub fn expanded_queue_sections(area: Rect) -> (Rect, Rect) {
    if area.width <= 2 || area.height <= 2 {
        return (Rect::default(), Rect::default());
    }

    let inner = area.inner(Margin {
        vertical: 1,
        horizontal: 1,
    });
    if inner.width == 0 || inner.height == 0 {
        return (Rect::default(), Rect::default());
    }

    match crate::cli::tui::components::queue_board::board_regions(inner) {
        Some((lanes, details)) => (lanes, details),
        // Summary-only fallback: no per-task rows to click.
        None => (Rect::default(), Rect::default()),
    }
}
