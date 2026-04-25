//! Attention summary shown at the top of the workspace.
//!
//! The workspace needs to surface urgent work without pushing the package
//! table down. Render attention as a compact "Today" strip instead of stacked
//! cards.

use crate::cli::tui::app::App;
use crate::cli::tui::format::truncate_to_width;
use crate::cli::tui::theme::{accent, dim, palette, success, text, warning};
use crate::models::{PackageStatus, UpdateCategory};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

/// Severity of an attention block, used to pick an accent color and ordering.
#[derive(Copy, Clone, Eq, PartialEq)]
pub enum Severity {
    Critical,
    Warning,
    Info,
    Ok,
}

impl Severity {
    fn accent_style(self) -> Style {
        match self {
            Severity::Critical => Style::default()
                .fg(palette::RED())
                .add_modifier(Modifier::BOLD),
            Severity::Warning => warning().add_modifier(Modifier::BOLD),
            Severity::Info => accent().add_modifier(Modifier::BOLD),
            Severity::Ok => success().add_modifier(Modifier::BOLD),
        }
    }
}

/// A single attention card. `headline` is the human summary, `cta` is a list
/// of `(hotkey, label)` pairs rendered as `[k] label` chips.
pub struct Block {
    pub severity: Severity,
    pub headline: String,
    pub detail: Option<String>,
    pub cta: Vec<(&'static str, String)>,
}

/// Collect the prioritized list of attention blocks from current app state.
/// Order: security updates → safe updates → failed tasks → (disk, cache hooks
/// are placeholders until those signals land in App).
pub fn build_blocks(app: &App) -> Vec<Block> {
    let mut blocks = Vec::new();

    let security = app.filter_counts[4];
    if security > 0 {
        let preview: Vec<String> = app
            .packages
            .iter()
            .filter(|p| {
                p.status == PackageStatus::UpdateAvailable
                    && matches!(p.update_category, Some(UpdateCategory::Security))
            })
            .take(2)
            .map(|p| p.name.clone())
            .collect();
        let detail = if preview.is_empty() {
            None
        } else {
            Some(preview.join("  ·  "))
        };
        blocks.push(Block {
            severity: Severity::Critical,
            headline: format!(
                "{} security update{} need your attention",
                security,
                if security == 1 { "" } else { "s" }
            ),
            detail,
            cta: vec![("5", "review".into()), ("w", "queue all security".into())],
        });
    }

    let updates = app.filter_counts[2].saturating_sub(security);
    if updates > 0 {
        blocks.push(Block {
            severity: Severity::Warning,
            headline: format!(
                "{} safe update{} available",
                updates,
                if updates == 1 { "" } else { "s" }
            ),
            detail: None,
            cta: vec![("3", "review".into()), ("A", "queue all safe".into())],
        });
    }

    let (_queued, running, _completed, failed, _cancelled) = app.queue_counts();
    if failed > 0 {
        blocks.push(Block {
            severity: Severity::Warning,
            headline: format!(
                "{} task{} failed last run",
                failed,
                if failed == 1 { "" } else { "s" }
            ),
            detail: None,
            cta: vec![("l", "open queue".into()), ("R", "retry safe".into())],
        });
    } else if running > 0 {
        blocks.push(Block {
            severity: Severity::Info,
            headline: format!(
                "{} task{} running",
                running,
                if running == 1 { "" } else { "s" }
            ),
            detail: None,
            cta: vec![("l", "open queue".into())],
        });
    }

    if blocks.is_empty() {
        blocks.push(Block {
            severity: Severity::Ok,
            headline: "All clear. System is up to date.".into(),
            detail: Some(format!(
                "{} packages managed across {} sources.",
                app.filter_counts[0],
                app.visible_sources().len()
            )),
            cta: vec![("/", "search".into()), (":", "commands".into())],
        });
    }

    blocks
}

/// Two rows: signal summary, then recommended action/detail.
pub fn desired_height(blocks: &[Block]) -> u16 {
    if blocks.is_empty() {
        0
    } else {
        2
    }
}

pub fn draw(frame: &mut Frame, blocks: &[Block], area: Rect) {
    if blocks.is_empty() || area.height == 0 {
        return;
    }

    let mut summary_spans = vec![
        Span::styled(" Today ", accent().add_modifier(Modifier::BOLD)),
        Span::styled("│ ", dim()),
    ];
    for (index, block) in blocks.iter().enumerate() {
        if index > 0 {
            summary_spans.push(Span::styled("  │  ", dim()));
        }
        summary_spans.push(Span::styled(
            signal_label(block),
            block.severity.accent_style(),
        ));
        summary_spans.push(Span::styled(" ", dim()));
        summary_spans.push(Span::styled(block.headline.clone(), text()));
    }

    let recommended = &blocks[0];
    let mut action_spans = vec![Span::styled(" Action ", dim()), Span::styled("│ ", dim())];
    if let Some((key, label)) = recommended.cta.first() {
        action_spans.push(Span::styled(
            format!("[{}]", key),
            accent().add_modifier(Modifier::BOLD),
        ));
        action_spans.push(Span::styled(format!(" {}", label), text()));
    }
    if let Some(detail) = recommended.detail.as_deref() {
        action_spans.push(Span::styled("  ·  ", dim()));
        action_spans.push(Span::styled(
            truncate_to_width(detail, area.width.saturating_sub(18) as usize),
            dim(),
        ));
    }

    let lines = if area.height == 1 {
        vec![Line::from(summary_spans)]
    } else {
        vec![Line::from(summary_spans), Line::from(action_spans)]
    };
    frame.render_widget(Paragraph::new(lines), area);
}

fn signal_label(block: &Block) -> &'static str {
    match block.severity {
        Severity::Critical => "SEC",
        Severity::Warning => "UPD",
        Severity::Info => "RUN",
        Severity::Ok => "OK",
    }
}
