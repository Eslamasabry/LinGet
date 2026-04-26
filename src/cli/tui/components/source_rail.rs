//! Bordered source list for the package workspace.

use crate::cli::tui::app::App;
use crate::cli::tui::state::filters::{Filter, Focus};
use crate::cli::tui::theme::{
    accent, border_focused, border_unfocused, dim, muted, row_cursor, source_color, text, ROUNDED,
};
use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub const RAIL_WIDTH: u16 = 31;

pub fn draw(frame: &mut Frame, app: &App, area: Rect) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    let focused = app.focus == Focus::Sources && !app.queue_expanded;
    let title = format!(
        " Sources ({}) ",
        app.filter_counts[filter_to_index(app.filter)]
    );
    let block = Block::default()
        .borders(Borders::ALL)
        .border_set(ROUNDED)
        .border_style(if focused {
            border_focused()
        } else {
            border_unfocused()
        })
        .title(title)
        .title_style(accent());
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let visible_rows = inner.height as usize;
    let source_count = app.visible_sources().len() + 1;
    let start =
        crate::cli::tui::ui::window_start(source_count, visible_rows.max(1), app.source_index());
    let mut rows = Vec::new();

    for index in start..(start + visible_rows).min(source_count) {
        if index == 0 {
            rows.push(source_row(
                "All",
                app.filter_counts[filter_to_index(app.filter)],
                accent(),
                app.source_index() == index,
                inner.width,
            ));
        } else if let Some(source) = app.visible_sources().get(index - 1).copied() {
            let counts = app
                .source_counts
                .get(&source)
                .copied()
                .unwrap_or([0, 0, 0, 0, 0, 0]);
            rows.push(source_row(
                source_label(source),
                counts[filter_to_index(app.filter)],
                source_color(source),
                app.source_index() == index,
                inner.width,
            ));
        }
    }
    frame.render_widget(Paragraph::new(rows), inner);
}

fn source_row(
    label: &str,
    count: usize,
    style: ratatui::style::Style,
    active: bool,
    width: u16,
) -> Line<'static> {
    let available = width.saturating_sub(8) as usize;
    let display_label = crate::cli::tui::format::truncate_to_width(label, available);
    let count_label = if count > 9999 {
        "9999+".to_string()
    } else {
        count.to_string()
    };
    let row_style = if active { row_cursor() } else { text() };
    let marker = if active { "> " } else { "  " };
    let mut line = Line::from(vec![
        Span::styled(marker, if active { row_cursor() } else { dim() }),
        Span::styled(display_label, if active { row_cursor() } else { style }),
    ]);
    let used = marker.len() + label.len();
    let padding = width
        .saturating_sub(1)
        .saturating_sub(used as u16)
        .saturating_sub(count_label.len() as u16) as usize;
    line.spans
        .push(Span::styled(" ".repeat(padding), row_style));
    line.spans.push(Span::styled(
        count_label,
        if active { row_cursor() } else { muted() },
    ));
    line.style(row_style)
}

fn source_label(source: crate::models::PackageSource) -> &'static str {
    match source {
        crate::models::PackageSource::Apt => "APT",
        crate::models::PackageSource::Dnf => "DNF",
        crate::models::PackageSource::Pacman => "Pacman",
        crate::models::PackageSource::Zypper => "Zypper",
        crate::models::PackageSource::Flatpak => "Flatpak",
        crate::models::PackageSource::Snap => "Snap",
        crate::models::PackageSource::Npm => "npm",
        crate::models::PackageSource::Pip => "pip",
        crate::models::PackageSource::Pipx => "pipx",
        crate::models::PackageSource::Cargo => "Cargo",
        crate::models::PackageSource::Brew => "Brew",
        crate::models::PackageSource::Aur => "AUR",
        crate::models::PackageSource::Conda => "conda",
        crate::models::PackageSource::Mamba => "mamba",
        crate::models::PackageSource::Dart => "Dart",
        crate::models::PackageSource::Deb => "DEB",
        crate::models::PackageSource::AppImage => "AppImage",
        crate::models::PackageSource::Winget => "WinGet",
        crate::models::PackageSource::Chocolatey => "Chocolatey",
        crate::models::PackageSource::Scoop => "Scoop",
    }
}

fn filter_to_index(filter: Filter) -> usize {
    match filter {
        Filter::All => 0,
        Filter::Installed => 1,
        Filter::Updates => 2,
        Filter::Favorites => 3,
        Filter::SecurityUpdates => 4,
        Filter::Duplicates => 5,
    }
}
