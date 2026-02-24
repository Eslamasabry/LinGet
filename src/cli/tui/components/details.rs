use crate::cli::tui::app::App;
use crate::cli::tui::state::filters::DetailsTab;
use crate::cli::tui::state::queue::QueueJourneyLane;
use crate::cli::tui::theme::{
    accent, dim, footer_label, key_hint, loading, muted, primary_action_button, source_color, text,
    warning,
};
use crate::cli::tui::ui::{
    format_package_version, panel_block, truncate_to_width, update_priority_label, wrap_text,
};
use crate::models::PackageStatus;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Paragraph, Tabs, Wrap},
    Frame,
};

pub fn draw_details_panel(frame: &mut Frame, app: &App, area: Rect) {
    let block = panel_block(" Details ".to_string(), false, app.compact);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if app.loading && app.current_package().is_none() {
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

    let (now, next, attention, done) = app.queue_lane_counts();
    let retryable = app.retryable_failed_task_count();
    let detail_width = inner.width as usize;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6), // Metadata
            Constraint::Length(2), // Tabs
            Constraint::Min(1),    // Tab Content
            Constraint::Length(5), // NBA / Queue
        ])
        .split(inner);

    // 1. Metadata Section
    if let Some(package) = app.current_package() {
        let mut meta_lines = vec![
            Line::from(vec![
                Span::styled("Name: ", dim()),
                Span::styled(package.name.clone(), accent()),
            ]),
            Line::from(vec![
                Span::styled("Version: ", dim()),
                Span::styled(format_package_version(package), text()),
            ]),
            Line::from(vec![
                Span::styled("Source: ", dim()),
                Span::styled(package.source.to_string(), source_color(package.source)),
            ]),
            Line::from(vec![
                Span::styled("Changelog: ", dim()),
                if App::changelog_supported_for_source(package.source) {
                    Span::styled("[c] view release notes", key_hint())
                } else {
                    Span::styled("not available for this source yet", dim())
                },
            ]),
        ];

        if package.status == PackageStatus::UpdateAvailable
            || package.status == PackageStatus::Updating
        {
            if let Some(priority) = update_priority_label(package) {
                meta_lines.push(Line::from(vec![
                    Span::styled("Priority: ", dim()),
                    Span::styled(priority, warning()),
                ]));
            }
        }

        if matches!(
            package.status,
            PackageStatus::Installing | PackageStatus::Removing | PackageStatus::Updating
        ) {
            meta_lines.push(Line::from(vec![
                Span::styled("Status: ", dim()),
                Span::styled("Operation in progress...", loading()),
            ]));
        }
        frame.render_widget(Paragraph::new(meta_lines), chunks[0]);

        // 2. Tabs Section
        let tab_titles = vec![
            Line::from(Span::styled(" Alt+1: Info ", if app.active_details_tab == DetailsTab::Info { text() } else { dim() })),
            Line::from(Span::styled(" Alt+2: Deps ", if app.active_details_tab == DetailsTab::Dependencies { text() } else { dim() })),
            Line::from(Span::styled(" Alt+3: Log  ", if app.active_details_tab == DetailsTab::Changelog { text() } else { dim() })),
        ];
        let tabs = Tabs::new(tab_titles)
            .divider(Span::raw("|"))
            .style(dim());
        frame.render_widget(tabs, chunks[1]);

        // 3. Tab Content Section
        match app.active_details_tab {
            DetailsTab::Info => {
                let mut desc_lines = vec![Line::from(Span::styled("Description:", dim()))];
                for line in wrap_text(&package.description, detail_width) {
                    desc_lines.push(Line::from(Span::styled(line, muted())));
                }
                frame.render_widget(
                    Paragraph::new(desc_lines).wrap(Wrap { trim: true }),
                    chunks[2],
                );
            }
            DetailsTab::Dependencies => {
                let paragraph = Paragraph::new(vec![
                    Line::from(Span::styled("Dependencies:", dim())),
                    Line::from(Span::styled("Feature coming soon...", muted())),
                ]).wrap(Wrap { trim: true });
                frame.render_widget(paragraph, chunks[2]);
            }
            DetailsTab::Changelog => {
                let paragraph = Paragraph::new(vec![
                    Line::from(Span::styled("Changelog:", dim())),
                    Line::from(Span::styled("Press [c] to view full release notes (if supported)", muted())),
                ]).wrap(Wrap { trim: true });
                frame.render_widget(paragraph, chunks[2]);
            }
        }
    } else {
        let paragraph = Paragraph::new("Select a package for details")
            .style(dim())
            .alignment(ratatui::layout::Alignment::Center);

        let vertical_padding = chunks[0]
            .height
            .saturating_add(chunks[1].height)
            .saturating_sub(1)
            / 2;
        let empty_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(vertical_padding), Constraint::Min(1)])
            .split(inner.union(chunks[1])); // Use the upper area for centering

        frame.render_widget(paragraph, empty_chunks[1]);
    }

    // 4. NBA / Queue Section
    let nba_text = format!(" {} ", app.recommended_action_label());
    let mut nba_lines = vec![
        Line::from(vec![
            Span::styled("Next Best Action: ", dim()),
            Span::styled(nba_text, primary_action_button()),
            Span::styled("  [w] to execute", key_hint()),
        ]),
        Line::from(Span::styled(
            truncate_to_width(&app.recommended_action_detail(), detail_width),
            muted(),
        )),
        Line::from(vec![
            Span::styled("Queue: ", dim()),
            Span::styled(
                format!(
                    "{} {} · {} {} · {} {} · {} {}",
                    QueueJourneyLane::Now.label(),
                    now,
                    QueueJourneyLane::Next.label(),
                    next,
                    QueueJourneyLane::NeedsAttention.label(),
                    attention,
                    QueueJourneyLane::Done.label(),
                    done
                ),
                muted(),
            ),
        ]),
    ];
    if attention > 0 {
        nba_lines.push(Line::from(vec![
            Span::styled("Failed tasks: ", dim()),
            Span::styled("R retry  M fix filtered", footer_label()),
            if retryable > 0 {
                Span::styled(format!("  A retry safe ({})", retryable), warning())
            } else {
                Span::styled("", footer_label())
            },
        ]));
    }
    frame.render_widget(Paragraph::new(nba_lines), chunks[3]);
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
        &format!("Next [w]: {}", app.recommended_action_label()),
        area.width as usize,
    );

    let lines = vec![
        Line::from(Span::styled(first, text())),
        Line::from(Span::styled(second, muted())),
    ];
    frame.render_widget(Paragraph::new(lines), area);
}
