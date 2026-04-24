//! Single-view workspace layout.
//!
//! Replaces the legacy F1/F2/F3 mode dispatch with one attention-first view:
//!
//! ```text
//! ┌ header band ──────────────────────────────────────┐
//! │ attention blocks (hero cards)                     │
//! │ ─────────────────────────────────── APT 48 ─┐    │
//! │   /search or :command                       │    │
//! │   package list rows…                        │    │
//! │ ────────────────────────────────────────────┘    │
//! │ details one-liner (i to expand)                   │
//! │ footer keys (6)                                   │
//! └───────────────────────────────────────────────────┘
//! ```
//!
//! Overlays (palette, help, preflight, changelog, import, queue-expanded)
//! are drawn on top in `ui::draw` and are unchanged by this module.

use super::attention;
use super::packages::draw_packages_panel;
use super::source_rail;
use crate::cli::tui::app::App;
use crate::cli::tui::components::details::draw_compact_details_summary;
use crate::cli::tui::state::filters::Focus;
use crate::cli::tui::theme::{accent, dim, muted, palette, text};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

pub fn draw(frame: &mut Frame, app: &mut App, area: Rect) {
    // Split vertically: attention · list(with right-rail) · details · footer
    //
    // The attention section is sized dynamically (up to ~40% of area) based
    // on the number of blocks. Footer is always 1 line, details always 2.
    let blocks = attention::build_blocks(app);
    let attention_height = attention::desired_height(&blocks)
        .min(area.height.saturating_sub(8)) // leave room for list+details+footer
        .min(area.height / 2);

    let details_height: u16 = 2;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(attention_height),
            Constraint::Min(5),
            Constraint::Length(details_height),
        ])
        .split(area);

    if attention_height > 0 {
        attention::draw(frame, &blocks, chunks[0]);
    }

    draw_list_region(frame, app, chunks[1]);
    draw_details_strip(frame, app, chunks[2]);
}

fn draw_list_region(frame: &mut Frame, app: &mut App, area: Rect) {
    if app.focus == Focus::Sources {
        if let Some(source) = app.source {
            app.load_repositories(source);
        }
    }

    // Split horizontally: list · source rail
    let rail_width = if area.width > 40 {
        source_rail::RAIL_WIDTH
    } else {
        0
    };
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(20),
            Constraint::Length(if rail_width > 0 { 1 } else { 0 }), // gutter
            Constraint::Length(rail_width),
        ])
        .split(area);

    // Above the list: the search/command combo input (one line) + divider.
    let list_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // search/command bar
            Constraint::Length(1), // divider
            Constraint::Min(1),    // table
        ])
        .split(columns[0]);

    draw_search_combo(frame, app, list_chunks[0]);
    draw_divider(frame, app, list_chunks[1]);
    // Re-use existing packages panel (will be refined in a later phase).
    draw_packages_panel(frame, app, list_chunks[2], true);

    if rail_width > 0 {
        source_rail::draw(frame, app, columns[2]);
    }
}

fn draw_search_combo(frame: &mut Frame, app: &App, area: Rect) {
    let mut spans: Vec<Span> = Vec::new();
    let placeholder_style = dim().add_modifier(Modifier::ITALIC);
    spans.push(Span::styled(" ", text()));
    if app.searching {
        spans.push(Span::styled("/ ", accent().add_modifier(Modifier::BOLD)));
        spans.push(Span::styled(app.search.clone(), text()));
        spans.push(Span::styled("▌", accent().add_modifier(Modifier::BOLD)));
    } else if !app.search.is_empty() {
        spans.push(Span::styled("/ ", muted()));
        spans.push(Span::styled(app.search.clone(), text()));
        spans.push(Span::styled("  (Esc clears)", dim()));
    } else {
        spans.push(Span::styled("/ search", placeholder_style));
        spans.push(Span::styled("   ", dim()));
        spans.push(Span::styled(": command", placeholder_style));
        spans.push(Span::styled("   ", dim()));
        spans.push(Span::styled("? help", placeholder_style));
    }
    // Right-align current filter chip.
    let filter_chip = filter_chip_for(app);
    let right = Line::from(vec![filter_chip]).alignment(ratatui::layout::Alignment::Right);
    let left = Line::from(spans);
    frame.render_widget(Paragraph::new(left), area);
    frame.render_widget(Paragraph::new(right), area);
}

fn filter_chip_for(app: &App) -> Span<'static> {
    use crate::cli::tui::state::filters::Filter;
    let (label, count) = match app.filter {
        Filter::All => ("all", app.filter_counts[0]),
        Filter::Installed => ("installed", app.filter_counts[1]),
        Filter::Updates => ("updates", app.filter_counts[2]),
        Filter::Favorites => ("favorites", app.filter_counts[3]),
        Filter::SecurityUpdates => ("security", app.filter_counts[4]),
        Filter::Duplicates => ("duplicates", app.filter_counts[5]),
    };
    Span::styled(
        format!(" {} · {} ", label, count),
        accent().add_modifier(Modifier::REVERSED),
    )
}

fn draw_divider(frame: &mut Frame, _app: &App, area: Rect) {
    if area.width == 0 {
        return;
    }
    let line: String = "─".repeat(area.width as usize);
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            line,
            Style::default().fg(palette::DARK_GRAY()),
        ))),
        area,
    );
}

fn draw_details_strip(frame: &mut Frame, app: &App, area: Rect) {
    if area.height == 0 {
        return;
    }
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(area);
    // Top row: divider
    let line: String = "─".repeat(area.width as usize);
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            line,
            Style::default().fg(palette::DARK_GRAY()),
        ))),
        rows[0],
    );
    // Bottom row: reuse existing compact details summary
    draw_compact_details_summary(frame, app, rows[1]);
}
