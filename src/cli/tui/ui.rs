use super::app::{
    action_label, App, ChangelogState, PendingAction, PreflightCertainty, PreflightRiskLevel,
    PreflightSummary, MIN_HEIGHT, MIN_WIDTH,
};
use super::format::truncate_to_width;
use super::theme::*;
use crate::cli::tui::components::header::draw_filter_bar;
pub use crate::cli::tui::components::header::QueueHintAction;
use crate::cli::tui::state::filters::{Filter, Focus};
use crate::models::history::{TaskQueueEntry, TaskQueueStatus};
use crate::models::{Package, PackageSource, PackageStatus};
use chrono::Local;
use ratatui::{
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{
        Block, Borders, Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap,
    },
    Frame,
};
use unicode_width::UnicodeWidthStr;

pub fn draw(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
        draw_too_small(frame, area);
        return;
    }

    let queue_height = app.queue_bar_height();
    let constraints = vec![
        Constraint::Length(1),
        Constraint::Min(1),
        Constraint::Length(queue_height),
        Constraint::Length(1),
    ];
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    draw_filter_bar(frame, app, chunks[0]);

    draw_main_content(frame, app, chunks[1]);

    let footer_chunk = if queue_height > 0 {
        draw_queue_bar(frame, app, chunks[2]);
        chunks[3]
    } else {
        chunks[2]
    };
    draw_footer(frame, app, footer_chunk);

    if app.showing_palette {
        draw_palette_overlay(frame, app);
    } else if app.showing_changelog {
        draw_changelog_overlay(frame, app);
    } else if app.showing_help {
        draw_help_overlay(frame, app);
    } else if let Some(confirming) = &app.confirming {
        draw_preflight_overlay(frame, app, confirming);
    } else if app.showing_import_preview {
        draw_import_preview_overlay(frame, app, frame.area());
    }
}

fn draw_too_small(frame: &mut Frame, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_set(ROUNDED)
        .title(" LinGet ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines = vec![
        Line::from(Span::styled("Terminal too small", error())),
        Line::from(Span::styled(
            format!("Minimum size: {}x{}", MIN_WIDTH, MIN_HEIGHT),
            muted(),
        )),
    ];
    let paragraph = Paragraph::new(lines)
        .alignment(ratatui::layout::Alignment::Center)
        .wrap(Wrap { trim: true });
    frame.render_widget(paragraph, inner);
}

pub fn preflight_modal_hit_test(modal_rect: Rect, col: u16, row: u16) -> Option<bool> {
    if modal_rect.width <= 4 || modal_rect.height <= 4 {
        return None;
    }
    if col < modal_rect.x || col >= modal_rect.x + modal_rect.width {
        return None;
    }
    if row < modal_rect.y || row >= modal_rect.y + modal_rect.height {
        return None;
    }

    let inner = modal_rect.inner(Margin {
        vertical: 1,
        horizontal: 1,
    });
    if inner.height == 0 || inner.width == 0 {
        return None;
    }

    let footer_row = inner.y + inner.height.saturating_sub(1);
    if row != footer_row {
        return None;
    }
    if col < inner.x || col >= inner.x + inner.width {
        return None;
    }

    let rel_col = (col - inner.x) as usize;
    let confirm_start = 0usize;
    let confirm_width = UnicodeWidthStr::width("[y] confirm");
    if rel_col >= confirm_start && rel_col < confirm_start + confirm_width {
        return Some(true);
    }

    let cancel_start = UnicodeWidthStr::width("[y] confirm  ");
    let cancel_width = UnicodeWidthStr::width("[n] cancel");
    if rel_col >= cancel_start && rel_col < cancel_start + cancel_width {
        return Some(false);
    }

    None
}

pub fn header_filter_hit_test(
    app: &App,
    header_filter_row: Rect,
    col: u16,
    row: u16,
) -> Option<crate::cli::tui::components::header::HeaderAction> {
    crate::cli::tui::components::header::header_action_hit_test(app, header_filter_row, col, row)
}

pub fn queue_hint_hit_test(
    hint_rect: Rect,
    _has_log_actions: bool,
    col: u16,
    row: u16,
) -> Option<QueueHintAction> {
    const REMEDIATE_HINT: &str = "M apply filtered fixes";
    const RETRY_SAFE_HINT: &str = "A retry safe";

    if hint_rect.width == 0 || hint_rect.height == 0 || row != hint_rect.y {
        return None;
    }
    if col < hint_rect.x || col >= hint_rect.x + hint_rect.width {
        return None;
    }

    let rel_col = (col - hint_rect.x) as usize;
    let retry_start = 0usize;
    let retry_width = UnicodeWidthStr::width("R retry selected");
    if rel_col >= retry_start && rel_col < retry_start + retry_width {
        return Some(QueueHintAction::Retry);
    }

    let remediate_start = UnicodeWidthStr::width("R retry selected  ");
    let remediate_width = UnicodeWidthStr::width(REMEDIATE_HINT);
    if rel_col >= remediate_start && rel_col < remediate_start + remediate_width {
        return Some(QueueHintAction::Remediate);
    }

    let retry_safe_start = UnicodeWidthStr::width("R retry selected  M apply filtered fixes  ");
    let retry_safe_width = UnicodeWidthStr::width(RETRY_SAFE_HINT);
    if rel_col >= retry_safe_start && rel_col < retry_safe_start + retry_safe_width {
        return Some(QueueHintAction::RetrySafe);
    }

    None
}
pub fn compose_left_right<'a>(
    mut left: Vec<Span<'a>>,
    right: Vec<Span<'a>>,
    width: usize,
) -> Line<'a> {
    let left_width = spans_width(&left);
    let right_width = spans_width(&right);

    if right.is_empty() {
        return Line::from(left);
    }

    if left_width + right_width + 1 > width {
        return Line::from(left);
    }

    let padding = width.saturating_sub(left_width + right_width);
    left.push(Span::raw(" ".repeat(padding)));
    left.extend(right);

    Line::from(left)
}

struct DecisionCardContent {
    what_happens: String,
    certainty: String,
    certainty_style: Style,
    risk: String,
    risk_style: Style,
    privileges: String,
    privileges_style: Style,
    if_blocked: String,
    primary_action: String,
    primary_style: Style,
}

fn append_decision_card(
    lines: &mut Vec<Line<'static>>,
    max_width: usize,
    card: DecisionCardContent,
) {
    push_decision_field(lines, "What", &card.what_happens, muted(), max_width);
    push_decision_field(
        lines,
        "Certainty",
        &card.certainty,
        card.certainty_style,
        max_width,
    );
    push_decision_field(lines, "Risk", &card.risk, card.risk_style, max_width);
    push_decision_field(
        lines,
        "Privileges",
        &card.privileges,
        card.privileges_style,
        max_width,
    );
    push_decision_field(lines, "If blocked", &card.if_blocked, muted(), max_width);
    push_decision_field(
        lines,
        "Action",
        &card.primary_action,
        card.primary_style,
        max_width,
    );
    lines.push(Line::from(""));
}

fn push_decision_field(
    lines: &mut Vec<Line<'static>>,
    label: &str,
    value: &str,
    style: Style,
    max_width: usize,
) {
    let prefix = format!("{}: ", label);
    let prefix_width = UnicodeWidthStr::width(prefix.as_str());
    let wrapped = wrap_text(value, max_width.saturating_sub(prefix_width).max(1));
    let continuation_pad = " ".repeat(prefix_width);

    for (idx, segment) in wrapped.into_iter().enumerate() {
        if idx == 0 {
            lines.push(Line::from(vec![
                Span::styled(prefix.clone(), dim()),
                Span::styled(segment, style),
            ]));
        } else {
            lines.push(Line::from(vec![
                Span::styled(continuation_pad.clone(), dim()),
                Span::styled(segment, style),
            ]));
        }
    }
}

pub fn spans_width(spans: &[Span<'_>]) -> usize {
    spans
        .iter()
        .map(|span| UnicodeWidthStr::width(span.content.as_ref()))
        .sum()
}

fn draw_main_content(frame: &mut Frame, app: &mut App, area: Rect) {
    if app.queue_expanded {
        frame.render_widget(ratatui::widgets::Clear, area);
        draw_expanded_queue(frame, app, area);
        return;
    }
    crate::cli::tui::components::workspace::draw(frame, app, area);
}

pub fn panel_block(title: String, focused: bool, _compact: bool) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_set(ROUNDED)
        .border_style(if focused {
            border_focused()
        } else {
            border_unfocused()
        })
        .title(title)
        .title_style(if focused { accent() } else { text() })
}

pub fn source_count_label(filter: Filter, counts: [usize; 6]) -> String {
    match filter {
        Filter::All => {
            if counts[2] > 0 {
                format!(" {} (+{})", counts[0], counts[2])
            } else {
                format!(" {}", counts[0])
            }
        }
        Filter::Installed => format!(" {}", counts[1]),
        Filter::Updates => format!(" {}", counts[2]),
        Filter::Favorites => format!(" {}", counts[3]),
        Filter::SecurityUpdates => format!(" {}", counts[4]),
        Filter::Duplicates => format!(" {}", counts[5]),
    }
}

fn draw_queue_bar(frame: &mut Frame, app: &App, area: Rect) {
    if app.tasks.is_empty() {
        return;
    }

    let (queued, running, completed, failed, cancelled) = app.queue_counts();
    let total = app.tasks.len();
    let done = completed + failed + cancelled;
    let queued_work = total.saturating_sub(done) as f64;

    if running > 0 {
        let active_task = app
            .tasks
            .iter()
            .find(|task| task.status == TaskQueueStatus::Running);
        let phase = active_task
            .map(|task| running_task_phase(app, task))
            .unwrap_or(RunningTaskPhase::Resolve);
        let active_label = active_task
            .map(|task| format!("{} {}", action_label(task.action), task.package_name))
            .unwrap_or_else(|| "Working".to_string());
        let task_eta_hint = active_task.and_then(|task| running_task_eta_hint(task, &app.tasks));
        let trust_hint = active_task.and_then(|task| running_task_signal_hint(app, task));
        let eta_confidence = queue_eta_confidence(&app.tasks, trust_hint.as_deref());
        let progressed_units =
            (done as f64 + running as f64 * phase.progress_weight()).clamp(0.0, total as f64);
        let remaining_work = (total as f64 - progressed_units).max(0.0);
        let performance_hint = queue_performance_hint(&app.tasks, remaining_work, eta_confidence);
        let ratio = if total > 0 {
            (progressed_units / total as f64).clamp(0.0, 1.0)
        } else {
            0.0
        };

        let mut status_text = build_running_queue_label(RunningQueueLabelArgs {
            spinner: app.spinner_frame(),
            phase_label: phase.label(),
            active_label: &active_label,
            done,
            total,
            remaining: remaining_work.ceil().max(0.0) as usize,
            queued,
            failed,
        });
        if let Some(gauge_hint) = performance_hint.or(task_eta_hint) {
            if !gauge_hint.is_empty() {
                status_text.push_str(" · ");
                status_text.push_str(&gauge_hint);
            }
        }
        let progress_bar = inline_progress_bar(ratio, queue_progress_bar_width(area.width));
        let percent = format!("{:>3}%", (ratio * 100.0).round() as usize);
        let reserved_width = progress_bar.len() + percent.len() + 2;
        let summary_width = area.width.saturating_sub(reserved_width as u16) as usize;
        let line = Line::from(vec![
            Span::styled(progress_bar, if failed > 0 { warning() } else { accent() }),
            Span::raw(" "),
            Span::styled(percent, dim()),
            Span::raw(" "),
            Span::styled(truncate_to_width(&status_text, summary_width), muted()),
        ]);
        frame.render_widget(Paragraph::new(line), area);
    } else {
        let performance_hint = queue_performance_hint(
            &app.tasks,
            queued_work,
            queue_eta_confidence(&app.tasks, None),
        );
        let (message, state) = build_idle_queue_label(
            queued,
            completed,
            cancelled,
            failed,
            done,
            total,
            performance_hint.as_deref(),
        );
        let style = match state {
            QueueBarState::Queued => muted(),
            QueueBarState::Failed => warning(),
            QueueBarState::Complete => success(),
        };
        let line = truncate_to_width(&message, area.width as usize);
        frame.render_widget(Paragraph::new(line).style(style), area);
    }
}

fn queue_progress_bar_width(area_width: u16) -> usize {
    if area_width > 140 {
        12
    } else if area_width > 100 {
        10
    } else if area_width > 70 {
        8
    } else {
        6
    }
}

fn inline_progress_bar(ratio: f64, width: usize) -> String {
    let filled = (ratio.clamp(0.0, 1.0) * width as f64).round() as usize;
    let filled = filled.min(width);
    format!(
        " {}{} ",
        "█".repeat(filled),
        "░".repeat(width.saturating_sub(filled))
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum QueueBarState {
    Queued,
    Failed,
    Complete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum QueueEtaConfidence {
    Estimated,
    Verified,
}

impl QueueEtaConfidence {
    fn label(self) -> &'static str {
        match self {
            Self::Estimated => "estimated",
            Self::Verified => "verified",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RunningTaskPhase {
    Resolve,
    Download,
    Apply,
}

impl RunningTaskPhase {
    fn label(self) -> &'static str {
        match self {
            Self::Resolve => "resolve",
            Self::Download => "download",
            Self::Apply => "apply",
        }
    }

    fn progress_weight(self) -> f64 {
        match self {
            Self::Resolve => 0.25,
            Self::Download => 0.55,
            Self::Apply => 0.90,
        }
    }
}

struct RunningQueueLabelArgs<'a> {
    spinner: char,
    phase_label: &'a str,
    active_label: &'a str,
    done: usize,
    total: usize,
    remaining: usize,
    queued: usize,
    failed: usize,
}

fn build_running_queue_label(args: RunningQueueLabelArgs<'_>) -> String {
    let mut parts = Vec::new();

    parts.push(format!(
        "{} {} · {}",
        args.spinner, args.phase_label, args.active_label
    ));
    parts.push(format!("{}/{} done", args.done, args.total));

    if args.queued > 0 {
        parts.push(format!("{} queued", args.queued));
    }
    if args.failed > 0 {
        parts.push(format!("{} failed", args.failed));
    }
    if args.remaining > 0 {
        parts.push(format!("{} left", args.remaining));
    }

    parts.join(" · ")
}

fn build_idle_queue_label(
    queued: usize,
    completed: usize,
    cancelled: usize,
    failed: usize,
    done: usize,
    total: usize,
    performance_hint: Option<&str>,
) -> (String, QueueBarState) {
    if queued > 0 {
        let mut text_value = if done > 0 {
            format!("◻ {} queued · {}/{} done", queued, done, total)
        } else {
            format!("◻ {} queued", queued)
        };
        if let Some(hint) = performance_hint {
            text_value.push_str(" · ");
            text_value.push_str(hint);
        }
        return (text_value, QueueBarState::Queued);
    }

    if failed > 0 {
        let mut text_value = format!("⚠ {} done, {} failed", completed + cancelled, failed);
        if let Some(hint) = performance_hint {
            text_value.push_str(" · ");
            text_value.push_str(hint);
        }
        return (text_value, QueueBarState::Failed);
    }

    let mut text_value = format!("✓ {}/{} complete", done, total);
    if let Some(hint) = performance_hint {
        text_value.push_str(" · ");
        text_value.push_str(hint);
    }
    (text_value, QueueBarState::Complete)
}

fn queue_performance_hint(
    tasks: &[TaskQueueEntry],
    remaining_work: f64,
    confidence: Option<QueueEtaConfidence>,
) -> Option<String> {
    let avg_secs = average_completed_task_secs(tasks)?;
    let confidence_suffix = confidence
        .map(|confidence| format!(" ({})", confidence.label()))
        .unwrap_or_default();

    let throughput = 60.0 / avg_secs;
    if remaining_work <= 0.0 {
        return Some(format!("{:.1} t/m{}", throughput, confidence_suffix));
    }

    let eta_secs = (avg_secs * remaining_work).round().max(1.0) as u64;
    Some(format!(
        "{:.1} t/m • ETA {}{}",
        throughput,
        format_eta_seconds(eta_secs),
        confidence_suffix
    ))
}

fn queue_eta_confidence(
    tasks: &[TaskQueueEntry],
    trust_hint: Option<&str>,
) -> Option<QueueEtaConfidence> {
    let sample_count = completed_task_sample_count(tasks);
    if sample_count < 2 {
        return None;
    }

    if trust_hint.is_some_and(|hint| {
        hint.starts_with("stalled")
            || hint.starts_with("quiet")
            || hint.starts_with("awaiting output")
    }) {
        return Some(QueueEtaConfidence::Estimated);
    }

    if sample_count >= 5 {
        Some(QueueEtaConfidence::Verified)
    } else {
        Some(QueueEtaConfidence::Estimated)
    }
}

fn running_task_phase(app: &App, task: &TaskQueueEntry) -> RunningTaskPhase {
    let Some(logs) = app.task_logs.get(&task.id) else {
        return RunningTaskPhase::Resolve;
    };

    for line in logs.iter().rev().take(80) {
        let normalized = normalize_task_log_line_for_phase(line);
        if line_suggests_apply_phase(&normalized) {
            return RunningTaskPhase::Apply;
        }
        if line_suggests_download_phase(&normalized) {
            return RunningTaskPhase::Download;
        }
        if line_suggests_resolve_phase(&normalized) {
            return RunningTaskPhase::Resolve;
        }
    }

    RunningTaskPhase::Resolve
}

fn normalize_task_log_line_for_phase(line: &str) -> String {
    let raw = line
        .strip_prefix("[OUT] ")
        .or_else(|| line.strip_prefix("[ERR] "))
        .unwrap_or(line);
    raw.trim().to_ascii_lowercase()
}

fn line_suggests_resolve_phase(line: &str) -> bool {
    [
        "reading package lists",
        "building dependency tree",
        "resolving dependencies",
        "dependency",
        "transaction summary",
        "metadata expiration check",
        "looking for matches",
        "checking for updates",
    ]
    .iter()
    .any(|keyword| line.contains(keyword))
}

fn line_suggests_download_phase(line: &str) -> bool {
    [
        "get:",
        "download",
        "downloading",
        "retrieving",
        "fetch",
        "mb/s",
        "kb/s",
    ]
    .iter()
    .any(|keyword| line.contains(keyword))
}

fn line_suggests_apply_phase(line: &str) -> bool {
    [
        "unpacking",
        "setting up",
        "installing",
        "upgrading",
        "removing",
        "erasing",
        "running scriptlet",
        "running transaction",
        "processing triggers",
        "verifying",
        "cleanup",
    ]
    .iter()
    .any(|keyword| line.contains(keyword))
}

fn running_task_eta_hint(task: &TaskQueueEntry, tasks: &[TaskQueueEntry]) -> Option<String> {
    let avg_secs = average_completed_task_secs(tasks)?;
    let elapsed_secs = running_elapsed_secs(task)? as f64;
    if elapsed_secs <= 0.0 {
        return None;
    }

    if elapsed_secs > avg_secs * 1.35 {
        let overrun = (elapsed_secs - avg_secs).round().max(1.0) as u64;
        return Some(format!("overrun +{}", format_eta_seconds(overrun)));
    }

    let remaining = (avg_secs - elapsed_secs).round();
    if remaining <= 0.0 {
        return None;
    }

    Some(format!("task ETA {}", format_eta_seconds(remaining as u64)))
}

fn running_task_signal_hint(app: &App, task: &TaskQueueEntry) -> Option<String> {
    let elapsed_secs = running_elapsed_secs(task)?;
    let log_age_secs = app.task_last_log_age_secs(&task.id);
    running_task_signal_from(elapsed_secs, log_age_secs)
}

fn running_task_signal_from(elapsed_secs: u64, log_age_secs: Option<u64>) -> Option<String> {
    const QUIET_THRESHOLD_SECS: u64 = 60;
    const STALLED_THRESHOLD_SECS: u64 = 150;
    const FIRST_OUTPUT_WAIT_SECS: u64 = 45;

    match log_age_secs {
        Some(age) if age >= STALLED_THRESHOLD_SECS => {
            Some(format!("stalled: no output {}", format_eta_seconds(age)))
        }
        Some(age) if age >= QUIET_THRESHOLD_SECS => {
            Some(format!("quiet: no output {}", format_eta_seconds(age)))
        }
        Some(_) => None,
        None if elapsed_secs >= FIRST_OUTPUT_WAIT_SECS => Some(format!(
            "awaiting output {}",
            format_eta_seconds(elapsed_secs)
        )),
        None => None,
    }
}

fn running_elapsed_secs(task: &TaskQueueEntry) -> Option<u64> {
    let started_at = task.started_at.as_ref()?;
    let elapsed_secs = Local::now()
        .signed_duration_since(*started_at)
        .num_seconds();
    (elapsed_secs > 0).then_some(elapsed_secs as u64)
}

fn average_completed_task_secs(tasks: &[TaskQueueEntry]) -> Option<f64> {
    const MIN_SAMPLES: usize = 2;
    let duration_secs = completed_task_durations_secs(tasks);
    if duration_secs.len() < MIN_SAMPLES {
        return None;
    }

    let avg_secs = duration_secs.iter().sum::<f64>() / duration_secs.len() as f64;
    (avg_secs > 0.0).then_some(avg_secs)
}

fn completed_task_sample_count(tasks: &[TaskQueueEntry]) -> usize {
    completed_task_durations_secs(tasks).len()
}

fn completed_task_durations_secs(tasks: &[TaskQueueEntry]) -> Vec<f64> {
    const MAX_SAMPLES: usize = 8;

    let mut duration_secs = Vec::new();
    for task in tasks.iter().rev() {
        if task.status != TaskQueueStatus::Completed {
            continue;
        }
        let (Some(started_at), Some(completed_at)) =
            (task.started_at.as_ref(), task.completed_at.as_ref())
        else {
            continue;
        };
        let elapsed_ms = completed_at
            .signed_duration_since(*started_at)
            .num_milliseconds();
        if elapsed_ms <= 0 {
            continue;
        }
        duration_secs.push(elapsed_ms as f64 / 1000.0);
        if duration_secs.len() >= MAX_SAMPLES {
            break;
        }
    }
    duration_secs
}

fn format_eta_seconds(total_seconds: u64) -> String {
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;
    if hours > 0 {
        return format!("{}h{:02}m", hours, minutes);
    }
    if minutes > 0 {
        return format!("{}m{:02}s", minutes, seconds);
    }
    format!("{}s", seconds)
}

fn draw_footer(frame: &mut Frame, app: &App, area: Rect) {
    let mut spans = Vec::new();

    // Search entry has its own keymap and should override normal panel hints.
    if app.searching {
        push_hint(&mut spans, "Type", "filter");
        push_hint(&mut spans, "Enter", "provider search");
        push_hint(&mut spans, "Esc", app.search_escape_hint_label());
        push_hint(&mut spans, "?", "help");
    } else {
        // Contextual footer hints based on focus
        match app.focus {
            Focus::Sources => {
                push_hint(&mut spans, "↑↓", "move");
                push_hint(&mut spans, "Tab", "panels");
                push_hint(&mut spans, "/", app.search_query_hint_label());
                if !app.search.is_empty() {
                    push_hint(&mut spans, "Esc", app.search_escape_hint_label());
                }
                push_hint(&mut spans, "?", "help");
            }
            Focus::Packages => {
                push_hint(&mut spans, "↑↓", "move");
                push_hint(&mut spans, "Enter", "action");
                push_hint(&mut spans, "Space", "select");
                push_hint(&mut spans, "/", app.search_query_hint_label());
                if !app.search.is_empty() {
                    push_hint(&mut spans, "Esc", app.search_escape_hint_label());
                }
                push_hint(&mut spans, "?", "help");
            }
            Focus::Queue => {
                push_hint(&mut spans, "↑↓", "move");
                push_hint(&mut spans, "l", "close");
                if app.queue_expanded {
                    push_hint(&mut spans, "R", "retry");
                    push_hint(&mut spans, "M", "fix");
                }
                if !app.search.is_empty() {
                    push_hint(&mut spans, "Esc", app.search_escape_hint_label());
                }
                push_hint(&mut spans, "?", "help");
            }
        }
    }

    // Always show palette and quit
    push_hint(&mut spans, ":", "commands");
    push_hint(&mut spans, "q", "quit");

    let selection = if app.hidden_selected_count() > 0 {
        format!(
            "{} selected ({} hidden)",
            app.selected.len(),
            app.hidden_selected_count()
        )
    } else {
        format!("{} selected", app.selected.len())
    };
    let mode_and_selection = format!("{} · {}", app.tui_mode_label(), selection);

    let right = if app.compact && !app.status.is_empty() {
        vec![Span::styled(
            format!("{} · {}", app.tui_mode_label(), app.status),
            italic_status(),
        )]
    } else {
        vec![Span::styled(mode_and_selection, muted())]
    };

    let line = compose_left_right(spans, right, area.width as usize);

    // Use a Block to fill the entire footer area with background
    let footer_block = Block::default().style(footer_bg());
    let inner = footer_block.inner(area);
    frame.render_widget(footer_block, area);
    frame.render_widget(Paragraph::new(line), inner);
}

fn push_hint(spans: &mut Vec<Span<'static>>, key: &'static str, label: &'static str) {
    if !spans.is_empty() {
        spans.push(Span::raw("  "));
    }
    spans.push(Span::styled(key, key_hint()));
    spans.push(Span::raw(" "));
    spans.push(Span::styled(label, footer_label()));
}

fn draw_expanded_queue(frame: &mut Frame, app: &App, area: Rect) {
    let focused = app.focus == Focus::Queue;
    let title = " Queue ".to_string();
    let block = panel_block(title, focused, app.compact);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if app.tasks.is_empty() {
        let empty = Paragraph::new("No tasks queued — press Esc to close")
            .alignment(ratatui::layout::Alignment::Center)
            .style(dim());
        frame.render_widget(empty, inner);
        return;
    }

    crate::cli::tui::components::queue_board::draw(frame, app, inner);
}

pub fn preflight_overlay_rect(area: Rect) -> Rect {
    centered_rect(area, 76, 76, 62, 16)
}

fn preflight_touched_package_estimate(preflight: &PreflightSummary) -> usize {
    if let Some(impact) = preflight.dependency_impact.as_ref() {
        let probed_total = impact.install_count
            + impact.upgrade_count
            + impact.remove_count
            + impact.held_back_count;
        if probed_total > 0 {
            return probed_total;
        }
    }

    let selected_total = preflight
        .target_count
        .saturating_sub(preflight.skipped_count);
    preflight
        .executable_count
        .max(selected_total)
        .max(preflight.target_count)
}

fn preflight_conflict_signal(preflight: &PreflightSummary) -> (String, bool) {
    let held_back = preflight
        .dependency_impact
        .as_ref()
        .map(|impact| impact.held_back_count)
        .unwrap_or(0);
    if held_back > 0 {
        return (format!("possible ({} held back)", held_back), true);
    }

    if preflight.risk_reasons.iter().any(|reason| {
        let lowered = reason.to_ascii_lowercase();
        lowered.contains("conflict")
            || lowered.contains("lock")
            || lowered.contains("held back")
            || lowered.contains("dependency problem")
    }) {
        return ("possible (provider signal)".to_string(), true);
    }

    if preflight.verification_in_progress {
        return ("pending verification".to_string(), true);
    }

    if preflight.dependency_impact_known {
        ("none detected".to_string(), false)
    } else {
        ("best-effort estimate".to_string(), false)
    }
}

fn preflight_forecast_text(preflight: &PreflightSummary) -> (String, bool) {
    let touched = preflight_touched_package_estimate(preflight);
    let package_word = if touched == 1 { "package" } else { "packages" };
    let (conflict_signal, conflicts_active) = preflight_conflict_signal(preflight);
    let certainty = if preflight.dependency_impact_known {
        "verified"
    } else if preflight.verification_in_progress {
        "verifying"
    } else {
        "estimated"
    };

    (
        format!(
            "~{} {} touched · conflicts: {} · {}",
            touched, package_word, conflict_signal, certainty
        ),
        conflicts_active,
    )
}

fn draw_preflight_overlay(frame: &mut Frame, _app: &App, confirming: &PendingAction) {
    let area = preflight_overlay_rect(frame.area());
    frame.render_widget(Clear, area);

    let risk_style = match confirming.preflight.risk_level {
        PreflightRiskLevel::Safe => success(),
        PreflightRiskLevel::Caution => warning(),
        PreflightRiskLevel::High => error(),
    };
    let certainty_style = match confirming.preflight.certainty {
        PreflightCertainty::Estimated => warning(),
        PreflightCertainty::Verified => success(),
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_set(ROUNDED)
        .border_style(border_focused())
        .title(" Preflight ")
        .title_style(accent());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.width <= 2 || inner.height <= 4 {
        return;
    }

    let body_height = inner.height.saturating_sub(1);
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(Rect {
            x: inner.x,
            y: inner.y,
            width: inner.width,
            height: body_height,
        });

    let source_summary = if confirming.preflight.source_breakdown.is_empty() {
        "none".to_string()
    } else {
        confirming
            .preflight
            .source_breakdown
            .iter()
            .map(|(source, count)| format!("{} {}", source, count))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let impact_summary = if let Some(impact) = confirming.preflight.dependency_impact.as_ref() {
        impact.summary()
    } else if confirming.preflight.verification_in_progress {
        "Dependency impact verification in progress.".to_string()
    } else {
        "No verified dependency impact breakdown yet.".to_string()
    };
    let impact_style = if confirming.preflight.dependency_impact_known {
        success()
    } else if confirming.preflight.verification_in_progress {
        warning()
    } else {
        muted()
    };
    let (forecast_summary, forecast_attention) = preflight_forecast_text(&confirming.preflight);

    let action_name = action_label(confirming.preflight.action).to_string();
    let executable = confirming.preflight.executable_count;
    let targets = confirming.preflight.target_count;
    let _skipped = confirming.preflight.skipped_count;
    let package_word = if executable == 1 {
        "package"
    } else {
        "packages"
    };
    let _mode_label = if confirming.preflight.selection_mode {
        "selection"
    } else {
        "current filter"
    };
    let primary_action = if confirming.preflight.risk_level == PreflightRiskLevel::High
        && !confirming.risk_acknowledged
    {
        "Press [y] to acknowledge high-risk gate.".to_string()
    } else {
        format!(
            "Press [y] to queue {} {} from {} selected targets.",
            executable, package_word, targets
        )
    };

    let mut lines = Vec::new();
    append_decision_card(
        &mut lines,
        sections[0].width.saturating_sub(2) as usize,
        DecisionCardContent {
            what_happens: format!(
                "{} {} {} from {} targets",
                action_name, executable, package_word, targets
            ),
            certainty: format!(
                "{}; dependency impact {}. {}",
                confirming.preflight.certainty.label(),
                if confirming.preflight.verification_in_progress {
                    "verification in progress"
                } else if confirming.preflight.dependency_impact_known {
                    "analyzed"
                } else {
                    "estimated at execution"
                },
                impact_summary
            ),
            certainty_style,
            risk: confirming.preflight.risk_level.label().to_string(),
            risk_style,
            privileges: if confirming.preflight.elevated_privileges_likely {
                "May prompt for elevated privileges.".to_string()
            } else {
                "No elevated privileges needed.".to_string()
            },
            privileges_style: if confirming.preflight.elevated_privileges_likely {
                warning()
            } else {
                muted()
            },
            if_blocked: "Failures surface in the queue with recovery steps.".to_string(),
            primary_action,
            primary_style: if confirming.preflight.risk_level == PreflightRiskLevel::High
                && !confirming.risk_acknowledged
            {
                warning()
            } else {
                footer_label()
            },
        },
    );

    lines.push(Line::from(vec![
        Span::styled("Summary: ", dim()),
        Span::styled(
            truncate_to_width(
                &confirming.label,
                sections[0].width.saturating_sub(10) as usize,
            ),
            muted(),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled("Sources: ", dim()),
        Span::styled(
            truncate_to_width(
                &source_summary,
                sections[0].width.saturating_sub(10) as usize,
            ),
            muted(),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled("Impact: ", dim()),
        Span::styled(
            truncate_to_width(
                &impact_summary,
                sections[0].width.saturating_sub(10) as usize,
            ),
            impact_style,
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled("Forecast: ", dim()),
        Span::styled(
            truncate_to_width(
                &forecast_summary,
                sections[0].width.saturating_sub(10) as usize,
            ),
            if forecast_attention {
                warning()
            } else {
                muted()
            },
        ),
    ]));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        confirming.preflight.certainty.copy(),
        certainty_style,
    )));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        confirming.preflight.risk_level.copy(),
        risk_style,
    )));

    for reason in &confirming.preflight.risk_reasons {
        lines.push(Line::from(vec![
            Span::styled("• ", dim()),
            Span::styled(
                truncate_to_width(reason, sections[0].width.saturating_sub(3) as usize),
                muted(),
            ),
        ]));
    }

    if confirming.preflight.risk_level == PreflightRiskLevel::High && !confirming.risk_acknowledged
    {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "High-risk gate: first confirm acknowledges risk, second confirm queues tasks.",
            warning(),
        )));
    }

    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: true }), sections[0]);

    let confirm_label = if confirming.preflight.risk_level == PreflightRiskLevel::High
        && !confirming.risk_acknowledged
    {
        " acknowledge"
    } else {
        " confirm"
    };

    let controls = Line::from(vec![
        Span::styled("[y]", key_hint()),
        Span::styled(confirm_label, footer_label()),
        Span::styled("  [n]", key_hint()),
        Span::styled(" cancel  ", footer_label()),
        Span::styled("Esc", key_hint()),
        Span::styled(" close", footer_label()),
    ]);
    frame.render_widget(Paragraph::new(controls), sections[1]);
}

pub fn palette_overlay_rect(area: Rect) -> Rect {
    centered_rect(area, 72, 72, 60, 14)
}

fn draw_palette_overlay(frame: &mut Frame, app: &App) {
    let area = palette_overlay_rect(frame.area());
    frame.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_set(ROUNDED)
        .border_style(border_focused())
        .title(" Command Palette ")
        .title_style(accent());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.width == 0 || inner.height < 3 {
        return;
    }

    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(inner);

    let query_line = if app.palette_query.is_empty() {
        Line::from(vec![
            Span::styled("> ", key_hint()),
            Span::styled("Type to filter commands", dim()),
        ])
    } else {
        Line::from(vec![
            Span::styled("> ", key_hint()),
            Span::styled(app.palette_query.clone(), text()),
            Span::styled("█", accent()),
        ])
    };
    frame.render_widget(Paragraph::new(query_line), sections[0]);

    let entries = app.palette_entries();
    let mut rows = Vec::new();
    if entries.is_empty() {
        rows.push(Line::from(Span::styled("No matching commands", muted())));
    } else {
        let visible_rows = sections[1].height as usize;
        let start = window_start(entries.len(), visible_rows.max(1), app.palette_cursor);
        let end = (start + visible_rows.max(1)).min(entries.len());

        for (idx, entry) in entries
            .iter()
            .enumerate()
            .skip(start)
            .take(end.saturating_sub(start))
        {
            let selected = idx == app.palette_cursor;
            let label_style = if entry.enabled {
                if selected {
                    row_cursor()
                } else {
                    text()
                }
            } else {
                dim()
            };
            let mut left = vec![Span::styled(
                if selected { "▸ " } else { "  " },
                row_selected(),
            )];
            left.push(Span::styled(format!("[{}] ", entry.group), dim()));
            left.push(Span::styled(entry.label.to_string(), label_style));

            let right = vec![Span::styled(
                entry.shortcut.to_string(),
                if entry.enabled { key_hint() } else { dim() },
            )];
            rows.push(compose_left_right(left, right, sections[1].width as usize));
        }
    }

    frame.render_widget(Paragraph::new(rows), sections[1]);

    let footer = if let Some(entry) = entries.get(app.palette_cursor) {
        if !entry.enabled {
            Line::from(Span::styled(
                format!(
                    "Disabled: {}",
                    entry
                        .disabled_reason
                        .clone()
                        .unwrap_or_else(|| "Unavailable".to_string())
                ),
                error(),
            ))
        } else {
            Line::from(vec![
                Span::styled("Enter", key_hint()),
                Span::styled(" run  ", footer_label()),
                Span::styled("Esc", key_hint()),
                Span::styled(" close", footer_label()),
            ])
        }
    } else {
        Line::from(Span::styled("Enter run  Esc close", footer_label()))
    };
    frame.render_widget(Paragraph::new(footer), sections[2]);
}

fn source_likely_requires_elevation(source: PackageSource) -> bool {
    matches!(
        source,
        PackageSource::Apt
            | PackageSource::Dnf
            | PackageSource::Pacman
            | PackageSource::Zypper
            | PackageSource::Deb
            | PackageSource::Aur
            | PackageSource::Snap
    )
}

fn changelog_decision_card_content(
    package: &Package,
    state: Option<&ChangelogState>,
    diff_only: bool,
    supported: bool,
) -> DecisionCardContent {
    let mode_label = if diff_only {
        "version delta"
    } else {
        "full history"
    };
    let what_happens = format!(
        "Review {} changelog for {} from {}.",
        mode_label, package.name, package.source
    );

    let (certainty, certainty_style) = match state {
        Some(ChangelogState::Ready { .. }) => (
            "Verified: changelog content loaded from provider.".to_string(),
            success(),
        ),
        Some(ChangelogState::Loading) => (
            "Estimated: changelog fetch is currently in progress.".to_string(),
            warning(),
        ),
        Some(ChangelogState::Empty) if supported => (
            "Verified: provider returned no published changelog for this package.".to_string(),
            warning(),
        ),
        Some(ChangelogState::Empty) => (
            "Unavailable: this source does not expose changelog data in LinGet yet.".to_string(),
            dim(),
        ),
        Some(ChangelogState::Error(_)) => (
            "Unverified: last fetch failed; retry is available.".to_string(),
            warning(),
        ),
        None => (
            "Unverified: changelog fetch has not started for this selection.".to_string(),
            dim(),
        ),
    };

    let (primary_action, primary_style, risk, risk_style) = if !supported {
        (
            "Press [c]/Esc to close this changelog panel.".to_string(),
            footer_label(),
            "None (read-only view).".to_string(),
            success(),
        )
    } else {
        match state {
            Some(ChangelogState::Loading) | Some(ChangelogState::Error(_)) => (
                "Press [r] to retry changelog fetch.".to_string(),
                warning(),
                "Low (read-only until queued).".to_string(),
                success(),
            ),
            _ => match package.status {
                PackageStatus::UpdateAvailable => (
                    "Press [u] to queue update preflight.".to_string(),
                    footer_label(),
                    "Low (read-only until queued).".to_string(),
                    success(),
                ),
                PackageStatus::NotInstalled => (
                    "Press [i] to queue install preflight.".to_string(),
                    footer_label(),
                    "Low (read-only until queued).".to_string(),
                    success(),
                ),
                PackageStatus::Installed => (
                    "Press [x] to queue remove preflight.".to_string(),
                    warning(),
                    "Caution (remove action is destructive).".to_string(),
                    warning(),
                ),
                PackageStatus::Installing | PackageStatus::Removing | PackageStatus::Updating => (
                    "Press [c]/Esc to close after review.".to_string(),
                    footer_label(),
                    "Low (actions already in progress elsewhere).".to_string(),
                    success(),
                ),
            },
        }
    };

    DecisionCardContent {
        what_happens,
        certainty,
        certainty_style,
        risk,
        risk_style,
        privileges: if source_likely_requires_elevation(package.source) {
            "Package actions may require elevated privileges.".to_string()
        } else {
            "No elevated privileges expected for changelog actions.".to_string()
        },
        privileges_style: if source_likely_requires_elevation(package.source) {
            warning()
        } else {
            muted()
        },
        if_blocked: match state {
            Some(ChangelogState::Error(_)) => {
                "Use [r] to retry fetch. Actions [u/i/x] still route through preflight.".to_string()
            }
            Some(ChangelogState::Empty) if supported => {
                "No changelog was published; proceed via preflight if you still intend to act."
                    .to_string()
            }
            Some(ChangelogState::Empty) => {
                "This source is currently unsupported for changelogs in LinGet.".to_string()
            }
            _ => "Actions from this panel always open preflight before queueing.".to_string(),
        },
        primary_action,
        primary_style,
    }
}

fn changelog_overlay_rect(area: Rect) -> Rect {
    centered_rect(area, 82, 88, 72, 18)
}

fn draw_changelog_overlay(frame: &mut Frame, app: &App) {
    let area = changelog_overlay_rect(frame.area());
    frame.render_widget(Clear, area);
    let target_package = app.changelog_target_package();

    let package_title = target_package
        .map(|package| format!(" {} ", package.name))
        .unwrap_or_else(|| " package unavailable ".to_string());

    let block = Block::default()
        .borders(Borders::ALL)
        .border_set(ROUNDED)
        .border_style(border_focused())
        .title(format!(" Changelog{}", package_title))
        .title_style(accent());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.width == 0 || inner.height < 3 {
        return;
    }

    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(inner);

    let header = match (target_package, app.changelog_state_for_target()) {
        (Some(package), Some(ChangelogState::Loading)) => Line::from(vec![
            Span::styled(format!("{} ", app.spinner_frame()), loading()),
            Span::styled(
                format!("Loading release notes from {}", package.source),
                muted(),
            ),
        ]),
        (Some(package), Some(ChangelogState::Ready { summary, .. })) => Line::from(vec![
            Span::styled(format!("{} ", package.source), source_color(package.source)),
            Span::styled("Summary: ", dim()),
            Span::styled(summary.summary_text(), muted()),
        ]),
        (Some(package), Some(ChangelogState::Empty)) => Line::from(vec![
            Span::styled(format!("{} ", package.source), source_color(package.source)),
            Span::styled(
                if app.changelog_supported_for_target() {
                    "No changelog published for this package"
                } else {
                    "Changelog not supported for this source yet"
                },
                dim(),
            ),
        ]),
        (Some(package), Some(ChangelogState::Error(_))) => Line::from(vec![
            Span::styled(format!("{} ", package.source), source_color(package.source)),
            Span::styled("Failed to load changelog", error()),
        ]),
        (Some(package), None) => Line::from(vec![
            Span::styled(format!("{} ", package.source), source_color(package.source)),
            Span::styled(
                if app.changelog_supported_for_target() {
                    "Press r to retry fetch"
                } else {
                    "This source may not expose changelogs yet"
                },
                dim(),
            ),
        ]),
        (None, _) => Line::from(Span::styled(
            "Package is no longer available in the current result set",
            error(),
        )),
    };
    frame.render_widget(Paragraph::new(header), sections[0]);

    let content_width = sections[1].width.saturating_sub(1) as usize;
    let mut lines = Vec::new();
    if let Some(package) = target_package {
        append_decision_card(
            &mut lines,
            content_width,
            changelog_decision_card_content(
                package,
                app.changelog_state_for_target(),
                app.changelog_diff_only,
                App::changelog_supported_for_source(package.source),
            ),
        );
    }

    let mut state_lines = match app.changelog_state_for_target() {
        Some(ChangelogState::Loading) => {
            vec![Line::from(Span::styled("Fetching changelog…", loading()))]
        }
        Some(ChangelogState::Ready { content, summary }) => {
            let mut formatted = Vec::new();
            let (content_view, mode_note, mode_style) =
                changelog_render_plan(content, target_package, app.changelog_diff_only);
            push_wrapped_styled_line(&mut formatted, &mode_note, content_width, mode_style);
            formatted.push(Line::from(""));
            if !summary.highlights.is_empty() {
                formatted.push(Line::from(Span::styled("Highlights:", section_header())));
                for highlight in summary.highlights.iter().take(3) {
                    push_wrapped_styled_line(
                        &mut formatted,
                        &format!("• {}", highlight),
                        content_width,
                        muted(),
                    );
                }
                formatted.push(Line::from(""));
            }
            formatted.extend(format_changelog_content(&content_view, content_width));
            formatted
        }
        Some(ChangelogState::Empty) => vec![Line::from(Span::styled(
            if app.changelog_supported_for_target() {
                "No changelog is available for this package."
            } else {
                "This package source does not currently expose changelog data in LinGet."
            },
            muted(),
        ))],
        Some(ChangelogState::Error(error_text)) => {
            let mut formatted = vec![Line::from(Span::styled(
                "Could not load changelog:",
                error(),
            ))];
            push_wrapped_styled_line(&mut formatted, error_text, content_width, muted());
            formatted.push(Line::from(""));
            formatted.push(Line::from(Span::styled(
                "Try [r] to retry or [c]/Esc to close.",
                dim(),
            )));
            formatted
        }
        None => vec![Line::from(Span::styled(
            "No changelog request has been started for this package.",
            dim(),
        ))],
    };
    lines.append(&mut state_lines);

    if lines.is_empty() {
        lines.push(Line::from(Span::styled("No changelog content", dim())));
    }

    let visible_rows = sections[1].height as usize;
    let max_scroll = lines.len().saturating_sub(visible_rows);
    let scroll = app.changelog_scroll.min(max_scroll);
    let end = (scroll + visible_rows).min(lines.len());
    let window: Vec<Line<'static>> = lines
        .iter()
        .skip(scroll)
        .take(end.saturating_sub(scroll))
        .cloned()
        .collect();

    frame.render_widget(
        Paragraph::new(window).wrap(Wrap { trim: false }),
        sections[1],
    );

    if max_scroll > 0 {
        let mut scrollbar_state = ScrollbarState::new(lines.len()).position(scroll);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .style(scrollbar_style())
            .thumb_style(scrollbar_thumb());
        frame.render_stateful_widget(scrollbar, sections[1], &mut scrollbar_state);
    }

    let left = vec![
        Span::styled("j/k", key_hint()),
        Span::styled(" scroll  ", footer_label()),
        Span::styled("u/i/x", key_hint()),
        Span::styled(" actions  ", footer_label()),
        Span::styled("v", key_hint()),
        Span::styled(" mode  ", footer_label()),
        Span::styled("c/Esc", key_hint()),
        Span::styled(" close", footer_label()),
    ];
    let mode_label = if app.changelog_diff_only {
        "delta"
    } else {
        "full"
    };
    let right = vec![Span::styled(
        format!("{} {}/{}", mode_label, scroll + 1, lines.len()),
        dim(),
    )];
    frame.render_widget(
        Paragraph::new(compose_left_right(left, right, sections[2].width as usize)),
        sections[2],
    );
}

fn draw_help_overlay(frame: &mut Frame, app: &App) {
    let area = centered_rect(frame.area(), 60, 88, 64, 22);
    frame.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_set(ROUNDED)
        .border_style(border_focused())
        .title(" Help ")
        .title_style(accent());

    let focus_label = match app.focus {
        Focus::Sources => "Sources",
        Focus::Packages => "Packages",
        Focus::Queue => "Queue",
    };
    let mut lines = vec![
        Line::from(Span::styled("Context", section_header())),
        Line::from(format!("  Mode: {}", app.tui_mode_label())),
        Line::from(format!("  Focus: {}", focus_label)),
        Line::from(format!("  Recommended: {}", app.recommended_action_label())),
        Line::from(format!("  {}", app.recommended_action_detail())),
    ];

    if app.queue_expanded {
        lines.push(Line::from(format!(
            "  Queue filter: {}",
            app.queue_failure_filter_label()
        )));
    }

    if app.searching || !app.search.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled("Search", section_header())));
        lines.push(Line::from(format!("  Current query: {}", app.search)));
        lines.push(Line::from(format!(
            "  Current scope: {}",
            app.search_current_scope_label()
        )));
        lines.push(Line::from(format!("  {}", app.search_typing_hint_text())));
        if let Some(summary) = app.provider_search_summary() {
            lines.push(Line::from(format!("  Provider mix: {}", summary)));
        }
        lines.push(Line::from(format!(
            "  Enter runs provider search   Esc {}",
            app.search_escape_hint_label()
        )));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled("Navigation", section_header())));
    lines.push(Line::from(
        "  ↑↓/jk move   g/G top/bottom   ^d/^u half-page",
    ));
    lines.push(Line::from("  Tab cycle panels   Ctrl+b toggle sidebar"));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled("Filters", section_header())));
    lines.push(Line::from(
        "  1 All   2 Installed   3 Updates   4 Favorites",
    ));
    lines.push(Line::from(
        "  5 Security   6 Duplicates   v favorites updates-only",
    ));
    lines.push(Line::from(""));

    if app.queue_expanded {
        lines.push(Line::from(Span::styled("Queue", section_header())));
        lines.push(Line::from("  R retry selected failure"));
        lines.push(Line::from(
            "  M apply filtered fixes   A retry safe failures",
        ));
        lines.push(Line::from(
            "  1 permissions   2 network   3 conflict   4 other   0 all",
        ));
        lines.push(Line::from("  [ ] scroll selected task log"));
        lines.push(Line::from("  l close queue"));
    } else {
        lines.push(Line::from(Span::styled("Packages", section_header())));
        lines.push(Line::from("  Space select   f favorite   F bulk favorite"));
        lines.push(Line::from(
            "  a select all   Enter default action   w recommended action",
        ));
        lines.push(Line::from("  i install   d/x remove   u update"));
        lines.push(Line::from(
            "  c changelog (apt/dnf/pip/npm/cargo/conda/mamba sources)",
        ));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled("Global", section_header())));
    lines.push(Line::from(
        "  : or Ctrl+P command palette   / search   r refresh",
    ));
    lines.push(Line::from("  ? help   q quit   T cycle theme"));
    lines.push(Line::from(format!(
        "  Theme: {}   (env: LINGET_THEME, NO_COLOR)",
        crate::cli::tui::theme::current_theme_name()
    )));
    lines.push(Line::from(
        "  Changelog: u update   i install   d/x remove   v mode   c/Esc close",
    ));
    lines.push(Line::from(""));
    lines.push(Line::from("  ? or Esc to close"));

    frame.render_widget(Paragraph::new(lines).block(block).style(text()), area);
}

fn draw_import_preview_overlay(frame: &mut Frame, app: &App, area: Rect) {
    let n = app.import_preview.len();
    let height = (n as u16 + 4).min(20);
    let popup_area = centered_rect(area, 80, 90, 60, height);
    frame.render_widget(Clear, popup_area);

    let title = format!(" Import Preview — {} packages to install ", n);
    let block = panel_block(title, true, app.compact);
    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    if inner.height == 0 {
        return;
    }

    let list_height = inner.height.saturating_sub(1) as usize;
    let mut lines: Vec<Line> = app
        .import_preview
        .iter()
        .take(list_height)
        .map(|ep| {
            Line::from(vec![
                Span::raw("  • "),
                Span::styled(ep.name.clone(), accent()),
                Span::styled(format!("  ({})  ", ep.source), muted()),
                Span::styled(ep.version.clone(), dim()),
            ])
        })
        .collect();

    while lines.len() < list_height {
        lines.push(Line::from(""));
    }

    lines.push(Line::from(vec![
        Span::styled("  Enter", key_hint()),
        Span::styled(" install all   ", footer_label()),
        Span::styled("Esc", key_hint()),
        Span::styled(" cancel", footer_label()),
    ]));

    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    let list_lines: Vec<Line> = lines[..lines.len().saturating_sub(1)].to_vec();
    let footer_line = lines.last().cloned().unwrap_or_default();

    frame.render_widget(Paragraph::new(list_lines).style(text()), sections[0]);
    frame.render_widget(Paragraph::new(vec![footer_line]), sections[1]);
}

fn centered_rect(
    area: Rect,
    width_percent: u16,
    height_percent: u16,
    min_width: u16,
    min_height: u16,
) -> Rect {
    let width = ((area.width as u32 * width_percent as u32) / 100) as u16;
    let height = ((area.height as u32 * height_percent as u32) / 100) as u16;
    let width = width.max(min_width).min(area.width);
    let height = height.max(min_height).min(area.height);

    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;

    Rect {
        x,
        y,
        width,
        height,
    }
}

pub fn window_start(total: usize, visible: usize, selected: usize) -> usize {
    if total <= visible || visible == 0 {
        return 0;
    }
    let half = visible / 2;
    let mut start = selected.saturating_sub(half);
    if start + visible > total {
        start = total - visible;
    }
    start
}

fn push_wrapped_styled_line(
    lines: &mut Vec<Line<'static>>,
    text_value: &str,
    max_width: usize,
    style: Style,
) {
    let wrapped = wrap_text(text_value, max_width.max(1));
    for segment in wrapped {
        lines.push(Line::from(Span::styled(segment, style)));
    }
}

fn parse_markdown_bold_pair(line: &str) -> Option<(String, String)> {
    if !line.starts_with("**") {
        return None;
    }
    let without_open = &line[2..];
    let close = without_open.find("**")?;
    let key = without_open[..close].trim().trim_end_matches(':').trim();
    let rest = without_open[close + 2..].trim_start();
    let value = rest.strip_prefix(':').unwrap_or(rest).trim();
    if key.is_empty() || value.is_empty() {
        return None;
    }
    Some((key.to_string(), value.to_string()))
}

fn parse_markdown_link(line: &str) -> Option<(String, String)> {
    let (left, right) = line.split_once("](")?;
    let label = left.strip_prefix('[')?.trim();
    let url = right.strip_suffix(')')?.trim();
    if label.is_empty() || url.is_empty() {
        return None;
    }
    Some((label.to_string(), url.to_string()))
}

fn is_metadata_key(key: &str) -> bool {
    matches!(
        key.to_ascii_lowercase().as_str(),
        "repository"
            | "homepage"
            | "license"
            | "author"
            | "documentation"
            | "categories"
            | "keywords"
            | "total downloads"
            | "downloads"
            | "released"
            | "package"
            | "source"
            | "maintainer"
            | "urgency"
    )
}

fn parse_metadata_pair(line: &str) -> Option<(String, String, bool)> {
    if let Some((key, value)) = parse_markdown_bold_pair(line) {
        let is_url = value.starts_with("http://") || value.starts_with("https://");
        return Some((key, value, is_url));
    }

    let (key, value) = line.split_once(':')?;
    let key = key.trim();
    let value = value.trim();
    if key.is_empty() || value.is_empty() || !is_metadata_key(key) {
        return None;
    }
    let is_url = value.starts_with("http://") || value.starts_with("https://");
    Some((key.to_string(), value.to_string(), is_url))
}

fn parse_apt_entry_header(line: &str) -> Option<(String, String, Option<String>)> {
    let open = line.find('(')?;
    let close = line[open + 1..].find(')')? + open + 1;
    if open == 0 || close <= open + 1 {
        return None;
    }

    let package = line[..open].trim();
    let version = line[open + 1..close].trim();
    if package.is_empty() || !looks_like_version_token(version) {
        return None;
    }

    let trailing = line[close + 1..].trim();
    let channel = trailing
        .split(';')
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty() && !value.starts_with("urgency="))
        .map(ToString::to_string);

    Some((package.to_string(), version.to_string(), channel))
}

fn parse_dnf_entry_header(line: &str) -> Option<(String, Option<String>)> {
    if !line.starts_with('*') {
        return None;
    }

    let body = line.trim_start_matches('*').trim();
    let (left, right) = body.rsplit_once(" - ")?;
    let version_token = sanitize_version_token(right.split_whitespace().next()?);
    if !looks_like_version_token(&version_token) {
        return None;
    }

    let words: Vec<&str> = left.split_whitespace().collect();
    let date_hint = if words.len() >= 4
        && words[0].len() <= 3
        && words[1].len() >= 3
        && words[2].chars().all(|ch| ch.is_ascii_digit())
        && words[3].chars().all(|ch| ch.is_ascii_digit())
    {
        Some(words[..4].join(" "))
    } else {
        None
    };

    Some((version_token, date_hint))
}

fn parse_debian_signature_line(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim();
    if !trimmed.starts_with("--") {
        return None;
    }

    let body = trimmed
        .trim_start_matches('-')
        .trim_start_matches('-')
        .trim();
    let (author, date) = body.split_once("  ")?;
    let author = author.trim();
    let date = date.trim();
    if author.is_empty() || date.is_empty() {
        return None;
    }

    Some((author.to_string(), date.to_string()))
}

#[derive(Debug, Clone)]
struct VersionSection {
    start_line: usize,
    version: String,
}

fn changelog_render_plan(
    content: &str,
    package: Option<&Package>,
    diff_only: bool,
) -> (String, String, Style) {
    if !diff_only {
        return (
            content.to_string(),
            "Mode: full history [v toggles]".to_string(),
            dim(),
        );
    }

    let Some(package) = package else {
        return (
            content.to_string(),
            "Mode: delta fallback (package unavailable)".to_string(),
            warning(),
        );
    };
    let Some(target_version) = package.available_version.as_deref() else {
        return (
            content.to_string(),
            "Mode: delta unavailable for this package".to_string(),
            warning(),
        );
    };
    let current_version = package.version.as_str();
    if normalize_version_token(current_version) == normalize_version_token(target_version) {
        return (
            content.to_string(),
            format!(
                "Mode: delta unavailable ({} already current)",
                target_version
            ),
            warning(),
        );
    }

    match slice_version_delta_content(content, current_version, target_version) {
        Some(focused) if !focused.trim().is_empty() => (
            focused,
            format!(
                "Mode: delta {} -> {} [v toggles]",
                current_version, target_version
            ),
            accent(),
        ),
        Some(_) => (
            format!(
                "## Version delta\n\nNo changelog entries found between {} and {}.",
                current_version, target_version
            ),
            format!(
                "Mode: delta {} -> {} [v toggles]",
                current_version, target_version
            ),
            accent(),
        ),
        None => (
            content.to_string(),
            format!(
                "Mode: delta fallback (could not map {} -> {})",
                current_version, target_version
            ),
            warning(),
        ),
    }
}

fn slice_version_delta_content(content: &str, current: &str, target: &str) -> Option<String> {
    let lines: Vec<&str> = content.lines().collect();
    if lines.is_empty() {
        return None;
    }

    let sections = extract_version_sections(&lines);
    if sections.is_empty() {
        return None;
    }

    let target_index = find_section_for_version(&sections, target);
    let current_index = find_section_for_version(&sections, current);

    let (start_idx, end_idx_exclusive) = match (target_index, current_index) {
        (Some(target_idx), Some(current_idx)) if target_idx == current_idx => {
            return Some(String::new());
        }
        (Some(target_idx), Some(current_idx)) if target_idx < current_idx => {
            (target_idx, current_idx)
        }
        (Some(target_idx), Some(current_idx)) => (current_idx + 1, target_idx + 1),
        (Some(target_idx), None) => (target_idx, (target_idx + 1).min(sections.len())),
        (None, Some(current_idx)) => {
            if current_idx == 0 {
                return Some(String::new());
            }
            (0, current_idx)
        }
        (None, None) => return None,
    };

    if start_idx >= sections.len() || start_idx >= end_idx_exclusive {
        return Some(String::new());
    }

    let start_line = sections[start_idx].start_line;
    let end_line = if end_idx_exclusive < sections.len() {
        sections[end_idx_exclusive].start_line
    } else {
        lines.len()
    };
    Some(lines[start_line..end_line].join("\n"))
}

fn extract_version_sections(lines: &[&str]) -> Vec<VersionSection> {
    lines
        .iter()
        .enumerate()
        .filter_map(|(idx, line)| {
            extract_version_heading(line).map(|version| VersionSection {
                start_line: idx,
                version,
            })
        })
        .collect()
}

fn find_section_for_version(sections: &[VersionSection], version: &str) -> Option<usize> {
    let keys = version_match_keys(version);
    sections.iter().position(|section| {
        keys.iter().any(|key| {
            section.version == *key
                || section.version.starts_with(key)
                || key.starts_with(&section.version)
        })
    })
}

fn version_match_keys(version: &str) -> Vec<String> {
    let mut keys = Vec::new();
    let normalized = normalize_version_token(version);
    push_unique_version_key(&mut keys, normalized.clone());

    if let Some((_, rest)) = normalized.split_once(':') {
        push_unique_version_key(&mut keys, rest.to_string());
    }

    for seed in keys.clone() {
        if let Some((prefix, _)) = seed.split_once('-') {
            push_unique_version_key(&mut keys, prefix.to_string());
        }
        if let Some((prefix, _)) = seed.split_once('+') {
            push_unique_version_key(&mut keys, prefix.to_string());
        }
        if let Some((prefix, _)) = seed.split_once('~') {
            push_unique_version_key(&mut keys, prefix.to_string());
        }
    }

    keys
}

fn push_unique_version_key(keys: &mut Vec<String>, key: String) {
    if !key.is_empty() && !keys.contains(&key) {
        keys.push(key);
    }
}

fn extract_version_heading(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }

    if trimmed.starts_with('#') {
        let heading = trimmed.trim_start_matches('#').trim();
        for token in heading.split_whitespace().take(6) {
            let candidate = sanitize_version_token(token);
            if looks_like_version_token(&candidate) {
                return Some(normalize_version_token(&candidate));
            }
        }
        return None;
    }

    if let Some(open) = trimmed.find('(') {
        if open > 0 && open < 48 {
            let after = &trimmed[open + 1..];
            if let Some(close) = after.find(')') {
                let candidate = sanitize_version_token(&after[..close]);
                if looks_like_version_token(&candidate) {
                    return Some(normalize_version_token(&candidate));
                }
            }
        }
    }

    if !trimmed.contains(' ') {
        let candidate = sanitize_version_token(trimmed);
        if looks_like_version_token(&candidate) {
            return Some(normalize_version_token(&candidate));
        }
    }

    None
}

fn sanitize_version_token(token: &str) -> String {
    token
        .trim_matches(|c: char| {
            c.is_whitespace()
                || matches!(
                    c,
                    '*' | '_' | '`' | '~' | '(' | ')' | '[' | ']' | '{' | '}' | ',' | ';' | ':'
                )
        })
        .to_string()
}

fn normalize_version_token(token: &str) -> String {
    let mut value = sanitize_version_token(token);
    if (value.starts_with('v') || value.starts_with('V'))
        && value.chars().nth(1).is_some_and(|ch| ch.is_ascii_digit())
    {
        value.remove(0);
    }

    while value
        .chars()
        .last()
        .is_some_and(|ch| !ch.is_ascii_alphanumeric())
    {
        value.pop();
    }

    value.to_lowercase()
}

fn looks_like_version_token(token: &str) -> bool {
    let candidate = sanitize_version_token(token);
    if candidate.len() < 2 || candidate.len() > 64 {
        return false;
    }
    if !candidate.chars().any(|ch| ch.is_ascii_digit()) {
        return false;
    }
    let Some(first) = candidate.chars().next() else {
        return false;
    };
    if !(first.is_ascii_digit() || first == 'v' || first == 'V') {
        return false;
    }
    candidate
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ".:_+-~".contains(ch))
}

fn format_changelog_content(content: &str, max_width: usize) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let width = max_width.max(1);
    let mut in_code_block = false;

    for raw in content.lines() {
        let line = raw.trim_end();
        let trimmed = line.trim();

        if trimmed.starts_with("```") {
            in_code_block = !in_code_block;
            continue;
        }
        if trimmed.is_empty() {
            lines.push(Line::from(""));
            continue;
        }

        if let Some((package, version, channel)) = parse_apt_entry_header(trimmed) {
            let mut spans = vec![
                Span::styled("▸ ", accent()),
                Span::styled(format!("{} {}", package, version), accent()),
            ];
            if let Some(channel) = channel {
                spans.push(Span::styled(format!("  {}", channel), dim()));
            }
            lines.push(Line::from(spans));
            continue;
        }
        if let Some((version, date_hint)) = parse_dnf_entry_header(trimmed) {
            let mut spans = vec![
                Span::styled("▸ ", accent()),
                Span::styled(version, accent()),
            ];
            if let Some(date_hint) = date_hint {
                spans.push(Span::styled(format!("  {}", date_hint), dim()));
            }
            lines.push(Line::from(spans));
            continue;
        }
        if let Some((author, date)) = parse_debian_signature_line(trimmed) {
            push_wrapped_styled_line(&mut lines, &format!("{}  {}", author, date), width, dim());
            continue;
        }

        if let Some(title) = trimmed.strip_prefix("# ") {
            push_wrapped_styled_line(&mut lines, title, width, section_header());
            continue;
        }
        if let Some(title) = trimmed.strip_prefix("## ") {
            push_wrapped_styled_line(&mut lines, title, width, section_header());
            continue;
        }
        if let Some(title) = trimmed.strip_prefix("### ") {
            push_wrapped_styled_line(&mut lines, &format!("▸ {}", title), width, accent());
            continue;
        }
        if trimmed == "---" || trimmed.chars().all(|ch| ch == '-') {
            lines.push(Line::from(Span::styled("─".repeat(width.min(48)), dim())));
            continue;
        }
        if let Some((key, value, is_url)) = parse_metadata_pair(trimmed) {
            let style = if is_url {
                accent()
            } else if in_code_block {
                muted()
            } else {
                text()
            };
            push_wrapped_styled_line(&mut lines, &format!("{}: {}", key, value), width, style);
            continue;
        }
        if let Some((label, url)) = parse_markdown_link(trimmed) {
            push_wrapped_styled_line(&mut lines, &format!("{}: {}", label, url), width, accent());
            continue;
        }
        if let Some(italic) = trimmed.strip_prefix('*').and_then(|s| s.strip_suffix('*')) {
            push_wrapped_styled_line(&mut lines, italic.trim(), width, dim());
            continue;
        }
        if let Some(item) = trimmed
            .strip_prefix("- ")
            .or_else(|| trimmed.strip_prefix("* "))
            .or_else(|| trimmed.strip_prefix("• "))
        {
            let wrapped = wrap_text(item, width.saturating_sub(2).max(1));
            for (idx, segment) in wrapped.into_iter().enumerate() {
                let prefix = if idx == 0 { "• " } else { "  " };
                lines.push(Line::from(Span::styled(
                    format!("{}{}", prefix, segment),
                    muted(),
                )));
            }
            continue;
        }

        push_wrapped_styled_line(
            &mut lines,
            trimmed,
            width,
            if in_code_block { muted() } else { text() },
        );
    }

    lines
}

pub fn wrap_text(text_value: &str, max_width: usize) -> Vec<String> {
    if text_value.trim().is_empty() {
        return vec!["No description available".to_string()];
    }
    if max_width == 0 {
        return vec![text_value.to_string()];
    }

    let mut lines = Vec::new();
    let mut current = String::new();
    let mut current_width = 0usize;

    for word in text_value.split_whitespace() {
        let word_width = UnicodeWidthStr::width(word);

        if current.is_empty() {
            if word_width <= max_width {
                current.push_str(word);
                current_width = word_width;
            } else {
                lines.push(truncate_to_width(word, max_width));
            }
            continue;
        }

        if current_width + 1 + word_width <= max_width {
            current.push(' ');
            current.push_str(word);
            current_width += 1 + word_width;
        } else {
            lines.push(current);
            if word_width <= max_width {
                current = word.to_string();
                current_width = word_width;
            } else {
                lines.push(truncate_to_width(word, max_width));
                current = String::new();
                current_width = 0;
            }
        }
    }

    if !current.is_empty() {
        lines.push(current);
    }

    lines
}

#[cfg(test)]
mod tests {
    use super::super::app::App;
    use super::super::components::layout::sources_panel_width;
    use super::super::format::truncate_middle_to_width;
    use super::*;
    use crate::backend::PackageManager;
    use crate::models::history::TaskQueueAction;
    use crate::models::PackageSource;
    use chrono::{Duration, Local};
    use std::collections::VecDeque;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    #[test]
    fn truncate_middle_preserves_edges() {
        let truncated = truncate_middle_to_width("super-long-package-name", 12);
        assert_eq!(truncated, "super-…-name");
    }

    #[test]
    fn truncate_middle_handles_small_width() {
        assert_eq!(truncate_middle_to_width("abcdef", 1), "…");
        assert_eq!(truncate_middle_to_width("abcdef", 2), "a…");
    }

    fn make_package(name: &str, source: PackageSource, status: PackageStatus) -> Package {
        Package {
            name: name.to_string(),
            version: "1.0.0".to_string(),
            available_version: (status == PackageStatus::UpdateAvailable)
                .then(|| "1.1.0".to_string()),
            description: format!("{} package", name),
            source,
            status,
            size: None,
            homepage: None,
            license: None,
            maintainer: None,
            dependencies: Vec::new(),
            install_date: None,
            update_category: None,
            enrichment: None,
        }
    }

    #[test]
    fn decision_card_renders_must_have_fields() {
        let mut lines = Vec::new();
        append_decision_card(
            &mut lines,
            80,
            DecisionCardContent {
                what_happens: "Update 3 packages from 5 selected.".to_string(),
                certainty: "Estimated; dependency impact resolved at execution.".to_string(),
                certainty_style: warning(),
                risk: "Caution".to_string(),
                risk_style: warning(),
                privileges: "Likely elevated privileges prompt.".to_string(),
                privileges_style: warning(),
                if_blocked: "Failures surface as E_* code with playbook guidance.".to_string(),
                primary_action: "Press [y] to queue updates.".to_string(),
                primary_style: footer_label(),
            },
        );
        let rendered: Vec<String> = lines.iter().map(|line| line.to_string()).collect();

        assert!(rendered.iter().any(|line| line.contains("What:")));
        assert!(rendered.iter().any(|line| line.contains("Certainty:")));
        assert!(rendered.iter().any(|line| line.contains("Risk:")));
        assert!(rendered.iter().any(|line| line.contains("Privileges:")));
        assert!(rendered.iter().any(|line| line.contains("If blocked:")));
        assert!(rendered.iter().any(|line| line.contains("Action:")));
    }

    #[test]
    fn changelog_decision_card_prefers_update_action_for_updatable_package() {
        let pkg = make_package("pkg", PackageSource::Apt, PackageStatus::UpdateAvailable);
        let card = changelog_decision_card_content(
            &pkg,
            Some(&ChangelogState::Ready {
                content: "notes".to_string(),
                summary: crate::models::ChangelogSummary::parse("notes"),
            }),
            true,
            true,
        );

        assert!(card.primary_action.contains("[u]"));
        assert!(card.what_happens.contains("version delta"));
    }

    #[test]
    fn changelog_decision_card_falls_back_to_close_when_source_unsupported() {
        let pkg = make_package("pkg", PackageSource::Snap, PackageStatus::Installed);
        let card =
            changelog_decision_card_content(&pkg, Some(&ChangelogState::Empty), false, false);

        assert!(card.primary_action.contains("[c]/Esc"));
        assert!(card.if_blocked.contains("unsupported"));
    }

    #[test]
    fn source_count_label_snapshots() {
        assert_eq!(
            source_count_label(Filter::All, [42, 31, 3, 9, 1, 4]),
            " 42 (+3)"
        );
        assert_eq!(source_count_label(Filter::All, [42, 31, 0, 9, 0, 0]), " 42");
        assert_eq!(
            source_count_label(Filter::Installed, [42, 31, 3, 9, 1, 4]),
            " 31"
        );
        assert_eq!(
            source_count_label(Filter::Updates, [42, 31, 3, 9, 1, 4]),
            " 3"
        );
        assert_eq!(
            source_count_label(Filter::Favorites, [42, 31, 3, 9, 1, 4]),
            " 9"
        );
        assert_eq!(
            source_count_label(Filter::SecurityUpdates, [42, 31, 3, 9, 1, 4]),
            " 1"
        );
        assert_eq!(
            source_count_label(Filter::Duplicates, [42, 31, 3, 9, 1, 4]),
            " 4"
        );
    }

    #[test]
    fn sources_panel_width_defaults_to_minimum() {
        let app = App::new(
            Arc::new(Mutex::new(PackageManager::new())),
            Arc::new(Mutex::new(None)),
            None,
            None,
        );
        assert_eq!(sources_panel_width(&app, 120), 16);
    }

    #[test]
    fn sources_panel_width_expands_for_long_source_counts() {
        let mut app = App::new(
            Arc::new(Mutex::new(PackageManager::new())),
            Arc::new(Mutex::new(None)),
            None,
            None,
        );
        app.filter = Filter::All;
        app.available_sources = vec![PackageSource::Flatpak];
        app.source_counts
            .insert(PackageSource::Flatpak, [243, 231, 12, 0, 0, 0]);
        app.filter_counts = [243, 231, 12, 0, 0, 0];

        assert!(sources_panel_width(&app, 120) > 16);
    }

    #[test]
    fn sources_panel_width_respects_available_space() {
        let mut app = App::new(
            Arc::new(Mutex::new(PackageManager::new())),
            Arc::new(Mutex::new(None)),
            None,
            None,
        );
        app.filter = Filter::All;
        app.available_sources = vec![PackageSource::Flatpak];
        app.source_counts
            .insert(PackageSource::Flatpak, [243, 231, 12, 0, 0, 0]);
        app.filter_counts = [243, 231, 12, 0, 0, 0];

        assert_eq!(sources_panel_width(&app, 20), 19);
    }

    #[test]
    fn queue_running_label_snapshot() {
        let line = build_running_queue_label(RunningQueueLabelArgs {
            spinner: '◐',
            phase_label: "download",
            active_label: "Update vim",
            done: 2,
            total: 8,
            remaining: 5,
            queued: 4,
            failed: 1,
        });
        assert_eq!(
            line,
            "◐ download · Update vim · 2/8 done · 4 queued · 1 failed · 5 left"
        );
    }

    #[test]
    fn queue_idle_label_snapshots() {
        let (queued_line, queued_state) =
            build_idle_queue_label(4, 0, 0, 0, 0, 4, Some("2.0 t/m · ETA 2m00s"));
        assert_eq!(queued_state, QueueBarState::Queued);
        assert_eq!(queued_line, "◻ 4 queued · 2.0 t/m · ETA 2m00s");

        let (failed_line, failed_state) =
            build_idle_queue_label(0, 2, 1, 1, 4, 5, Some("1.0 t/m · ETA 1m00s"));
        assert_eq!(failed_state, QueueBarState::Failed);
        assert_eq!(failed_line, "⚠ 3 done, 1 failed · 1.0 t/m · ETA 1m00s");

        let (done_line, done_state) = build_idle_queue_label(0, 3, 0, 0, 3, 3, None);
        assert_eq!(done_state, QueueBarState::Complete);
        assert_eq!(done_line, "✓ 3/3 complete");

        let (mixed_line, mixed_state) =
            build_idle_queue_label(2, 2, 0, 0, 2, 4, Some("2.0 t/m · ETA 1m00s"));
        assert_eq!(mixed_state, QueueBarState::Queued);
        assert_eq!(mixed_line, "◻ 2 queued · 2/4 done · 2.0 t/m · ETA 1m00s");
    }

    #[test]
    fn queue_performance_hint_snapshot() {
        let now = Local::now();
        let make_completed = |id: &str, started_secs_ago: i64, duration_secs: i64| TaskQueueEntry {
            id: id.to_string(),
            action: TaskQueueAction::Update,
            package_id: id.to_string(),
            package_name: id.to_string(),
            package_source: PackageSource::Apt,
            status: TaskQueueStatus::Completed,
            queued_at: now - Duration::seconds(started_secs_ago + 1),
            started_at: Some(now - Duration::seconds(started_secs_ago)),
            completed_at: Some(now - Duration::seconds(started_secs_ago - duration_secs)),
            error: None,
        };

        let tasks = vec![
            make_completed("a", 120, 30),
            make_completed("b", 60, 30),
            make_completed("c", 30, 30),
        ];
        assert_eq!(
            queue_performance_hint(&tasks, 3.0, Some(QueueEtaConfidence::Estimated)).as_deref(),
            Some("2.0 t/m • ETA 1m30s (estimated)")
        );
        assert_eq!(
            queue_performance_hint(&tasks, 0.0, Some(QueueEtaConfidence::Verified)).as_deref(),
            Some("2.0 t/m (verified)")
        );
    }

    #[test]
    fn queue_performance_hint_requires_sample_size() {
        let app = App::new(
            Arc::new(Mutex::new(PackageManager::new())),
            Arc::new(Mutex::new(None)),
            None,
            None,
        );
        assert_eq!(queue_performance_hint(&app.tasks, 4.0, None), None);
    }

    #[test]
    fn queue_eta_confidence_reflects_sample_quality_and_signal() {
        let now = Local::now();
        let make_completed = |id: &str, started_secs_ago: i64, duration_secs: i64| TaskQueueEntry {
            id: id.to_string(),
            action: TaskQueueAction::Update,
            package_id: id.to_string(),
            package_name: id.to_string(),
            package_source: PackageSource::Apt,
            status: TaskQueueStatus::Completed,
            queued_at: now - Duration::seconds(started_secs_ago + 1),
            started_at: Some(now - Duration::seconds(started_secs_ago)),
            completed_at: Some(now - Duration::seconds(started_secs_ago - duration_secs)),
            error: None,
        };

        let two_samples = vec![make_completed("a", 120, 30), make_completed("b", 60, 30)];
        let five_samples = vec![
            make_completed("a", 300, 30),
            make_completed("b", 240, 30),
            make_completed("c", 180, 30),
            make_completed("d", 120, 30),
            make_completed("e", 60, 30),
        ];

        assert_eq!(
            queue_eta_confidence(&two_samples, None),
            Some(QueueEtaConfidence::Estimated)
        );
        assert_eq!(
            queue_eta_confidence(&five_samples, None),
            Some(QueueEtaConfidence::Verified)
        );
        assert_eq!(
            queue_eta_confidence(&five_samples, Some("quiet: no output 1m10s")),
            Some(QueueEtaConfidence::Estimated)
        );
    }

    #[test]
    fn running_task_phase_tracks_log_progression() {
        let now = Local::now();
        let mut app = App::new(
            Arc::new(Mutex::new(PackageManager::new())),
            Arc::new(Mutex::new(None)),
            None,
            None,
        );
        let running = TaskQueueEntry {
            id: "run".to_string(),
            action: TaskQueueAction::Update,
            package_id: "APT:run".to_string(),
            package_name: "run".to_string(),
            package_source: PackageSource::Apt,
            status: TaskQueueStatus::Running,
            queued_at: now - Duration::seconds(60),
            started_at: Some(now - Duration::seconds(55)),
            completed_at: None,
            error: None,
        };
        app.tasks = vec![running.clone()];

        app.task_logs.insert(
            running.id.clone(),
            VecDeque::from(vec![
                "[OUT] Reading package lists...".to_string(),
                "[OUT] Building dependency tree...".to_string(),
            ]),
        );
        assert_eq!(
            running_task_phase(&app, &running),
            RunningTaskPhase::Resolve
        );

        app.task_logs.insert(
            running.id.clone(),
            VecDeque::from(vec![
                "[OUT] Reading package lists...".to_string(),
                "[OUT] Get:1 http://mirror/pool pkg 2.3 MB".to_string(),
            ]),
        );
        assert_eq!(
            running_task_phase(&app, &running),
            RunningTaskPhase::Download
        );

        app.task_logs.insert(
            running.id.clone(),
            VecDeque::from(vec![
                "[OUT] Get:1 http://mirror/pool pkg 2.3 MB".to_string(),
                "[OUT] Setting up pkg (1.2.3-1) ...".to_string(),
            ]),
        );
        assert_eq!(running_task_phase(&app, &running), RunningTaskPhase::Apply);
    }

    #[test]
    fn preflight_forecast_text_reflects_touched_packages_and_conflicts() {
        let preflight = PreflightSummary {
            action: TaskQueueAction::Update,
            target_count: 3,
            executable_count: 3,
            skipped_count: 0,
            source_breakdown: vec![(PackageSource::Apt, 3)],
            risk_level: PreflightRiskLevel::Caution,
            risk_reasons: vec!["Package manager reported lock/dependency conflicts.".to_string()],
            certainty: PreflightCertainty::Verified,
            elevated_privileges_likely: true,
            dependency_impact_known: true,
            dependency_impact: Some(super::super::app::PreflightDependencyImpact {
                install_count: 1,
                upgrade_count: 4,
                remove_count: 0,
                held_back_count: 2,
            }),
            verification_in_progress: false,
            selection_mode: false,
        };

        let (forecast, attention) = preflight_forecast_text(&preflight);
        assert!(attention);
        assert!(forecast.contains("~7 packages touched"));
        assert!(forecast.contains("possible (2 held back)"));
        assert!(forecast.contains("verified"));
    }

    #[test]
    fn preflight_forecast_text_falls_back_to_selected_targets_when_unverified() {
        let preflight = PreflightSummary {
            action: TaskQueueAction::Install,
            target_count: 2,
            executable_count: 2,
            skipped_count: 0,
            source_breakdown: vec![(PackageSource::Pip, 2)],
            risk_level: PreflightRiskLevel::Safe,
            risk_reasons: vec!["No additional guardrails triggered.".to_string()],
            certainty: PreflightCertainty::Estimated,
            elevated_privileges_likely: false,
            dependency_impact_known: false,
            dependency_impact: None,
            verification_in_progress: false,
            selection_mode: true,
        };

        let (forecast, attention) = preflight_forecast_text(&preflight);
        assert!(!attention);
        assert!(forecast.contains("~2 packages touched"));
        assert!(forecast.contains("best-effort estimate"));
        assert!(forecast.contains("estimated"));
    }

    #[test]
    fn running_task_signal_thresholds() {
        assert_eq!(
            running_task_signal_from(80, None).as_deref(),
            Some("awaiting output 1m20s")
        );
        assert_eq!(
            running_task_signal_from(90, Some(70)).as_deref(),
            Some("quiet: no output 1m10s")
        );
        assert_eq!(
            running_task_signal_from(220, Some(170)).as_deref(),
            Some("stalled: no output 2m50s")
        );
        assert_eq!(running_task_signal_from(20, Some(10)), None);
    }

    #[test]
    fn running_task_eta_from_history_detects_overrun() {
        let now = Local::now();
        let running = TaskQueueEntry {
            id: "run".to_string(),
            action: TaskQueueAction::Update,
            package_id: "run".to_string(),
            package_name: "run".to_string(),
            package_source: PackageSource::Apt,
            status: TaskQueueStatus::Running,
            queued_at: now - Duration::seconds(50),
            started_at: Some(now - Duration::seconds(50)),
            completed_at: None,
            error: None,
        };

        let completed = |id: &str| TaskQueueEntry {
            id: id.to_string(),
            action: TaskQueueAction::Update,
            package_id: id.to_string(),
            package_name: id.to_string(),
            package_source: PackageSource::Apt,
            status: TaskQueueStatus::Completed,
            queued_at: now - Duration::seconds(31),
            started_at: Some(now - Duration::seconds(30)),
            completed_at: Some(now),
            error: None,
        };

        let tasks = vec![completed("a"), completed("b"), running];
        let hint = running_task_eta_hint(tasks.last().unwrap(), &tasks)
            .expect("eta/overrun hint should be available");
        assert!(
            hint.starts_with("overrun +"),
            "expected overrun hint, got {hint}"
        );
    }

    #[test]
    fn slice_version_delta_content_extracts_target_window() {
        let raw = r#"
# demo
## Version History
### v3.0.0
- breaking changes
### v2.5.0
- target release
### v2.0.0
- intermediate release
### v1.0.0
- installed baseline
"#;

        let slice = slice_version_delta_content(raw, "1.0.0", "2.5.0")
            .expect("version delta slice should exist");
        assert!(slice.contains("v2.5.0"));
        assert!(slice.contains("v2.0.0"));
        assert!(!slice.contains("v3.0.0"));
        assert!(!slice.contains("v1.0.0"));
    }

    #[test]
    fn version_heading_extraction_handles_markdown_and_apt_lines() {
        assert_eq!(
            extract_version_heading("### ~~v2.4.1~~ (Yanked)"),
            Some("2.4.1".to_string())
        );
        assert_eq!(
            extract_version_heading("mypkg (1:2.3.4-1ubuntu1) unstable; urgency=medium"),
            Some("1:2.3.4-1ubuntu1".to_string())
        );
    }

    #[test]
    fn metadata_parser_handles_markdown_and_plain_pairs() {
        assert_eq!(
            parse_metadata_pair("**Repository:** https://example.com/project"),
            Some((
                "Repository".to_string(),
                "https://example.com/project".to_string(),
                true
            ))
        );
        assert_eq!(
            parse_metadata_pair("License: MIT"),
            Some(("License".to_string(), "MIT".to_string(), false))
        );
        assert_eq!(parse_metadata_pair("RandomField: value"), None);
    }

    #[test]
    fn source_header_parsers_handle_apt_and_dnf() {
        assert_eq!(
            parse_apt_entry_header("mypkg (1:2.3.4-1ubuntu1) unstable; urgency=medium"),
            Some((
                "mypkg".to_string(),
                "1:2.3.4-1ubuntu1".to_string(),
                Some("unstable".to_string())
            ))
        );
        assert_eq!(
            parse_dnf_entry_header("* Wed Jan 10 2024 Jane Dev <j@x> - 2.4.1-1.fc40"),
            Some((
                "2.4.1-1.fc40".to_string(),
                Some("Wed Jan 10 2024".to_string())
            ))
        );
        assert_eq!(
            parse_debian_signature_line("-- Jane Dev <j@x>  Tue, 09 Jan 2024 12:00:00 +0000"),
            Some((
                "Jane Dev <j@x>".to_string(),
                "Tue, 09 Jan 2024 12:00:00 +0000".to_string()
            ))
        );
    }

    #[test]
    fn changelog_formatter_polishes_source_specific_blocks() {
        let raw = r#"```txt
mypkg (1:2.3.4-1ubuntu1) unstable; urgency=medium
  * Fix crash when loading profile
 -- Jane Dev <j@x>  Tue, 09 Jan 2024 12:00:00 +0000
* Wed Jan 10 2024 Jane Dev <j@x> - 2.4.1-1.fc40
- Resolve dependency lock issue
Repository: https://example.com/project
```"#;

        let lines = format_changelog_content(raw, 80);
        let rendered: Vec<String> = lines.iter().map(|line| line.to_string()).collect();

        assert!(
            rendered
                .iter()
                .any(|line| line.contains("▸ mypkg 1:2.3.4-1ubuntu1")),
            "apt entry header should be normalized"
        );
        assert!(
            rendered.iter().any(|line| line.contains("▸ 2.4.1-1.fc40")),
            "dnf entry header should be normalized"
        );
        assert!(
            rendered
                .iter()
                .any(|line| line.contains("Fix crash when loading profile")),
            "apt bullet items should remain readable"
        );
        assert!(
            rendered
                .iter()
                .any(|line| line.contains("Repository: https://example.com/project")),
            "plain metadata should be normalized"
        );
    }

    #[test]
    fn parse_markdown_helpers_handle_common_patterns() {
        assert_eq!(
            parse_markdown_bold_pair("**Repository:** https://example.com"),
            Some(("Repository".to_string(), "https://example.com".to_string()))
        );
        assert_eq!(
            parse_markdown_link("[View on npm](https://npmjs.com/pkg)"),
            Some((
                "View on npm".to_string(),
                "https://npmjs.com/pkg".to_string()
            ))
        );
    }

    #[test]
    fn changelog_formatter_snapshot() {
        let raw = r#"# demo

## Version History
### v2.0.0 (Latest)
*Released: 2026-02-01*
- Fixed startup crash
- Added safe queue retry
**Repository:** https://example.com/demo
[View on npm](https://npmjs.com/package/demo)
```txt
literal output
```
"#;

        let lines = format_changelog_content(raw, 42);
        let rendered: Vec<String> = lines.iter().map(|line| line.to_string()).collect();
        assert!(rendered.iter().any(|line| line.contains("demo")));
        assert!(rendered.iter().any(|line| line.contains("Version History")));
        assert!(rendered.iter().any(|line| line.contains("v2.0.0")));
        assert!(rendered.iter().any(|line| line.contains("Repository:")));
        assert!(rendered
            .iter()
            .any(|line| line.contains("example.com/demo")));
        assert!(rendered.iter().any(|line| line.contains("View on npm:")));
        assert!(rendered
            .iter()
            .any(|line| line.contains("npmjs.com/package/demo")));
        assert!(rendered.iter().any(|line| line.contains("literal output")));
    }
}
