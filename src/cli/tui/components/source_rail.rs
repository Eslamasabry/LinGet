//! Thin right-rail replacement for the bordered Sources panel.
//!
//! Each source gets one row: a colored sigil + short name + count. The
//! currently-filtered-through source is highlighted with a ▸ marker. The
//! rail width is fixed (10 cells) and borderless — the list uses whitespace
//! + accent color instead of boxes.

use crate::cli::tui::app::App;
use crate::cli::tui::state::filters::Focus;
use crate::cli::tui::theme::{accent, dim, muted, row_cursor, source_color};
use crate::models::PackageSource;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

/// Preferred rail width. Wide enough for `APT 139` plus one-cell gutter.
pub const RAIL_WIDTH: u16 = 12;

/// Short 3–4 character label for a source. Keeps the rail narrow.
fn short_label(source: PackageSource) -> &'static str {
    match source {
        PackageSource::Apt => "APT",
        PackageSource::Dnf => "DNF",
        PackageSource::Pacman => "Pac",
        PackageSource::Zypper => "Zyp",
        PackageSource::Flatpak => "Fla",
        PackageSource::Snap => "Snp",
        PackageSource::AppImage => "App",
        PackageSource::Npm => "npm",
        PackageSource::Pip => "pip",
        PackageSource::Pipx => "pix",
        PackageSource::Cargo => "crg",
        PackageSource::Dart => "drt",
        PackageSource::Brew => "brw",
        PackageSource::Deb => "Deb",
        PackageSource::Aur => "aur",
        PackageSource::Conda => "cnd",
        PackageSource::Mamba => "mmb",
        PackageSource::Winget => "wgt",
        PackageSource::Chocolatey => "cho",
        PackageSource::Scoop => "scp",
    }
}

pub fn draw(frame: &mut Frame, app: &App, area: Rect) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let focused = app.focus == Focus::Sources && !app.queue_expanded;
    let visible = app.visible_sources();
    let selected = app.source_index();

    // Index into source_counts / filter_counts that matches current filter.
    let filter_idx = filter_to_index(app);

    let mut lines: Vec<Line> = Vec::new();

    // "All" entry at index 0.
    let is_all_active = selected == 0;
    let total = app.filter_counts[filter_idx];
    lines.push(rail_row("All", total, accent(), is_all_active, focused));

    for (i, source) in visible.iter().enumerate() {
        let row_idx = i + 1;
        let is_active = selected == row_idx;
        let counts = app
            .source_counts
            .get(source)
            .copied()
            .unwrap_or([0, 0, 0, 0, 0, 0]);
        let style = source_color(*source);
        lines.push(rail_row(
            short_label(*source),
            counts[filter_idx],
            style,
            is_active,
            focused,
        ));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        " source",
        dim().add_modifier(Modifier::ITALIC),
    )));

    frame.render_widget(Paragraph::new(lines), area);
}

fn filter_to_index(app: &App) -> usize {
    use crate::cli::tui::state::filters::Filter;
    match app.filter {
        Filter::All => 0,
        Filter::Installed => 1,
        Filter::Updates => 2,
        Filter::Favorites => 3,
        Filter::SecurityUpdates => 4,
        Filter::Duplicates => 5,
    }
}

fn rail_row(
    label: &str,
    count: usize,
    color: Style,
    is_active: bool,
    focused: bool,
) -> Line<'static> {
    let marker = if is_active { "▸" } else { " " };
    let name_style = if is_active {
        if focused {
            row_cursor().add_modifier(Modifier::BOLD)
        } else {
            accent().add_modifier(Modifier::BOLD)
        }
    } else {
        color
    };
    let count_style = if count == 0 { dim() } else { muted() };
    Line::from(vec![
        Span::styled(format!("{} ", marker), accent()),
        Span::styled(format!("{:<4}", label), name_style),
        Span::styled(
            if count > 999 {
                "999+".to_string()
            } else {
                format!("{:>4}", count)
            },
            count_style,
        ),
    ])
}
