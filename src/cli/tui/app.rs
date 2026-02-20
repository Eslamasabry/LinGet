mod input;
use super::ui;
use super::update_center;
use crate::backend::{HistoryTracker, PackageManager, TaskQueueEvent, TaskQueueExecutor};
use crate::cli::tui::components::layout::{compute_layout, LayoutRegions};
use crate::cli::tui::state::filters::{Filter, Focus};
use crate::cli::tui::state::queue::{
    ClinicRemediationPlan, FailureCategory, QueueClinicActionability, QueueFailureFilter,
    QueueJourneyLane, RecoveryState,
};
use crate::models::history::{TaskQueueAction, TaskQueueEntry, TaskQueueStatus};
use crate::models::{ChangelogSummary, Config, Package, PackageSource, PackageStatus};
use anyhow::{Context, Result};
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind,
        KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, layout::Rect, Terminal};
use std::collections::{HashMap, HashSet, VecDeque};
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::process::Command;
use tokio::sync::{mpsc, Mutex};
use tracing::{debug, error};

const COMPACT_WIDTH: u16 = 90;
const COMPACT_HEIGHT: u16 = 24;
pub const MIN_WIDTH: u16 = 60;
pub const MIN_HEIGHT: u16 = 15;
const HALF_PAGE: usize = 10;
const MAX_TASK_LOG_LINES: usize = 500;
const FILTER_ALL_INDEX: usize = 0;
const FILTER_INSTALLED_INDEX: usize = 1;
const FILTER_UPDATES_INDEX: usize = 2;
const FILTER_FAVORITES_INDEX: usize = 3;
const QUEUE_AUTO_HIDE_AFTER: Duration = Duration::from_secs(10);

fn rect_contains(rect: Rect, pos: (u16, u16)) -> bool {
    rect.width > 0
        && rect.height > 0
        && pos.0 >= rect.x
        && pos.0 < rect.x + rect.width
        && pos.1 >= rect.y
        && pos.1 < rect.y + rect.height
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreflightRiskLevel {
    Safe,
    Caution,
    High,
}

impl PreflightRiskLevel {
    pub fn label(self) -> &'static str {
        match self {
            Self::Safe => "Safe",
            Self::Caution => "Caution",
            Self::High => "High Risk",
        }
    }

    pub fn copy(self) -> &'static str {
        match self {
            Self::Safe => "No major risk signals detected for this queue operation.",
            Self::Caution => "Review target scope and source mix before queueing.",
            Self::High => {
                "This operation is potentially destructive. Confirm only if this is intentional."
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreflightCertainty {
    Estimated,
    Verified,
}

impl PreflightCertainty {
    pub fn label(self) -> &'static str {
        match self {
            Self::Estimated => "Estimated",
            Self::Verified => "Verified",
        }
    }

    pub fn copy(self) -> &'static str {
        match self {
            Self::Estimated => {
                "Best-effort preview from current package state. Exact dependency resolution happens at execution time."
            }
            Self::Verified => "Preview was verified with source dependency impact probes.",
        }
    }
}

#[derive(Debug, Clone)]
pub struct PreflightSummary {
    pub action: TaskQueueAction,
    pub target_count: usize,
    pub executable_count: usize,
    pub skipped_count: usize,
    pub source_breakdown: Vec<(PackageSource, usize)>,
    pub risk_level: PreflightRiskLevel,
    pub risk_reasons: Vec<String>,
    pub certainty: PreflightCertainty,
    pub elevated_privileges_likely: bool,
    pub dependency_impact_known: bool,
    pub dependency_impact: Option<PreflightDependencyImpact>,
    pub verification_in_progress: bool,
    pub selection_mode: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PreflightDependencyImpact {
    pub install_count: usize,
    pub upgrade_count: usize,
    pub remove_count: usize,
    pub held_back_count: usize,
}

impl PreflightDependencyImpact {
    fn merge(&mut self, other: &Self) {
        self.install_count += other.install_count;
        self.upgrade_count += other.upgrade_count;
        self.remove_count += other.remove_count;
        self.held_back_count += other.held_back_count;
    }

    fn has_changes(&self) -> bool {
        self.install_count > 0
            || self.upgrade_count > 0
            || self.remove_count > 0
            || self.held_back_count > 0
    }

    pub fn summary(&self) -> String {
        let mut parts = Vec::new();
        if self.install_count > 0 {
            parts.push(format!(
                "{} install{}",
                self.install_count,
                if self.install_count == 1 { "" } else { "s" }
            ));
        }
        if self.upgrade_count > 0 {
            parts.push(format!(
                "{} upgrade{}",
                self.upgrade_count,
                if self.upgrade_count == 1 { "" } else { "s" }
            ));
        }
        if self.remove_count > 0 {
            parts.push(format!(
                "{} removal{}",
                self.remove_count,
                if self.remove_count == 1 { "" } else { "s" }
            ));
        }
        if self.held_back_count > 0 {
            parts.push(format!("{} held back", self.held_back_count));
        }

        if parts.is_empty() {
            "No transaction delta detected in source probe.".to_string()
        } else {
            parts.join(", ")
        }
    }
}

#[derive(Debug, Clone)]
pub struct PendingAction {
    pub label: String,
    pub packages: Vec<Package>,
    pub action: TaskQueueAction,
    pub preflight: PreflightSummary,
    pub risk_acknowledged: bool,
}

type LoadResult = Result<Vec<Package>, String>;

#[derive(Debug, Clone)]
pub enum ChangelogState {
    Loading,
    Ready {
        content: String,
        summary: ChangelogSummary,
    },
    Empty,
    Error(String),
}

#[derive(Debug, Clone)]
struct ChangelogResult {
    package_id: String,
    package_name: String,
    result: Result<Option<String>, String>,
}

#[derive(Debug, Clone)]
struct PreflightVerificationResult {
    request_id: u64,
    action: TaskQueueAction,
    package_ids: Vec<String>,
    dependency_impact_known: bool,
    dependency_impact: Option<PreflightDependencyImpact>,
    note: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CommandId {
    Quit,
    ShowHelp,
    OpenPalette,
    CycleFocus,
    MoveUp,
    MoveDown,
    MoveTop,
    MoveBottom,
    PageUp,
    PageDown,
    FilterAll,
    FilterInstalled,
    FilterUpdates,
    FilterFavorites,
    ToggleFavorite,
    BulkToggleFavorite,
    ToggleFavoritesUpdatesOnly,
    ToggleSelection,
    SelectAll,
    Install,
    Remove,
    Update,
    RunRecommended,
    ViewChangelog,
    Search,
    Refresh,
    ToggleQueue,
    QueueCancel,
    QueueRetry,
    QueueRetrySafe,
    QueueRemediate,
    QueueLogOlder,
    QueueLogNewer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RecommendedAction {
    RetrySafeFailures(usize),
    ReviewFailures(usize),
    QueueAllUpdates(usize),
    ReviewQueue,
    RefreshPackages,
}

#[derive(Clone, Copy)]
pub struct CommandDefinition {
    pub id: CommandId,
    pub label: &'static str,
    pub shortcut: &'static str,
    pub enabled: fn(&App) -> bool,
}

impl CommandDefinition {
    pub fn is_enabled(&self, app: &App) -> bool {
        (self.enabled)(app)
    }
}

#[derive(Debug, Clone)]
pub struct PaletteCommandEntry {
    pub id: CommandId,
    pub label: &'static str,
    pub shortcut: &'static str,
    pub group: &'static str,
    pub enabled: bool,
    pub disabled_reason: Option<String>,
}

const COMMAND_REGISTRY: &[CommandDefinition] = &[
    CommandDefinition {
        id: CommandId::Quit,
        label: "Quit",
        shortcut: "q / Ctrl+C",
        enabled: command_always_enabled,
    },
    CommandDefinition {
        id: CommandId::ShowHelp,
        label: "Show help",
        shortcut: "?",
        enabled: command_always_enabled,
    },
    CommandDefinition {
        id: CommandId::OpenPalette,
        label: "Open command palette",
        shortcut: ": / Ctrl+P",
        enabled: command_open_palette_enabled,
    },
    CommandDefinition {
        id: CommandId::CycleFocus,
        label: "Cycle focus",
        shortcut: "Tab",
        enabled: command_cycle_focus_enabled,
    },
    CommandDefinition {
        id: CommandId::MoveUp,
        label: "Move up",
        shortcut: "k / Up",
        enabled: command_move_up_enabled,
    },
    CommandDefinition {
        id: CommandId::MoveDown,
        label: "Move down",
        shortcut: "j / Down",
        enabled: command_move_down_enabled,
    },
    CommandDefinition {
        id: CommandId::MoveTop,
        label: "Jump top",
        shortcut: "g / Home",
        enabled: command_move_top_enabled,
    },
    CommandDefinition {
        id: CommandId::MoveBottom,
        label: "Jump bottom",
        shortcut: "G / End",
        enabled: command_move_bottom_enabled,
    },
    CommandDefinition {
        id: CommandId::PageUp,
        label: "Page up",
        shortcut: "PgUp / Ctrl+u",
        enabled: command_page_up_enabled,
    },
    CommandDefinition {
        id: CommandId::PageDown,
        label: "Page down",
        shortcut: "PgDn / Ctrl+d",
        enabled: command_page_down_enabled,
    },
    CommandDefinition {
        id: CommandId::FilterAll,
        label: "Filter all",
        shortcut: "1",
        enabled: command_always_enabled,
    },
    CommandDefinition {
        id: CommandId::FilterInstalled,
        label: "Filter installed",
        shortcut: "2",
        enabled: command_always_enabled,
    },
    CommandDefinition {
        id: CommandId::FilterUpdates,
        label: "Filter updates",
        shortcut: "3",
        enabled: command_always_enabled,
    },
    CommandDefinition {
        id: CommandId::FilterFavorites,
        label: "Filter favorites",
        shortcut: "4",
        enabled: command_always_enabled,
    },
    CommandDefinition {
        id: CommandId::ToggleFavorite,
        label: "Toggle favorite",
        shortcut: "f",
        enabled: command_toggle_favorite_enabled,
    },
    CommandDefinition {
        id: CommandId::BulkToggleFavorite,
        label: "Bulk toggle favorites",
        shortcut: "F",
        enabled: command_bulk_toggle_favorite_enabled,
    },
    CommandDefinition {
        id: CommandId::ToggleFavoritesUpdatesOnly,
        label: "Favorites updates only",
        shortcut: "v",
        enabled: command_toggle_favorites_updates_only_enabled,
    },
    CommandDefinition {
        id: CommandId::ToggleSelection,
        label: "Toggle selection",
        shortcut: "Space",
        enabled: command_toggle_selection_enabled,
    },
    CommandDefinition {
        id: CommandId::SelectAll,
        label: "Select all visible",
        shortcut: "a",
        enabled: command_select_all_enabled,
    },
    CommandDefinition {
        id: CommandId::Install,
        label: "Queue install",
        shortcut: "i",
        enabled: command_install_enabled,
    },
    CommandDefinition {
        id: CommandId::Remove,
        label: "Queue remove",
        shortcut: "x",
        enabled: command_remove_enabled,
    },
    CommandDefinition {
        id: CommandId::Update,
        label: "Queue update",
        shortcut: "u",
        enabled: command_update_enabled,
    },
    CommandDefinition {
        id: CommandId::RunRecommended,
        label: "Run recommended action",
        shortcut: "w",
        enabled: command_always_enabled,
    },
    CommandDefinition {
        id: CommandId::ViewChangelog,
        label: "View package changelog",
        shortcut: "c",
        enabled: command_view_changelog_enabled,
    },
    CommandDefinition {
        id: CommandId::Search,
        label: "Search packages",
        shortcut: "/",
        enabled: command_always_enabled,
    },
    CommandDefinition {
        id: CommandId::Refresh,
        label: "Refresh packages",
        shortcut: "r",
        enabled: command_refresh_enabled,
    },
    CommandDefinition {
        id: CommandId::ToggleQueue,
        label: "Toggle queue",
        shortcut: "l",
        enabled: command_toggle_queue_enabled,
    },
    CommandDefinition {
        id: CommandId::QueueCancel,
        label: "Cancel queued task",
        shortcut: "C",
        enabled: command_queue_cancel_enabled,
    },
    CommandDefinition {
        id: CommandId::QueueRetry,
        label: "Retry failed task",
        shortcut: "R",
        enabled: command_queue_retry_enabled,
    },
    CommandDefinition {
        id: CommandId::QueueRetrySafe,
        label: "Retry safe failed tasks",
        shortcut: "A",
        enabled: command_queue_retry_safe_enabled,
    },
    CommandDefinition {
        id: CommandId::QueueRemediate,
        label: "Apply remediation",
        shortcut: "M",
        enabled: command_queue_remediate_enabled,
    },
    CommandDefinition {
        id: CommandId::QueueLogOlder,
        label: "Show older queue logs",
        shortcut: "[",
        enabled: command_queue_log_older_enabled,
    },
    CommandDefinition {
        id: CommandId::QueueLogNewer,
        label: "Show newer queue logs",
        shortcut: "]",
        enabled: command_queue_log_newer_enabled,
    },
];

pub struct App {
    pub packages: Vec<Package>,
    pub filtered: Vec<usize>,
    pub filter_counts: [usize; 4],
    pub cursor: usize,
    pub selected: HashSet<String>,
    pub favorite_packages: HashSet<String>,
    pub favorites_updates_only: bool,

    pub filter: Filter,
    pub source: Option<PackageSource>,
    pub search: String,
    pub searching: bool,

    pub tasks: Vec<TaskQueueEntry>,
    pub task_cursor: usize,
    pub task_log_scroll: usize,
    pub task_logs: HashMap<String, VecDeque<String>>,
    task_last_log_at: HashMap<String, Instant>,
    pub previous_statuses: HashMap<String, PackageStatus>,
    pub task_failure_categories: HashMap<String, FailureCategory>,
    pub task_recovery_states: HashMap<String, RecoveryState>,
    pub retry_parent: HashMap<String, String>,
    pub retry_attempt: HashMap<String, usize>,
    pub queue_expanded: bool,
    pub queue_failure_filter: QueueFailureFilter,
    pub queue_completed_at: Option<Instant>,
    pub executor_running: Arc<AtomicBool>,
    pub queue_failures_acknowledged: bool,
    pub queue_completion_digest_emitted: bool,

    pub focus: Focus,
    pub compact: bool,
    pub confirming: Option<PendingAction>,
    pub showing_help: bool,
    pub showing_palette: bool,
    pub showing_changelog: bool,
    pub changelog_diff_only: bool,
    pub changelog_scroll: usize,
    pub changelog_target_package_id: Option<String>,
    pub palette_query: String,
    pub palette_cursor: usize,

    pub status: String,
    pub clear_status_on_key: bool,
    pub loading: bool,
    pub should_quit: bool,
    pub tick: u64,

    pub available_sources: Vec<PackageSource>,
    pub source_counts: HashMap<PackageSource, [usize; 4]>,

    pub pm: Arc<Mutex<PackageManager>>,
    pub history_tracker: Arc<Mutex<Option<HistoryTracker>>>,
    pub load_rx: Option<mpsc::Receiver<LoadResult>>,
    pub task_events_rx: Option<mpsc::Receiver<TaskQueueEvent>>,
    pub task_events_tx: Option<mpsc::Sender<TaskQueueEvent>>,
    changelog_rx: Option<mpsc::Receiver<ChangelogResult>>,
    changelog_tx: Option<mpsc::Sender<ChangelogResult>>,
    preflight_verification_rx: Option<mpsc::Receiver<PreflightVerificationResult>>,
    preflight_verification_tx: Option<mpsc::Sender<PreflightVerificationResult>>,
    next_preflight_verification_id: u64,
    active_preflight_verification_id: Option<u64>,
    changelog_cache: HashMap<String, ChangelogState>,
    pub refresh_after_idle: bool,
    pub cursor_anchor_id: Option<String>,
    pub last_click: Option<(u16, u16, Instant)>,
    pub drag_select_anchor: Option<usize>,
    pub favorites_persistence_enabled: bool,
    pub session_persistence_enabled: bool,
    pub favorites_dirty: bool,
}

impl App {
    pub fn new(
        pm: Arc<Mutex<PackageManager>>,
        history_tracker: Arc<Mutex<Option<HistoryTracker>>>,
        task_events_rx: Option<mpsc::Receiver<TaskQueueEvent>>,
        task_events_tx: Option<mpsc::Sender<TaskQueueEvent>>,
    ) -> Self {
        let (changelog_tx, changelog_rx) = mpsc::channel(32);
        let (preflight_verification_tx, preflight_verification_rx) = mpsc::channel(32);
        Self {
            packages: Vec::new(),
            filtered: Vec::new(),
            filter_counts: [0, 0, 0, 0],
            cursor: 0,
            selected: HashSet::new(),
            favorite_packages: HashSet::new(),
            favorites_updates_only: false,
            filter: Filter::All,
            source: None,
            search: String::new(),
            searching: false,
            tasks: Vec::new(),
            task_cursor: 0,
            task_log_scroll: 0,
            task_logs: HashMap::new(),
            task_last_log_at: HashMap::new(),
            previous_statuses: HashMap::new(),
            task_failure_categories: HashMap::new(),
            task_recovery_states: HashMap::new(),
            retry_parent: HashMap::new(),
            retry_attempt: HashMap::new(),
            queue_expanded: false,
            queue_failure_filter: QueueFailureFilter::All,
            queue_completed_at: None,
            executor_running: Arc::new(AtomicBool::new(false)),
            queue_failures_acknowledged: false,
            queue_completion_digest_emitted: false,
            focus: Focus::Sources,
            compact: false,
            confirming: None,
            showing_help: false,
            showing_palette: false,
            showing_changelog: false,
            changelog_diff_only: false,
            changelog_scroll: 0,
            changelog_target_package_id: None,
            palette_query: String::new(),
            palette_cursor: 0,
            status: String::new(),
            clear_status_on_key: false,
            loading: false,
            should_quit: false,
            tick: 0,
            available_sources: Vec::new(),
            source_counts: HashMap::new(),
            pm,
            history_tracker,
            load_rx: None,
            task_events_rx,
            task_events_tx,
            changelog_rx: Some(changelog_rx),
            changelog_tx: Some(changelog_tx),
            preflight_verification_rx: Some(preflight_verification_rx),
            preflight_verification_tx: Some(preflight_verification_tx),
            next_preflight_verification_id: 1,
            active_preflight_verification_id: None,
            changelog_cache: HashMap::new(),
            refresh_after_idle: false,
            cursor_anchor_id: None,
            last_click: None,
            drag_select_anchor: None,
            favorites_persistence_enabled: true,
            session_persistence_enabled: true,
            favorites_dirty: false,
        }
    }

    pub fn set_status(&mut self, message: impl Into<String>, clear_on_next_key: bool) {
        self.status = message.into();
        self.clear_status_on_key = clear_on_next_key;
    }

    pub fn clear_status_if_needed(&mut self) {
        if self.clear_status_on_key {
            self.status.clear();
            self.clear_status_on_key = false;
        }
    }

    pub async fn initialize_history_tracker(&mut self) {
        match HistoryTracker::load().await {
            Ok(tracker) => {
                let mut guard = self.history_tracker.lock().await;
                *guard = Some(tracker);
            }
            Err(e) => {
                self.set_status(format!("History tracker unavailable: {}", e), true);
            }
        }
    }

    pub async fn sync_task_queue_from_history(&mut self) {
        let retain_history = Config::load().retain_task_queue_history;
        let mut save_error: Option<String> = None;

        let entries = {
            let mut guard = self.history_tracker.lock().await;
            if let Some(tracker) = guard.as_mut() {
                if !retain_history {
                    tracker.history_mut().task_queue.retain_active();
                    if let Err(error) = tracker.save().await {
                        save_error = Some(error.to_string());
                    }
                }
                tracker.history().task_queue.entries.clone()
            } else {
                Vec::new()
            }
        };

        if let Some(error) = save_error {
            self.set_status(format!("Failed to prune queue history: {}", error), true);
        }

        self.tasks = Self::session_queue_entries(entries, retain_history);
        self.rebuild_failure_categories();
        self.clamp_task_cursor();
        self.ensure_queue_cursor_matches_filter();
    }

    pub async fn load_sources(&mut self) {
        let manager = self.pm.lock().await;
        self.available_sources = manager.available_sources().into_iter().collect();
        self.available_sources
            .sort_by_key(|source| source.to_string());
    }

    pub fn load_favorites(&mut self) {
        self.favorite_packages = Config::load().favorite_packages.into_iter().collect();
        self.favorites_dirty = false;
    }

    pub fn load_session_state(&mut self) {
        if !self.session_persistence_enabled {
            return;
        }

        let config = Config::load();
        self.apply_session_from_config(&config);
    }

    fn apply_session_from_config(&mut self, config: &Config) {
        self.filter = Filter::from_config_value(config.tui_last_filter.as_deref());
        self.focus = Focus::from_config_value(config.tui_last_focus.as_deref());
        self.search = config.tui_last_search.clone();
        self.source = config
            .last_source_filter
            .as_deref()
            .and_then(PackageSource::from_str);
        self.favorites_updates_only = config.tui_favorites_updates_only;
        self.apply_filters();
        self.cursor = config
            .tui_last_cursor
            .min(self.filtered.len().saturating_sub(1));

        if self.focus == Focus::Queue && !self.queue_expanded {
            self.focus = Focus::Packages;
        }
    }

    fn write_session_to_config(&self, config: &mut Config) {
        config.tui_last_filter = Some(self.filter.as_config_value().to_string());
        config.tui_last_focus = Some(self.focus.as_config_value().to_string());
        config.tui_last_search = self.search.clone();
        config.tui_last_cursor = self.cursor;
        config.tui_favorites_updates_only = self.favorites_updates_only;
        config.last_source_filter = self.source.map(|source| source.as_config_str().to_string());
    }

    fn persist_session_state(&self) -> Result<()> {
        if !self.session_persistence_enabled {
            return Ok(());
        }

        let mut config = Config::load();
        self.write_session_to_config(&mut config);
        config
            .save()
            .context("Failed to persist TUI session state to config")
    }

    fn persist_favorites(&mut self) -> Result<()> {
        if !self.favorites_persistence_enabled || !self.favorites_dirty {
            return Ok(());
        }

        let mut config = Config::load();
        let mut favorites: Vec<String> = self.favorite_packages.iter().cloned().collect();
        favorites.sort();
        favorites.dedup();
        config.favorite_packages = favorites;
        config
            .save()
            .context("Failed to persist favorites to config")?;
        self.favorites_dirty = false;
        Ok(())
    }

    pub fn source_count(&self) -> usize {
        self.visible_sources().len() + 1
    }

    pub fn visible_sources(&self) -> Vec<PackageSource> {
        match self.filter {
            Filter::Updates | Filter::Favorites => {
                let count_index = if self.filter == Filter::Updates {
                    FILTER_UPDATES_INDEX
                } else {
                    FILTER_FAVORITES_INDEX
                };
                self.available_sources
                    .iter()
                    .filter(|source| {
                        self.source_counts
                            .get(source)
                            .is_some_and(|counts| counts[count_index] > 0)
                    })
                    .copied()
                    .collect()
            }
            Filter::All | Filter::Installed => self.available_sources.clone(),
        }
    }

    pub fn command_registry() -> &'static [CommandDefinition] {
        COMMAND_REGISTRY
    }

    pub fn command_enabled(&self, id: CommandId) -> bool {
        Self::command_definition(id)
            .map(|definition| {
                !definition.label.trim().is_empty()
                    && !definition.shortcut.trim().is_empty()
                    && definition.is_enabled(self)
            })
            .unwrap_or(false)
    }

    pub fn command_disabled_reason(&self, id: CommandId) -> Option<String> {
        if self.command_enabled(id) {
            return None;
        }

        Some(
            match id {
                CommandId::CycleFocus => {
                    "Cycle focus is unavailable in compact layout or while queue is expanded"
                }
                CommandId::MoveUp
                | CommandId::MoveDown
                | CommandId::MoveTop
                | CommandId::MoveBottom
                | CommandId::PageUp
                | CommandId::PageDown => "No further items in that direction",
                CommandId::ToggleFavorite | CommandId::ToggleSelection => "Select a package first",
                CommandId::BulkToggleFavorite => {
                    "Select one or more packages (or place the cursor on a package)"
                }
                CommandId::ToggleFavoritesUpdatesOnly => "Switch to Favorites filter first",
                CommandId::SelectAll => "No visible packages to select",
                CommandId::Install => "No installable package in current selection",
                CommandId::Remove => "No removable package in current selection",
                CommandId::Update => "No updatable package in current selection",
                CommandId::RunRecommended => "No recommendation available",
                CommandId::ViewChangelog => {
                    if self.current_package().is_some() {
                        "Changelog is not supported for this source yet"
                    } else {
                        "Select a package to view changelog"
                    }
                }
                CommandId::Refresh => "Refresh is already in progress",
                CommandId::ToggleQueue => "Queue is empty",
                CommandId::QueueCancel => "Select a queued task in expanded queue",
                CommandId::QueueRetry => "Select a failed task in expanded queue",
                CommandId::QueueRetrySafe => {
                    if self.queue_expanded && self.focus == Focus::Queue {
                        return Some(self.safe_retry_unavailable_reason());
                    }
                    "Focus expanded queue to run safe retry bundle"
                }
                CommandId::QueueRemediate => {
                    if self.queue_expanded && self.focus == Focus::Queue {
                        return Some(self.remediation_unavailable_reason());
                    }
                    "Focus expanded queue to run remediation bundle"
                }
                CommandId::QueueLogOlder => "No older queue logs available",
                CommandId::QueueLogNewer => "No newer queue logs available",
                _ => "Command unavailable in current context",
            }
            .to_string(),
        )
    }

    pub fn palette_entries(&self) -> Vec<PaletteCommandEntry> {
        let query = self.palette_query.trim().to_lowercase();

        let mut entries: Vec<(usize, usize, usize, PaletteCommandEntry)> = Self::command_registry()
            .iter()
            .enumerate()
            .filter_map(|(order, definition)| {
                let group = command_group(definition.id);
                let haystack = format!(
                    "{} {} {}",
                    definition.label.to_lowercase(),
                    definition.shortcut.to_lowercase(),
                    group.to_lowercase()
                );
                let score = fuzzy_subsequence_score(&haystack, &query)?;
                let enabled = self.command_enabled(definition.id);
                Some((
                    command_group_order(definition.id),
                    score,
                    order,
                    PaletteCommandEntry {
                        id: definition.id,
                        label: definition.label,
                        shortcut: definition.shortcut,
                        group,
                        enabled,
                        disabled_reason: self.command_disabled_reason(definition.id),
                    },
                ))
            })
            .collect();

        entries.sort_by_key(|(group, score, order, _)| (*group, *score, *order));
        entries.into_iter().map(|(_, _, _, entry)| entry).collect()
    }

    pub fn palette_selected_entry(&self) -> Option<PaletteCommandEntry> {
        self.palette_entries().get(self.palette_cursor).cloned()
    }

    fn clamp_palette_cursor(&mut self) {
        let len = self.palette_entries().len();
        self.palette_cursor = if len == 0 {
            0
        } else {
            self.palette_cursor.min(len.saturating_sub(1))
        };
    }

    fn command_definition(id: CommandId) -> Option<&'static CommandDefinition> {
        Self::command_registry()
            .iter()
            .find(|definition| definition.id == id)
    }

    pub fn source_index(&self) -> usize {
        match self.source {
            None => 0,
            Some(source) => self
                .visible_sources()
                .iter()
                .position(|s| *s == source)
                .map(|idx| idx + 1)
                .unwrap_or(0),
        }
    }

    fn queue_focus_active(&self) -> bool {
        self.queue_expanded && self.focus == Focus::Queue
    }

    fn can_cycle_focus_command(&self) -> bool {
        !self.compact && !self.queue_expanded
    }

    fn can_move_up_command(&self) -> bool {
        if self.queue_focus_active() {
            return self.task_cursor > 0;
        }

        match self.focus {
            Focus::Sources => self.source_index() > 0,
            Focus::Packages | Focus::Queue => self.cursor > 0,
        }
    }

    fn can_move_down_command(&self) -> bool {
        if self.queue_focus_active() {
            return self.task_cursor + 1 < self.tasks.len();
        }

        match self.focus {
            Focus::Sources => self.source_index() + 1 < self.source_count(),
            Focus::Packages | Focus::Queue => self.cursor + 1 < self.filtered.len(),
        }
    }

    fn can_move_top_command(&self) -> bool {
        if self.queue_focus_active() {
            return !self.tasks.is_empty() && self.task_cursor > 0;
        }

        match self.focus {
            Focus::Sources => self.source_index() > 0,
            Focus::Packages | Focus::Queue => !self.filtered.is_empty() && self.cursor > 0,
        }
    }

    fn can_move_bottom_command(&self) -> bool {
        if self.queue_focus_active() {
            return self.task_cursor + 1 < self.tasks.len();
        }

        match self.focus {
            Focus::Sources => self.source_index() + 1 < self.source_count(),
            Focus::Packages | Focus::Queue => self.cursor + 1 < self.filtered.len(),
        }
    }

    fn can_page_up_command(&self) -> bool {
        if self.queue_focus_active() {
            return self.task_cursor > 0;
        }

        match self.focus {
            Focus::Sources => self.source_index() > 0,
            Focus::Packages | Focus::Queue => self.cursor > 0,
        }
    }

    fn can_page_down_command(&self) -> bool {
        if self.queue_focus_active() {
            return self.task_cursor + 1 < self.tasks.len();
        }

        match self.focus {
            Focus::Sources => self.source_index() + 1 < self.source_count(),
            Focus::Packages | Focus::Queue => self.cursor + 1 < self.filtered.len(),
        }
    }

    fn can_toggle_selection_command(&self) -> bool {
        self.current_package().is_some()
    }

    fn can_toggle_favorite_command(&self) -> bool {
        self.current_package().is_some()
    }

    fn can_select_all_command(&self) -> bool {
        !self.filtered.is_empty()
    }

    fn can_bulk_toggle_favorite_command(&self) -> bool {
        !self.selected.is_empty() || self.current_package().is_some()
    }

    fn can_toggle_favorites_updates_only_command(&self) -> bool {
        self.filter == Filter::Favorites
    }

    fn can_prepare_command(&self, action: TaskQueueAction) -> bool {
        let targets = self.collect_action_targets();
        !targets.is_empty()
            && targets
                .iter()
                .any(|package| Self::is_valid_target(action, package))
    }

    fn can_view_changelog_command(&self) -> bool {
        self.current_package()
            .is_some_and(|package| Self::changelog_supported_for_source(package.source))
    }

    fn can_refresh_command(&self) -> bool {
        !self.loading
    }

    fn can_toggle_queue_command(&self) -> bool {
        !self.tasks.is_empty()
    }

    fn can_cancel_selected_task_command(&self) -> bool {
        self.queue_focus_active()
            && self
                .tasks
                .get(self.task_cursor)
                .is_some_and(|task| task.status == TaskQueueStatus::Queued)
    }

    fn can_retry_selected_task_command(&self) -> bool {
        self.queue_focus_active()
            && self
                .tasks
                .get(self.task_cursor)
                .is_some_and(|task| task.status == TaskQueueStatus::Failed)
    }

    fn can_retry_safe_failed_tasks_command(&self) -> bool {
        self.queue_focus_active() && self.queue_clinic_actionability().safe_retry_count > 0
    }

    fn queue_log_max_scroll(&self) -> usize {
        let Some(task) = self.tasks.get(self.task_cursor) else {
            return 0;
        };
        self.task_logs
            .get(&task.id)
            .map(|logs| logs.len().saturating_sub(1))
            .unwrap_or(0)
    }

    fn can_queue_log_older_command(&self) -> bool {
        self.queue_focus_active() && self.task_log_scroll < self.queue_log_max_scroll()
    }

    fn can_queue_log_newer_command(&self) -> bool {
        self.queue_focus_active() && self.task_log_scroll > 0
    }

    fn can_queue_remediate_command(&self) -> bool {
        self.queue_focus_active()
            && self
                .queue_clinic_actionability()
                .remediation_actionable_count()
                > 0
    }

    fn classify_failure(error_text: &str) -> FailureCategory {
        let lowered = error_text.to_lowercase();

        if lowered.contains("lock")
            || lowered.contains("conflict")
            || lowered.contains("held broken")
            || lowered.contains("another process")
            || lowered.contains("already running")
            || lowered.contains("dependency problem")
            || lowered.contains("dpkg was interrupted")
        {
            return FailureCategory::Conflict;
        }

        if lowered.contains("permission denied")
            || lowered.contains("not permitted")
            || lowered.contains("operation not permitted")
            || lowered.contains("must be root")
            || lowered.contains("authentication")
            || lowered.contains("authorization")
            || lowered.contains("pkexec")
            || lowered.contains("access denied")
            || lowered.contains("eacces")
            || lowered.contains("sudo")
        {
            return FailureCategory::Permissions;
        }

        if lowered.contains("timed out")
            || lowered.contains("timeout")
            || lowered.contains("temporary failure")
            || lowered.contains("could not resolve")
            || lowered.contains("name resolution")
            || lowered.contains("network")
            || lowered.contains("connection refused")
            || lowered.contains("connection reset")
            || lowered.contains("unreachable")
            || lowered.contains("failed to fetch")
        {
            return FailureCategory::Network;
        }

        if lowered.contains("not found")
            || lowered.contains("unable to locate")
            || lowered.contains("no package")
            || lowered.contains("could not find")
            || lowered.contains("no matching")
            || lowered.contains("404")
        {
            return FailureCategory::NotFound;
        }

        FailureCategory::Unknown
    }

    pub fn failure_category_for_task(&self, task: &TaskQueueEntry) -> Option<FailureCategory> {
        if task.status != TaskQueueStatus::Failed {
            return None;
        }

        self.task_failure_categories
            .get(&task.id)
            .copied()
            .or_else(|| task.error.as_deref().map(Self::classify_failure))
            .or(Some(FailureCategory::Unknown))
    }

    pub fn recovery_state_for_task(&self, task_id: &str) -> Option<&RecoveryState> {
        self.task_recovery_states.get(task_id)
    }

    pub fn retry_parent_for_task(&self, task_id: &str) -> Option<&TaskQueueEntry> {
        let parent_id = self.retry_parent.get(task_id)?;
        self.tasks.iter().find(|task| task.id == *parent_id)
    }

    pub fn retry_attempt_for_task(&self, task_id: &str) -> Option<usize> {
        self.retry_attempt.get(task_id).copied()
    }

    pub fn task_last_log_age_secs(&self, task_id: &str) -> Option<u64> {
        self.task_last_log_at
            .get(task_id)
            .map(|instant| instant.elapsed().as_secs())
    }

    pub fn queue_lane_for_task(&self, task: &TaskQueueEntry) -> QueueJourneyLane {
        match task.status {
            TaskQueueStatus::Running => QueueJourneyLane::Now,
            TaskQueueStatus::Queued => QueueJourneyLane::Next,
            TaskQueueStatus::Completed | TaskQueueStatus::Cancelled => QueueJourneyLane::Done,
            TaskQueueStatus::Failed => {
                let recovered = self
                    .recovery_state_for_task(&task.id)
                    .is_some_and(|state| state.last_outcome == Some(TaskQueueStatus::Completed));
                if recovered {
                    QueueJourneyLane::Done
                } else {
                    QueueJourneyLane::NeedsAttention
                }
            }
        }
    }

    pub fn queue_lane_counts(&self) -> (usize, usize, usize, usize) {
        let mut now = 0usize;
        let mut next = 0usize;
        let mut attention = 0usize;
        let mut done = 0usize;
        for task in &self.tasks {
            match self.queue_lane_for_task(task) {
                QueueJourneyLane::Now => now += 1,
                QueueJourneyLane::Next => next += 1,
                QueueJourneyLane::NeedsAttention => attention += 1,
                QueueJourneyLane::Done => done += 1,
            }
        }
        (now, next, attention, done)
    }

    pub fn queue_failure_filter_label(&self) -> &'static str {
        self.queue_failure_filter.label()
    }

    pub fn queue_visible_task_indices(&self) -> Vec<usize> {
        if self.queue_failure_filter == QueueFailureFilter::All {
            return (0..self.tasks.len()).collect();
        }

        self.tasks
            .iter()
            .enumerate()
            .filter(|(_, task)| self.queue_lane_for_task(task) == QueueJourneyLane::NeedsAttention)
            .filter(|(_, task)| {
                self.failure_category_for_task(task)
                    .is_some_and(|category| self.queue_failure_filter.matches(category))
            })
            .map(|(index, _)| index)
            .collect()
    }

    pub fn queue_visible_cursor_position(&self, visible_indices: &[usize]) -> usize {
        visible_indices
            .iter()
            .position(|index| *index == self.task_cursor)
            .unwrap_or(0)
    }

    fn ensure_queue_cursor_matches_filter(&mut self) {
        if self.tasks.is_empty() {
            self.queue_failure_filter = QueueFailureFilter::All;
            self.set_task_cursor(0);
            return;
        }

        if self.queue_failure_filter != QueueFailureFilter::All
            && self.unresolved_failure_count() == 0
        {
            self.queue_failure_filter = QueueFailureFilter::All;
        }

        let visible = self.queue_visible_task_indices();
        if visible.is_empty() {
            self.set_task_cursor(self.task_cursor.min(self.tasks.len() - 1));
            return;
        }

        if !visible.contains(&self.task_cursor) {
            self.set_task_cursor(visible[0]);
        }
    }

    fn set_queue_failure_filter(&mut self, filter: QueueFailureFilter) {
        self.queue_failure_filter = filter;
        self.ensure_queue_cursor_matches_filter();

        let visible = self.queue_visible_task_indices().len();
        if visible == 0 && filter != QueueFailureFilter::All {
            self.set_status(
                format!(
                    "Failure filter: {} (no matching failures, press 0 for all)",
                    filter.label()
                ),
                true,
            );
            return;
        }

        self.set_status(
            format!(
                "Failure filter: {} ({} visible task{})",
                filter.label(),
                visible,
                if visible == 1 { "" } else { "s" }
            ),
            true,
        );
    }

    pub fn unresolved_failure_count(&self) -> usize {
        self.tasks
            .iter()
            .filter(|task| self.queue_lane_for_task(task) == QueueJourneyLane::NeedsAttention)
            .count()
    }

    pub fn retryable_failed_task_count(&self) -> usize {
        let mut seen = HashSet::new();
        self.tasks
            .iter()
            .filter(|task| {
                self.queue_lane_for_task(task) == QueueJourneyLane::NeedsAttention
                    && self
                        .failure_category_for_task(task)
                        .is_some_and(Self::safe_retry_category)
                    && !self.has_active_task_for_package(&task.package_id)
                    && seen.insert(task.package_id.clone())
            })
            .count()
    }

    pub fn queue_clinic_actionability(&self) -> QueueClinicActionability {
        let safe_retry_count = self.clinic_safe_retry_candidates().len();
        let remediation = self.clinic_remediation_plan();

        QueueClinicActionability {
            failed_in_scope: remediation.preview_count,
            safe_retry_count,
            remediation_retry_count: remediation.retries.len(),
            remediation_guidance_count: remediation.guidance_only,
            remediation_skipped_count: remediation.skipped,
        }
    }

    fn safe_retry_unavailable_reason(&self) -> String {
        let scope = self.queue_failure_filter.label();
        let actionability = self.queue_clinic_actionability();

        if actionability.failed_in_scope == 0 {
            if self.queue_failure_filter == QueueFailureFilter::All {
                "No failed tasks in queue".to_string()
            } else {
                format!("No safe retries for {} failures (press 0 for all)", scope)
            }
        } else if actionability.remediation_skipped_count >= actionability.failed_in_scope {
            format!("No safe retries for {} failures (already active)", scope)
        } else {
            format!("No safe retries for {} failures", scope)
        }
    }

    fn remediation_unavailable_reason(&self) -> String {
        let scope = self.queue_failure_filter.label();
        let actionability = self.queue_clinic_actionability();

        if actionability.failed_in_scope == 0 {
            if self.queue_failure_filter == QueueFailureFilter::All {
                "No failed tasks in queue".to_string()
            } else {
                format!(
                    "No remediation needed for {} failures (press 0 for all)",
                    scope
                )
            }
        } else if actionability.remediation_skipped_count >= actionability.failed_in_scope {
            format!(
                "No remediation needed for {} failures (already active)",
                scope
            )
        } else {
            format!("No remediation needed for {} failures", scope)
        }
    }

    pub fn update_candidate_count(&self) -> usize {
        self.packages
            .iter()
            .filter(|package| package.status == PackageStatus::UpdateAvailable)
            .count()
    }

    pub fn recommended_action_label(&self) -> String {
        match self.recommended_action() {
            RecommendedAction::RetrySafeFailures(count) => {
                format!(
                    "Retry {} safe failure{}",
                    count,
                    if count == 1 { "" } else { "s" }
                )
            }
            RecommendedAction::ReviewFailures(count) => {
                format!(
                    "Review {} failure{}",
                    count,
                    if count == 1 { "" } else { "s" }
                )
            }
            RecommendedAction::QueueAllUpdates(count) => format!(
                "Queue {} update{}",
                count,
                if count == 1 { "" } else { "s" }
            ),
            RecommendedAction::ReviewQueue => "Review queue progress".to_string(),
            RecommendedAction::RefreshPackages => "Refresh package metadata".to_string(),
        }
    }

    pub fn recommended_action_detail(&self) -> String {
        match self.recommended_action() {
            RecommendedAction::RetrySafeFailures(_) => {
                "Best next step: retry transient/system conflicts in one bundle.".to_string()
            }
            RecommendedAction::ReviewFailures(_) => {
                "Best next step: inspect failed tasks and pick retry/remediation.".to_string()
            }
            RecommendedAction::QueueAllUpdates(_) => {
                "Best next step: stage one update batch and confirm in preflight.".to_string()
            }
            RecommendedAction::ReviewQueue => {
                "Best next step: open queue and monitor remaining work.".to_string()
            }
            RecommendedAction::RefreshPackages => {
                "Best next step: refresh package index to discover available actions.".to_string()
            }
        }
    }

    fn recommended_action(&self) -> RecommendedAction {
        let retryable = self.retryable_failed_task_count();
        if retryable > 0 {
            return RecommendedAction::RetrySafeFailures(retryable);
        }

        let unresolved = self.unresolved_failure_count();
        if unresolved > 0 {
            return RecommendedAction::ReviewFailures(unresolved);
        }

        let updates = self.update_candidate_count();
        if updates > 0 {
            return RecommendedAction::QueueAllUpdates(updates);
        }

        if !self.tasks.is_empty() {
            return RecommendedAction::ReviewQueue;
        }

        RecommendedAction::RefreshPackages
    }

    fn safe_retry_category(category: FailureCategory) -> bool {
        matches!(
            category,
            FailureCategory::Permissions | FailureCategory::Network | FailureCategory::Conflict
        )
    }

    fn rebuild_failure_categories(&mut self) {
        self.task_failure_categories.clear();
        for task in &self.tasks {
            if task.status == TaskQueueStatus::Failed {
                let category = task
                    .error
                    .as_deref()
                    .map(Self::classify_failure)
                    .unwrap_or(FailureCategory::Unknown);
                self.task_failure_categories
                    .insert(task.id.clone(), category);
            }
        }
    }

    fn build_preflight_summary(
        action: TaskQueueAction,
        targets: &[Package],
        valid: &[Package],
        selection_mode: bool,
    ) -> PreflightSummary {
        let mut source_breakdown: HashMap<PackageSource, usize> = HashMap::new();
        for package in valid {
            *source_breakdown.entry(package.source).or_insert(0) += 1;
        }

        let mut source_breakdown: Vec<(PackageSource, usize)> =
            source_breakdown.into_iter().collect();
        source_breakdown.sort_by_key(|(source, _)| source.to_string());

        let has_system_source = source_breakdown
            .iter()
            .any(|(source, _)| Self::source_treated_as_system(*source));
        let elevated_privileges_likely = valid
            .iter()
            .any(|package| Self::source_likely_requires_elevation(package.source));
        let dependency_impact_known = false;
        let verification_in_progress =
            Self::preflight_dependency_verification_supported(action, valid);
        let (risk_level, risk_reasons, certainty) = Self::assess_preflight(
            action,
            valid.len(),
            has_system_source,
            elevated_privileges_likely,
            dependency_impact_known,
            verification_in_progress,
        );

        PreflightSummary {
            action,
            target_count: targets.len(),
            executable_count: valid.len(),
            skipped_count: targets.len().saturating_sub(valid.len()),
            source_breakdown,
            risk_level,
            risk_reasons,
            certainty,
            elevated_privileges_likely,
            dependency_impact_known,
            dependency_impact: None,
            verification_in_progress,
            selection_mode,
        }
    }

    fn source_supports_dependency_verification(
        action: TaskQueueAction,
        source: PackageSource,
    ) -> bool {
        matches!(
            (action, source),
            (
                TaskQueueAction::Remove,
                PackageSource::Apt
                    | PackageSource::Dnf
                    | PackageSource::Pacman
                    | PackageSource::Zypper
                    | PackageSource::Flatpak
            ) | (
                TaskQueueAction::Install | TaskQueueAction::Update,
                PackageSource::Apt | PackageSource::Dnf
            )
        )
    }

    fn preflight_dependency_verification_supported(
        action: TaskQueueAction,
        valid: &[Package],
    ) -> bool {
        let sources: HashSet<PackageSource> = valid.iter().map(|package| package.source).collect();
        !sources.is_empty()
            && sources
                .iter()
                .all(|source| Self::source_supports_dependency_verification(action, *source))
    }

    fn assess_preflight(
        action: TaskQueueAction,
        executable_count: usize,
        has_system_source: bool,
        elevated_privileges_likely: bool,
        dependency_impact_known: bool,
        verification_in_progress: bool,
    ) -> (PreflightRiskLevel, Vec<String>, PreflightCertainty) {
        let mut risk_reasons = Vec::new();
        let mut risk_level = PreflightRiskLevel::Safe;

        match action {
            TaskQueueAction::Remove => {
                risk_level = PreflightRiskLevel::Caution;
                risk_reasons.push("Removal operations can disrupt dependent tooling.".to_string());
                if has_system_source {
                    risk_level = PreflightRiskLevel::High;
                    risk_reasons.push(
                        "Includes system-level package sources (APT/DNF/Pacman/etc).".to_string(),
                    );
                }
            }
            TaskQueueAction::Update => {
                if executable_count >= 10 {
                    risk_level = PreflightRiskLevel::Caution;
                    risk_reasons
                        .push("Large update batch may contain breaking changes.".to_string());
                }
            }
            TaskQueueAction::Install => {
                if executable_count >= 15 {
                    risk_level = PreflightRiskLevel::Caution;
                    risk_reasons
                        .push("Large install batch may pull significant dependencies.".to_string());
                }
            }
        }

        if executable_count >= 20 {
            risk_level = PreflightRiskLevel::High;
            risk_reasons.push("Very large queue size; review before confirming.".to_string());
        }

        if elevated_privileges_likely {
            if risk_level == PreflightRiskLevel::Safe {
                risk_level = PreflightRiskLevel::Caution;
            }
            risk_reasons.push("This action may prompt for elevated privileges.".to_string());
        }

        if has_system_source && !dependency_impact_known {
            if risk_level == PreflightRiskLevel::Safe {
                risk_level = PreflightRiskLevel::Caution;
            }
            risk_reasons.push(
                if verification_in_progress {
                    "Dependency impact verification is in progress."
                } else {
                    "Dependency impact is estimated; exact changes are resolved at execution time."
                }
                .to_string(),
            );
        } else if dependency_impact_known {
            risk_reasons.push(
                "Dependency impact was verified through source capability probes.".to_string(),
            );
        }

        if risk_reasons.is_empty() {
            risk_reasons.push("No additional guardrails triggered.".to_string());
        }

        let certainty = if dependency_impact_known {
            PreflightCertainty::Verified
        } else {
            PreflightCertainty::Estimated
        };

        (risk_level, risk_reasons, certainty)
    }

    fn refresh_preflight_assessment(preflight: &mut PreflightSummary, note: Option<String>) {
        let has_system_source = preflight
            .source_breakdown
            .iter()
            .any(|(source, _)| Self::source_treated_as_system(*source));
        let (risk_level, mut risk_reasons, certainty) = Self::assess_preflight(
            preflight.action,
            preflight.executable_count,
            has_system_source,
            preflight.elevated_privileges_likely,
            preflight.dependency_impact_known,
            preflight.verification_in_progress,
        );
        if let Some(note) = note {
            risk_reasons.push(note);
        }

        preflight.risk_level = risk_level;
        preflight.risk_reasons = risk_reasons;
        preflight.certainty = certainty;
    }

    fn preflight_target_ids(packages: &[Package]) -> Vec<String> {
        let mut ids: Vec<String> = packages.iter().map(Package::id).collect();
        ids.sort();
        ids
    }

    fn source_treated_as_system(source: PackageSource) -> bool {
        matches!(
            source,
            PackageSource::Apt
                | PackageSource::Dnf
                | PackageSource::Pacman
                | PackageSource::Zypper
                | PackageSource::Deb
                | PackageSource::Aur
        )
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

    fn set_source_by_index(&mut self, index: usize) {
        let visible = self.visible_sources();
        if index == 0 {
            self.source = None;
        } else if let Some(source) = visible.get(index.saturating_sub(1)).copied() {
            self.source = Some(source);
        }
        self.apply_filters();
    }

    fn next_source(&mut self) {
        let total = self.source_count();
        if total == 0 {
            return;
        }
        let idx = (self.source_index() + 1) % total;
        self.set_source_by_index(idx);
    }

    fn prev_source(&mut self) {
        let total = self.source_count();
        if total == 0 {
            return;
        }
        let idx = if self.source_index() == 0 {
            total - 1
        } else {
            self.source_index() - 1
        };
        self.set_source_by_index(idx);
    }

    pub fn current_package(&self) -> Option<&Package> {
        self.filtered
            .get(self.cursor)
            .and_then(|idx| self.packages.get(*idx))
    }

    fn current_package_id(&self) -> Option<String> {
        self.current_package().map(Package::id)
    }

    fn package_by_id(&self, package_id: &str) -> Option<&Package> {
        self.packages
            .iter()
            .find(|package| package.id() == package_id)
    }

    pub fn changelog_target_package(&self) -> Option<&Package> {
        self.changelog_target_package_id
            .as_deref()
            .and_then(|package_id| self.package_by_id(package_id))
    }

    pub fn changelog_state_for_target(&self) -> Option<&ChangelogState> {
        self.changelog_target_package_id
            .as_ref()
            .and_then(|package_id| self.changelog_cache.get(package_id))
    }

    pub fn changelog_supported_for_source(source: PackageSource) -> bool {
        matches!(
            source,
            PackageSource::Apt
                | PackageSource::Dnf
                | PackageSource::Pip
                | PackageSource::Npm
                | PackageSource::Cargo
                | PackageSource::Conda
                | PackageSource::Mamba
        )
    }

    pub fn changelog_supported_for_target(&self) -> bool {
        self.changelog_target_package()
            .is_some_and(|package| Self::changelog_supported_for_source(package.source))
    }

    pub fn visible_selected_count(&self) -> usize {
        self.filtered
            .iter()
            .filter(|idx| {
                self.packages
                    .get(**idx)
                    .is_some_and(|p| self.selected.contains(&p.id()))
            })
            .count()
    }

    pub fn hidden_selected_count(&self) -> usize {
        self.selected
            .len()
            .saturating_sub(self.visible_selected_count())
    }

    pub fn is_favorite_id(&self, package_id: &str) -> bool {
        self.favorite_packages.contains(package_id)
    }

    pub fn apply_filters(&mut self) {
        let mut n_all = 0usize;
        let mut n_installed = 0usize;
        let mut n_updates = 0usize;
        let mut n_favorites = 0usize;

        let mut per_source: HashMap<PackageSource, [usize; 4]> = HashMap::new();

        for package in &self.packages {
            n_all += 1;
            let entry = per_source.entry(package.source).or_insert([0, 0, 0, 0]);
            entry[FILTER_ALL_INDEX] += 1;

            let is_installed = matches!(
                package.status,
                PackageStatus::Installed
                    | PackageStatus::UpdateAvailable
                    | PackageStatus::Installing
                    | PackageStatus::Removing
                    | PackageStatus::Updating
            );
            let is_update = matches!(
                package.status,
                PackageStatus::UpdateAvailable | PackageStatus::Updating
            );
            let is_favorite = self.favorite_packages.contains(&package.id());
            let is_favorite_visible = is_favorite && (!self.favorites_updates_only || is_update);

            if is_installed {
                n_installed += 1;
                entry[FILTER_INSTALLED_INDEX] += 1;
            }
            if is_update {
                n_updates += 1;
                entry[FILTER_UPDATES_INDEX] += 1;
            }
            if is_favorite_visible {
                n_favorites += 1;
                entry[FILTER_FAVORITES_INDEX] += 1;
            }
        }
        self.filter_counts = [n_all, n_installed, n_updates, n_favorites];
        self.source_counts = per_source;

        // Reset source if it's not visible under the current filter
        if let Some(source) = self.source {
            if !self.visible_sources().contains(&source) {
                self.source = None;
            }
        }

        let query = self.search.to_lowercase();
        self.filtered = self
            .packages
            .iter()
            .enumerate()
            .filter(|(_, package)| {
                self.matches_filter(package, self.filter)
                    && self.source.is_none_or(|source| package.source == source)
                    && (query.is_empty()
                        || package.name.to_lowercase().contains(&query)
                        || package.description.to_lowercase().contains(&query))
            })
            .map(|(idx, _)| idx)
            .collect();

        if self.filter == Filter::Updates {
            self.sort_updates_by_priority();
        } else {
            self.sort_favorites_then_name();
        }

        self.cursor = self.cursor.min(self.filtered.len().saturating_sub(1));
        self.clamp_task_cursor();
    }

    fn sort_updates_by_priority(&mut self) {
        if self.filtered.len() < 2 {
            return;
        }
        let packages: Vec<Package> = self
            .filtered
            .iter()
            .filter_map(|idx| self.packages.get(*idx).cloned())
            .collect();
        let ranked = update_center::classify_updates(&packages);
        let _summary = update_center::build_summary(&ranked);
        let _selected_updates = update_center::selected_packages(&ranked, &self.selected);
        let _all_updates = update_center::all_packages(&ranked);
        let _recommended_updates = update_center::recommended_packages(&ranked);
        if let Some(source) = self.source {
            let _source_updates = update_center::by_source_packages(&ranked, source);
        }
        let rank: HashMap<String, usize> = ranked
            .iter()
            .enumerate()
            .map(|(idx, candidate)| (candidate.package.id(), idx))
            .collect();
        self.filtered.sort_by_key(|idx| {
            rank.get(&self.packages[*idx].id())
                .copied()
                .unwrap_or(usize::MAX)
        });
    }

    fn sort_favorites_then_name(&mut self) {
        self.filtered.sort_by(|left_idx, right_idx| {
            let left = &self.packages[*left_idx];
            let right = &self.packages[*right_idx];
            let left_favorite = self.favorite_packages.contains(&left.id());
            let right_favorite = self.favorite_packages.contains(&right.id());

            right_favorite
                .cmp(&left_favorite)
                .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
                .then_with(|| left.source.cmp(&right.source))
        });
    }

    fn matches_filter(&self, package: &Package, filter: Filter) -> bool {
        match filter {
            Filter::All => true,
            Filter::Installed => matches!(
                package.status,
                PackageStatus::Installed
                    | PackageStatus::UpdateAvailable
                    | PackageStatus::Installing
                    | PackageStatus::Removing
                    | PackageStatus::Updating
            ),
            Filter::Updates => matches!(
                package.status,
                PackageStatus::UpdateAvailable | PackageStatus::Updating
            ),
            Filter::Favorites => {
                self.favorite_packages.contains(&package.id())
                    && (!self.favorites_updates_only
                        || matches!(
                            package.status,
                            PackageStatus::UpdateAvailable | PackageStatus::Updating
                        ))
            }
        }
    }

    pub fn start_loading(&mut self) -> bool {
        if self.loading {
            return false;
        }

        self.loading = true;
        self.cursor_anchor_id = self.current_package_id();
        self.set_status("Refreshing packages...", false);

        let (tx, rx) = mpsc::channel(1);
        self.load_rx = Some(rx);
        let pm = self.pm.clone();

        tokio::spawn(async move {
            let result: LoadResult = {
                let manager = pm.lock().await;
                match manager.list_all_installed().await {
                    Ok(installed) => match manager.check_all_updates().await {
                        Ok(updates) => Ok(Self::merge_installed_with_updates(installed, updates)),
                        Err(error) => Err(error.to_string()),
                    },
                    Err(error) => Err(error.to_string()),
                }
            };
            let _ = tx.send(result).await;
        });

        true
    }

    fn restore_cursor_anchor(&mut self) {
        let Some(anchor_id) = self.cursor_anchor_id.take() else {
            return;
        };
        if let Some((pos, _)) = self.filtered.iter().enumerate().find(|(_, idx)| {
            self.packages
                .get(**idx)
                .is_some_and(|pkg| pkg.id() == anchor_id)
        }) {
            self.cursor = pos;
        }
    }

    pub fn poll_loading(&mut self) {
        let Some(mut rx) = self.load_rx.take() else {
            return;
        };

        match rx.try_recv() {
            Ok(Ok(packages)) => {
                self.packages = packages;
                self.cleanup_stale_selections();
                self.previous_statuses.clear();
                self.apply_filters();
                self.restore_cursor_anchor();
                self.loading = false;
                self.set_status(
                    format!(
                        "Loaded {} packages ({} updates)",
                        self.filter_counts[0], self.filter_counts[2]
                    ),
                    true,
                );
            }
            Ok(Err(error)) => {
                self.loading = false;
                self.set_status(format!("Load error: {}", error), true);
            }
            Err(mpsc::error::TryRecvError::Empty) => {
                self.load_rx = Some(rx);
            }
            Err(mpsc::error::TryRecvError::Disconnected) => {
                self.loading = false;
                self.set_status("Load failed: channel disconnected", true);
            }
        }
    }

    async fn request_changelog_for_package(&mut self, package: Package, force_refresh: bool) {
        let package_id = package.id();
        let package_name = package.name.clone();

        if !force_refresh {
            match self.changelog_cache.get(&package_id) {
                Some(ChangelogState::Loading)
                | Some(ChangelogState::Ready { .. })
                | Some(ChangelogState::Empty) => return,
                Some(ChangelogState::Error(_)) | None => {}
            }
        } else if matches!(
            self.changelog_cache.get(&package_id),
            Some(ChangelogState::Loading)
        ) {
            self.set_status(
                format!("Changelog request already running for {}", package_name),
                true,
            );
            return;
        }

        self.changelog_cache
            .insert(package_id.clone(), ChangelogState::Loading);
        self.set_status(format!("Loading changelog for {}...", package_name), false);

        let Some(sender) = self.changelog_tx.clone() else {
            self.changelog_cache.insert(
                package_id,
                ChangelogState::Error("changelog channel unavailable".to_string()),
            );
            self.set_status("Unable to load changelog right now", true);
            return;
        };

        let pm = self.pm.clone();
        tokio::spawn(async move {
            let result = {
                let manager = pm.lock().await;
                manager.get_changelog(&package).await
            }
            .map_err(|error| error.to_string());

            let _ = sender
                .send(ChangelogResult {
                    package_id,
                    package_name,
                    result,
                })
                .await;
        });
    }

    async fn open_changelog_overlay(&mut self, force_refresh: bool) {
        let Some(package) = self.current_package().cloned() else {
            self.set_status("Select a package first", true);
            return;
        };

        self.showing_changelog = true;
        self.changelog_diff_only = matches!(
            package.status,
            PackageStatus::UpdateAvailable | PackageStatus::Updating
        ) && package.available_version.is_some();
        self.changelog_target_package_id = Some(package.id());
        self.changelog_scroll = 0;
        self.request_changelog_for_package(package, force_refresh)
            .await;
    }

    fn close_changelog_overlay(&mut self) {
        self.showing_changelog = false;
        self.changelog_diff_only = false;
        self.changelog_target_package_id = None;
        self.changelog_scroll = 0;
    }

    async fn refresh_changelog_overlay(&mut self) {
        let Some(package) = self.changelog_target_package().cloned() else {
            self.set_status("Package details are no longer available", true);
            return;
        };
        self.changelog_scroll = 0;
        self.request_changelog_for_package(package, true).await;
    }

    pub fn poll_changelog(&mut self) {
        let Some(mut rx) = self.changelog_rx.take() else {
            return;
        };

        let mut events = Vec::new();
        loop {
            match rx.try_recv() {
                Ok(event) => events.push(event),
                Err(mpsc::error::TryRecvError::Empty) => {
                    self.changelog_rx = Some(rx);
                    break;
                }
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    self.set_status("Changelog updates disconnected", true);
                    break;
                }
            }
        }

        for event in events {
            let is_active = self.changelog_target_package_id.as_deref() == Some(&event.package_id);
            match event.result {
                Ok(Some(content)) => {
                    let summary = ChangelogSummary::parse(&content);
                    self.changelog_cache
                        .insert(event.package_id, ChangelogState::Ready { content, summary });
                    if is_active {
                        self.set_status(
                            format!("Loaded changelog for {}", event.package_name),
                            true,
                        );
                    }
                }
                Ok(None) => {
                    self.changelog_cache
                        .insert(event.package_id, ChangelogState::Empty);
                    if is_active {
                        self.set_status(
                            format!("No changelog available for {}", event.package_name),
                            true,
                        );
                    }
                }
                Err(error) => {
                    self.changelog_cache
                        .insert(event.package_id, ChangelogState::Error(error.clone()));
                    if is_active {
                        self.set_status(
                            format!(
                                "Failed to load changelog for {}: {}",
                                event.package_name, error
                            ),
                            true,
                        );
                    }
                }
            }
        }
    }

    fn clear_preflight_verification_tracking(&mut self) {
        self.active_preflight_verification_id = None;
    }

    fn start_preflight_verification(&mut self, action: TaskQueueAction, packages: Vec<Package>) {
        self.clear_preflight_verification_tracking();

        let Some(sender) = self.preflight_verification_tx.clone() else {
            return;
        };
        if packages.is_empty()
            || !Self::preflight_dependency_verification_supported(action, &packages)
        {
            return;
        }

        let request_id = self.next_preflight_verification_id;
        self.next_preflight_verification_id = self.next_preflight_verification_id.saturating_add(1);
        self.active_preflight_verification_id = Some(request_id);

        let package_ids = Self::preflight_target_ids(&packages);
        let pm = self.pm.clone();
        let task = async move {
            let (dependency_impact_known, dependency_impact, note) =
                Self::run_preflight_verification_probe(pm, action, packages).await;
            let _ = sender
                .send(PreflightVerificationResult {
                    request_id,
                    action,
                    package_ids,
                    dependency_impact_known,
                    dependency_impact,
                    note,
                })
                .await;
        };

        match tokio::runtime::Handle::try_current() {
            Ok(handle) => {
                handle.spawn(task);
            }
            Err(_) => {
                self.clear_preflight_verification_tracking();
                if let Some(confirming) = self.confirming.as_mut() {
                    confirming.preflight.verification_in_progress = false;
                    Self::refresh_preflight_assessment(
                        &mut confirming.preflight,
                        Some(
                            "Dependency verification unavailable without async runtime; using estimated impact."
                                .to_string(),
                        ),
                    );
                }
            }
        }
    }

    async fn run_preflight_verification_probe(
        pm: Arc<Mutex<PackageManager>>,
        action: TaskQueueAction,
        packages: Vec<Package>,
    ) -> (bool, Option<PreflightDependencyImpact>, Option<String>) {
        let mut sources: HashSet<PackageSource> = HashSet::new();
        let mut unsupported_sources: HashSet<PackageSource> = HashSet::new();
        for package in &packages {
            sources.insert(package.source);
            if !Self::source_supports_dependency_verification(action, package.source) {
                unsupported_sources.insert(package.source);
            }
        }

        if !unsupported_sources.is_empty() {
            let mut names: Vec<String> = unsupported_sources
                .into_iter()
                .map(|source| source.to_string())
                .collect();
            names.sort();
            return (
                false,
                None,
                Some(format!(
                    "Dependency verification is unsupported for {}.",
                    names.join(", ")
                )),
            );
        }

        match action {
            TaskQueueAction::Remove => {
                let mut impact = PreflightDependencyImpact {
                    remove_count: packages.len(),
                    ..PreflightDependencyImpact::default()
                };
                let manager = pm.lock().await;
                for package in &packages {
                    if let Err(error) = manager.get_reverse_dependencies(package).await {
                        let detail = error
                            .to_string()
                            .lines()
                            .next()
                            .unwrap_or("unknown verification error")
                            .to_string();
                        return (
                            false,
                            None,
                            Some(format!(
                                "Dependency probe failed for {} ({}): {}",
                                package.name, package.source, detail
                            )),
                        );
                    }
                }
                impact.held_back_count = 0;
                let mut source_names: Vec<String> = sources
                    .into_iter()
                    .map(|source| source.to_string())
                    .collect();
                source_names.sort();
                return (
                    true,
                    Some(impact),
                    Some(format!(
                        "Dependency impact verified via reverse dependency probes for {}.",
                        source_names.join(", ")
                    )),
                );
            }
            TaskQueueAction::Install | TaskQueueAction::Update => {}
        }

        let mut packages_by_source: HashMap<PackageSource, Vec<String>> = HashMap::new();
        for package in &packages {
            packages_by_source
                .entry(package.source)
                .or_default()
                .push(package.name.clone());
        }

        let mut combined = PreflightDependencyImpact::default();
        for (source, names) in packages_by_source {
            let mut names = names;
            names.sort();
            names.dedup();

            let source_impact = match source {
                PackageSource::Apt => Self::probe_apt_transaction_impact(action, &names).await,
                PackageSource::Dnf => Self::probe_dnf_transaction_impact(action, &names).await,
                _ => Err(format!(
                    "Dependency verification is unsupported for {}.",
                    source
                )),
            };

            match source_impact {
                Ok(impact) => combined.merge(&impact),
                Err(detail) => {
                    return (
                        false,
                        None,
                        Some(format!("{} probe failed: {}", source, detail)),
                    );
                }
            }
        }

        let mut source_names: Vec<String> = sources
            .into_iter()
            .map(|source| source.to_string())
            .collect();
        source_names.sort();

        (
            true,
            Some(combined),
            Some(format!(
                "Dependency impact verified via dry-run transaction probes for {}.",
                source_names.join(", ")
            )),
        )
    }

    async fn probe_apt_transaction_impact(
        action: TaskQueueAction,
        package_names: &[String],
    ) -> std::result::Result<PreflightDependencyImpact, String> {
        let mut args = vec!["-s".to_string(), "install".to_string()];
        if action == TaskQueueAction::Update {
            args.push("--only-upgrade".to_string());
        }
        args.extend(package_names.iter().cloned());

        let output = Command::new("apt-get")
            .args(&args)
            .output()
            .await
            .map_err(|error| format!("failed to run apt-get simulation: {}", error))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let merged = format!("{}\n{}", stdout, stderr);
        if let Some(impact) = Self::parse_apt_dry_run_impact(&merged) {
            return Ok(impact);
        }

        let detail = Self::first_non_empty_line(&stderr)
            .or_else(|| Self::first_non_empty_line(&stdout))
            .unwrap_or_else(|| format!("exit status {}", output.status));
        Err(format!(
            "apt-get simulation did not expose transaction summary ({})",
            detail
        ))
    }

    async fn probe_dnf_transaction_impact(
        action: TaskQueueAction,
        package_names: &[String],
    ) -> std::result::Result<PreflightDependencyImpact, String> {
        let mut args = vec![
            match action {
                TaskQueueAction::Install => "install".to_string(),
                TaskQueueAction::Update => "upgrade".to_string(),
                TaskQueueAction::Remove => "remove".to_string(),
            },
            "--assumeno".to_string(),
            "--setopt=tsflags=test".to_string(),
        ];
        args.extend(package_names.iter().cloned());

        let output = Command::new("dnf")
            .args(&args)
            .output()
            .await
            .map_err(|error| format!("failed to run dnf simulation: {}", error))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let merged = format!("{}\n{}", stdout, stderr);
        if let Some(impact) = Self::parse_dnf_dry_run_impact(&merged) {
            return Ok(impact);
        }

        let detail = Self::first_non_empty_line(&stderr)
            .or_else(|| Self::first_non_empty_line(&stdout))
            .unwrap_or_else(|| format!("exit status {}", output.status));
        Err(format!(
            "dnf simulation did not expose transaction summary ({})",
            detail
        ))
    }

    fn parse_apt_dry_run_impact(output: &str) -> Option<PreflightDependencyImpact> {
        for line in output.lines() {
            let normalized = line.trim();
            if !normalized.contains("upgraded")
                || !normalized.contains("newly installed")
                || !normalized.contains("to remove")
            {
                continue;
            }

            let values: Vec<usize> = normalized
                .split(|ch: char| !ch.is_ascii_digit())
                .filter(|part| !part.is_empty())
                .filter_map(|part| part.parse::<usize>().ok())
                .collect();
            if values.len() < 4 {
                continue;
            }

            return Some(PreflightDependencyImpact {
                upgrade_count: values[0],
                install_count: values[1],
                remove_count: values[2],
                held_back_count: values[3],
            });
        }

        None
    }

    fn parse_dnf_dry_run_impact(output: &str) -> Option<PreflightDependencyImpact> {
        let mut in_summary = false;
        let mut impact = PreflightDependencyImpact::default();

        for line in output.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                if in_summary && impact.has_changes() {
                    break;
                }
                continue;
            }

            if trimmed.eq_ignore_ascii_case("Transaction Summary") {
                in_summary = true;
                continue;
            }
            if !in_summary {
                continue;
            }

            let Some(value) = Self::first_usize(trimmed) else {
                continue;
            };
            let lower = trimmed.to_ascii_lowercase();
            if lower.starts_with("install") || lower.starts_with("reinstall") {
                impact.install_count += value;
            } else if lower.starts_with("upgrade")
                || lower.starts_with("upgrading")
                || lower.starts_with("downgrade")
            {
                impact.upgrade_count += value;
            } else if lower.starts_with("remove")
                || lower.starts_with("removing")
                || lower.starts_with("obsoleting")
                || lower.starts_with("erase")
            {
                impact.remove_count += value;
            } else if lower.starts_with("skip") {
                impact.held_back_count += value;
            }
        }

        impact.has_changes().then_some(impact)
    }

    fn first_usize(text: &str) -> Option<usize> {
        text.split(|ch: char| !ch.is_ascii_digit())
            .find(|part| !part.is_empty())
            .and_then(|part| part.parse::<usize>().ok())
    }

    fn first_non_empty_line(text: &str) -> Option<String> {
        text.lines()
            .map(str::trim)
            .find(|line| !line.is_empty())
            .map(ToString::to_string)
    }

    pub fn poll_preflight_verification(&mut self) {
        let Some(mut rx) = self.preflight_verification_rx.take() else {
            return;
        };

        let mut events = Vec::new();
        loop {
            match rx.try_recv() {
                Ok(event) => events.push(event),
                Err(mpsc::error::TryRecvError::Empty) => {
                    self.preflight_verification_rx = Some(rx);
                    break;
                }
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    self.preflight_verification_rx = None;
                    self.clear_preflight_verification_tracking();
                    break;
                }
            }
        }

        for event in events {
            let Some(active_request_id) = self.active_preflight_verification_id else {
                continue;
            };
            if event.request_id != active_request_id {
                continue;
            }

            let Some(confirming) = self.confirming.as_mut() else {
                self.clear_preflight_verification_tracking();
                continue;
            };
            if confirming.action != event.action
                || Self::preflight_target_ids(&confirming.packages) != event.package_ids
            {
                continue;
            }

            confirming.preflight.dependency_impact_known = event.dependency_impact_known;
            confirming.preflight.dependency_impact = event.dependency_impact;
            confirming.preflight.verification_in_progress = false;
            Self::refresh_preflight_assessment(&mut confirming.preflight, event.note);
            self.clear_preflight_verification_tracking();
        }
    }

    fn merge_installed_with_updates(
        mut installed: Vec<Package>,
        updates: Vec<Package>,
    ) -> Vec<Package> {
        let mut by_id: HashMap<String, usize> = HashMap::new();
        for (idx, package) in installed.iter().enumerate() {
            by_id.insert(package.id(), idx);
        }

        for update in updates {
            let update_id = update.id();
            if let Some(position) = by_id.get(&update_id).copied() {
                if let Some(existing) = installed.get_mut(position) {
                    existing.status = PackageStatus::UpdateAvailable;
                    existing.available_version = update.available_version.clone();
                    if !update.version.is_empty() {
                        existing.version = update.version;
                    }
                    if !update.description.is_empty() {
                        existing.description = update.description;
                    }
                    existing.update_category = update.update_category;
                }
            } else {
                by_id.insert(update_id, installed.len());
                installed.push(update);
            }
        }

        installed.sort_by(|a, b| {
            a.name
                .to_lowercase()
                .cmp(&b.name.to_lowercase())
                .then_with(|| a.source.cmp(&b.source))
        });
        installed
    }

    fn cleanup_stale_selections(&mut self) {
        let valid: HashSet<String> = self.packages.iter().map(Package::id).collect();
        self.selected.retain(|id| valid.contains(id));
    }

    pub fn poll_task_events(&mut self) {
        let Some(mut rx) = self.task_events_rx.take() else {
            return;
        };

        let running_before = self
            .tasks
            .iter()
            .any(|task| task.status == TaskQueueStatus::Running);

        let mut changed = false;
        loop {
            match rx.try_recv() {
                Ok(event) => {
                    self.apply_task_event(event);
                    changed = true;
                }
                Err(mpsc::error::TryRecvError::Empty) => {
                    self.task_events_rx = Some(rx);
                    break;
                }
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    self.task_events_rx = None;
                    break;
                }
            }
        }

        let running_after = self
            .tasks
            .iter()
            .any(|task| task.status == TaskQueueStatus::Running);

        if changed && running_before && !running_after {
            self.queue_completed_at = Some(Instant::now());
        }

        if changed && self.refresh_after_idle && !self.loading && !running_after {
            self.refresh_after_idle = false;
            let _ = self.start_loading();
        }
    }

    fn upsert_task(&mut self, entry: TaskQueueEntry) {
        if let Some(position) = self.tasks.iter().position(|task| task.id == entry.id) {
            self.tasks[position] = entry;
        } else {
            self.tasks.push(entry);
        }
        self.clamp_task_cursor();
    }

    fn find_package_pos_by_id(&self, package_id: &str) -> Option<usize> {
        self.packages
            .iter()
            .position(|package| package.id() == package_id)
    }

    fn mark_package_started(&mut self, entry: &TaskQueueEntry) {
        if let Some(position) = self.find_package_pos_by_id(&entry.package_id) {
            if let Some(package) = self.packages.get(position) {
                self.previous_statuses
                    .entry(entry.package_id.clone())
                    .or_insert(package.status);
            }
            if let Some(package) = self.packages.get_mut(position) {
                package.status = match entry.action {
                    TaskQueueAction::Install => PackageStatus::Installing,
                    TaskQueueAction::Remove => PackageStatus::Removing,
                    TaskQueueAction::Update => PackageStatus::Updating,
                };
            }
        }
    }

    fn mark_package_completed(&mut self, entry: &TaskQueueEntry) {
        match entry.action {
            TaskQueueAction::Install => {
                if let Some(position) = self.find_package_pos_by_id(&entry.package_id) {
                    if let Some(package) = self.packages.get_mut(position) {
                        package.status = PackageStatus::Installed;
                    }
                }
            }
            TaskQueueAction::Remove => {
                if let Some(position) = self.find_package_pos_by_id(&entry.package_id) {
                    self.packages.remove(position);
                    self.selected.remove(&entry.package_id);
                }
            }
            TaskQueueAction::Update => {
                if let Some(position) = self.find_package_pos_by_id(&entry.package_id) {
                    if let Some(package) = self.packages.get_mut(position) {
                        package.status = PackageStatus::Installed;
                        package.available_version = None;
                    }
                }
            }
        }
        self.previous_statuses.remove(&entry.package_id);
    }

    fn mark_package_failed(&mut self, entry: &TaskQueueEntry) {
        if let Some(position) = self.find_package_pos_by_id(&entry.package_id) {
            let fallback = match entry.action {
                TaskQueueAction::Install => PackageStatus::NotInstalled,
                TaskQueueAction::Remove | TaskQueueAction::Update => PackageStatus::Installed,
            };
            let status = self
                .previous_statuses
                .remove(&entry.package_id)
                .unwrap_or(fallback);
            if let Some(package) = self.packages.get_mut(position) {
                package.status = status;
            }
        }
    }

    fn append_task_log(&mut self, entry_id: &str, line: String) {
        let pin_manual_scroll = self
            .tasks
            .get(self.task_cursor)
            .is_some_and(|task| task.id == entry_id)
            && self.task_log_scroll > 0;

        let logs = self.task_logs.entry(entry_id.to_string()).or_default();
        logs.push_back(line);
        self.task_last_log_at
            .insert(entry_id.to_string(), Instant::now());
        while logs.len() > MAX_TASK_LOG_LINES {
            logs.pop_front();
        }

        // Keep older logs stable while the operator is manually scrolled up.
        if pin_manual_scroll {
            self.task_log_scroll = (self.task_log_scroll + 1).min(self.queue_log_max_scroll());
        }
    }

    fn cleanup_task_logs(&mut self) {
        let valid: HashSet<&str> = self.tasks.iter().map(|task| task.id.as_str()).collect();
        self.task_logs
            .retain(|task_id, _| valid.contains(task_id.as_str()));
        self.task_last_log_at
            .retain(|task_id, _| valid.contains(task_id.as_str()));
        self.task_failure_categories
            .retain(|task_id: &String, _| valid.contains(task_id.as_str()));
        self.task_recovery_states
            .retain(|task_id: &String, _| valid.contains(task_id.as_str()));
        self.retry_parent.retain(|task_id, parent| {
            valid.contains(task_id.as_str()) && valid.contains(parent.as_str())
        });
        self.retry_attempt
            .retain(|task_id, _| valid.contains(task_id.as_str()));
    }

    fn session_queue_entries(
        entries: Vec<TaskQueueEntry>,
        retain_history: bool,
    ) -> Vec<TaskQueueEntry> {
        if retain_history {
            return entries;
        }

        entries
            .into_iter()
            .filter(|task| {
                matches!(
                    task.status,
                    TaskQueueStatus::Queued | TaskQueueStatus::Running
                )
            })
            .collect()
    }

    fn apply_task_event(&mut self, event: TaskQueueEvent) {
        match event {
            TaskQueueEvent::Started(entry) => {
                self.task_failure_categories.remove(&entry.id);
                self.task_last_log_at.remove(&entry.id);
                self.upsert_task(entry.clone());
                self.mark_package_started(&entry);
                self.queue_completed_at = None;
                self.queue_completion_digest_emitted = false;
                self.apply_filters();
            }
            TaskQueueEvent::Completed(entry) => {
                if let Some(parent) = self.retry_parent.get(&entry.id).cloned() {
                    let state = self.task_recovery_states.entry(parent.clone()).or_default();
                    state.last_outcome = Some(TaskQueueStatus::Completed);
                    self.set_status(
                        format!("Recovery retry succeeded for {}", entry.package_name),
                        true,
                    );
                }
                self.task_failure_categories.remove(&entry.id);
                self.upsert_task(entry.clone());
                self.mark_package_completed(&entry);
                self.refresh_after_idle = true;
                self.apply_filters();
            }
            TaskQueueEvent::Failed(entry) => {
                let category = entry
                    .error
                    .as_deref()
                    .map(Self::classify_failure)
                    .unwrap_or(FailureCategory::Unknown);
                self.task_failure_categories
                    .insert(entry.id.clone(), category);

                if let Some(parent) = self.retry_parent.get(&entry.id).cloned() {
                    let state = self.task_recovery_states.entry(parent).or_default();
                    state.last_outcome = Some(TaskQueueStatus::Failed);
                    self.set_status(
                        format!(
                            "Recovery retry failed [{}]. {}",
                            category.code(),
                            category.action_hint()
                        ),
                        true,
                    );
                } else {
                    self.set_status(
                        format!(
                            "{} failed for {} [{}]. {}",
                            action_label(entry.action),
                            entry.package_name,
                            category.code(),
                            category.action_hint()
                        ),
                        true,
                    );
                }

                self.upsert_task(entry.clone());
                self.mark_package_failed(&entry);
                self.refresh_after_idle = true;
                self.queue_failures_acknowledged = false;
                self.apply_filters();
            }
            TaskQueueEvent::Log { entry_id, line } => {
                let (kind, text) = match line {
                    crate::backend::streaming::StreamLine::Stdout(text) => ("OUT", text),
                    crate::backend::streaming::StreamLine::Stderr(text) => ("ERR", text),
                };
                debug!(entry_id = %entry_id, kind = %kind, line = %text, "task output");
                self.append_task_log(&entry_id, format!("[{}] {}", kind, text));
            }
        }
        self.ensure_queue_cursor_matches_filter();
        self.cleanup_task_logs();
        self.maybe_emit_queue_completion_digest();
    }

    fn clamp_task_cursor(&mut self) {
        self.set_task_cursor(self.task_cursor);
    }

    fn set_task_cursor(&mut self, cursor: usize) {
        let next = if self.tasks.is_empty() {
            0
        } else {
            cursor.min(self.tasks.len() - 1)
        };

        if self.task_cursor != next {
            self.task_cursor = next;
            self.task_log_scroll = 0;
        } else {
            self.task_cursor = next;
        }
    }

    pub fn maybe_autohide_queue(&mut self) {
        if self.queue_expanded || self.tasks.is_empty() {
            return;
        }
        let has_running_or_queued = self.tasks.iter().any(|task| {
            matches!(
                task.status,
                TaskQueueStatus::Queued | TaskQueueStatus::Running
            )
        });
        if has_running_or_queued {
            return;
        }

        let has_failures = self
            .tasks
            .iter()
            .any(|task| self.queue_lane_for_task(task) == QueueJourneyLane::NeedsAttention);
        if has_failures && !self.queue_failures_acknowledged {
            return;
        }

        let Some(completed_at) = self.queue_completed_at else {
            self.queue_completed_at = Some(Instant::now());
            return;
        };

        if completed_at.elapsed() > QUEUE_AUTO_HIDE_AFTER {
            self.tasks.clear();
            self.task_logs.clear();
            self.task_last_log_at.clear();
            self.previous_statuses.clear();
            self.task_failure_categories.clear();
            self.task_recovery_states.clear();
            self.retry_parent.clear();
            self.retry_attempt.clear();
            self.task_cursor = 0;
            self.task_log_scroll = 0;
            self.queue_completed_at = None;
            self.queue_failures_acknowledged = false;
            self.queue_completion_digest_emitted = false;
            self.queue_failure_filter = QueueFailureFilter::All;
        }
    }

    fn maybe_emit_queue_completion_digest(&mut self) {
        if self.queue_completion_digest_emitted || self.tasks.is_empty() {
            return;
        }

        if self.tasks.iter().any(|task| {
            matches!(
                task.status,
                TaskQueueStatus::Queued | TaskQueueStatus::Running
            )
        }) {
            return;
        }

        let (_, _, completed, failed, cancelled) = self.queue_counts();
        let recovered = self
            .tasks
            .iter()
            .filter(|task| {
                task.status == TaskQueueStatus::Failed
                    && self
                        .recovery_state_for_task(&task.id)
                        .is_some_and(|state| state.last_outcome == Some(TaskQueueStatus::Completed))
            })
            .count();

        let mut message = format!("Queue finished: {} done", completed + cancelled);
        if recovered > 0 {
            message.push_str(&format!(", {} recovered", recovered));
        }
        if failed > 0 {
            message.push_str(&format!(", {} need attention", failed));
        }
        message.push('.');
        if failed > 0 {
            if self.retryable_failed_task_count() > 0 {
                message.push_str(" Press A for safe retries or l for failures.");
            } else {
                message.push_str(" Press l for failure details.");
            }
        } else {
            message.push_str(" Press l for details.");
        }
        self.set_status(message, true);
        self.queue_completion_digest_emitted = true;
    }

    pub fn toggle_queue_expanded(&mut self) {
        if self.tasks.is_empty() {
            self.set_status("No tasks in queue", true);
            return;
        }
        self.queue_expanded = !self.queue_expanded;
        if self.queue_expanded {
            self.focus = Focus::Queue;
            self.task_log_scroll = 0;
            self.ensure_queue_cursor_matches_filter();
            if self
                .tasks
                .iter()
                .any(|task| self.queue_lane_for_task(task) == QueueJourneyLane::NeedsAttention)
            {
                self.queue_failures_acknowledged = true;
            }
        } else if self.focus == Focus::Queue {
            self.focus = Focus::Packages;
        }
    }

    fn next_package(&mut self) {
        if self.filtered.is_empty() {
            self.cursor = 0;
            return;
        }
        self.cursor = (self.cursor + 1).min(self.filtered.len() - 1);
    }

    fn prev_package(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    fn page_down(&mut self) {
        if self.filtered.is_empty() {
            return;
        }
        self.cursor = (self.cursor + HALF_PAGE).min(self.filtered.len() - 1);
    }

    fn page_up(&mut self) {
        self.cursor = self.cursor.saturating_sub(HALF_PAGE);
    }

    fn top(&mut self) {
        self.cursor = 0;
    }

    fn bottom(&mut self) {
        if !self.filtered.is_empty() {
            self.cursor = self.filtered.len() - 1;
        }
    }

    fn queue_top(&mut self) {
        let visible = self.queue_visible_task_indices();
        if let Some(first) = visible.first().copied() {
            self.set_task_cursor(first);
        }
    }

    fn queue_bottom(&mut self) {
        let visible = self.queue_visible_task_indices();
        if let Some(last) = visible.last().copied() {
            self.set_task_cursor(last);
        }
    }

    fn queue_next(&mut self) {
        let visible = self.queue_visible_task_indices();
        if visible.is_empty() {
            return;
        }
        let position = self.queue_visible_cursor_position(&visible);
        let next = (position + 1).min(visible.len() - 1);
        self.set_task_cursor(visible[next]);
    }

    fn queue_prev(&mut self) {
        let visible = self.queue_visible_task_indices();
        if visible.is_empty() {
            return;
        }
        let position = self.queue_visible_cursor_position(&visible);
        let previous = position.saturating_sub(1);
        self.set_task_cursor(visible[previous]);
    }

    fn queue_page_down(&mut self) {
        let visible = self.queue_visible_task_indices();
        if visible.is_empty() {
            return;
        }
        let position = self.queue_visible_cursor_position(&visible);
        let next = (position + HALF_PAGE).min(visible.len() - 1);
        self.set_task_cursor(visible[next]);
    }

    fn queue_page_up(&mut self) {
        let visible = self.queue_visible_task_indices();
        if visible.is_empty() {
            return;
        }
        let position = self.queue_visible_cursor_position(&visible);
        let previous = position.saturating_sub(HALF_PAGE);
        self.set_task_cursor(visible[previous]);
    }

    fn queue_log_scroll_up(&mut self) {
        self.task_log_scroll = (self.task_log_scroll + 1).min(self.queue_log_max_scroll());
    }

    fn queue_log_scroll_down(&mut self) {
        self.task_log_scroll = self.task_log_scroll.saturating_sub(1);
    }

    fn has_active_queue_tasks(&self) -> bool {
        self.tasks.iter().any(|task| {
            matches!(
                task.status,
                TaskQueueStatus::Queued | TaskQueueStatus::Running
            )
        })
    }

    fn has_active_task_for_package(&self, package_id: &str) -> bool {
        self.tasks.iter().any(|task| {
            task.package_id == package_id
                && matches!(
                    task.status,
                    TaskQueueStatus::Queued | TaskQueueStatus::Running
                )
        })
    }

    fn prune_terminal_tasks(&mut self) -> bool {
        let original_len = self.tasks.len();
        self.tasks.retain(|task| !task.status.is_terminal());
        self.cleanup_task_logs();
        self.clamp_task_cursor();
        self.ensure_queue_cursor_matches_filter();
        if self.tasks.is_empty() {
            self.queue_completed_at = None;
            self.queue_failures_acknowledged = false;
        }
        self.tasks.len() != original_len
    }

    async fn persist_task_queue_state(&mut self) {
        let persisted = {
            let mut guard = self.history_tracker.lock().await;
            if let Some(tracker) = guard.as_mut() {
                tracker
                    .replace_task_queue(self.tasks.clone())
                    .await
                    .map(|_| true)
            } else {
                Ok(false)
            }
        };

        if let Err(error) = persisted {
            self.set_status(format!("Failed to persist task queue: {}", error), true);
        }
    }

    fn collect_action_targets(&self) -> Vec<Package> {
        if self.selected.is_empty() {
            return self.current_package().cloned().into_iter().collect();
        }
        self.packages
            .iter()
            .filter(|package| self.selected.contains(&package.id()))
            .cloned()
            .collect()
    }

    fn is_valid_target(action: TaskQueueAction, package: &Package) -> bool {
        match action {
            TaskQueueAction::Install => package.status == PackageStatus::NotInstalled,
            TaskQueueAction::Remove => {
                matches!(
                    package.status,
                    PackageStatus::Installed | PackageStatus::UpdateAvailable
                )
            }
            TaskQueueAction::Update => package.status == PackageStatus::UpdateAvailable,
        }
    }

    fn invalid_single_target_message(action: TaskQueueAction, package: &Package) -> String {
        match action {
            TaskQueueAction::Install => format!("{} is already installed", package.name),
            TaskQueueAction::Remove => format!("{} is not installed", package.name),
            TaskQueueAction::Update => format!("{} has no update available", package.name),
        }
    }

    fn invalid_batch_message(action: TaskQueueAction) -> &'static str {
        match action {
            TaskQueueAction::Install => "No installable packages in selection",
            TaskQueueAction::Remove => "No removable packages in selection",
            TaskQueueAction::Update => "No updatable packages in selection",
        }
    }

    fn skipped_reason(action: TaskQueueAction) -> &'static str {
        match action {
            TaskQueueAction::Install => "already installed",
            TaskQueueAction::Remove => "not installed",
            TaskQueueAction::Update => "already current",
        }
    }

    fn queued_result_message(action: TaskQueueAction, queued: usize) -> String {
        if queued == 0 {
            return format!(
                "No new {} tasks queued (already queued/running)",
                action_label(action).to_lowercase()
            );
        }

        format!(
            "Queued {} {} task{}",
            queued,
            action_label(action).to_lowercase(),
            if queued == 1 { "" } else { "s" }
        )
    }

    fn build_confirm_label(
        action: TaskQueueAction,
        valid: &[Package],
        total: usize,
        skipped: usize,
        selection_mode: bool,
    ) -> String {
        let verb = action_label(action);

        if !selection_mode && valid.len() == 1 {
            return format!("{} {}?", verb, valid[0].name);
        }

        if skipped > 0 {
            return format!(
                "{} {} of {} selected ({} {})?",
                verb,
                valid.len(),
                total,
                skipped,
                Self::skipped_reason(action)
            );
        }

        if valid.len() <= 3 {
            let names = valid
                .iter()
                .map(|package| package.name.clone())
                .collect::<Vec<_>>()
                .join(", ");
            return format!("{} {}?", verb, names);
        }

        format!("{} {} packages?", verb, valid.len())
    }

    fn prepare_action_for_targets(
        &mut self,
        action: TaskQueueAction,
        targets: Vec<Package>,
        selection_mode: bool,
    ) {
        if targets.is_empty() {
            self.clear_preflight_verification_tracking();
            self.set_status("No package selected", true);
            return;
        }

        let valid: Vec<Package> = targets
            .iter()
            .filter(|package| Self::is_valid_target(action, package))
            .cloned()
            .collect();

        if valid.is_empty() {
            self.clear_preflight_verification_tracking();
            if !selection_mode {
                if let Some(target) = targets.first() {
                    self.set_status(Self::invalid_single_target_message(action, target), true);
                }
            } else {
                self.set_status(Self::invalid_batch_message(action), true);
            }
            return;
        }

        let skipped = targets.len().saturating_sub(valid.len());
        let label =
            Self::build_confirm_label(action, &valid, targets.len(), skipped, selection_mode);
        let preflight = Self::build_preflight_summary(action, &targets, &valid, selection_mode);
        let verification_in_progress = preflight.verification_in_progress;
        let risk_acknowledged = preflight.risk_level != PreflightRiskLevel::High;
        let verification_targets = valid.clone();

        self.confirming = Some(PendingAction {
            label,
            packages: valid,
            action,
            preflight,
            risk_acknowledged,
        });

        if verification_in_progress {
            self.start_preflight_verification(action, verification_targets);
        } else {
            self.clear_preflight_verification_tracking();
        }
    }

    fn prepare_action(&mut self, action: TaskQueueAction) {
        let selection_mode = !self.selected.is_empty();
        let targets = self.collect_action_targets();
        self.prepare_action_for_targets(action, targets, selection_mode);
    }

    fn toggle_selection_on_cursor(&mut self) {
        let Some(package) = self.current_package() else {
            return;
        };
        let package_id = package.id();
        if self.selected.contains(&package_id) {
            self.selected.remove(&package_id);
        } else {
            self.selected.insert(package_id);
        }
    }

    fn toggle_favorite_on_cursor(&mut self) {
        let Some(package) = self.current_package() else {
            return;
        };

        let package_id = package.id();
        let package_name = package.name.clone();

        let added = if self.favorite_packages.contains(&package_id) {
            self.favorite_packages.remove(&package_id);
            false
        } else {
            self.favorite_packages.insert(package_id);
            true
        };

        self.favorites_dirty = true;
        self.apply_filters();

        match self.persist_favorites() {
            Ok(()) => {
                if added {
                    self.set_status(format!("Added {} to favorites", package_name), true);
                } else {
                    self.set_status(format!("Removed {} from favorites", package_name), true);
                }
            }
            Err(error) => {
                self.set_status(format!("Failed to save favorites: {}", error), true);
            }
        }
    }

    fn favorite_toggle_targets(&self) -> Vec<String> {
        if self.selected.is_empty() {
            return self
                .current_package()
                .map(Package::id)
                .into_iter()
                .collect();
        }

        self.packages
            .iter()
            .filter(|package| self.selected.contains(&package.id()))
            .map(Package::id)
            .collect()
    }

    fn bulk_toggle_favorites(&mut self) {
        let mut targets = self.favorite_toggle_targets();
        if targets.is_empty() {
            self.set_status("No packages selected", true);
            return;
        }

        targets.sort();
        targets.dedup();

        let all_favorited = targets
            .iter()
            .all(|package_id| self.favorite_packages.contains(package_id));

        if all_favorited {
            for package_id in &targets {
                self.favorite_packages.remove(package_id);
            }
        } else {
            for package_id in &targets {
                self.favorite_packages.insert(package_id.clone());
            }
        }

        self.favorites_dirty = true;
        self.apply_filters();

        let message = if all_favorited {
            format!(
                "Removed {} package{} from favorites",
                targets.len(),
                if targets.len() == 1 { "" } else { "s" }
            )
        } else {
            format!(
                "Added {} package{} to favorites",
                targets.len(),
                if targets.len() == 1 { "" } else { "s" }
            )
        };

        match self.persist_favorites() {
            Ok(()) => self.set_status(message, true),
            Err(error) => self.set_status(format!("Failed to save favorites: {}", error), true),
        }
    }

    fn toggle_favorites_updates_only(&mut self) {
        self.favorites_updates_only = !self.favorites_updates_only;
        self.apply_filters();
        if self.favorites_updates_only {
            self.set_status("Favorites mode: updates only", true);
        } else {
            self.set_status("Favorites mode: all favorites", true);
        }
    }

    fn open_palette(&mut self) {
        self.showing_help = false;
        self.searching = false;
        self.showing_palette = true;
        self.palette_query.clear();
        self.palette_cursor = 0;
    }

    fn close_palette(&mut self) {
        self.showing_palette = false;
        self.palette_query.clear();
        self.palette_cursor = 0;
    }

    fn select_all_visible(&mut self) {
        for index in &self.filtered {
            if let Some(package) = self.packages.get(*index) {
                self.selected.insert(package.id());
            }
        }
    }

    fn clear_selection(&mut self) {
        self.selected.clear();
    }

    async fn handle_help_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Char('?') => self.showing_help = false,
            _ => {}
        }
    }

    async fn handle_confirm_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                if let Some(confirming) = self.confirming.as_mut() {
                    if confirming.preflight.risk_level == PreflightRiskLevel::High
                        && !confirming.risk_acknowledged
                    {
                        confirming.risk_acknowledged = true;
                        self.set_status(
                            "High-risk operation acknowledged. Press y again to queue.",
                            true,
                        );
                        return;
                    }
                }

                if let Some(action) = self.confirming.take() {
                    self.clear_preflight_verification_tracking();
                    let queued = self.queue_tasks(action.packages, action.action).await;
                    self.clear_selection();
                    self.set_status(Self::queued_result_message(action.action, queued), true);
                }
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                self.confirming = None;
                self.clear_preflight_verification_tracking();
                self.set_status("Cancelled", true);
            }
            _ => {}
        }
    }

    fn handle_search_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.searching = false;
                self.search.clear();
                self.apply_filters();
                self.set_status("Search cleared", true);
            }
            KeyCode::Enter => {
                self.searching = false;
                self.apply_filters();
            }
            KeyCode::Backspace | KeyCode::Delete => {
                self.search.pop();
                self.apply_filters();
            }
            KeyCode::Char(ch) if !ch.is_control() => {
                self.search.push(ch);
                self.apply_filters();
            }
            _ => {}
        }
    }

    async fn handle_queue_shortcuts(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('c') | KeyCode::Char('C') => {
                self.execute_command(CommandId::QueueCancel).await;
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                self.execute_command(CommandId::QueueRetry).await;
            }
            KeyCode::Char('a') | KeyCode::Char('A') => {
                self.execute_command(CommandId::QueueRetrySafe).await;
            }
            KeyCode::Char('m') | KeyCode::Char('M') => {
                self.execute_command(CommandId::QueueRemediate).await;
            }
            KeyCode::Char('0') => self.set_queue_failure_filter(QueueFailureFilter::All),
            KeyCode::Char('1') => self.set_queue_failure_filter(QueueFailureFilter::Permissions),
            KeyCode::Char('2') => self.set_queue_failure_filter(QueueFailureFilter::Network),
            KeyCode::Char('3') => self.set_queue_failure_filter(QueueFailureFilter::Conflict),
            KeyCode::Char('4') => self.set_queue_failure_filter(QueueFailureFilter::Other),
            _ => {}
        }
    }

    pub async fn queue_tasks(&mut self, packages: Vec<Package>, action: TaskQueueAction) -> usize {
        if !self.has_active_queue_tasks() && self.prune_terminal_tasks() {
            self.persist_task_queue_state().await;
        }

        let mut queued = 0usize;
        let mut seen_ids = HashSet::new();

        for package in packages {
            let package_id = package.id();
            if !seen_ids.insert(package_id.clone()) {
                continue;
            }
            if self.has_active_task_for_package(&package_id) {
                continue;
            }

            let entry =
                TaskQueueEntry::new(action, package_id, package.name.clone(), package.source);
            self.enqueue_task_entry(entry).await;
            queued += 1;
        }
        if queued > 0 {
            self.queue_completed_at = None;
            self.queue_completion_digest_emitted = false;
            self.ensure_queue_cursor_matches_filter();
            self.spawn_task_executor();
        }
        queued
    }

    async fn enqueue_task_entry(&mut self, entry: TaskQueueEntry) {
        let persisted = {
            let mut guard = self.history_tracker.lock().await;
            if let Some(tracker) = guard.as_mut() {
                tracker.enqueue_task(entry.clone()).await.map(|_| true)
            } else {
                Ok(false)
            }
        };

        if let Err(error) = persisted {
            self.set_status(format!("Failed to persist task: {}", error), true);
        }

        self.upsert_task(entry);
    }

    fn spawn_task_executor(&self) {
        if self.executor_running.swap(true, Ordering::SeqCst) {
            return;
        }

        let running = self.executor_running.clone();
        let pm = self.pm.clone();
        let history_tracker = self.history_tracker.clone();
        let sender = self.task_events_tx.clone();

        tokio::spawn(async move {
            loop {
                let executor = TaskQueueExecutor::new(pm.clone(), history_tracker.clone());
                if let Err(error) = executor.run(sender.clone()).await {
                    error!(error = %error, "Task queue executor stopped");
                }

                // Release the running flag before checking again so a newly queued batch can
                // either restart us externally or be picked up by this worker.
                running.store(false, Ordering::SeqCst);

                let has_pending = {
                    let guard = history_tracker.lock().await;
                    guard.as_ref().is_some_and(|tracker| {
                        tracker
                            .history()
                            .task_queue
                            .entries
                            .iter()
                            .any(|entry| entry.status == TaskQueueStatus::Queued)
                    })
                };

                if !has_pending {
                    break;
                }

                if running
                    .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
                    .is_err()
                {
                    break;
                }
            }
        });
    }

    async fn cancel_selected_task(&mut self) {
        if !self.queue_expanded {
            return;
        }
        let Some(task) = self.tasks.get(self.task_cursor).cloned() else {
            self.set_status("No task selected", true);
            return;
        };

        match task.status {
            TaskQueueStatus::Queued => {
                if let Some(position) = self.tasks.iter().position(|entry| entry.id == task.id) {
                    self.tasks[position].mark_cancelled();
                }
                let result = {
                    let mut guard = self.history_tracker.lock().await;
                    if let Some(tracker) = guard.as_mut() {
                        tracker.mark_task_cancelled(&task.id).await
                    } else {
                        Ok(None)
                    }
                };
                if let Err(error) = result {
                    self.set_status(format!("Failed to cancel task: {}", error), true);
                } else {
                    self.set_status(
                        format!(
                            "Cancelled {} for {}",
                            action_label(task.action),
                            task.package_name
                        ),
                        true,
                    );
                }
            }
            TaskQueueStatus::Running => self.set_status("Cannot cancel running task", true),
            _ => self.set_status("Only queued tasks can be cancelled", true),
        }

        self.maybe_emit_queue_completion_digest();
    }

    async fn queue_retry_for_parent_task(&mut self, task: &TaskQueueEntry) {
        let retry = TaskQueueEntry::new(
            task.action,
            task.package_id.clone(),
            task.package_name.clone(),
            task.package_source,
        );

        let state = self
            .task_recovery_states
            .entry(task.id.clone())
            .or_default();
        state.attempts += 1;
        let attempt = state.attempts;
        self.retry_parent.insert(retry.id.clone(), task.id.clone());
        self.retry_attempt.insert(retry.id.clone(), attempt);

        self.enqueue_task_entry(retry).await;
    }

    async fn retry_selected_task(&mut self) {
        if !self.queue_expanded {
            return;
        }
        let Some(task) = self.tasks.get(self.task_cursor).cloned() else {
            self.set_status("No task selected", true);
            return;
        };

        if task.status != TaskQueueStatus::Failed {
            self.set_status("Only failed tasks can be retried", true);
            return;
        }

        self.queue_retry_for_parent_task(&task).await;
        self.queue_completed_at = None;
        self.queue_completion_digest_emitted = false;
        self.spawn_task_executor();
        self.set_status(
            format!(
                "Re-queued {} for {}",
                action_label(task.action),
                task.package_name
            ),
            true,
        );
    }

    fn clinic_failure_candidates(&self) -> Vec<TaskQueueEntry> {
        self.queue_visible_task_indices()
            .into_iter()
            .filter_map(|index| self.tasks.get(index))
            .filter(|task| self.queue_lane_for_task(task) == QueueJourneyLane::NeedsAttention)
            .cloned()
            .collect()
    }

    fn clinic_safe_retry_candidates(&self) -> Vec<TaskQueueEntry> {
        let mut seen_packages = HashSet::new();
        self.clinic_failure_candidates()
            .into_iter()
            .filter(|task| {
                let category = self
                    .failure_category_for_task(task)
                    .unwrap_or(FailureCategory::Unknown);
                Self::safe_retry_category(category)
                    && !self.has_active_task_for_package(&task.package_id)
                    && seen_packages.insert(task.package_id.clone())
            })
            .collect()
    }

    fn clinic_remediation_plan(&self) -> ClinicRemediationPlan {
        let mut retries = Vec::new();
        let mut guidance_only = 0usize;
        let mut skipped = 0usize;
        let mut seen_packages = HashSet::new();
        let mut candidates = self.clinic_failure_candidates();
        let preview_count = candidates.len();

        for task in candidates.drain(..) {
            if !seen_packages.insert(task.package_id.clone()) {
                skipped += 1;
                continue;
            }
            if self.has_active_task_for_package(&task.package_id) {
                skipped += 1;
                continue;
            }

            let category = self
                .failure_category_for_task(&task)
                .unwrap_or(FailureCategory::Unknown);
            match category {
                FailureCategory::NotFound => guidance_only += 1,
                FailureCategory::Permissions
                | FailureCategory::Network
                | FailureCategory::Conflict
                | FailureCategory::Unknown => retries.push(task),
            }
        }

        ClinicRemediationPlan {
            retries,
            guidance_only,
            skipped,
            preview_count,
        }
    }

    async fn retry_safe_failed_tasks(&mut self) {
        if !self.queue_expanded {
            self.set_status("Open queue first to run retry bundle", true);
            return;
        }

        let scope = self.queue_failure_filter.label();
        let retryable = self.clinic_safe_retry_candidates();

        if retryable.is_empty() {
            self.set_status(self.safe_retry_unavailable_reason(), true);
            return;
        }

        let actionability = self.queue_clinic_actionability();
        let total = actionability.safe_retry_count;
        for task in &retryable {
            self.queue_retry_for_parent_task(task).await;
        }
        self.queue_completed_at = None;
        self.queue_completion_digest_emitted = false;
        self.spawn_task_executor();
        if self.queue_failure_filter == QueueFailureFilter::All {
            self.set_status(
                format!(
                    "Queued safe retry bundle for {} failure{}",
                    total,
                    if total == 1 { "" } else { "s" }
                ),
                true,
            );
        } else {
            self.set_status(
                format!(
                    "Queued safe retry bundle [{}] for {} failure{}",
                    scope,
                    total,
                    if total == 1 { "" } else { "s" }
                ),
                true,
            );
        }
    }

    async fn run_recommended_action(&mut self) {
        match self.recommended_action() {
            RecommendedAction::RetrySafeFailures(_) => {
                if !self.queue_expanded {
                    self.toggle_queue_expanded();
                }
                self.focus = Focus::Queue;
                if let Some(index) = self.tasks.iter().position(|task| {
                    self.queue_lane_for_task(task) == QueueJourneyLane::NeedsAttention
                }) {
                    self.set_task_cursor(index);
                }
                self.retry_safe_failed_tasks().await;
            }
            RecommendedAction::ReviewFailures(_) => {
                if !self.queue_expanded {
                    self.toggle_queue_expanded();
                }
                self.focus = Focus::Queue;
                if let Some(index) = self.tasks.iter().position(|task| {
                    self.queue_lane_for_task(task) == QueueJourneyLane::NeedsAttention
                }) {
                    self.set_task_cursor(index);
                }
                self.set_status("Failures view ready. Use R/M or A for safe retries.", true);
            }
            RecommendedAction::QueueAllUpdates(_) => {
                self.filter = Filter::Updates;
                self.apply_filters();
                self.clear_selection();
                let targets: Vec<Package> = self
                    .packages
                    .iter()
                    .filter(|package| package.status == PackageStatus::UpdateAvailable)
                    .cloned()
                    .collect();
                self.prepare_action_for_targets(TaskQueueAction::Update, targets, false);
            }
            RecommendedAction::ReviewQueue => {
                if !self.queue_expanded {
                    self.toggle_queue_expanded();
                }
                self.focus = Focus::Queue;
                self.set_status("Queue journey opened.", true);
            }
            RecommendedAction::RefreshPackages => {
                if !self.start_loading() {
                    self.set_status("Already refreshing", true);
                } else {
                    self.set_status("Refreshing package metadata...", true);
                }
            }
        }
    }

    async fn apply_selected_task_remediation(&mut self) {
        if !self.queue_expanded {
            return;
        }

        self.apply_filtered_failure_remediation().await;
    }

    async fn apply_filtered_failure_remediation(&mut self) {
        let scope = self.queue_failure_filter.label();
        let plan = self.clinic_remediation_plan();
        if plan.preview_count == 0 {
            self.set_status(self.remediation_unavailable_reason(), true);
            return;
        }

        let mut retried = 0usize;
        for task in &plan.retries {
            let category = self
                .failure_category_for_task(task)
                .unwrap_or(FailureCategory::Unknown);
            if category == FailureCategory::Network && !self.loading {
                let _ = self.start_loading();
            }
            self.queue_retry_for_parent_task(task).await;
            retried += 1;
        }

        if plan.guidance_only > 0 && !self.loading {
            let _ = self.start_loading();
        }

        if retried > 0 {
            self.queue_completed_at = None;
            self.queue_completion_digest_emitted = false;
            self.spawn_task_executor();
        }

        let guidance_note = if plan.guidance_only > 0 {
            " · verify package/source before retry"
        } else {
            ""
        };
        self.set_status(
            format!(
                "Remediation bundle [{}] preview {} task{}: {} queued retry, {} guidance-only, {} skipped{}",
                scope,
                plan.preview_count,
                if plan.preview_count == 1 { "" } else { "s" },
                retried,
                plan.guidance_only,
                plan.skipped,
                guidance_note
            ),
            true,
        );
    }

    pub fn queue_counts(&self) -> (usize, usize, usize, usize, usize) {
        let mut queued = 0usize;
        let mut running = 0usize;
        let mut completed = 0usize;
        let mut failed = 0usize;
        let mut cancelled = 0usize;

        for task in &self.tasks {
            match self.queue_lane_for_task(task) {
                QueueJourneyLane::Now => running += 1,
                QueueJourneyLane::Next => queued += 1,
                QueueJourneyLane::NeedsAttention => failed += 1,
                QueueJourneyLane::Done => match task.status {
                    TaskQueueStatus::Cancelled => cancelled += 1,
                    _ => completed += 1,
                },
            }
        }
        (queued, running, completed, failed, cancelled)
    }

    pub fn should_show_queue_bar(&self) -> bool {
        !self.tasks.is_empty()
    }

    pub fn spinner_frame(&self) -> char {
        const FRAMES: [char; 4] = ['◐', '◓', '◑', '◒'];
        FRAMES[(self.tick as usize) % FRAMES.len()]
    }
}

fn command_always_enabled(_: &App) -> bool {
    true
}

fn command_open_palette_enabled(app: &App) -> bool {
    !app.showing_palette
}

fn command_cycle_focus_enabled(app: &App) -> bool {
    app.can_cycle_focus_command()
}

fn command_move_up_enabled(app: &App) -> bool {
    app.can_move_up_command()
}

fn command_move_down_enabled(app: &App) -> bool {
    app.can_move_down_command()
}

fn command_move_top_enabled(app: &App) -> bool {
    app.can_move_top_command()
}

fn command_move_bottom_enabled(app: &App) -> bool {
    app.can_move_bottom_command()
}

fn command_page_up_enabled(app: &App) -> bool {
    app.can_page_up_command()
}

fn command_page_down_enabled(app: &App) -> bool {
    app.can_page_down_command()
}

fn command_toggle_selection_enabled(app: &App) -> bool {
    app.can_toggle_selection_command()
}

fn command_toggle_favorite_enabled(app: &App) -> bool {
    app.can_toggle_favorite_command()
}

fn command_bulk_toggle_favorite_enabled(app: &App) -> bool {
    app.can_bulk_toggle_favorite_command()
}

fn command_toggle_favorites_updates_only_enabled(app: &App) -> bool {
    app.can_toggle_favorites_updates_only_command()
}

fn command_select_all_enabled(app: &App) -> bool {
    app.can_select_all_command()
}

fn command_install_enabled(app: &App) -> bool {
    app.can_prepare_command(TaskQueueAction::Install)
}

fn command_remove_enabled(app: &App) -> bool {
    app.can_prepare_command(TaskQueueAction::Remove)
}

fn command_update_enabled(app: &App) -> bool {
    app.can_prepare_command(TaskQueueAction::Update)
}

fn command_view_changelog_enabled(app: &App) -> bool {
    app.can_view_changelog_command()
}

fn command_refresh_enabled(app: &App) -> bool {
    app.can_refresh_command()
}

fn command_toggle_queue_enabled(app: &App) -> bool {
    app.can_toggle_queue_command()
}

fn command_queue_cancel_enabled(app: &App) -> bool {
    app.can_cancel_selected_task_command()
}

fn command_queue_retry_enabled(app: &App) -> bool {
    app.can_retry_selected_task_command()
}

fn command_queue_retry_safe_enabled(app: &App) -> bool {
    app.can_retry_safe_failed_tasks_command()
}

fn command_queue_remediate_enabled(app: &App) -> bool {
    app.can_queue_remediate_command()
}

fn command_queue_log_older_enabled(app: &App) -> bool {
    app.can_queue_log_older_command()
}

fn command_queue_log_newer_enabled(app: &App) -> bool {
    app.can_queue_log_newer_command()
}

fn command_group(command: CommandId) -> &'static str {
    match command {
        CommandId::Quit | CommandId::ShowHelp | CommandId::OpenPalette => "Global",
        CommandId::CycleFocus
        | CommandId::MoveUp
        | CommandId::MoveDown
        | CommandId::MoveTop
        | CommandId::MoveBottom
        | CommandId::PageUp
        | CommandId::PageDown => "Navigation",
        CommandId::FilterAll
        | CommandId::FilterInstalled
        | CommandId::FilterUpdates
        | CommandId::FilterFavorites
        | CommandId::RunRecommended
        | CommandId::ViewChangelog
        | CommandId::Search
        | CommandId::Refresh => "Views",
        CommandId::ToggleFavorite
        | CommandId::BulkToggleFavorite
        | CommandId::ToggleFavoritesUpdatesOnly
        | CommandId::ToggleSelection
        | CommandId::SelectAll => "Selection",
        CommandId::Install | CommandId::Remove | CommandId::Update => "Actions",
        CommandId::ToggleQueue
        | CommandId::QueueCancel
        | CommandId::QueueRetry
        | CommandId::QueueRetrySafe
        | CommandId::QueueRemediate
        | CommandId::QueueLogOlder
        | CommandId::QueueLogNewer => "Queue",
    }
}

fn command_group_order(command: CommandId) -> usize {
    match command_group(command) {
        "Global" => 0,
        "Navigation" => 1,
        "Views" => 2,
        "Selection" => 3,
        "Actions" => 4,
        "Queue" => 5,
        _ => usize::MAX,
    }
}

fn fuzzy_subsequence_score(haystack: &str, query: &str) -> Option<usize> {
    if query.is_empty() {
        return Some(0);
    }

    let mut consumed = 0usize;
    let mut score = 0usize;
    for expected in query.chars() {
        let mut found = None;
        for (offset, ch) in haystack[consumed..].char_indices() {
            if ch == expected {
                found = Some((offset, ch.len_utf8()));
                break;
            }
        }
        let (offset, width) = found?;
        score += offset;
        consumed += offset + width;
    }

    Some(score)
}

pub fn action_label(action: TaskQueueAction) -> &'static str {
    match action {
        TaskQueueAction::Install => "Install",
        TaskQueueAction::Remove => "Remove",
        TaskQueueAction::Update => "Update",
    }
}

pub async fn run() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), DisableMouseCapture, LeaveAlternateScreen);
        default_hook(info);
    }));

    let pm = Arc::new(Mutex::new(PackageManager::new()));
    let history_tracker = Arc::new(Mutex::new(None));
    let (task_tx, task_rx) = mpsc::channel(200);

    let mut app = App::new(pm, history_tracker, Some(task_rx), Some(task_tx));
    app.initialize_history_tracker().await;
    app.sync_task_queue_from_history().await;
    app.load_sources().await;
    app.load_favorites();
    app.load_session_state();
    let _ = app.start_loading();
    app.spawn_task_executor();

    let result = run_app(&mut terminal, &mut app).await;

    if let Err(error) = app.persist_favorites() {
        error!(error = %error, "Failed to persist favorites");
    }
    if let Err(error) = app.persist_session_state() {
        error!(error = %error, "Failed to persist TUI session state");
    }

    let _ = std::panic::take_hook();
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        DisableMouseCapture,
        LeaveAlternateScreen
    )?;
    terminal.show_cursor()?;

    result
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> Result<()> {
    loop {
        app.tick = app.tick.wrapping_add(1);

        let size = terminal.size()?;
        app.compact = size.width < COMPACT_WIDTH || size.height < COMPACT_HEIGHT;

        app.poll_loading();
        app.poll_changelog();
        app.poll_preflight_verification();
        app.poll_task_events();
        app.maybe_autohide_queue();

        terminal.draw(|frame| ui::draw(frame, app))?;

        if event::poll(Duration::from_millis(50))? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    app.handle_key(key).await;
                }
                Event::Mouse(mouse) => {
                    let regions = compute_layout(app, Rect::new(0, 0, size.width, size.height));
                    app.handle_mouse(mouse, &regions).await;
                }
                _ => {}
            }
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_pkg(name: &str, source: PackageSource, status: PackageStatus) -> Package {
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

    fn test_app() -> App {
        let mut app = App::new(
            Arc::new(Mutex::new(PackageManager::new())),
            Arc::new(Mutex::new(None)),
            None,
            None,
        );
        app.favorites_persistence_enabled = false;
        app.session_persistence_enabled = false;
        app
    }

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn ctrl(code: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(code), KeyModifiers::CONTROL)
    }

    fn mouse(kind: MouseEventKind, column: u16, row: u16) -> MouseEvent {
        MouseEvent {
            kind,
            column,
            row,
            modifiers: KeyModifiers::NONE,
        }
    }

    fn layout_regions(app: &App) -> ui::LayoutRegions {
        ui::compute_layout(app, Rect::new(0, 0, 120, 40))
    }

    #[test]
    fn command_registry_has_metadata_and_required_coverage() {
        let registry = App::command_registry();

        assert!(!registry.is_empty());
        assert!(registry
            .iter()
            .all(|command| !command.label.trim().is_empty()));
        assert!(registry
            .iter()
            .all(|command| !command.shortcut.trim().is_empty()));
        assert!(registry
            .iter()
            .any(|command| command.id == CommandId::MoveDown));
        assert!(registry
            .iter()
            .any(|command| command.id == CommandId::FilterAll));
        assert!(registry
            .iter()
            .any(|command| command.id == CommandId::FilterFavorites));
        assert!(registry
            .iter()
            .any(|command| command.id == CommandId::QueueCancel));
        assert!(registry
            .iter()
            .any(|command| command.id == CommandId::ShowHelp));
        assert!(registry
            .iter()
            .any(|command| command.id == CommandId::OpenPalette));
        assert!(registry
            .iter()
            .any(|command| command.id == CommandId::BulkToggleFavorite));
        assert!(registry
            .iter()
            .any(|command| command.id == CommandId::RunRecommended));
        assert!(registry
            .iter()
            .any(|command| command.id == CommandId::QueueRetrySafe));
        assert!(registry
            .iter()
            .any(|command| command.id == CommandId::ViewChangelog));
    }

    #[test]
    fn command_enabled_reflects_navigation_and_actions() {
        let mut app = test_app();

        assert!(app.command_enabled(CommandId::CycleFocus));
        app.compact = true;
        assert!(!app.command_enabled(CommandId::CycleFocus));

        app.compact = false;
        app.focus = Focus::Packages;
        app.packages = vec![make_pkg(
            "pkg",
            PackageSource::Apt,
            PackageStatus::NotInstalled,
        )];
        app.apply_filters();

        assert!(app.command_enabled(CommandId::ToggleSelection));
        assert!(app.command_enabled(CommandId::ToggleFavorite));
        assert!(app.command_enabled(CommandId::BulkToggleFavorite));
        assert!(app.command_enabled(CommandId::ViewChangelog));
        assert!(!app.command_enabled(CommandId::ToggleFavoritesUpdatesOnly));
        assert!(app.command_enabled(CommandId::Install));
        assert!(!app.command_enabled(CommandId::Remove));
        assert!(!app.command_enabled(CommandId::Update));

        app.packages[0].status = PackageStatus::UpdateAvailable;
        app.apply_filters();

        assert!(app.command_enabled(CommandId::Remove));
        assert!(app.command_enabled(CommandId::Update));

        app.packages[0].source = PackageSource::Snap;
        app.apply_filters();
        assert!(!app.command_enabled(CommandId::ViewChangelog));

        app.filter = Filter::Favorites;
        app.apply_filters();
        assert!(app.command_enabled(CommandId::ToggleFavoritesUpdatesOnly));
    }

    #[test]
    fn changelog_support_matrix_is_explicit() {
        assert!(App::changelog_supported_for_source(PackageSource::Apt));
        assert!(App::changelog_supported_for_source(PackageSource::Dnf));
        assert!(App::changelog_supported_for_source(PackageSource::Pip));
        assert!(App::changelog_supported_for_source(PackageSource::Npm));
        assert!(App::changelog_supported_for_source(PackageSource::Cargo));
        assert!(App::changelog_supported_for_source(PackageSource::Conda));
        assert!(App::changelog_supported_for_source(PackageSource::Mamba));

        assert!(!App::changelog_supported_for_source(PackageSource::Snap));
        assert!(!App::changelog_supported_for_source(PackageSource::Flatpak));
    }

    #[test]
    fn changelog_disabled_reason_is_contextual() {
        let mut app = test_app();
        assert_eq!(
            app.command_disabled_reason(CommandId::ViewChangelog),
            Some("Select a package to view changelog".to_string())
        );

        app.packages = vec![make_pkg(
            "pkg",
            PackageSource::Snap,
            PackageStatus::Installed,
        )];
        app.apply_filters();

        assert_eq!(
            app.command_disabled_reason(CommandId::ViewChangelog),
            Some("Changelog is not supported for this source yet".to_string())
        );
    }

    #[test]
    fn preflight_summary_flags_privilege_and_dependency_uncertainty() {
        let pkg = make_pkg("pkg", PackageSource::Apt, PackageStatus::Installed);
        let targets = vec![pkg.clone()];
        let valid = vec![pkg];
        let summary =
            App::build_preflight_summary(TaskQueueAction::Remove, &targets, &valid, false);

        assert_eq!(summary.certainty, PreflightCertainty::Estimated);
        assert!(summary.elevated_privileges_likely);
        assert!(!summary.dependency_impact_known);
        assert!(summary.verification_in_progress);
        assert!(summary
            .risk_reasons
            .iter()
            .any(|reason| reason.contains("elevated privileges")));
        assert!(summary
            .risk_reasons
            .iter()
            .any(|reason| reason.contains("verification is in progress")));
    }

    #[test]
    fn preflight_summary_keeps_low_risk_user_scoped_installs() {
        let pkg = make_pkg("pkg", PackageSource::Pip, PackageStatus::NotInstalled);
        let targets = vec![pkg.clone()];
        let valid = vec![pkg];
        let summary =
            App::build_preflight_summary(TaskQueueAction::Install, &targets, &valid, false);

        assert_eq!(summary.risk_level, PreflightRiskLevel::Safe);
        assert_eq!(summary.certainty, PreflightCertainty::Estimated);
        assert!(!summary.elevated_privileges_likely);
        assert!(!summary.dependency_impact_known);
        assert!(!summary.verification_in_progress);
    }

    #[test]
    fn preflight_dependency_verification_support_is_source_aware() {
        let apt_remove = vec![make_pkg(
            "pkg",
            PackageSource::Apt,
            PackageStatus::Installed,
        )];
        let apt_update = vec![make_pkg(
            "pkg",
            PackageSource::Apt,
            PackageStatus::UpdateAvailable,
        )];
        let mixed_remove = vec![
            make_pkg("pkg", PackageSource::Apt, PackageStatus::Installed),
            make_pkg("tool", PackageSource::Pip, PackageStatus::Installed),
        ];

        assert!(App::preflight_dependency_verification_supported(
            TaskQueueAction::Remove,
            &apt_remove
        ));
        assert!(App::preflight_dependency_verification_supported(
            TaskQueueAction::Update,
            &apt_update
        ));
        assert!(App::preflight_dependency_verification_supported(
            TaskQueueAction::Install,
            &apt_update
        ));
        assert!(!App::preflight_dependency_verification_supported(
            TaskQueueAction::Remove,
            &mixed_remove
        ));
    }

    #[test]
    fn poll_preflight_verification_promotes_matching_confirmation_to_verified() {
        let mut app = test_app();
        let pkg = make_pkg("pkg", PackageSource::Apt, PackageStatus::Installed);
        let targets = vec![pkg.clone()];
        let valid = vec![pkg];
        let preflight =
            App::build_preflight_summary(TaskQueueAction::Remove, &targets, &valid, false);

        app.confirming = Some(PendingAction {
            label: "Remove pkg?".into(),
            packages: valid.clone(),
            action: TaskQueueAction::Remove,
            preflight,
            risk_acknowledged: false,
        });
        app.active_preflight_verification_id = Some(42);

        if let Some(tx) = app.preflight_verification_tx.clone() {
            tx.try_send(PreflightVerificationResult {
                request_id: 42,
                action: TaskQueueAction::Remove,
                package_ids: App::preflight_target_ids(&valid),
                dependency_impact_known: true,
                dependency_impact: Some(PreflightDependencyImpact {
                    install_count: 0,
                    upgrade_count: 0,
                    remove_count: 1,
                    held_back_count: 0,
                }),
                note: Some("Probe finished successfully.".to_string()),
            })
            .expect("send verification update");
        }

        app.poll_preflight_verification();

        let confirming = app.confirming.as_ref().expect("confirming state");
        assert_eq!(confirming.preflight.certainty, PreflightCertainty::Verified);
        assert!(confirming.preflight.dependency_impact_known);
        assert_eq!(
            confirming.preflight.dependency_impact,
            Some(PreflightDependencyImpact {
                install_count: 0,
                upgrade_count: 0,
                remove_count: 1,
                held_back_count: 0,
            })
        );
        assert!(!confirming.preflight.verification_in_progress);
        assert!(confirming
            .preflight
            .risk_reasons
            .iter()
            .any(|reason| reason.contains("Probe finished successfully")));
        assert!(app.active_preflight_verification_id.is_none());
    }

    #[test]
    fn parse_apt_dry_run_impact_extracts_counts() {
        let output = "0 upgraded, 2 newly installed, 1 to remove and 3 not upgraded.";
        let impact = App::parse_apt_dry_run_impact(output).expect("apt parse should succeed");
        assert_eq!(
            impact,
            PreflightDependencyImpact {
                install_count: 2,
                upgrade_count: 0,
                remove_count: 1,
                held_back_count: 3,
            }
        );
    }

    #[test]
    fn parse_dnf_dry_run_impact_extracts_transaction_summary() {
        let output = r#"
Transaction Summary
Install  4 Packages
Upgrade  2 Packages
Remove   1 Package
"#;
        let impact = App::parse_dnf_dry_run_impact(output).expect("dnf parse should succeed");
        assert_eq!(
            impact,
            PreflightDependencyImpact {
                install_count: 4,
                upgrade_count: 2,
                remove_count: 1,
                held_back_count: 0,
            }
        );
    }

    #[test]
    fn queue_command_enablement_follows_selected_task_state() {
        let mut app = test_app();
        assert!(!app.command_enabled(CommandId::ToggleQueue));

        let queued = TaskQueueEntry::new(
            TaskQueueAction::Install,
            "APT:a".into(),
            "a".into(),
            PackageSource::Apt,
        );
        let mut failed = TaskQueueEntry::new(
            TaskQueueAction::Update,
            "APT:b".into(),
            "b".into(),
            PackageSource::Apt,
        );
        failed.mark_failed("err".into());

        app.tasks = vec![queued.clone(), failed.clone()];
        app.queue_expanded = true;
        app.focus = Focus::Queue;

        assert!(app.command_enabled(CommandId::ToggleQueue));

        app.task_cursor = 0;
        assert!(app.command_enabled(CommandId::QueueCancel));
        assert!(!app.command_enabled(CommandId::QueueRetry));

        app.task_cursor = 1;
        assert!(!app.command_enabled(CommandId::QueueCancel));
        assert!(app.command_enabled(CommandId::QueueRetry));

        app.task_logs.insert(
            failed.id.clone(),
            VecDeque::from(vec!["one".to_string(), "two".to_string()]),
        );
        assert!(app.command_enabled(CommandId::QueueLogOlder));
        assert!(!app.command_enabled(CommandId::QueueLogNewer));

        app.task_log_scroll = 1;
        assert!(!app.command_enabled(CommandId::QueueLogOlder));
        assert!(app.command_enabled(CommandId::QueueLogNewer));
    }

    #[test]
    fn apply_filters_all() {
        let mut app = test_app();
        app.packages = vec![
            make_pkg("a", PackageSource::Apt, PackageStatus::Installed),
            make_pkg("b", PackageSource::Snap, PackageStatus::UpdateAvailable),
            make_pkg("c", PackageSource::Deb, PackageStatus::NotInstalled),
        ];
        app.filter = Filter::All;
        app.apply_filters();

        assert_eq!(app.filtered.len(), 3);
        assert_eq!(app.filter_counts, [3, 2, 1, 0]);
    }

    #[test]
    fn apply_filters_installed_includes_in_progress() {
        let mut app = test_app();
        app.packages = vec![
            make_pkg("a", PackageSource::Apt, PackageStatus::Installed),
            make_pkg("b", PackageSource::Apt, PackageStatus::UpdateAvailable),
            make_pkg("c", PackageSource::Apt, PackageStatus::Updating),
            make_pkg("d", PackageSource::Apt, PackageStatus::NotInstalled),
        ];
        app.filter = Filter::Installed;
        app.apply_filters();

        assert_eq!(app.filtered.len(), 3);
    }

    #[test]
    fn apply_filters_updates_include_updating() {
        let mut app = test_app();
        app.packages = vec![
            make_pkg("a", PackageSource::Apt, PackageStatus::UpdateAvailable),
            make_pkg("b", PackageSource::Apt, PackageStatus::Updating),
            make_pkg("c", PackageSource::Apt, PackageStatus::Installed),
        ];
        app.filter = Filter::Updates;
        app.apply_filters();

        assert_eq!(app.filtered.len(), 2);
        assert_eq!(app.filter_counts[2], 2);
    }

    #[test]
    fn apply_filters_favorites_filter_and_source_visibility() {
        let mut app = test_app();
        app.available_sources = vec![PackageSource::Apt, PackageSource::Snap];

        let apt = make_pkg("apt", PackageSource::Apt, PackageStatus::Installed);
        let snap = make_pkg("snap", PackageSource::Snap, PackageStatus::Installed);
        app.favorite_packages.insert(apt.id());
        app.packages = vec![apt, snap];

        app.filter = Filter::Favorites;
        app.apply_filters();

        assert_eq!(app.filter_counts, [2, 2, 0, 1]);
        assert_eq!(app.filtered.len(), 1);
        assert_eq!(app.current_package().map(|p| p.name.as_str()), Some("apt"));
        assert_eq!(app.visible_sources(), vec![PackageSource::Apt]);
    }

    #[test]
    fn apply_filters_tracks_per_source_counts_and_resets_hidden_source() {
        let mut app = test_app();
        app.available_sources = vec![PackageSource::Apt, PackageSource::Snap];
        app.packages = vec![
            make_pkg(
                "apt-installed",
                PackageSource::Apt,
                PackageStatus::Installed,
            ),
            make_pkg(
                "apt-missing",
                PackageSource::Apt,
                PackageStatus::NotInstalled,
            ),
            make_pkg(
                "snap-update",
                PackageSource::Snap,
                PackageStatus::UpdateAvailable,
            ),
        ];
        app.filter = Filter::All;
        app.apply_filters();

        assert_eq!(
            app.source_counts.get(&PackageSource::Apt),
            Some(&[2, 1, 0, 0])
        );
        assert_eq!(
            app.source_counts.get(&PackageSource::Snap),
            Some(&[1, 1, 1, 0])
        );

        app.source = Some(PackageSource::Apt);
        app.filter = Filter::Updates;
        app.apply_filters();

        assert_eq!(app.visible_sources(), vec![PackageSource::Snap]);
        assert_eq!(app.source, None);
    }

    #[test]
    fn apply_filters_combined_source_search_and_cursor_clamp() {
        let mut app = test_app();
        app.available_sources = vec![PackageSource::Apt, PackageSource::Snap];
        app.packages = vec![
            make_pkg("firefox", PackageSource::Snap, PackageStatus::Installed),
            make_pkg("vim", PackageSource::Apt, PackageStatus::Installed),
        ];
        app.source = Some(PackageSource::Apt);
        app.search = "vim".to_string();
        app.cursor = 9;
        app.apply_filters();

        assert_eq!(app.filtered.len(), 1);
        assert_eq!(app.cursor, 0);
        assert_eq!(app.current_package().map(|p| p.name.as_str()), Some("vim"));
    }

    #[test]
    fn merge_updates_upgrades_installed_package() {
        let installed = vec![make_pkg(
            "vim",
            PackageSource::Apt,
            PackageStatus::Installed,
        )];
        let updates = vec![make_pkg(
            "vim",
            PackageSource::Apt,
            PackageStatus::UpdateAvailable,
        )];

        let merged = App::merge_installed_with_updates(installed, updates);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].status, PackageStatus::UpdateAvailable);
        assert_eq!(merged[0].available_version.as_deref(), Some("1.1.0"));
    }

    #[tokio::test]
    async fn key_dispatch_precedence_help_confirm_search_normal() {
        let mut app = test_app();
        app.showing_help = true;
        app.searching = true;
        app.confirming = Some(PendingAction {
            label: "x".into(),
            packages: Vec::new(),
            action: TaskQueueAction::Install,
            preflight: PreflightSummary {
                action: TaskQueueAction::Install,
                target_count: 0,
                executable_count: 0,
                skipped_count: 0,
                source_breakdown: Vec::new(),
                risk_level: PreflightRiskLevel::Safe,
                risk_reasons: vec!["No additional guardrails triggered.".to_string()],
                certainty: PreflightCertainty::Estimated,
                elevated_privileges_likely: false,
                dependency_impact_known: false,
                dependency_impact: None,
                verification_in_progress: false,
                selection_mode: false,
            },
            risk_acknowledged: true,
        });
        app.handle_key(key(KeyCode::Esc)).await;
        assert!(!app.showing_help);
        assert!(app.confirming.is_some());

        app.showing_help = false;
        app.searching = true;
        app.handle_key(key(KeyCode::Esc)).await;
        assert!(app.confirming.is_none());
        assert!(app.searching);

        app.searching = true;
        app.handle_key(key(KeyCode::Char('a'))).await;
        assert_eq!(app.search, "a");
    }

    #[tokio::test]
    async fn digits_do_not_leak_while_searching() {
        let mut app = test_app();
        app.searching = true;
        app.filter = Filter::All;

        app.handle_key(key(KeyCode::Char('1'))).await;

        assert_eq!(app.search, "1");
        assert_eq!(app.filter, Filter::All);
    }

    #[tokio::test]
    async fn navigation_jk_bounds_and_g_g() {
        let mut app = test_app();
        app.focus = Focus::Packages;
        app.packages = vec![
            make_pkg("a", PackageSource::Apt, PackageStatus::Installed),
            make_pkg("b", PackageSource::Apt, PackageStatus::Installed),
            make_pkg("c", PackageSource::Apt, PackageStatus::Installed),
        ];
        app.apply_filters();

        app.handle_key(key(KeyCode::Char('k'))).await;
        assert_eq!(app.cursor, 0);

        app.handle_key(key(KeyCode::Char('G'))).await;
        assert_eq!(app.cursor, 2);

        app.handle_key(key(KeyCode::Char('j'))).await;
        assert_eq!(app.cursor, 2);

        app.handle_key(key(KeyCode::Char('g'))).await;
        assert_eq!(app.cursor, 0);
    }

    #[tokio::test]
    async fn filter_keys_update_filter() {
        let mut app = test_app();
        app.packages = vec![
            make_pkg("a", PackageSource::Apt, PackageStatus::Installed),
            make_pkg("b", PackageSource::Apt, PackageStatus::UpdateAvailable),
        ];
        app.apply_filters();

        app.handle_key(key(KeyCode::Char('3'))).await;
        assert_eq!(app.filter, Filter::Updates);
        assert_eq!(app.filtered.len(), 1);
    }

    #[tokio::test]
    async fn favorite_key_toggles_and_filter_four_shows_only_favorites() {
        let mut app = test_app();
        app.focus = Focus::Packages;
        app.packages = vec![
            make_pkg("a", PackageSource::Apt, PackageStatus::Installed),
            make_pkg("b", PackageSource::Apt, PackageStatus::Installed),
        ];
        app.apply_filters();

        let favorite_id = app.packages[0].id();
        app.handle_key(key(KeyCode::Char('f'))).await;

        assert!(app.favorite_packages.contains(&favorite_id));
        assert_eq!(app.filter_counts[3], 1);

        app.handle_key(key(KeyCode::Char('4'))).await;
        assert_eq!(app.filter, Filter::Favorites);
        assert_eq!(app.filtered.len(), 1);
    }

    #[tokio::test]
    async fn selection_space_a_and_esc_clear() {
        let mut app = test_app();
        app.focus = Focus::Packages;
        app.packages = vec![
            make_pkg("a", PackageSource::Apt, PackageStatus::Installed),
            make_pkg("b", PackageSource::Apt, PackageStatus::Installed),
        ];
        app.apply_filters();

        app.handle_key(key(KeyCode::Char(' '))).await;
        assert_eq!(app.selected.len(), 1);

        app.handle_key(key(KeyCode::Char('a'))).await;
        assert_eq!(app.selected.len(), 2);

        app.handle_key(key(KeyCode::Esc)).await;
        assert!(app.selected.is_empty());
    }

    #[tokio::test]
    async fn selection_persists_across_filter_switch() {
        let mut app = test_app();
        app.focus = Focus::Packages;
        let update = make_pkg("update", PackageSource::Apt, PackageStatus::UpdateAvailable);
        let installed = make_pkg("installed", PackageSource::Apt, PackageStatus::Installed);
        app.packages = vec![update.clone(), installed];
        app.apply_filters();

        app.selected.insert(update.id());
        app.handle_key(key(KeyCode::Char('3'))).await;

        assert_eq!(app.selected.len(), 1);
    }

    #[test]
    fn cleanup_stale_selections_removes_gone_packages() {
        let mut app = test_app();
        let keep = make_pkg("keep", PackageSource::Apt, PackageStatus::Installed);
        let gone = make_pkg("gone", PackageSource::Apt, PackageStatus::Installed);
        app.packages = vec![keep.clone(), gone.clone()];
        app.selected.insert(keep.id());
        app.selected.insert(gone.id());

        app.packages = vec![keep.clone()];
        app.cleanup_stale_selections();

        assert_eq!(app.selected.len(), 1);
        assert!(app.selected.contains(&keep.id()));
    }

    #[tokio::test]
    async fn install_invalid_on_installed_shows_status() {
        let mut app = test_app();
        app.focus = Focus::Packages;
        app.packages = vec![make_pkg(
            "vim",
            PackageSource::Apt,
            PackageStatus::Installed,
        )];
        app.apply_filters();

        app.handle_key(key(KeyCode::Char('i'))).await;

        assert!(app.confirming.is_none());
        assert_eq!(app.status, "vim is already installed");
    }

    #[tokio::test]
    async fn remove_invalid_on_not_installed_shows_status() {
        let mut app = test_app();
        app.focus = Focus::Packages;
        app.packages = vec![make_pkg(
            "pkg",
            PackageSource::Deb,
            PackageStatus::NotInstalled,
        )];
        app.apply_filters();

        app.handle_key(key(KeyCode::Char('x'))).await;

        assert!(app.confirming.is_none());
        assert_eq!(app.status, "pkg is not installed");
    }

    #[tokio::test]
    async fn batch_update_filters_valid_targets_and_reports_skip() {
        let mut app = test_app();
        app.focus = Focus::Packages;
        let update = make_pkg("a", PackageSource::Apt, PackageStatus::UpdateAvailable);
        let installed = make_pkg("b", PackageSource::Apt, PackageStatus::Installed);
        app.packages = vec![update.clone(), installed.clone()];
        app.apply_filters();
        app.selected.insert(update.id());
        app.selected.insert(installed.id());

        app.handle_key(key(KeyCode::Char('u'))).await;

        let confirming = app.confirming.as_ref().expect("confirm expected");
        assert_eq!(confirming.packages.len(), 1);
        assert!(confirming.label.contains("1 of 2 selected"));
    }

    #[tokio::test]
    async fn search_enter_keeps_filter_and_esc_clears() {
        let mut app = test_app();
        app.packages = vec![
            make_pkg("firefox", PackageSource::Apt, PackageStatus::Installed),
            make_pkg("vim", PackageSource::Apt, PackageStatus::Installed),
        ];
        app.apply_filters();

        app.handle_key(key(KeyCode::Char('/'))).await;
        app.handle_key(key(KeyCode::Char('v'))).await;
        app.handle_key(key(KeyCode::Char('i'))).await;
        app.handle_key(key(KeyCode::Char('m'))).await;
        app.handle_key(key(KeyCode::Enter)).await;

        assert!(!app.searching);
        assert_eq!(app.search, "vim");
        assert_eq!(app.filtered.len(), 1);

        app.handle_key(key(KeyCode::Esc)).await;
        assert!(app.search.is_empty());
    }

    #[tokio::test]
    async fn enter_on_packages_triggers_default_action() {
        let mut app = test_app();
        app.focus = Focus::Packages;
        app.packages = vec![make_pkg(
            "vim",
            PackageSource::Apt,
            PackageStatus::Installed,
        )];
        app.apply_filters();

        app.handle_key(key(KeyCode::Enter)).await;

        let confirming = app.confirming.as_ref().expect("confirm expected");
        assert_eq!(confirming.action, TaskQueueAction::Remove);
        assert_eq!(confirming.packages.len(), 1);
        assert_eq!(confirming.packages[0].name, "vim");
    }

    #[tokio::test]
    async fn confirm_y_queues_tasks_and_n_clears() {
        let mut app = test_app();
        app.executor_running.store(true, Ordering::SeqCst);
        app.confirming = Some(PendingAction {
            label: "Install pkg?".into(),
            packages: vec![make_pkg(
                "pkg",
                PackageSource::Deb,
                PackageStatus::NotInstalled,
            )],
            action: TaskQueueAction::Install,
            preflight: PreflightSummary {
                action: TaskQueueAction::Install,
                target_count: 1,
                executable_count: 1,
                skipped_count: 0,
                source_breakdown: vec![(PackageSource::Deb, 1)],
                risk_level: PreflightRiskLevel::Safe,
                risk_reasons: vec!["No additional guardrails triggered.".to_string()],
                certainty: PreflightCertainty::Estimated,
                elevated_privileges_likely: false,
                dependency_impact_known: false,
                dependency_impact: None,
                verification_in_progress: false,
                selection_mode: false,
            },
            risk_acknowledged: true,
        });

        app.handle_key(key(KeyCode::Char('n'))).await;
        assert!(app.confirming.is_none());

        app.confirming = Some(PendingAction {
            label: "Install pkg?".into(),
            packages: vec![make_pkg(
                "pkg",
                PackageSource::Deb,
                PackageStatus::NotInstalled,
            )],
            action: TaskQueueAction::Install,
            preflight: PreflightSummary {
                action: TaskQueueAction::Install,
                target_count: 1,
                executable_count: 1,
                skipped_count: 0,
                source_breakdown: vec![(PackageSource::Deb, 1)],
                risk_level: PreflightRiskLevel::Safe,
                risk_reasons: vec!["No additional guardrails triggered.".to_string()],
                certainty: PreflightCertainty::Estimated,
                elevated_privileges_likely: false,
                dependency_impact_known: false,
                dependency_impact: None,
                verification_in_progress: false,
                selection_mode: false,
            },
            risk_acknowledged: true,
        });
        app.handle_key(key(KeyCode::Char('y'))).await;
        assert_eq!(app.tasks.len(), 1);
    }

    #[tokio::test]
    async fn queue_cancel_and_retry_paths() {
        let mut app = test_app();
        app.executor_running.store(true, Ordering::SeqCst);
        app.queue_expanded = true;
        app.focus = Focus::Queue;

        let queued = TaskQueueEntry::new(
            TaskQueueAction::Install,
            "APT:a".into(),
            "a".into(),
            PackageSource::Apt,
        );

        let mut failed = TaskQueueEntry::new(
            TaskQueueAction::Update,
            "APT:b".into(),
            "b".into(),
            PackageSource::Apt,
        );
        failed.mark_failed("err".into());

        app.tasks = vec![queued.clone(), failed.clone()];

        app.task_cursor = 0;
        app.handle_key(key(KeyCode::Char('C'))).await;
        assert_eq!(app.tasks[0].status, TaskQueueStatus::Cancelled);

        app.task_cursor = 1;
        let before = app.tasks.len();
        app.handle_key(key(KeyCode::Char('R'))).await;
        assert_eq!(app.tasks.len(), before + 1);
        assert_eq!(app.tasks.last().unwrap().status, TaskQueueStatus::Queued);
    }

    #[tokio::test]
    async fn run_recommended_action_prefers_safe_retry_bundle() {
        let mut app = test_app();
        app.executor_running.store(true, Ordering::SeqCst);

        let mut failed = TaskQueueEntry::new(
            TaskQueueAction::Update,
            "APT:broken".into(),
            "broken".into(),
            PackageSource::Apt,
        );
        failed.mark_failed("temporary failure resolving mirror".into());
        app.tasks = vec![failed];

        app.execute_command(CommandId::RunRecommended).await;

        assert!(app.queue_expanded);
        assert_eq!(app.focus, Focus::Queue);
        assert_eq!(app.tasks.len(), 2);
        assert_eq!(app.tasks.last().unwrap().status, TaskQueueStatus::Queued);
        assert!(app.status.contains("Queued safe retry bundle"));
    }

    #[tokio::test]
    async fn retry_safe_failed_tasks_skips_non_safe_categories() {
        let mut app = test_app();
        app.executor_running.store(true, Ordering::SeqCst);
        app.queue_expanded = true;
        app.focus = Focus::Queue;

        let mut unknown = TaskQueueEntry::new(
            TaskQueueAction::Update,
            "APT:unknown".into(),
            "unknown".into(),
            PackageSource::Apt,
        );
        unknown.mark_failed("unexpected parse issue".into());
        let mut network = TaskQueueEntry::new(
            TaskQueueAction::Update,
            "APT:net".into(),
            "net".into(),
            PackageSource::Apt,
        );
        network.mark_failed("temporary failure resolving mirror".into());
        app.tasks = vec![unknown, network];

        let before = app.tasks.len();
        app.retry_safe_failed_tasks().await;

        assert_eq!(app.tasks.len(), before + 1);
        assert_eq!(app.tasks.last().unwrap().status, TaskQueueStatus::Queued);
        assert_eq!(app.tasks.last().unwrap().package_name, "net");
        assert!(app.status.contains("Queued safe retry bundle"));
    }

    #[tokio::test]
    async fn retry_safe_failed_tasks_respect_active_clinic_filter() {
        let mut app = test_app();
        app.executor_running.store(true, Ordering::SeqCst);
        app.queue_expanded = true;
        app.focus = Focus::Queue;

        let mut network_a = TaskQueueEntry::new(
            TaskQueueAction::Update,
            "APT:net-a".into(),
            "net-a".into(),
            PackageSource::Apt,
        );
        network_a.mark_failed("temporary failure resolving mirror".into());

        let mut network_b = TaskQueueEntry::new(
            TaskQueueAction::Update,
            "APT:net-b".into(),
            "net-b".into(),
            PackageSource::Apt,
        );
        network_b.mark_failed("temporary failure resolving mirror".into());

        let mut permissions = TaskQueueEntry::new(
            TaskQueueAction::Update,
            "APT:perm".into(),
            "perm".into(),
            PackageSource::Apt,
        );
        permissions.mark_failed("permission denied".into());

        app.tasks = vec![permissions, network_a, network_b];
        app.rebuild_failure_categories();
        app.handle_key(key(KeyCode::Char('2'))).await;
        let preview = app.queue_clinic_actionability();
        assert_eq!(preview.safe_retry_count, 2);
        app.handle_key(key(KeyCode::Char('A'))).await;

        let queued_count = app
            .tasks
            .iter()
            .filter(|task| task.status == TaskQueueStatus::Queued)
            .count();
        assert_eq!(queued_count, 2);
        assert!(app.status.contains("for 2 failures"));
        assert!(app.status.contains("Queued safe retry bundle [network]"));
    }

    #[tokio::test]
    async fn remediation_bundle_uses_active_clinic_filter_scope() {
        let mut app = test_app();
        app.executor_running.store(true, Ordering::SeqCst);
        app.queue_expanded = true;
        app.focus = Focus::Queue;

        let mut network_a = TaskQueueEntry::new(
            TaskQueueAction::Update,
            "APT:net-a".into(),
            "net-a".into(),
            PackageSource::Apt,
        );
        network_a.mark_failed("temporary failure resolving mirror".into());

        let mut network_b = TaskQueueEntry::new(
            TaskQueueAction::Update,
            "APT:net-b".into(),
            "net-b".into(),
            PackageSource::Apt,
        );
        network_b.mark_failed("temporary failure resolving mirror".into());

        let mut conflict = TaskQueueEntry::new(
            TaskQueueAction::Update,
            "APT:conflict".into(),
            "conflict".into(),
            PackageSource::Apt,
        );
        conflict.mark_failed("dpkg was interrupted, lock is held".into());

        app.tasks = vec![network_a, conflict, network_b];
        app.rebuild_failure_categories();

        app.handle_key(key(KeyCode::Char('2'))).await;
        app.handle_key(key(KeyCode::Char('M'))).await;

        let queued_count = app
            .tasks
            .iter()
            .filter(|task| task.status == TaskQueueStatus::Queued)
            .count();
        assert_eq!(queued_count, 2);
        assert!(app
            .status
            .contains("Remediation bundle [network] preview 2 tasks"));
        assert!(app.loading);
    }

    #[tokio::test]
    async fn queue_actionability_reports_filter_reasons_when_empty() {
        let mut app = test_app();
        app.queue_expanded = true;
        app.focus = Focus::Queue;

        let mut missing = TaskQueueEntry::new(
            TaskQueueAction::Install,
            "APT:missing".into(),
            "missing".into(),
            PackageSource::Apt,
        );
        missing.mark_failed("unable to locate package missing".into());
        app.tasks = vec![missing];
        app.rebuild_failure_categories();

        app.handle_key(key(KeyCode::Char('2'))).await;

        let preview = app.queue_clinic_actionability();
        assert_eq!(preview.failed_in_scope, 0);
        assert_eq!(preview.safe_retry_count, 0);
        assert_eq!(preview.remediation_actionable_count(), 0);

        assert_eq!(
            app.command_disabled_reason(CommandId::QueueRetrySafe),
            Some("No safe retries for network failures (press 0 for all)".to_string())
        );
        assert_eq!(
            app.command_disabled_reason(CommandId::QueueRemediate),
            Some("No remediation needed for network failures (press 0 for all)".to_string())
        );

        app.handle_key(key(KeyCode::Char('A'))).await;
        assert!(app.status.contains("No safe retries for network failures"));
    }

    #[tokio::test]
    async fn remediation_status_matches_actionability_counts() {
        let mut app = test_app();
        app.executor_running.store(true, Ordering::SeqCst);
        app.queue_expanded = true;
        app.focus = Focus::Queue;

        let mut network = TaskQueueEntry::new(
            TaskQueueAction::Update,
            "APT:net".into(),
            "net".into(),
            PackageSource::Apt,
        );
        network.mark_failed("temporary failure resolving mirror".into());

        let active_same_pkg = TaskQueueEntry::new(
            TaskQueueAction::Update,
            "APT:net".into(),
            "net".into(),
            PackageSource::Apt,
        );

        let mut missing = TaskQueueEntry::new(
            TaskQueueAction::Install,
            "APT:missing".into(),
            "missing".into(),
            PackageSource::Apt,
        );
        missing.mark_failed("unable to locate package missing".into());

        app.tasks = vec![network, active_same_pkg, missing];
        app.rebuild_failure_categories();
        let preview = app.queue_clinic_actionability();
        assert_eq!(preview.failed_in_scope, 2);
        assert_eq!(preview.remediation_retry_count, 0);
        assert_eq!(preview.remediation_guidance_count, 1);
        assert_eq!(preview.remediation_skipped_count, 1);

        app.handle_key(key(KeyCode::Char('M'))).await;

        assert_eq!(app.tasks.len(), 3);
        assert!(app.status.contains("preview 2 tasks"));
        assert!(app.status.contains("0 queued retry"));
        assert!(app.status.contains("1 guidance-only"));
        assert!(app.status.contains("1 skipped"));
        assert!(app.status.contains("verify package/source"));
    }

    #[test]
    fn queue_completion_digest_emits_when_queue_finishes() {
        let mut app = test_app();
        let mut running = TaskQueueEntry::new(
            TaskQueueAction::Install,
            "APT:pkg".into(),
            "pkg".into(),
            PackageSource::Apt,
        );
        running.mark_running();
        app.tasks = vec![running.clone()];

        let mut completed = running;
        completed.mark_completed();
        app.apply_task_event(TaskQueueEvent::Completed(completed));

        assert!(app.queue_completion_digest_emitted);
        assert!(app.status.starts_with("Queue finished: 1 done"));
    }

    #[tokio::test]
    async fn queue_tasks_prunes_terminal_entries_before_new_batch() {
        let mut app = test_app();
        app.executor_running.store(true, Ordering::SeqCst);

        let mut completed = TaskQueueEntry::new(
            TaskQueueAction::Install,
            "APT:old".into(),
            "old".into(),
            PackageSource::Apt,
        );
        completed.mark_completed();
        app.tasks = vec![completed];

        let queued = app
            .queue_tasks(
                vec![make_pkg(
                    "new",
                    PackageSource::Apt,
                    PackageStatus::NotInstalled,
                )],
                TaskQueueAction::Install,
            )
            .await;

        assert_eq!(queued, 1);
        assert_eq!(app.tasks.len(), 1);
        assert_eq!(app.tasks[0].package_name, "new");
        assert_eq!(app.tasks[0].status, TaskQueueStatus::Queued);
    }

    #[tokio::test]
    async fn queue_tasks_skips_duplicate_active_tasks() {
        let mut app = test_app();
        app.executor_running.store(true, Ordering::SeqCst);

        let queued_task = TaskQueueEntry::new(
            TaskQueueAction::Install,
            "APT:dup".into(),
            "dup".into(),
            PackageSource::Apt,
        );
        app.tasks = vec![queued_task.clone()];

        let queued = app
            .queue_tasks(
                vec![make_pkg(
                    "dup",
                    PackageSource::Apt,
                    PackageStatus::NotInstalled,
                )],
                TaskQueueAction::Install,
            )
            .await;

        assert_eq!(queued, 0);
        assert_eq!(app.tasks.len(), 1);
        assert_eq!(app.tasks[0].id, queued_task.id);
    }

    #[tokio::test]
    async fn queue_tasks_skips_conflicting_action_for_same_package() {
        let mut app = test_app();
        app.executor_running.store(true, Ordering::SeqCst);

        let queued_task = TaskQueueEntry::new(
            TaskQueueAction::Update,
            "APT:dup".into(),
            "dup".into(),
            PackageSource::Apt,
        );
        app.tasks = vec![queued_task.clone()];

        let queued = app
            .queue_tasks(
                vec![make_pkg(
                    "dup",
                    PackageSource::Apt,
                    PackageStatus::UpdateAvailable,
                )],
                TaskQueueAction::Remove,
            )
            .await;

        assert_eq!(queued, 0);
        assert_eq!(app.tasks.len(), 1);
        assert_eq!(app.tasks[0].id, queued_task.id);
    }

    #[tokio::test]
    async fn queue_tasks_prune_terminal_entries_removes_failed_tasks_too() {
        let mut app = test_app();
        app.executor_running.store(true, Ordering::SeqCst);

        let mut failed = TaskQueueEntry::new(
            TaskQueueAction::Install,
            "APT:old-fail".into(),
            "old-fail".into(),
            PackageSource::Apt,
        );
        failed.mark_failed("err".into());
        app.tasks = vec![failed];

        let queued = app
            .queue_tasks(
                vec![make_pkg(
                    "new",
                    PackageSource::Apt,
                    PackageStatus::NotInstalled,
                )],
                TaskQueueAction::Install,
            )
            .await;

        assert_eq!(queued, 1);
        assert_eq!(app.tasks.len(), 1);
        assert_eq!(app.tasks[0].package_name, "new");
        assert_eq!(app.tasks[0].status, TaskQueueStatus::Queued);
    }

    #[test]
    fn queue_counts_treat_recovered_failures_as_completed() {
        let mut app = test_app();
        let mut failed = TaskQueueEntry::new(
            TaskQueueAction::Update,
            "APT:pkg".into(),
            "pkg".into(),
            PackageSource::Apt,
        );
        failed.mark_failed("err".into());
        let failed_id = failed.id.clone();
        app.tasks = vec![failed];

        app.task_recovery_states.insert(
            failed_id,
            RecoveryState {
                attempts: 1,
                last_outcome: Some(TaskQueueStatus::Completed),
            },
        );

        let (queued, running, completed, failed, cancelled) = app.queue_counts();
        assert_eq!(
            (queued, running, completed, failed, cancelled),
            (0, 0, 1, 0, 0)
        );
    }

    #[test]
    fn session_queue_entries_keep_terminal_history_when_retained() {
        let mut completed = TaskQueueEntry::new(
            TaskQueueAction::Install,
            "APT:done".into(),
            "done".into(),
            PackageSource::Apt,
        );
        completed.mark_completed();

        let entries = App::session_queue_entries(vec![completed], true);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].status, TaskQueueStatus::Completed);
    }

    #[test]
    fn session_queue_entries_drop_terminal_history_when_not_retained() {
        let queued = TaskQueueEntry::new(
            TaskQueueAction::Install,
            "APT:queued".into(),
            "queued".into(),
            PackageSource::Apt,
        );
        let mut failed = TaskQueueEntry::new(
            TaskQueueAction::Remove,
            "APT:failed".into(),
            "failed".into(),
            PackageSource::Apt,
        );
        failed.mark_failed("nope".into());
        let mut cancelled = TaskQueueEntry::new(
            TaskQueueAction::Update,
            "APT:cancel".into(),
            "cancel".into(),
            PackageSource::Apt,
        );
        cancelled.mark_cancelled();

        let entries = App::session_queue_entries(vec![queued, failed, cancelled], false);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].status, TaskQueueStatus::Queued);
    }

    #[test]
    fn failure_classifier_maps_common_errors() {
        assert_eq!(
            App::classify_failure("permission denied while running command"),
            FailureCategory::Permissions
        );
        assert_eq!(
            App::classify_failure("temporary failure resolving archive.ubuntu.com"),
            FailureCategory::Network
        );
        assert_eq!(
            App::classify_failure("unable to locate package missing-pkg"),
            FailureCategory::NotFound
        );
        assert_eq!(
            App::classify_failure("dpkg was interrupted, lock is held by another process"),
            FailureCategory::Conflict
        );
        assert_eq!(
            App::classify_failure("unexpected parse issue"),
            FailureCategory::Unknown
        );
    }

    #[test]
    fn failure_category_codes_and_playbooks_are_stable() {
        assert_eq!(FailureCategory::Permissions.code(), "E_PERMISSION");
        assert_eq!(FailureCategory::Network.code(), "E_NETWORK");
        assert_eq!(FailureCategory::NotFound.code(), "E_NOT_FOUND");
        assert_eq!(FailureCategory::Conflict.code(), "E_CONFLICT");
        assert_eq!(FailureCategory::Unknown.code(), "E_UNKNOWN");

        assert!(FailureCategory::Permissions
            .action_hint()
            .contains("re-authenticate"));
        assert!(FailureCategory::Network
            .action_hint()
            .contains("refresh metadata"));
    }

    #[test]
    fn failed_task_event_sets_status_with_code_and_playbook() {
        let mut app = test_app();
        app.queue_completion_digest_emitted = true;

        let mut failed = TaskQueueEntry::new(
            TaskQueueAction::Update,
            "APT:pkg".into(),
            "pkg".into(),
            PackageSource::Apt,
        );
        failed.mark_failed("temporary failure resolving mirror".into());

        app.apply_task_event(TaskQueueEvent::Failed(failed));

        assert!(app.status.contains("[E_NETWORK]"));
        assert!(app.status.contains("[M] refresh metadata"));
    }

    #[tokio::test]
    async fn high_risk_confirm_requires_explicit_ack_before_queue() {
        let mut app = test_app();
        app.executor_running.store(true, Ordering::SeqCst);
        app.confirming = Some(PendingAction {
            label: "Remove pkg?".into(),
            packages: vec![make_pkg(
                "pkg",
                PackageSource::Apt,
                PackageStatus::Installed,
            )],
            action: TaskQueueAction::Remove,
            preflight: PreflightSummary {
                action: TaskQueueAction::Remove,
                target_count: 1,
                executable_count: 1,
                skipped_count: 0,
                source_breakdown: vec![(PackageSource::Apt, 1)],
                risk_level: PreflightRiskLevel::High,
                risk_reasons: vec![
                    "Includes system-level package sources (APT/DNF/Pacman/etc).".to_string(),
                ],
                certainty: PreflightCertainty::Estimated,
                elevated_privileges_likely: true,
                dependency_impact_known: false,
                dependency_impact: None,
                verification_in_progress: false,
                selection_mode: false,
            },
            risk_acknowledged: false,
        });

        app.handle_key(key(KeyCode::Char('y'))).await;
        assert!(app.confirming.is_some());
        assert_eq!(app.tasks.len(), 0);
        assert!(app
            .confirming
            .as_ref()
            .is_some_and(|pending| pending.risk_acknowledged));

        app.handle_key(key(KeyCode::Char('y'))).await;
        assert!(app.confirming.is_none());
        assert_eq!(app.tasks.len(), 1);
    }

    #[test]
    fn queue_remediate_command_enablement_follows_scope_actionability() {
        let mut app = test_app();
        app.queue_expanded = true;
        app.focus = Focus::Queue;

        let mut failed = TaskQueueEntry::new(
            TaskQueueAction::Update,
            "APT:a".into(),
            "a".into(),
            PackageSource::Apt,
        );
        failed.mark_failed("network timeout".into());
        let queued_same_pkg = TaskQueueEntry::new(
            TaskQueueAction::Update,
            "APT:a".into(),
            "a".into(),
            PackageSource::Apt,
        );
        app.tasks = vec![failed, queued_same_pkg];
        app.task_cursor = 1;

        assert!(!app.command_enabled(CommandId::QueueRemediate));

        app.tasks.truncate(1);
        app.task_cursor = 0;
        assert!(app.command_enabled(CommandId::QueueRemediate));
    }

    #[tokio::test]
    async fn remediation_not_found_guides_without_auto_retry() {
        let mut app = test_app();
        app.queue_expanded = true;
        app.focus = Focus::Queue;

        let mut failed = TaskQueueEntry::new(
            TaskQueueAction::Install,
            "APT:missing".into(),
            "missing".into(),
            PackageSource::Apt,
        );
        failed.mark_failed("unable to locate package missing".into());
        app.tasks = vec![failed];
        app.task_cursor = 0;

        app.handle_key(key(KeyCode::Char('M'))).await;

        assert_eq!(app.tasks.len(), 1);
        assert!(app.status.contains("verify package/source"));
    }

    #[tokio::test]
    async fn queue_log_scroll_keys_and_task_change_reset() {
        let mut app = test_app();
        app.queue_expanded = true;
        app.focus = Focus::Queue;
        let first = TaskQueueEntry::new(
            TaskQueueAction::Install,
            "APT:a".into(),
            "a".into(),
            PackageSource::Apt,
        );
        let second = TaskQueueEntry::new(
            TaskQueueAction::Update,
            "APT:b".into(),
            "b".into(),
            PackageSource::Apt,
        );
        app.tasks = vec![first.clone(), second.clone()];
        app.task_logs.insert(
            first.id.clone(),
            VecDeque::from(vec![
                "one".to_string(),
                "two".to_string(),
                "three".to_string(),
            ]),
        );

        app.handle_key(key(KeyCode::Char('['))).await;
        assert_eq!(app.task_log_scroll, 1);

        app.handle_key(key(KeyCode::Char(']'))).await;
        assert_eq!(app.task_log_scroll, 0);

        app.handle_key(key(KeyCode::Char('['))).await;
        app.handle_key(key(KeyCode::Char('j'))).await;
        assert_eq!(app.task_cursor, 1);
        assert_eq!(app.task_log_scroll, 0);
    }

    #[tokio::test]
    async fn queue_failure_filter_shortcuts_limit_visible_tasks() {
        let mut app = test_app();
        app.queue_expanded = true;
        app.focus = Focus::Queue;

        let queued = TaskQueueEntry::new(
            TaskQueueAction::Install,
            "APT:queued".into(),
            "queued".into(),
            PackageSource::Apt,
        );

        let mut permissions = TaskQueueEntry::new(
            TaskQueueAction::Update,
            "APT:perm".into(),
            "perm".into(),
            PackageSource::Apt,
        );
        permissions.mark_failed("permission denied".into());

        let mut network = TaskQueueEntry::new(
            TaskQueueAction::Update,
            "APT:net".into(),
            "net".into(),
            PackageSource::Apt,
        );
        network.mark_failed("temporary failure resolving mirror".into());

        app.tasks = vec![queued, permissions, network.clone()];
        app.rebuild_failure_categories();
        app.task_cursor = 1;

        app.handle_key(key(KeyCode::Char('2'))).await;
        assert_eq!(app.queue_failure_filter, QueueFailureFilter::Network);
        assert_eq!(app.queue_visible_task_indices(), vec![2]);
        assert_eq!(app.task_cursor, 2);

        app.handle_key(key(KeyCode::Char('0'))).await;
        assert_eq!(app.queue_failure_filter, QueueFailureFilter::All);
        assert_eq!(app.queue_visible_task_indices().len(), 3);
    }

    #[test]
    fn append_task_log_preserves_manual_scroll_position() {
        let mut app = test_app();
        app.queue_expanded = true;
        app.focus = Focus::Queue;

        let first = TaskQueueEntry::new(
            TaskQueueAction::Install,
            "APT:a".into(),
            "a".into(),
            PackageSource::Apt,
        );
        let second = TaskQueueEntry::new(
            TaskQueueAction::Install,
            "APT:b".into(),
            "b".into(),
            PackageSource::Apt,
        );
        app.tasks = vec![first.clone(), second];
        app.task_cursor = 0;
        app.task_logs.insert(
            first.id.clone(),
            VecDeque::from(vec![
                "one".to_string(),
                "two".to_string(),
                "three".to_string(),
                "four".to_string(),
            ]),
        );

        app.task_log_scroll = 2;
        app.append_task_log(&first.id, "five".to_string());
        assert_eq!(app.task_log_scroll, 3);

        app.task_log_scroll = 0;
        app.append_task_log(&first.id, "six".to_string());
        assert_eq!(app.task_log_scroll, 0);

        app.task_cursor = 1;
        app.task_log_scroll = 2;
        app.append_task_log(&first.id, "seven".to_string());
        assert_eq!(app.task_log_scroll, 2);

        let mut many_logs = VecDeque::new();
        for idx in 0..MAX_TASK_LOG_LINES {
            many_logs.push_back(format!("line-{idx}"));
        }
        app.task_cursor = 0;
        app.task_log_scroll = 2;
        app.task_logs.insert(first.id.clone(), many_logs);
        app.append_task_log(&first.id, "trim-trigger".to_string());
        assert_eq!(app.task_log_scroll, 3);
    }

    #[tokio::test]
    async fn expanding_queue_resets_log_scroll() {
        let mut app = test_app();
        let task = TaskQueueEntry::new(
            TaskQueueAction::Install,
            "APT:a".into(),
            "a".into(),
            PackageSource::Apt,
        );
        app.tasks = vec![task];
        app.task_log_scroll = 5;

        app.handle_key(key(KeyCode::Char('l'))).await;

        assert!(app.queue_expanded);
        assert_eq!(app.focus, Focus::Queue);
        assert_eq!(app.task_log_scroll, 0);
    }

    #[tokio::test]
    async fn ctrl_d_and_ctrl_u_navigation_work() {
        let mut app = test_app();
        app.focus = Focus::Packages;
        app.packages = (0..30)
            .map(|idx| {
                make_pkg(
                    &format!("p{}", idx),
                    PackageSource::Apt,
                    PackageStatus::Installed,
                )
            })
            .collect();
        app.apply_filters();

        app.handle_key(ctrl('d')).await;
        assert!(app.cursor > 0);

        app.handle_key(ctrl('u')).await;
        assert_eq!(app.cursor, 0);
    }

    #[tokio::test]
    async fn mouse_click_filter_tab_updates_filter() {
        let mut app = test_app();
        app.packages = vec![
            make_pkg("a", PackageSource::Apt, PackageStatus::Installed),
            make_pkg("b", PackageSource::Apt, PackageStatus::UpdateAvailable),
        ];
        app.apply_filters();

        let regions = layout_regions(&app);
        let row = regions.header_filter_row.y;
        let installed_col = (regions.header_filter_row.x
            ..regions.header_filter_row.x + regions.header_filter_row.width)
            .find(|col| {
                ui::header_filter_hit_test(&app, regions.header_filter_row, *col, row)
                    == Some(Filter::Installed)
            })
            .expect("installed tab column");

        app.handle_mouse(
            mouse(MouseEventKind::Down(MouseButton::Left), installed_col, row),
            &regions,
        )
        .await;

        assert_eq!(app.filter, Filter::Installed);
    }

    #[tokio::test]
    async fn mouse_drag_selects_package_range() {
        let mut app = test_app();
        app.focus = Focus::Packages;
        app.packages = (0..6)
            .map(|idx| {
                make_pkg(
                    &format!("pkg{}", idx),
                    PackageSource::Apt,
                    PackageStatus::Installed,
                )
            })
            .collect();
        app.apply_filters();

        let regions = layout_regions(&app);
        let col = regions.packages.x + 6;
        let first_row = regions.packages.y + 2;
        let third_row = first_row + 2;

        app.handle_mouse(
            mouse(MouseEventKind::Down(MouseButton::Left), col, first_row),
            &regions,
        )
        .await;
        app.handle_mouse(
            mouse(MouseEventKind::Drag(MouseButton::Left), col, third_row),
            &regions,
        )
        .await;
        app.handle_mouse(
            mouse(MouseEventKind::Up(MouseButton::Left), col, third_row),
            &regions,
        )
        .await;

        assert_eq!(app.selected.len(), 3);
    }

    #[tokio::test]
    async fn mouse_click_favorite_column_toggles_favorite() {
        let mut app = test_app();
        app.focus = Focus::Packages;
        app.packages = vec![make_pkg(
            "pkg",
            PackageSource::Apt,
            PackageStatus::Installed,
        )];
        app.apply_filters();

        let package_id = app.packages[0].id();
        let regions = layout_regions(&app);
        let favorite_col = regions.packages.x + 4;
        let row = regions.packages.y + 2;

        app.handle_mouse(
            mouse(MouseEventKind::Down(MouseButton::Left), favorite_col, row),
            &regions,
        )
        .await;

        assert!(app.favorite_packages.contains(&package_id));
        assert_eq!(app.filter_counts[3], 1);
    }

    #[tokio::test]
    async fn mouse_right_click_uses_default_package_action() {
        let mut app = test_app();
        app.focus = Focus::Packages;
        app.packages = vec![make_pkg(
            "pkg",
            PackageSource::Apt,
            PackageStatus::NotInstalled,
        )];
        app.apply_filters();

        let regions = layout_regions(&app);
        let col = regions.packages.x + 6;
        let row = regions.packages.y + 2;

        app.handle_mouse(
            mouse(MouseEventKind::Down(MouseButton::Right), col, row),
            &regions,
        )
        .await;

        let confirming = app.confirming.as_ref().expect("confirming action");
        assert_eq!(confirming.action, TaskQueueAction::Install);
    }

    #[tokio::test]
    async fn mouse_confirm_yes_queues_action() {
        let mut app = test_app();
        app.executor_running.store(true, Ordering::SeqCst);
        app.confirming = Some(PendingAction {
            label: "Install pkg?".into(),
            packages: vec![make_pkg(
                "pkg",
                PackageSource::Deb,
                PackageStatus::NotInstalled,
            )],
            action: TaskQueueAction::Install,
            preflight: PreflightSummary {
                action: TaskQueueAction::Install,
                target_count: 1,
                executable_count: 1,
                skipped_count: 0,
                source_breakdown: vec![(PackageSource::Deb, 1)],
                risk_level: PreflightRiskLevel::Safe,
                risk_reasons: vec!["No additional guardrails triggered.".to_string()],
                certainty: PreflightCertainty::Estimated,
                elevated_privileges_likely: false,
                dependency_impact_known: false,
                dependency_impact: None,
                verification_in_progress: false,
                selection_mode: false,
            },
            risk_acknowledged: true,
        });
        app.tasks.push(TaskQueueEntry::new(
            TaskQueueAction::Install,
            "APT:seed".into(),
            "seed".into(),
            PackageSource::Apt,
        ));

        let regions = layout_regions(&app);
        let before = app.tasks.len();
        let row = regions
            .preflight_modal
            .y
            .saturating_add(regions.preflight_modal.height.saturating_sub(2));
        let yes_col = (regions.preflight_modal.x
            ..regions.preflight_modal.x + regions.preflight_modal.width)
            .find(|col| {
                ui::preflight_modal_hit_test(regions.preflight_modal, *col, row) == Some(true)
            })
            .expect("yes area");

        app.handle_mouse(
            mouse(MouseEventKind::Down(MouseButton::Left), yes_col, row),
            &regions,
        )
        .await;

        assert!(app.confirming.is_none());
        assert_eq!(app.tasks.len(), before + 1);
    }

    #[tokio::test]
    async fn mouse_queue_scroll_respects_task_and_log_regions() {
        let mut app = test_app();
        app.queue_expanded = true;
        app.focus = Focus::Queue;

        let first = TaskQueueEntry::new(
            TaskQueueAction::Install,
            "APT:a".into(),
            "a".into(),
            PackageSource::Apt,
        );
        let second = TaskQueueEntry::new(
            TaskQueueAction::Update,
            "APT:b".into(),
            "b".into(),
            PackageSource::Apt,
        );
        app.tasks = vec![first.clone(), second];
        app.task_cursor = 1;
        app.task_logs.insert(
            first.id.clone(),
            VecDeque::from(vec![
                "one".to_string(),
                "two".to_string(),
                "three".to_string(),
            ]),
        );

        let regions = layout_regions(&app);
        assert!(regions.expanded_queue_tasks.width > 0);
        assert!(regions.expanded_queue_logs.width > 0);

        app.handle_mouse(
            mouse(
                MouseEventKind::ScrollUp,
                regions.expanded_queue_tasks.x,
                regions.expanded_queue_tasks.y,
            ),
            &regions,
        )
        .await;
        assert_eq!(app.task_cursor, 0);

        app.task_log_scroll = 0;
        app.handle_mouse(
            mouse(
                MouseEventKind::ScrollUp,
                regions.expanded_queue_logs.x,
                regions.expanded_queue_logs.y,
            ),
            &regions,
        )
        .await;
        assert_eq!(app.task_cursor, 0);
        assert_eq!(app.task_log_scroll, 1);
    }

    #[tokio::test]
    async fn mouse_queue_hint_clicks_retry() {
        let mut app = test_app();
        app.executor_running.store(true, Ordering::SeqCst);
        app.queue_expanded = true;
        app.focus = Focus::Queue;

        let mut failed = TaskQueueEntry::new(
            TaskQueueAction::Update,
            "APT:b".into(),
            "b".into(),
            PackageSource::Apt,
        );
        failed.mark_failed("err".into());
        app.tasks = vec![failed];

        let regions = layout_regions(&app);
        let hint_row = regions.expanded_queue_hints.y;
        let before = app.tasks.len();
        let retry_col = (regions.expanded_queue_hints.x
            ..regions.expanded_queue_hints.x + regions.expanded_queue_hints.width)
            .find(|col| {
                ui::queue_hint_hit_test(
                    regions.expanded_queue_hints,
                    regions.expanded_queue_logs.width > 0,
                    *col,
                    hint_row,
                ) == Some(ui::QueueHintAction::Retry)
            })
            .expect("retry area");
        app.handle_mouse(
            mouse(MouseEventKind::Down(MouseButton::Left), retry_col, hint_row),
            &regions,
        )
        .await;
        assert_eq!(app.tasks.len(), before + 1);
        assert_eq!(app.tasks.last().unwrap().status, TaskQueueStatus::Queued);
    }

    #[tokio::test]
    async fn retry_lineage_persists_after_retry_completes() {
        let mut app = test_app();
        app.executor_running.store(true, Ordering::SeqCst);
        app.queue_expanded = true;
        app.focus = Focus::Queue;

        let mut failed = TaskQueueEntry::new(
            TaskQueueAction::Update,
            "APT:pkg".into(),
            "pkg".into(),
            PackageSource::Apt,
        );
        failed.mark_failed("err".into());
        let parent_id = failed.id.clone();
        app.tasks = vec![failed];
        app.task_cursor = 0;

        app.retry_selected_task().await;

        let retry = app.tasks.last().cloned().expect("retry task");
        assert_eq!(
            app.retry_attempt_for_task(&retry.id),
            Some(1),
            "retry attempt metadata should be set"
        );
        assert!(
            app.retry_parent_for_task(&retry.id).is_some(),
            "retry parent mapping should exist"
        );

        let mut completed_retry = retry.clone();
        completed_retry.mark_completed();
        app.apply_task_event(TaskQueueEvent::Completed(completed_retry));

        assert_eq!(app.retry_attempt_for_task(&retry.id), Some(1));
        assert!(app.retry_parent_for_task(&retry.id).is_some());
        assert_eq!(
            app.recovery_state_for_task(&parent_id)
                .and_then(|state| state.last_outcome),
            Some(TaskQueueStatus::Completed)
        );
    }

    #[test]
    fn palette_entries_include_disabled_reasons_and_fuzzy_filter() {
        let mut app = test_app();
        app.focus = Focus::Packages;
        app.packages = vec![make_pkg(
            "pkg",
            PackageSource::Apt,
            PackageStatus::NotInstalled,
        )];
        app.apply_filters();
        app.palette_query = "remove".to_string();

        let entries = app.palette_entries();
        let remove = entries
            .iter()
            .find(|entry| entry.id == CommandId::Remove)
            .expect("remove command present");
        assert!(!remove.enabled);
        assert!(remove
            .disabled_reason
            .as_deref()
            .is_some_and(|reason| reason.contains("removable package")));
    }

    #[tokio::test]
    async fn execute_command_surfaces_disabled_reason_and_can_queue_install() {
        let mut app = test_app();
        app.focus = Focus::Packages;
        app.packages = vec![make_pkg(
            "pkg",
            PackageSource::Apt,
            PackageStatus::NotInstalled,
        )];
        app.apply_filters();

        app.execute_command(CommandId::Remove).await;
        assert_eq!(app.status, "pkg is not installed");

        app.execute_command(CommandId::Install).await;
        assert!(app.confirming.is_some());
    }

    #[tokio::test]
    async fn bulk_favorites_toggle_adds_and_removes_all_targets() {
        let mut app = test_app();
        app.focus = Focus::Packages;
        app.packages = vec![
            make_pkg("a", PackageSource::Apt, PackageStatus::Installed),
            make_pkg("b", PackageSource::Apt, PackageStatus::Installed),
        ];
        app.apply_filters();
        app.selected.insert(app.packages[0].id());
        app.selected.insert(app.packages[1].id());

        app.execute_command(CommandId::BulkToggleFavorite).await;
        assert_eq!(app.favorite_packages.len(), 2);

        app.execute_command(CommandId::BulkToggleFavorite).await;
        assert!(app.favorite_packages.is_empty());
    }

    #[tokio::test]
    async fn favorites_updates_only_mode_filters_favorites_view() {
        let mut app = test_app();
        app.focus = Focus::Packages;
        let stale = make_pkg("stale", PackageSource::Apt, PackageStatus::Installed);
        let update = make_pkg("update", PackageSource::Apt, PackageStatus::UpdateAvailable);
        app.favorite_packages.insert(stale.id());
        app.favorite_packages.insert(update.id());
        app.packages = vec![stale, update];
        app.filter = Filter::Favorites;
        app.apply_filters();
        assert_eq!(app.filtered.len(), 2);

        app.execute_command(CommandId::ToggleFavoritesUpdatesOnly)
            .await;
        assert!(app.favorites_updates_only);
        assert_eq!(app.filtered.len(), 1);
        assert_eq!(
            app.current_package().map(|pkg| pkg.name.as_str()),
            Some("update")
        );
    }

    #[test]
    fn session_state_round_trip_config_helpers() {
        let mut app = test_app();
        let favorite = make_pkg("vim", PackageSource::Apt, PackageStatus::UpdateAvailable);
        app.available_sources = vec![PackageSource::Apt, PackageSource::Snap];
        app.favorite_packages.insert(favorite.id());
        app.packages = vec![favorite.clone()];

        let mut config = Config::default();
        config.tui_last_filter = Some("favorites".to_string());
        config.tui_last_focus = Some("queue".to_string());
        config.tui_last_search = "vim".to_string();
        config.tui_last_cursor = 9;
        config.tui_favorites_updates_only = true;
        config.last_source_filter = Some("apt".to_string());

        app.apply_session_from_config(&config);
        assert_eq!(app.filter, Filter::Favorites);
        assert_eq!(app.focus, Focus::Packages);
        assert_eq!(app.search, "vim");
        assert_eq!(app.source, Some(PackageSource::Apt));
        assert!(app.favorites_updates_only);
        assert_eq!(app.cursor, 0);

        let mut saved = Config::default();
        app.write_session_to_config(&mut saved);
        assert_eq!(saved.tui_last_filter.as_deref(), Some("favorites"));
        assert_eq!(saved.tui_last_focus.as_deref(), Some("packages"));
        assert_eq!(saved.tui_last_search, "vim");
        assert_eq!(saved.tui_last_cursor, 0);
        assert!(saved.tui_favorites_updates_only);
        assert_eq!(saved.last_source_filter.as_deref(), Some("apt"));
    }

    #[tokio::test]
    async fn palette_enter_executes_selected_command() {
        let mut app = test_app();
        app.focus = Focus::Packages;
        app.packages = vec![make_pkg(
            "pkg",
            PackageSource::Apt,
            PackageStatus::Installed,
        )];
        app.apply_filters();

        app.handle_key(key(KeyCode::Char(':'))).await;
        assert!(app.showing_palette);

        let entries = app.palette_entries();
        let idx = entries
            .iter()
            .position(|entry| entry.id == CommandId::FilterFavorites)
            .expect("favorites filter command present");
        app.palette_cursor = idx;

        app.handle_key(key(KeyCode::Enter)).await;
        assert!(!app.showing_palette);
        assert_eq!(app.filter, Filter::Favorites);
    }

    #[tokio::test]
    async fn changelog_overlay_open_close_and_scroll_keys() {
        let mut app = test_app();
        app.focus = Focus::Packages;
        app.packages = vec![make_pkg(
            "pkg",
            PackageSource::Apt,
            PackageStatus::Installed,
        )];
        app.apply_filters();

        app.handle_key(key(KeyCode::Char('c'))).await;
        assert!(app.showing_changelog);
        assert!(!app.changelog_diff_only);
        assert_eq!(
            app.changelog_target_package().map(|p| p.name.as_str()),
            Some("pkg")
        );
        assert!(matches!(
            app.changelog_state_for_target(),
            Some(ChangelogState::Loading)
        ));

        app.handle_key(key(KeyCode::Char('j'))).await;
        assert_eq!(app.changelog_scroll, 3);
        app.handle_key(key(KeyCode::Char('k'))).await;
        assert_eq!(app.changelog_scroll, 0);
        app.handle_key(key(KeyCode::Char('v'))).await;
        assert!(app.changelog_diff_only);

        app.handle_key(key(KeyCode::Esc)).await;
        assert!(!app.showing_changelog);
        assert!(!app.changelog_diff_only);
        assert!(app.changelog_target_package_id.is_none());
    }

    #[tokio::test]
    async fn changelog_overlay_defaults_to_delta_for_updates() {
        let mut app = test_app();
        app.focus = Focus::Packages;
        app.packages = vec![make_pkg(
            "pkg",
            PackageSource::Apt,
            PackageStatus::UpdateAvailable,
        )];
        app.apply_filters();

        app.handle_key(key(KeyCode::Char('c'))).await;
        assert!(app.showing_changelog);
        assert!(app.changelog_diff_only);
    }

    #[tokio::test]
    async fn changelog_overlay_action_keys_open_preflight_when_valid() {
        let mut app = test_app();
        app.focus = Focus::Packages;
        app.packages = vec![make_pkg(
            "pkg",
            PackageSource::Apt,
            PackageStatus::UpdateAvailable,
        )];
        app.apply_filters();

        app.handle_key(key(KeyCode::Char('c'))).await;
        assert!(app.showing_changelog);

        app.handle_key(key(KeyCode::Char('u'))).await;
        assert!(!app.showing_changelog);
        let confirming = app.confirming.as_ref().expect("preflight expected");
        assert_eq!(confirming.action, TaskQueueAction::Update);
        assert_eq!(confirming.packages.len(), 1);
        assert_eq!(confirming.packages[0].name, "pkg");
    }

    #[tokio::test]
    async fn changelog_overlay_action_keys_keep_overlay_open_when_invalid() {
        let mut app = test_app();
        app.focus = Focus::Packages;
        app.packages = vec![make_pkg(
            "pkg",
            PackageSource::Apt,
            PackageStatus::Installed,
        )];
        app.apply_filters();

        app.handle_key(key(KeyCode::Char('c'))).await;
        assert!(app.showing_changelog);

        app.handle_key(key(KeyCode::Char('i'))).await;
        assert!(app.showing_changelog);
        assert!(app.confirming.is_none());
        assert!(
            app.status.contains("already installed"),
            "expected invalid install feedback, got: {}",
            app.status
        );
    }

    #[tokio::test]
    async fn poll_changelog_caches_ready_state() {
        let mut app = test_app();
        app.packages = vec![make_pkg(
            "pkg",
            PackageSource::Apt,
            PackageStatus::Installed,
        )];
        app.apply_filters();
        app.showing_changelog = true;
        let pkg_id = app.packages[0].id();
        app.changelog_target_package_id = Some(pkg_id.clone());

        if let Some(tx) = app.changelog_tx.clone() {
            tx.send(ChangelogResult {
                package_id: pkg_id.clone(),
                package_name: "pkg".to_string(),
                result: Ok(Some(
                    "# pkg\n\n## release\n\n- fixed crash\n- security hardening".to_string(),
                )),
            })
            .await
            .expect("send changelog event");
        }

        app.poll_changelog();

        let Some(ChangelogState::Ready { summary, .. }) = app.changelog_cache.get(&pkg_id) else {
            panic!("expected ready changelog state");
        };
        assert!(summary.total_changes() >= 1);
    }
}
