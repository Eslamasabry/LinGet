//! Attention blocks — prioritized, actionable cards shown at the top of the
//! workspace. Each block is a small hero ("accent bar · headline · CTA line")
//! that surfaces the most important thing the user can do right now.

use crate::cli::tui::app::App;
use crate::cli::tui::theme::{accent, dim, muted, palette, success, text, warning};
use crate::models::{PackageStatus, UpdateCategory};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
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

/// How tall the attention area wants to be given the provided blocks and
/// available width. Each block is 2 rows (headline + CTA) plus an optional
/// detail row and one blank row between blocks. Caller clamps.
pub fn desired_height(blocks: &[Block]) -> u16 {
    let mut h: u16 = 0;
    for (i, b) in blocks.iter().enumerate() {
        if i > 0 {
            h += 1;
        }
        h += 1; // headline
        if b.detail.is_some() {
            h += 1;
        }
        h += 1; // cta
    }
    h
}

pub fn draw(frame: &mut Frame, blocks: &[Block], area: Rect) {
    if blocks.is_empty() || area.height == 0 {
        return;
    }
    let mut lines: Vec<Line> = Vec::new();
    for (i, block) in blocks.iter().enumerate() {
        if i > 0 {
            lines.push(Line::from(""));
        }
        lines.push(Line::from(vec![
            Span::styled(" ▌ ", block.severity.accent_style()),
            Span::styled(block.headline.clone(), text().add_modifier(Modifier::BOLD)),
        ]));
        if let Some(detail) = &block.detail {
            lines.push(Line::from(vec![
                Span::styled("   ", dim()),
                Span::styled(detail.clone(), muted()),
            ]));
        }
        let mut cta_spans: Vec<Span> = vec![Span::styled("   › ", dim())];
        for (i, (key, label)) in block.cta.iter().enumerate() {
            if i > 0 {
                cta_spans.push(Span::styled("   ", dim()));
            }
            cta_spans.push(Span::styled(
                format!("[{}]", key),
                accent().add_modifier(Modifier::BOLD),
            ));
            cta_spans.push(Span::styled(format!(" {}", label), text()));
        }
        lines.push(Line::from(cta_spans));
    }
    let constraints = [Constraint::Min(1)];
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);
    frame.render_widget(Paragraph::new(lines), layout[0]);
}
