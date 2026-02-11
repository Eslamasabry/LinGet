use super::ui;
use super::update_center;
use crate::backend::{HistoryTracker, PackageManager, TaskQueueEvent, TaskQueueExecutor};
use crate::models::history::{TaskQueueAction, TaskQueueEntry, TaskQueueStatus};
use crate::models::{Package, PackageSource, PackageStatus};
use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::collections::{HashMap, HashSet, VecDeque};
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, Mutex};
use tracing::{debug, error};

const COMPACT_WIDTH: u16 = 90;
const COMPACT_HEIGHT: u16 = 24;
pub const MIN_WIDTH: u16 = 60;
pub const MIN_HEIGHT: u16 = 15;
const HALF_PAGE: usize = 10;
const MAX_TASK_LOG_LINES: usize = 500;
const QUEUE_AUTO_HIDE_AFTER: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Filter {
    All,
    Installed,
    Updates,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Sources,
    Packages,
    Queue,
}

#[derive(Debug, Clone)]
pub struct PendingAction {
    pub label: String,
    pub packages: Vec<Package>,
    pub action: TaskQueueAction,
}

type LoadResult = Result<Vec<Package>, String>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CommandId {
    Quit,
    ShowHelp,
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
    ToggleSelection,
    SelectAll,
    Install,
    Remove,
    Update,
    Search,
    Refresh,
    ToggleQueue,
    QueueCancel,
    QueueRetry,
    QueueLogOlder,
    QueueLogNewer,
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
    pub filter_counts: [usize; 3],
    pub cursor: usize,
    pub selected: HashSet<String>,

    pub filter: Filter,
    pub source: Option<PackageSource>,
    pub search: String,
    pub searching: bool,

    pub tasks: Vec<TaskQueueEntry>,
    pub task_cursor: usize,
    pub task_log_scroll: usize,
    pub task_logs: HashMap<String, VecDeque<String>>,
    pub previous_statuses: HashMap<String, PackageStatus>,
    pub queue_expanded: bool,
    pub queue_completed_at: Option<Instant>,
    pub executor_running: Arc<AtomicBool>,
    pub queue_failures_acknowledged: bool,

    pub focus: Focus,
    pub compact: bool,
    pub confirming: Option<PendingAction>,
    pub showing_help: bool,

    pub status: String,
    pub clear_status_on_key: bool,
    pub loading: bool,
    pub should_quit: bool,
    pub tick: u64,

    pub available_sources: Vec<PackageSource>,
    pub source_counts: HashMap<PackageSource, (usize, usize)>,

    pub pm: Arc<Mutex<PackageManager>>,
    pub history_tracker: Arc<Mutex<Option<HistoryTracker>>>,
    pub load_rx: Option<mpsc::Receiver<LoadResult>>,
    pub task_events_rx: Option<mpsc::Receiver<TaskQueueEvent>>,
    pub task_events_tx: Option<mpsc::Sender<TaskQueueEvent>>,
    pub refresh_after_idle: bool,
    pub cursor_anchor_id: Option<String>,
}

impl App {
    pub fn new(
        pm: Arc<Mutex<PackageManager>>,
        history_tracker: Arc<Mutex<Option<HistoryTracker>>>,
        task_events_rx: Option<mpsc::Receiver<TaskQueueEvent>>,
        task_events_tx: Option<mpsc::Sender<TaskQueueEvent>>,
    ) -> Self {
        Self {
            packages: Vec::new(),
            filtered: Vec::new(),
            filter_counts: [0, 0, 0],
            cursor: 0,
            selected: HashSet::new(),
            filter: Filter::All,
            source: None,
            search: String::new(),
            searching: false,
            tasks: Vec::new(),
            task_cursor: 0,
            task_log_scroll: 0,
            task_logs: HashMap::new(),
            previous_statuses: HashMap::new(),
            queue_expanded: false,
            queue_completed_at: None,
            executor_running: Arc::new(AtomicBool::new(false)),
            queue_failures_acknowledged: false,
            focus: Focus::Sources,
            compact: false,
            confirming: None,
            showing_help: false,
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
            refresh_after_idle: false,
            cursor_anchor_id: None,
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
        let entries = {
            let guard = self.history_tracker.lock().await;
            guard
                .as_ref()
                .map(|tracker| tracker.history().task_queue.entries.clone())
                .unwrap_or_default()
        };
        self.tasks = entries;
        self.clamp_task_cursor();
    }

    pub async fn load_sources(&mut self) {
        let manager = self.pm.lock().await;
        self.available_sources = manager.available_sources().into_iter().collect();
        self.available_sources
            .sort_by_key(|source| source.to_string());
    }

    pub fn source_count(&self) -> usize {
        self.visible_sources().len() + 1
    }

    pub fn visible_sources(&self) -> Vec<PackageSource> {
        if self.filter == Filter::Updates {
            self.available_sources
                .iter()
                .filter(|s| {
                    self.source_counts
                        .get(s)
                        .is_some_and(|(_, updates)| *updates > 0)
                })
                .copied()
                .collect()
        } else {
            self.available_sources.clone()
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

    fn can_select_all_command(&self) -> bool {
        !self.filtered.is_empty()
    }

    fn can_prepare_command(&self, action: TaskQueueAction) -> bool {
        let targets = self.collect_action_targets();
        !targets.is_empty()
            && targets
                .iter()
                .any(|package| Self::is_valid_target(action, package))
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

    pub fn apply_filters(&mut self) {
        let mut n_all = 0usize;
        let mut n_installed = 0usize;
        let mut n_updates = 0usize;

        let mut per_source: HashMap<PackageSource, (usize, usize)> = HashMap::new();

        for package in &self.packages {
            n_all += 1;
            let entry = per_source.entry(package.source).or_insert((0, 0));
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
            if is_installed {
                n_installed += 1;
                entry.0 += 1;
            }
            if is_update {
                n_updates += 1;
                entry.1 += 1;
            }
        }
        self.filter_counts = [n_all, n_installed, n_updates];
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
                Self::matches_filter(package, self.filter)
                    && self.source.is_none_or(|source| package.source == source)
                    && (query.is_empty()
                        || package.name.to_lowercase().contains(&query)
                        || package.description.to_lowercase().contains(&query))
            })
            .map(|(idx, _)| idx)
            .collect();

        if self.filter == Filter::Updates {
            self.sort_updates_by_priority();
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

    fn matches_filter(package: &Package, filter: Filter) -> bool {
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
        let logs = self.task_logs.entry(entry_id.to_string()).or_default();
        logs.push_back(line);
        while logs.len() > MAX_TASK_LOG_LINES {
            logs.pop_front();
        }
    }

    fn cleanup_task_logs(&mut self) {
        let valid: HashSet<&str> = self.tasks.iter().map(|task| task.id.as_str()).collect();
        self.task_logs
            .retain(|task_id, _| valid.contains(task_id.as_str()));
    }

    fn apply_task_event(&mut self, event: TaskQueueEvent) {
        match event {
            TaskQueueEvent::Started(entry) => {
                self.upsert_task(entry.clone());
                self.mark_package_started(&entry);
                self.queue_completed_at = None;
                self.apply_filters();
            }
            TaskQueueEvent::Completed(entry) => {
                self.upsert_task(entry.clone());
                self.mark_package_completed(&entry);
                self.refresh_after_idle = true;
                self.apply_filters();
            }
            TaskQueueEvent::Failed(entry) => {
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
        self.cleanup_task_logs();
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
            .any(|task| task.status == TaskQueueStatus::Failed);
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
            self.previous_statuses.clear();
            self.task_cursor = 0;
            self.task_log_scroll = 0;
            self.queue_completed_at = None;
            self.queue_failures_acknowledged = false;
        }
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
            if self
                .tasks
                .iter()
                .any(|task| task.status == TaskQueueStatus::Failed)
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
        self.set_task_cursor(0);
    }

    fn queue_bottom(&mut self) {
        if !self.tasks.is_empty() {
            self.set_task_cursor(self.tasks.len() - 1);
        }
    }

    fn queue_next(&mut self) {
        if self.tasks.is_empty() {
            self.set_task_cursor(0);
            return;
        }
        self.set_task_cursor((self.task_cursor + 1).min(self.tasks.len() - 1));
    }

    fn queue_prev(&mut self) {
        self.set_task_cursor(self.task_cursor.saturating_sub(1));
    }

    fn queue_page_down(&mut self) {
        if self.tasks.is_empty() {
            return;
        }
        self.set_task_cursor((self.task_cursor + HALF_PAGE).min(self.tasks.len() - 1));
    }

    fn queue_page_up(&mut self) {
        self.set_task_cursor(self.task_cursor.saturating_sub(HALF_PAGE));
    }

    fn queue_log_scroll_up(&mut self) {
        self.task_log_scroll = (self.task_log_scroll + 1).min(self.queue_log_max_scroll());
    }

    fn queue_log_scroll_down(&mut self) {
        self.task_log_scroll = self.task_log_scroll.saturating_sub(1);
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

    fn prepare_action(&mut self, action: TaskQueueAction) {
        let targets = self.collect_action_targets();
        if targets.is_empty() {
            self.set_status("No package selected", true);
            return;
        }

        let valid: Vec<Package> = targets
            .iter()
            .filter(|package| Self::is_valid_target(action, package))
            .cloned()
            .collect();

        if valid.is_empty() {
            if self.selected.is_empty() {
                if let Some(target) = targets.first() {
                    self.set_status(Self::invalid_single_target_message(action, target), true);
                }
            } else {
                self.set_status(Self::invalid_batch_message(action), true);
            }
            return;
        }

        let skipped = targets.len().saturating_sub(valid.len());
        let label = Self::build_confirm_label(
            action,
            &valid,
            targets.len(),
            skipped,
            !self.selected.is_empty(),
        );
        self.confirming = Some(PendingAction {
            label,
            packages: valid,
            action,
        });
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
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                if let Some(action) = self.confirming.take() {
                    let queued = self.queue_tasks(action.packages, action.action).await;
                    self.clear_selection();
                    self.set_status(
                        format!(
                            "Queued {} {} task{}",
                            queued,
                            action_label(action.action).to_lowercase(),
                            if queued == 1 { "" } else { "s" }
                        ),
                        true,
                    );
                }
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                self.confirming = None;
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
            KeyCode::Char('c') | KeyCode::Char('C') => self.cancel_selected_task().await,
            KeyCode::Char('r') | KeyCode::Char('R') => self.retry_selected_task().await,
            _ => {}
        }
    }

    async fn handle_normal_key(&mut self, key: KeyEvent) {
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.should_quit = true;
            return;
        }

        match key.code {
            KeyCode::Char('q') if self.command_enabled(CommandId::Quit) => {
                self.should_quit = true;
                return;
            }
            KeyCode::Char('?') if self.command_enabled(CommandId::ShowHelp) => {
                self.showing_help = true;
                return;
            }
            _ => {}
        }

        if self.queue_expanded && self.focus == Focus::Queue {
            match key.code {
                KeyCode::Esc | KeyCode::Char('l') => {
                    self.toggle_queue_expanded();
                    return;
                }
                KeyCode::Char('j') | KeyCode::Down => {
                    self.queue_next();
                    return;
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    self.queue_prev();
                    return;
                }
                KeyCode::Char('g') | KeyCode::Home => {
                    self.queue_top();
                    return;
                }
                KeyCode::Char('G') | KeyCode::End => {
                    self.queue_bottom();
                    return;
                }
                KeyCode::PageDown => {
                    self.queue_page_down();
                    return;
                }
                KeyCode::PageUp => {
                    self.queue_page_up();
                    return;
                }
                KeyCode::Char('[') => {
                    self.queue_log_scroll_up();
                    return;
                }
                KeyCode::Char(']') => {
                    self.queue_log_scroll_down();
                    return;
                }
                _ if key.code == KeyCode::Char('d')
                    && key.modifiers.contains(KeyModifiers::CONTROL) =>
                {
                    self.queue_page_down();
                    return;
                }
                _ if key.code == KeyCode::Char('u')
                    && key.modifiers.contains(KeyModifiers::CONTROL) =>
                {
                    self.queue_page_up();
                    return;
                }
                _ => {
                    self.handle_queue_shortcuts(key).await;
                    return;
                }
            }
        }

        match key.code {
            KeyCode::Tab => {
                if self.command_enabled(CommandId::CycleFocus) {
                    self.focus = match self.focus {
                        Focus::Sources => Focus::Packages,
                        Focus::Packages | Focus::Queue => Focus::Sources,
                    };
                }
            }
            KeyCode::Char('j') | KeyCode::Down => match self.focus {
                Focus::Sources => self.next_source(),
                Focus::Packages | Focus::Queue => self.next_package(),
            },
            KeyCode::Char('k') | KeyCode::Up => match self.focus {
                Focus::Sources => self.prev_source(),
                Focus::Packages | Focus::Queue => self.prev_package(),
            },
            KeyCode::Char('g') | KeyCode::Home => match self.focus {
                Focus::Sources => self.set_source_by_index(0),
                Focus::Packages | Focus::Queue => self.top(),
            },
            KeyCode::Char('G') | KeyCode::End => match self.focus {
                Focus::Sources => self.set_source_by_index(self.source_count().saturating_sub(1)),
                Focus::Packages | Focus::Queue => self.bottom(),
            },
            KeyCode::PageDown => self.page_down(),
            KeyCode::PageUp => self.page_up(),
            _ if key.code == KeyCode::Char('d')
                && key.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                self.page_down()
            }
            _ if key.code == KeyCode::Char('u')
                && key.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                self.page_up()
            }

            KeyCode::Char('1') => {
                self.filter = Filter::All;
                self.apply_filters();
            }
            KeyCode::Char('2') => {
                self.filter = Filter::Installed;
                self.apply_filters();
            }
            KeyCode::Char('3') => {
                self.filter = Filter::Updates;
                self.apply_filters();
            }

            KeyCode::Char(' ') => self.toggle_selection_on_cursor(),
            KeyCode::Char('a') => self.select_all_visible(),

            KeyCode::Char('i') => self.prepare_action(TaskQueueAction::Install),
            KeyCode::Char('x') => self.prepare_action(TaskQueueAction::Remove),
            KeyCode::Char('u') => self.prepare_action(TaskQueueAction::Update),

            KeyCode::Char('/') => {
                self.searching = true;
            }
            KeyCode::Char('r') => {
                if !self.start_loading() {
                    self.set_status("Already refreshing", true);
                }
            }
            KeyCode::Char('l') => self.toggle_queue_expanded(),

            KeyCode::Esc => {
                if self.queue_expanded {
                    self.toggle_queue_expanded();
                } else if !self.search.is_empty() {
                    self.search.clear();
                    self.apply_filters();
                    self.set_status("Search cleared", true);
                } else if !self.selected.is_empty() {
                    self.clear_selection();
                    self.set_status("Selection cleared", true);
                }
            }

            KeyCode::Char('C') | KeyCode::Char('R') => {
                self.handle_queue_shortcuts(key).await;
            }
            _ => {}
        }
    }

    pub async fn handle_key(&mut self, key: KeyEvent) {
        self.clear_status_if_needed();

        if self.showing_help {
            self.handle_help_key(key).await;
            return;
        }
        if self.confirming.is_some() {
            self.handle_confirm_key(key).await;
            return;
        }
        if self.searching {
            self.handle_search_key(key);
            return;
        }
        self.handle_normal_key(key).await;
    }

    pub async fn queue_tasks(&mut self, packages: Vec<Package>, action: TaskQueueAction) -> usize {
        let mut queued = 0usize;
        for package in packages {
            let entry =
                TaskQueueEntry::new(action, package.id(), package.name.clone(), package.source);
            self.enqueue_task_entry(entry).await;
            queued += 1;
        }
        if queued > 0 {
            self.queue_completed_at = None;
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
            let executor = TaskQueueExecutor::new(pm, history_tracker);
            if let Err(error) = executor.run(sender).await {
                error!(error = %error, "Task queue executor stopped");
            }
            running.store(false, Ordering::SeqCst);
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

        let retry = TaskQueueEntry::new(
            task.action,
            task.package_id.clone(),
            task.package_name.clone(),
            task.package_source,
        );
        self.enqueue_task_entry(retry).await;
        self.queue_completed_at = None;
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

    pub fn queue_counts(&self) -> (usize, usize, usize, usize, usize) {
        let mut queued = 0usize;
        let mut running = 0usize;
        let mut completed = 0usize;
        let mut failed = 0usize;
        let mut cancelled = 0usize;

        for task in &self.tasks {
            match task.status {
                TaskQueueStatus::Queued => queued += 1,
                TaskQueueStatus::Running => running += 1,
                TaskQueueStatus::Completed => completed += 1,
                TaskQueueStatus::Failed => failed += 1,
                TaskQueueStatus::Cancelled => cancelled += 1,
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

fn command_queue_log_older_enabled(app: &App) -> bool {
    app.can_queue_log_older_command()
}

fn command_queue_log_newer_enabled(app: &App) -> bool {
    app.can_queue_log_newer_command()
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
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        default_hook(info);
    }));

    let pm = Arc::new(Mutex::new(PackageManager::new()));
    let history_tracker = Arc::new(Mutex::new(None));
    let (task_tx, task_rx) = mpsc::channel(200);

    let mut app = App::new(pm, history_tracker, Some(task_rx), Some(task_tx));
    app.initialize_history_tracker().await;
    app.sync_task_queue_from_history().await;
    app.load_sources().await;
    let _ = app.start_loading();
    app.spawn_task_executor();

    let result = run_app(&mut terminal, &mut app).await;

    let _ = std::panic::take_hook();
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
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
        app.poll_task_events();
        app.maybe_autohide_queue();

        terminal.draw(|frame| ui::draw(frame, app))?;

        if event::poll(Duration::from_millis(50))? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    app.handle_key(key).await;
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
        App::new(
            Arc::new(Mutex::new(PackageManager::new())),
            Arc::new(Mutex::new(None)),
            None,
            None,
        )
    }

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn ctrl(code: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(code), KeyModifiers::CONTROL)
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
            .any(|command| command.id == CommandId::QueueCancel));
        assert!(registry
            .iter()
            .any(|command| command.id == CommandId::ShowHelp));
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
        assert!(app.command_enabled(CommandId::Install));
        assert!(!app.command_enabled(CommandId::Remove));
        assert!(!app.command_enabled(CommandId::Update));

        app.packages[0].status = PackageStatus::UpdateAvailable;
        app.apply_filters();

        assert!(app.command_enabled(CommandId::Remove));
        assert!(app.command_enabled(CommandId::Update));
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
        assert_eq!(app.filter_counts, [3, 2, 1]);
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
}
