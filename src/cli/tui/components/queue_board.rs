//! Queue board — a three-lane kanban view of the task queue.
//!
//! Replaces the legacy card-in-card-in-card `draw_expanded_queue` with a
//! clean Running / Attention / Done layout. One line per task; actionable
//! chips for failures; aggregate "more" rows at the foot of each lane.
//!
//! Layout:
//!
//! ```text
//! ┌ Queue · 3 running · 0 pending · 16 failed · 181 done ─────────────┐
//! │ ◉ RUNNING            ⚠ NEEDS ATTENTION       ✓ DONE               │
//! │ ─────────────────── ───────────────────────  ────────────────────│
//! │ ⠏ tailscale  32%    ✗ docker-ce      lock   ✓ @google/gemini 56s │
//! │   APT update 12s       APT install                               │
//! │                        ▸ [R] retry                               │
//! │ ⠏ firefox    67%    ✗ cosmic-applets disk                        │
//! │   APT update  8s       ▸ [M] fix                                 │
//! │                                                                   │
//! │ (0 pending)          (14 more — [A] retry all)   (177 more)      │
//! ├───────────────────────────────────────────────────────────────────┤
//! │ tailscale · APT update · runtime 12s · pid 98234                  │
//! │ last: "Setting up tailscale (1.58.0-1) …"                         │
//! └───────────────────────────────────────────────────────────────────┘
//! ```

use crate::cli::tui::app::App;
use crate::cli::tui::state::queue::QueueJourneyLane;
use crate::cli::tui::theme::{
    accent, dim, error, loading, muted, source_color, success, text, warning,
};
use crate::models::history::{TaskQueueAction, TaskQueueEntry, TaskQueueStatus};
use chrono::Local;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

/// Entry point. Given the inner area of the queue container (no outer border
/// drawn here), renders the three-lane board plus a details strip.
pub fn draw(frame: &mut Frame, app: &App, area: Rect) {
    if area.height < 8 {
        // Too small for the full board; fall back to a one-line summary.
        draw_summary_only(frame, app, area);
        return;
    }

    // Split: legend (1) + recommended action (2) + lanes (min) + divider (1) + details strip (2).
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(2),
            Constraint::Min(4),
            Constraint::Length(1),
            Constraint::Length(2),
        ])
        .split(area);

    draw_legend(frame, app, rows[0]);
    draw_recommendation_strip(frame, app, rows[1]);
    draw_lanes(frame, app, rows[2]);
    draw_divider(frame, rows[3]);
    draw_details_strip(frame, app, rows[4]);
}

fn draw_summary_only(frame: &mut Frame, app: &App, area: Rect) {
    let (now, next, attention, done) = app.queue_lane_counts();
    let line = Line::from(vec![
        Span::styled(" ◉ ", loading()),
        Span::styled(format!("{} running", now), text()),
        Span::styled(" · ", dim()),
        Span::styled(format!("{} pending", next), muted()),
        Span::styled(" · ", dim()),
        Span::styled(format!("{} failed", attention), error()),
        Span::styled(" · ", dim()),
        Span::styled(format!("{} done", done), success()),
    ]);
    frame.render_widget(Paragraph::new(line), area);
}

fn draw_legend(frame: &mut Frame, app: &App, area: Rect) {
    let (now, next, attention, done) = app.queue_lane_counts();
    let spans = vec![
        Span::styled(" Queue ", accent().add_modifier(Modifier::BOLD)),
        Span::styled(" · ", dim()),
        Span::styled("◉ ", loading()),
        Span::styled(format!("{} running", now), text()),
        Span::styled("  · ", dim()),
        Span::styled(format!("{} pending", next), muted()),
        Span::styled("  · ", dim()),
        Span::styled("⚠ ", warning()),
        Span::styled(
            format!("{} failed", attention),
            if attention > 0 { warning() } else { muted() },
        ),
        Span::styled("  · ", dim()),
        Span::styled("✓ ", success()),
        Span::styled(format!("{} done", done), muted()),
    ];
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn draw_recommendation_strip(frame: &mut Frame, app: &App, area: Rect) {
    if area.height == 0 {
        return;
    }

    let actionability = app.queue_clinic_actionability();
    let recommendation = queue_recommendation(app, actionability);
    let lines = vec![
        Line::from(vec![
            Span::styled(" Recommended ", success().add_modifier(Modifier::BOLD)),
            Span::styled(
                truncate(
                    &recommendation.title,
                    area.width.saturating_sub(14) as usize,
                ),
                text().add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled(" ", dim()),
            Span::styled(
                truncate(
                    &recommendation.prompt,
                    area.width.saturating_sub(1) as usize,
                ),
                recommendation.style,
            ),
        ]),
    ];
    frame.render_widget(Paragraph::new(lines), area);
}

struct QueueRecommendation {
    title: String,
    prompt: String,
    style: Style,
}

fn queue_recommendation(
    app: &App,
    actionability: crate::cli::tui::state::queue::QueueClinicActionability,
) -> QueueRecommendation {
    let scope = app.queue_failure_filter_label();
    if actionability.safe_retry_count > 0 {
        return QueueRecommendation {
            title: format!(
                "Retry {} safe {} failure{}",
                actionability.safe_retry_count,
                scope,
                if actionability.safe_retry_count == 1 {
                    ""
                } else {
                    "s"
                }
            ),
            prompt: "Press A to queue the safe retry bundle now.".to_string(),
            style: success(),
        };
    }

    let fix_count = actionability.remediation_actionable_count();
    if fix_count > 0 {
        return QueueRecommendation {
            title: format!(
                "Fix {} {} failure{}",
                fix_count,
                scope,
                if fix_count == 1 { "" } else { "s" }
            ),
            prompt: "Press M to apply the filtered fixes or guidance.".to_string(),
            style: warning(),
        };
    }

    if actionability.failed_in_scope > 0 {
        return QueueRecommendation {
            title: format!(
                "Review {} {} failure{}",
                actionability.failed_in_scope,
                scope,
                if actionability.failed_in_scope == 1 {
                    ""
                } else {
                    "s"
                }
            ),
            prompt: "Move to a failed task, then press R to retry or M to inspect fixes."
                .to_string(),
            style: warning(),
        };
    }

    let (running, pending, _attention, done) = app.queue_lane_counts();
    if running > 0 || pending > 0 {
        QueueRecommendation {
            title: format!("Monitor {} running and {} queued", running, pending),
            prompt: "Keep the queue open for progress; failed tasks will surface here.".to_string(),
            style: muted(),
        }
    } else {
        QueueRecommendation {
            title: format!(
                "Queue clear: {} task{} resolved",
                done,
                if done == 1 { "" } else { "s" }
            ),
            prompt: "Press l or Esc to return to packages.".to_string(),
            style: success(),
        }
    }
}

fn draw_divider(frame: &mut Frame, area: Rect) {
    if area.width == 0 {
        return;
    }
    let line = "─".repeat(area.width as usize);
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(line, dim()))),
        area,
    );
}

fn draw_lanes(frame: &mut Frame, app: &App, area: Rect) {
    // Three columns, with 1-cell gutters between.
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Ratio(1, 3),
            Constraint::Length(1),
            Constraint::Ratio(1, 3),
            Constraint::Length(1),
            Constraint::Ratio(1, 3),
        ])
        .split(area);

    let (running, pending, failed, done) = partition_tasks(app);

    // LANE: Running (+ pending folded in after a subtle break).
    draw_lane(
        frame,
        app,
        cols[0],
        LaneSpec {
            title: "RUNNING",
            title_glyph: "◉",
            title_style: loading().add_modifier(Modifier::BOLD),
            primary: &running,
            secondary: Some((pending.as_slice(), "pending")),
            more_note: None,
            bulk_action: None,
        },
    );

    // LANE: Needs attention.
    let failed_visible: Vec<&TaskQueueEntry> = failed.iter().take(6).copied().collect();
    let failed_overflow = failed.len().saturating_sub(failed_visible.len());
    let bulk = if failed.len() > 1 {
        Some(("A", format!("retry all ({})", failed.len())))
    } else {
        None
    };
    draw_lane(
        frame,
        app,
        cols[2],
        LaneSpec {
            title: "NEEDS ATTENTION",
            title_glyph: "⚠",
            title_style: warning().add_modifier(Modifier::BOLD),
            primary: &failed_visible,
            secondary: None,
            more_note: if failed_overflow > 0 {
                Some(format!("{} more failed", failed_overflow))
            } else {
                None
            },
            bulk_action: bulk,
        },
    );

    // LANE: Done.
    let done_visible: Vec<&TaskQueueEntry> = done.iter().take(10).copied().collect();
    let done_overflow = done.len().saturating_sub(done_visible.len());
    draw_lane(
        frame,
        app,
        cols[4],
        LaneSpec {
            title: "DONE",
            title_glyph: "✓",
            title_style: success().add_modifier(Modifier::BOLD),
            primary: &done_visible,
            secondary: None,
            more_note: if done_overflow > 0 {
                Some(format!("{} more done", done_overflow))
            } else {
                None
            },
            bulk_action: if !done.is_empty() {
                Some(("c", "clear".into()))
            } else {
                None
            },
        },
    );
}

fn partition_tasks(
    app: &App,
) -> (
    Vec<&TaskQueueEntry>,
    Vec<&TaskQueueEntry>,
    Vec<&TaskQueueEntry>,
    Vec<&TaskQueueEntry>,
) {
    let mut running = Vec::new();
    let mut pending = Vec::new();
    let mut failed = Vec::new();
    let mut done = Vec::new();
    for task in &app.tasks {
        match app.queue_lane_for_task(task) {
            QueueJourneyLane::Now => running.push(task),
            QueueJourneyLane::Next => pending.push(task),
            QueueJourneyLane::NeedsAttention => failed.push(task),
            QueueJourneyLane::Done => done.push(task),
        }
    }
    // Most-recent first for `done` so the user sees what just finished.
    done.sort_by_key(|t| std::cmp::Reverse(t.completed_at));
    (running, pending, failed, done)
}

struct LaneSpec<'a> {
    title: &'a str,
    title_glyph: &'a str,
    title_style: Style,
    primary: &'a [&'a TaskQueueEntry],
    secondary: Option<(&'a [&'a TaskQueueEntry], &'a str)>,
    more_note: Option<String>,
    bulk_action: Option<(&'a str, String)>,
}

fn draw_lane(frame: &mut Frame, app: &App, area: Rect, spec: LaneSpec) {
    let inner_width = area.width as usize;
    let mut lines: Vec<Line<'static>> = Vec::new();

    // Title row
    lines.push(Line::from(vec![
        Span::styled(format!(" {} ", spec.title_glyph), spec.title_style),
        Span::styled(spec.title.to_string(), spec.title_style),
    ]));
    lines.push(Line::from(""));

    // Primary task list
    if spec.primary.is_empty() && spec.secondary.is_none_or(|(s, _)| s.is_empty()) {
        lines.push(Line::from(vec![Span::styled("   empty", dim())]));
    } else {
        for task in spec.primary {
            append_task_rows(&mut lines, app, task, inner_width);
        }

        if let Some((secondary, label)) = spec.secondary {
            if !secondary.is_empty() {
                lines.push(Line::from(""));
                lines.push(Line::from(vec![
                    Span::styled("   ", dim()),
                    Span::styled(format!("{} {}", secondary.len(), label), muted()),
                ]));
                for task in secondary.iter().take(3) {
                    append_task_rows(&mut lines, app, task, inner_width);
                }
                if secondary.len() > 3 {
                    lines.push(Line::from(vec![
                        Span::styled("   ", dim()),
                        Span::styled(format!("…+{} more", secondary.len() - 3), dim()),
                    ]));
                }
            }
        }
    }

    if let Some(note) = spec.more_note {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled(" ", dim()),
            Span::styled(format!("({})", note), dim().add_modifier(Modifier::ITALIC)),
        ]));
    }

    if let Some((key, label)) = spec.bulk_action {
        lines.push(Line::from(vec![
            Span::styled(" › ", dim()),
            Span::styled(format!("[{}]", key), accent().add_modifier(Modifier::BOLD)),
            Span::styled(format!(" {}", label), text()),
        ]));
    }

    frame.render_widget(Paragraph::new(lines), area);
}

fn append_task_rows(
    lines: &mut Vec<Line<'static>>,
    app: &App,
    task: &TaskQueueEntry,
    width: usize,
) {
    let is_cursor = app.tasks.get(app.task_cursor).map(|t| &t.id) == Some(&task.id);
    let (cursor_prefix, name_style) = if is_cursor {
        ("▌", accent().add_modifier(Modifier::BOLD))
    } else {
        (" ", text())
    };

    let lane = app.queue_lane_for_task(task);
    let glyph = lane_glyph(task, lane, app);
    let glyph_style = match lane {
        QueueJourneyLane::Now => loading(),
        QueueJourneyLane::Next => muted(),
        QueueJourneyLane::NeedsAttention => error(),
        QueueJourneyLane::Done => {
            if task.status == TaskQueueStatus::Cancelled {
                muted()
            } else {
                success()
            }
        }
    };

    let name_budget = width.saturating_sub(6); // glyph + space + right meta room
    let name = truncate(&task.package_name, name_budget.max(6));
    let right = right_meta(task);

    // Line 1: " <glyph> <name>              <right-meta>"
    let name_width = name.chars().count();
    let right_width = right.chars().count();
    let gap = width.saturating_sub(3 + name_width + right_width).max(1);

    lines.push(Line::from(vec![
        Span::styled(
            cursor_prefix.to_string(),
            if is_cursor { accent() } else { dim() },
        ),
        Span::styled(format!("{} ", glyph), glyph_style),
        Span::styled(name, name_style),
        Span::styled(" ".repeat(gap), dim()),
        Span::styled(right, meta_style(task)),
    ]));

    // Line 2: "   source verb  elapsed/error" — bounded to the lane width so
    // long content is ellipsized instead of clipped at the lane edge.
    let verb = action_verb(task.action);
    let source = truncate(&task.package_source.to_string(), width.saturating_sub(4));
    let timing = task_timing(task);

    let mut used = 3 + source.chars().count();
    let verb = truncate(verb, width.saturating_sub(used + 1));
    used += 1 + verb.chars().count();

    let mut sub_spans = vec![
        Span::styled("   ", dim()),
        Span::styled(source, source_color(task.package_source)),
        Span::styled(" ", dim()),
        Span::styled(verb, muted()),
    ];
    if !timing.is_empty() && used + 2 + timing.chars().count() <= width {
        sub_spans.push(Span::styled("  ", dim()));
        sub_spans.push(Span::styled(timing, dim()));
    }

    // On failures: show the short error and an inline action chip.
    if lane == QueueJourneyLane::NeedsAttention {
        lines.push(Line::from(sub_spans));
        if let Some(err) = &task.error {
            let short = truncate(&short_error(err), width.saturating_sub(4));
            lines.push(Line::from(vec![
                Span::styled("   ", dim()),
                Span::styled(short, error()),
            ]));
        }
        lines.push(Line::from(vec![
            Span::styled(" › ", dim()),
            Span::styled("[R]", accent().add_modifier(Modifier::BOLD)),
            Span::styled(" retry  ", text()),
            Span::styled("[M]", accent().add_modifier(Modifier::BOLD)),
            Span::styled(" fix  ", text()),
            Span::styled("[X]", accent().add_modifier(Modifier::BOLD)),
            Span::styled(" cancel", text()),
        ]));
    } else {
        lines.push(Line::from(sub_spans));
    }
}

fn format_elapsed(secs: i64) -> String {
    let secs = secs.max(0);
    if secs < 60 {
        format!("{}s", secs)
    } else if secs < 3600 {
        format!("{}m{:02}s", secs / 60, secs % 60)
    } else if secs < 86_400 {
        format!("{}h{:02}m", secs / 3600, (secs % 3600) / 60)
    } else {
        format!("{}d", secs / 86_400)
    }
}

fn right_meta(task: &TaskQueueEntry) -> String {
    match task.status {
        TaskQueueStatus::Running => {
            if let Some(started) = task.started_at {
                let secs = (Local::now() - started).num_seconds();
                format_elapsed(secs)
            } else {
                "…".into()
            }
        }
        TaskQueueStatus::Queued => "queued".into(),
        TaskQueueStatus::Completed => {
            if let (Some(start), Some(end)) = (task.started_at, task.completed_at) {
                format_elapsed((end - start).num_seconds())
            } else {
                "done".into()
            }
        }
        TaskQueueStatus::Cancelled => "cancelled".into(),
        TaskQueueStatus::Failed => "failed".into(),
    }
}

fn meta_style(task: &TaskQueueEntry) -> Style {
    match task.status {
        TaskQueueStatus::Running => loading(),
        TaskQueueStatus::Queued => muted(),
        TaskQueueStatus::Completed => success(),
        TaskQueueStatus::Cancelled => muted(),
        TaskQueueStatus::Failed => error(),
    }
}

fn lane_glyph(task: &TaskQueueEntry, lane: QueueJourneyLane, app: &App) -> char {
    match lane {
        QueueJourneyLane::Now => app.spinner_frame(),
        QueueJourneyLane::Next => '◯',
        QueueJourneyLane::NeedsAttention => '✗',
        QueueJourneyLane::Done => match task.status {
            TaskQueueStatus::Cancelled => '◌',
            _ => '✓',
        },
    }
}

fn action_verb(action: TaskQueueAction) -> &'static str {
    match action {
        TaskQueueAction::Install => "install",
        TaskQueueAction::Remove => "remove",
        TaskQueueAction::Update => "update",
    }
}

fn task_timing(task: &TaskQueueEntry) -> String {
    match (task.started_at, task.completed_at, task.status) {
        (Some(start), Some(end), _) => format_elapsed((end - start).num_seconds()),
        (Some(start), None, TaskQueueStatus::Running) => {
            format!(
                "started {} ago",
                format_elapsed((Local::now() - start).num_seconds())
            )
        }
        _ => String::new(),
    }
}

fn short_error(err: &str) -> String {
    err.lines()
        .next()
        .unwrap_or(err)
        .chars()
        .take(80)
        .collect::<String>()
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else if max <= 1 {
        "…".into()
    } else {
        let mut out: String = s.chars().take(max - 1).collect();
        out.push('…');
        out
    }
}

fn draw_details_strip(frame: &mut Frame, app: &App, area: Rect) {
    if area.height == 0 {
        return;
    }
    let Some(task) = app.tasks.get(app.task_cursor) else {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                " (no task selected — press ↑↓ to pick one)",
                dim(),
            ))),
            area,
        );
        return;
    };

    let verb = action_verb(task.action);
    let first = Line::from(vec![
        Span::styled(" ", text()),
        Span::styled(
            task.package_name.clone(),
            accent().add_modifier(Modifier::BOLD),
        ),
        Span::styled("  · ", dim()),
        Span::styled(
            task.package_source.to_string(),
            source_color(task.package_source),
        ),
        Span::styled(" ", dim()),
        Span::styled(verb, muted()),
        Span::styled("  · ", dim()),
        Span::styled(task_timing(task), dim()),
    ]);

    let second = if let Some(err) = &task.error {
        Line::from(vec![
            Span::styled(" last: ", dim()),
            Span::styled(
                truncate(&short_error(err), area.width.saturating_sub(8) as usize),
                error(),
            ),
        ])
    } else if let Some(parent) = app.retry_parent_for_task(&task.id) {
        let attempt = app.retry_attempt_for_task(&task.id).unwrap_or(1);
        Line::from(vec![
            Span::styled(" retry ", dim()),
            Span::styled(format!("#{}", attempt), warning()),
            Span::styled(" of ", dim()),
            Span::styled(
                truncate(&parent.package_name, area.width.saturating_sub(16) as usize),
                muted().add_modifier(Modifier::ITALIC),
            ),
        ])
    } else {
        Line::from(vec![
            Span::styled(" id ", dim()),
            Span::styled(
                task.id.chars().take(8).collect::<String>(),
                muted().add_modifier(Modifier::ITALIC),
            ),
        ])
    };

    frame.render_widget(Paragraph::new(vec![first, second]), area);
}
