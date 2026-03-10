use crate::cli::tui::app::{App, ChangelogState};
use crate::cli::tui::format::{
    format_package_version, package_status_label, package_status_style, truncate_to_width,
};
use crate::cli::tui::state::filters::DetailsTab;
use crate::cli::tui::theme::{accent, dim, key_hint, loading, muted, source_color, text, warning};
use crate::cli::tui::ui::{panel_block, update_priority_label, wrap_text};
use crate::models::{Package, PackageStatus, UpdateCategory};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Paragraph, Tabs, Wrap},
    Frame,
};
use unicode_width::UnicodeWidthStr;

const INFO_TAB_LABEL: &str = "Info";
const DEPS_TAB_LABEL: &str = "Deps";
const NOTES_TAB_LABEL: &str = "Notes";
const TAB_DIVIDER: &str = " · ";

pub fn draw_details_panel(frame: &mut Frame, app: &App, area: Rect) {
    let block = panel_block(" Details ".to_string(), false, app.compact);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if app.is_catalog_busy() && app.current_package().is_none() {
        let paragraph = Paragraph::new("Loading details...")
            .style(loading())
            .alignment(ratatui::layout::Alignment::Center);

        let vertical_padding = inner.height.saturating_sub(1) / 2;
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(vertical_padding), Constraint::Min(1)])
            .split(inner);

        frame.render_widget(paragraph, chunks[1]);
        return;
    }

    let metadata_height = details_metadata_height(inner.height);
    let footer_height = details_footer_height(inner.height);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(metadata_height),
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(footer_height),
        ])
        .split(inner);

    let Some(package) = app.current_package() else {
        let paragraph = Paragraph::new("Select a package for details")
            .style(dim())
            .alignment(ratatui::layout::Alignment::Center);

        let vertical_padding = inner.height.saturating_sub(1) / 2;
        let empty_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(vertical_padding), Constraint::Min(1)])
            .split(inner);

        frame.render_widget(paragraph, empty_chunks[1]);
        return;
    };

    let detail_width = chunks[2].width.max(1) as usize;
    let meta_lines = build_metadata_lines(app, package, metadata_height, detail_width);
    frame.render_widget(
        Paragraph::new(meta_lines).wrap(Wrap { trim: true }),
        chunks[0],
    );

    let tabs = Tabs::new(details_tab_titles())
        .select(details_tab_index(app.active_details_tab))
        .divider(Span::styled(TAB_DIVIDER, dim()))
        .style(dim())
        .highlight_style(accent());
    frame.render_widget(tabs, chunks[1]);

    match app.active_details_tab {
        DetailsTab::Info => {
            let paragraph =
                Paragraph::new(build_info_lines(package, detail_width)).wrap(Wrap { trim: true });
            frame.render_widget(paragraph, chunks[2]);
        }
        DetailsTab::Dependencies => {
            let paragraph =
                Paragraph::new(build_dependency_lines(package, chunks[2], detail_width))
                    .wrap(Wrap { trim: true });
            frame.render_widget(paragraph, chunks[2]);
        }
        DetailsTab::Changelog => {
            let paragraph = Paragraph::new(build_changelog_lines(app, package, detail_width))
                .wrap(Wrap { trim: true });
            frame.render_widget(paragraph, chunks[2]);
        }
    }

    let footer_lines = build_footer_lines(app, package, footer_height);
    frame.render_widget(
        Paragraph::new(footer_lines).wrap(Wrap { trim: true }),
        chunks[3],
    );
}

pub fn draw_compact_details_summary(frame: &mut Frame, app: &App, area: Rect) {
    let Some(package) = app.current_package() else {
        frame.render_widget(Paragraph::new("No package selected").style(dim()), area);
        return;
    };

    let first = format!(
        "{} {} ({})",
        package.name,
        format_package_version(package),
        package.source
    );
    let second = truncate_to_width(
        &format!("Action: {}", package_action_summary(package)),
        area.width as usize,
    );

    let lines = vec![
        Line::from(Span::styled(first, text())),
        Line::from(Span::styled(second, muted())),
    ];
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

fn details_footer_height(inner_height: u16) -> u16 {
    if inner_height >= 11 {
        2
    } else {
        1
    }
}

fn details_tab_titles() -> Vec<Line<'static>> {
    vec![
        details_tab_title(INFO_TAB_LABEL),
        details_tab_title(DEPS_TAB_LABEL),
        details_tab_title(NOTES_TAB_LABEL),
    ]
}

fn details_tab_title(label: &'static str) -> Line<'static> {
    Line::from(label)
}

fn details_tab_index(active: DetailsTab) -> usize {
    match active {
        DetailsTab::Info => 0,
        DetailsTab::Dependencies => 1,
        DetailsTab::Changelog => 2,
    }
}

fn build_metadata_lines(
    app: &App,
    package: &Package,
    metadata_height: u16,
    detail_width: usize,
) -> Vec<Line<'static>> {
    let mut lines = vec![Line::from(vec![
        Span::styled(
            truncate_to_width(&package.name, detail_width.saturating_sub(18)),
            accent(),
        ),
        Span::raw("  "),
        Span::styled(
            format!("[{}]", package.source),
            source_color(package.source),
        ),
        Span::raw("  "),
        Span::styled(
            package_status_label(package.status),
            package_status_style(package.status),
        ),
    ])];

    let version_copy = if let Some(available) = package.available_version.as_deref() {
        if available != package.version {
            format!("Version {} -> {}", package.version, available)
        } else {
            format!("Version {}", package.version)
        }
    } else {
        format!("Version {}", package.version)
    };
    lines.push(Line::from(Span::styled(
        truncate_to_width(&version_copy, detail_width),
        if matches!(
            package.status,
            PackageStatus::UpdateAvailable | PackageStatus::Updating
        ) {
            warning()
        } else {
            text()
        },
    )));

    if metadata_height >= 3 {
        lines.push(Line::from(vec![
            Span::styled("Action: ", dim()),
            Span::styled(
                truncate_to_width(
                    package_action_summary(package),
                    detail_width.saturating_sub(8),
                ),
                muted(),
            ),
        ]));
    }

    if metadata_height >= 4 {
        let mut extra = Vec::new();
        if let Some(priority) = update_priority_label(package) {
            extra.push(Span::styled(format!("Priority {}", priority), warning()));
        }
        if let Some(source_note) = app.package_source_note(package) {
            if !extra.is_empty() {
                extra.push(Span::raw("  "));
            }
            extra.push(Span::styled(
                truncate_to_width(&source_note, detail_width.saturating_sub(2)),
                muted(),
            ));
        }
        if App::changelog_supported_for_source(package.source) {
            if !extra.is_empty() {
                extra.push(Span::raw("  "));
            }
            extra.push(Span::styled("[c] release notes", key_hint()));
        }
        if extra.is_empty() {
            extra.push(Span::styled("No additional package metadata yet.", dim()));
        }
        lines.push(Line::from(extra));
    }

    lines
}

fn build_info_lines(package: &Package, detail_width: usize) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let description = package.description.trim();

    if description.is_empty() {
        lines.push(Line::from(Span::styled(
            "No description metadata is available for this package yet.",
            muted(),
        )));
    } else {
        for line in wrap_text(description, detail_width) {
            lines.push(Line::from(Span::styled(line, muted())));
        }
    }

    lines.push(Line::from(""));
    let version_fact = if let Some(available) = package.available_version.as_deref() {
        if available != package.version {
            format!("{} -> {}", package.version, available)
        } else {
            package.version.clone()
        }
    } else {
        package.version.clone()
    };
    lines.push(info_fact_line(
        "Version",
        version_fact,
        if matches!(
            package.status,
            PackageStatus::UpdateAvailable | PackageStatus::Updating
        ) {
            warning()
        } else {
            text()
        },
        detail_width,
    ));
    if matches!(
        package.status,
        PackageStatus::UpdateAvailable | PackageStatus::Updating
    ) {
        lines.push(info_fact_line(
            "Update",
            update_category_label(package.detect_update_category()).to_string(),
            warning(),
            detail_width,
        ));
    }
    if let Some(license) = package.license.as_deref() {
        lines.push(info_fact_line(
            "License",
            license.to_string(),
            muted(),
            detail_width,
        ));
    }
    if let Some(maintainer) = package.maintainer.as_deref() {
        lines.push(info_fact_line(
            "Maintainer",
            maintainer.to_string(),
            muted(),
            detail_width,
        ));
    }
    if let Some(homepage) = package.homepage.as_deref() {
        lines.push(info_fact_line(
            "Homepage",
            homepage.to_string(),
            muted(),
            detail_width,
        ));
    }
    lines.push(info_fact_line(
        "Deps",
        if package.dependencies.is_empty() {
            "No dependency metadata yet".to_string()
        } else {
            format!("{} known dependencies", package.dependencies.len())
        },
        muted(),
        detail_width,
    ));
    lines.push(info_fact_line(
        "Notes",
        if App::changelog_supported_for_source(package.source) {
            "Press [c] to load release notes".to_string()
        } else {
            "Release notes are not supported yet".to_string()
        },
        if App::changelog_supported_for_source(package.source) {
            key_hint()
        } else {
            dim()
        },
        detail_width,
    ));
    lines.push(info_fact_line(
        "Managed by",
        package.source.to_string(),
        source_color(package.source),
        detail_width,
    ));

    lines
}

fn build_dependency_lines(
    package: &Package,
    area: Rect,
    detail_width: usize,
) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    if package.dependencies.is_empty() {
        lines.push(Line::from(Span::styled(
            "No dependency metadata is available for this package yet.",
            muted(),
        )));
        return lines;
    }

    let visible_dependencies = package
        .dependencies
        .iter()
        .take(area.height.saturating_sub(1) as usize);
    for dependency in visible_dependencies {
        lines.push(Line::from(vec![
            Span::styled("• ", accent()),
            Span::styled(
                truncate_to_width(dependency, detail_width.saturating_sub(4)),
                muted(),
            ),
        ]));
    }

    let remaining = package.dependencies.len().saturating_sub(lines.len());
    if remaining > 0 {
        lines.push(Line::from(Span::styled(
            format!("…and {} more", remaining),
            dim(),
        )));
    }

    lines
}

fn build_changelog_lines(app: &App, package: &Package, detail_width: usize) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    match app.changelog_state_for_current_package() {
        Some(ChangelogState::Loading) => {
            lines.push(Line::from(Span::styled(
                "Fetching release notes for the current selection…",
                loading(),
            )));
        }
        Some(ChangelogState::Ready { summary, content }) => {
            lines.push(Line::from(vec![
                Span::styled("Summary: ", dim()),
                Span::styled(summary.summary_text(), muted()),
            ]));
            for highlight in summary.highlights.iter().take(3) {
                lines.push(Line::from(Span::styled(
                    truncate_to_width(highlight, detail_width.saturating_sub(2)),
                    muted(),
                )));
            }

            let preview_lines: Vec<&str> = content
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .take(3)
                .collect();
            if !preview_lines.is_empty() {
                lines.push(Line::from(""));
                for preview in preview_lines {
                    for wrapped in wrap_text(preview, detail_width.saturating_sub(2)) {
                        lines.push(Line::from(Span::styled(wrapped, muted())));
                    }
                }
            }
        }
        Some(ChangelogState::Empty) => {
            lines.push(Line::from(Span::styled(
                "No release notes were returned for this package.",
                muted(),
            )));
        }
        Some(ChangelogState::Error(error)) => {
            lines.push(Line::from(Span::styled(
                truncate_to_width(
                    &format!("Could not load release notes: {}", error),
                    detail_width.saturating_sub(2),
                ),
                warning(),
            )));
        }
        None if App::changelog_supported_for_source(package.source) => {
            lines.push(Line::from(Span::styled(
                "Press [c] to load release notes for this package.",
                muted(),
            )));
        }
        None => {
            lines.push(Line::from(Span::styled(
                "Release notes are not available for this source yet.",
                muted(),
            )));
        }
    }

    lines
}

fn build_footer_lines(app: &App, package: &Package, footer_height: u16) -> Vec<Line<'static>> {
    let (now, next, attention, done) = app.queue_lane_counts();
    let retryable = app.retryable_failed_task_count();
    let mut lines = vec![Line::from(vec![
        Span::styled("Queue: ", dim()),
        Span::styled(
            format!(
                "{} now · {} queued · {} attention · {} done",
                now, next, attention, done
            ),
            muted(),
        ),
        if attention > 0 {
            Span::styled("  [l] review", key_hint())
        } else {
            Span::styled("", muted())
        },
    ])];

    if footer_height >= 2 {
        let secondary = if attention > 0 {
            if retryable > 0 {
                format!(
                    "Use [R] retry selected or [A] retry safe ({}) to recover failed tasks.",
                    retryable
                )
            } else {
                "Use [R] retry selected or [M] apply filtered fixes to recover failed tasks."
                    .to_string()
            }
        } else {
            format!(
                "Selected package action: {}",
                package_action_summary(package)
            )
        };
        lines.push(Line::from(Span::styled(secondary, dim())));
    }

    lines
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

fn info_fact_line(
    label: &str,
    value: String,
    value_style: Style,
    detail_width: usize,
) -> Line<'static> {
    let label_text = format!("{}: ", label);
    let value_width = detail_width.saturating_sub(label_text.len());
    Line::from(vec![
        Span::styled(label_text, dim()),
        Span::styled(truncate_to_width(&value, value_width), value_style),
    ])
}

fn update_category_label(category: UpdateCategory) -> &'static str {
    match category {
        UpdateCategory::Security => "Security update",
        UpdateCategory::Bugfix => "Bugfix release",
        UpdateCategory::Feature => "Feature release",
        UpdateCategory::Minor => "Routine update",
    }
}
