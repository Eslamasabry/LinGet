use crate::cli::tui::app::App;
use crate::cli::tui::format::{format_package_version, package_status_short, truncate_to_width};
use crate::cli::tui::state::filters::DetailsTab;
use crate::cli::tui::theme::{dim, source_color, text};
use crate::models::{Package, PackageStatus};
use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};
use unicode_width::UnicodeWidthStr;

const INFO_TAB_LABEL: &str = "Info";
const DEPS_TAB_LABEL: &str = "Deps";
const NOTES_TAB_LABEL: &str = "Notes";
const TAB_DIVIDER: &str = " · ";

pub fn draw_compact_details_summary(frame: &mut Frame, app: &App, area: Rect) {
    let Some(package) = app.current_package() else {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled("  No package selected", dim()))),
            area,
        );
        return;
    };

    let (status_short, status_style) = package_status_short(package.status);
    let first = vec![
        Span::styled(status_short, status_style),
        Span::styled(
            truncate_to_width(&package.name, area.width.saturating_sub(20) as usize),
            text(),
        ),
        Span::styled(" ", text()),
        Span::styled(
            truncate_to_width(&format_package_version(package), 13),
            dim(),
        ),
        Span::styled(" ", text()),
        Span::styled(
            truncate_to_width(&package.source.to_string(), 8),
            source_color(package.source),
        ),
    ];
    let second = truncate_to_width(
        &format!("  {}", package_action_summary(package)),
        area.width as usize,
    );

    let lines = vec![Line::from(first), Line::from(Span::styled(second, dim()))];
    frame.render_widget(Paragraph::new(lines), area);
}

pub fn details_tab_hit_test(details_rect: Rect, col: u16, row: u16) -> Option<DetailsTab> {
    if details_rect.width <= 2 || details_rect.height <= 2 {
        return None;
    }

    let inner_height = details_rect.height.saturating_sub(2);
    let tabs_y = details_rect.y + 1 + details_metadata_height(inner_height);
    if row != tabs_y {
        return None;
    }

    let inner_x = details_rect.x + 1;
    if col < inner_x {
        return None;
    }

    let rel_x = col.saturating_sub(inner_x);
    let tabs = [
        (DetailsTab::Info, INFO_TAB_LABEL),
        (DetailsTab::Dependencies, DEPS_TAB_LABEL),
        (DetailsTab::Changelog, NOTES_TAB_LABEL),
    ];
    let divider_width = UnicodeWidthStr::width(TAB_DIVIDER) as u16;
    let mut cursor = 0u16;

    for (index, (tab, label)) in tabs.iter().enumerate() {
        let width = UnicodeWidthStr::width(*label) as u16;
        if rel_x >= cursor && rel_x < cursor.saturating_add(width) {
            return Some(*tab);
        }
        cursor = cursor.saturating_add(width);
        if index < tabs.len() - 1 {
            if rel_x == cursor {
                return Some(*tab);
            }
            cursor = cursor.saturating_add(divider_width);
        }
    }

    None
}

fn details_metadata_height(inner_height: u16) -> u16 {
    if inner_height >= 14 {
        4
    } else if inner_height >= 10 {
        3
    } else {
        2
    }
}

fn package_action_summary(package: &Package) -> &'static str {
    match package.status {
        PackageStatus::UpdateAvailable => "[Enter]/[u] queue update",
        PackageStatus::Installed => "[Enter]/[d] remove package",
        PackageStatus::NotInstalled => "[Enter] install package",
        PackageStatus::Updating => "Update already in progress",
        PackageStatus::Installing => "Install already in progress",
        PackageStatus::Removing => "Removal already in progress",
    }
}
