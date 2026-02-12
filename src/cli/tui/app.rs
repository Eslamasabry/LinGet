use super::ui;
use super::update_center;
use crate::backend::{HistoryTracker, PackageManager, TaskQueueEvent, TaskQueueExecutor};
use crate::models::history::{TaskQueueAction, TaskQueueEntry, TaskQueueStatus};
use crate::models::{Config, Package, PackageSource, PackageStatus};
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
pub enum Filter {
    All,
    Installed,
    Updates,
    Favorites,
}

impl Filter {
    fn from_config_value(value: Option<&str>) -> Self {
        match value.unwrap_or_default().to_lowercase().as_str() {
            "installed" => Self::Installed,
            "updates" => Self::Updates,
            "favorites" => Self::Favorites,
            _ => Self::All,
        }
    }

    fn as_config_value(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Installed => "installed",
            Self::Updates => "updates",
            Self::Favorites => "favorites",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Sources,
    Packages,
    Queue,
}

impl Focus {
    fn from_config_value(value: Option<&str>) -> Self {
        match value.unwrap_or_default().to_lowercase().as_str() {
            "packages" => Self::Packages,
            "queue" => Self::Queue,
            _ => Self::Sources,
        }
    }

    fn as_config_value(self) -> &'static str {
        match self {
            Self::Sources => "sources",
            Self::Packages => "packages",
            Self::Queue => "queue",
        }
    }
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
    pub previous_statuses: HashMap<String, PackageStatus>,
    pub queue_expanded: bool,
    pub queue_completed_at: Option<Instant>,
    pub executor_running: Arc<AtomicBool>,
    pub queue_failures_acknowledged: bool,

    pub focus: Focus,
    pub compact: bool,
    pub confirming: Option<PendingAction>,
    pub showing_help: bool,
    pub showing_palette: bool,
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
            previous_statuses: HashMap::new(),
            queue_expanded: false,
            queue_completed_at: None,
            executor_running: Arc::new(AtomicBool::new(false)),
            queue_failures_acknowledged: false,
            focus: Focus::Sources,
            compact: false,
            confirming: None,
            showing_help: false,
            showing_palette: false,
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
                CommandId::Refresh => "Refresh is already in progress",
                CommandId::ToggleQueue => "Queue is empty",
                CommandId::QueueCancel => "Select a queued task in expanded queue",
                CommandId::QueueRetry => "Select a failed task in expanded queue",
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

    pub async fn execute_command(&mut self, command: CommandId) {
        let allow_invalid_execution = matches!(
            command,
            CommandId::Install | CommandId::Remove | CommandId::Update
        );

        if !allow_invalid_execution && !self.command_enabled(command) {
            if let Some(reason) = self.command_disabled_reason(command) {
                self.set_status(reason, true);
            }
            return;
        }

        match command {
            CommandId::Quit => self.should_quit = true,
            CommandId::ShowHelp => self.showing_help = true,
            CommandId::OpenPalette => self.open_palette(),
            CommandId::CycleFocus => {
                self.focus = match self.focus {
                    Focus::Sources => Focus::Packages,
                    Focus::Packages | Focus::Queue => Focus::Sources,
                };
            }
            CommandId::MoveUp => {
                if self.queue_focus_active() {
                    self.queue_prev();
                } else {
                    match self.focus {
                        Focus::Sources => self.prev_source(),
                        Focus::Packages | Focus::Queue => self.prev_package(),
                    }
                }
            }
            CommandId::MoveDown => {
                if self.queue_focus_active() {
                    self.queue_next();
                } else {
                    match self.focus {
                        Focus::Sources => self.next_source(),
                        Focus::Packages | Focus::Queue => self.next_package(),
                    }
                }
            }
            CommandId::MoveTop => {
                if self.queue_focus_active() {
                    self.queue_top();
                } else {
                    match self.focus {
                        Focus::Sources => self.set_source_by_index(0),
                        Focus::Packages | Focus::Queue => self.top(),
                    }
                }
            }
            CommandId::MoveBottom => {
                if self.queue_focus_active() {
                    self.queue_bottom();
                } else {
                    match self.focus {
                        Focus::Sources => {
                            self.set_source_by_index(self.source_count().saturating_sub(1))
                        }
                        Focus::Packages | Focus::Queue => self.bottom(),
                    }
                }
            }
            CommandId::PageUp => {
                if self.queue_focus_active() {
                    self.queue_page_up();
                } else {
                    self.page_up();
                }
            }
            CommandId::PageDown => {
                if self.queue_focus_active() {
                    self.queue_page_down();
                } else {
                    self.page_down();
                }
            }
            CommandId::FilterAll => {
                self.filter = Filter::All;
                self.apply_filters();
            }
            CommandId::FilterInstalled => {
                self.filter = Filter::Installed;
                self.apply_filters();
            }
            CommandId::FilterUpdates => {
                self.filter = Filter::Updates;
                self.apply_filters();
            }
            CommandId::FilterFavorites => {
                self.filter = Filter::Favorites;
                self.apply_filters();
            }
            CommandId::ToggleFavorite => self.toggle_favorite_on_cursor(),
            CommandId::BulkToggleFavorite => self.bulk_toggle_favorites(),
            CommandId::ToggleFavoritesUpdatesOnly => self.toggle_favorites_updates_only(),
            CommandId::ToggleSelection => self.toggle_selection_on_cursor(),
            CommandId::SelectAll => self.select_all_visible(),
            CommandId::Install => self.prepare_action(TaskQueueAction::Install),
            CommandId::Remove => self.prepare_action(TaskQueueAction::Remove),
            CommandId::Update => self.prepare_action(TaskQueueAction::Update),
            CommandId::Search => self.searching = true,
            CommandId::Refresh => {
                if !self.start_loading() {
                    self.set_status("Already refreshing", true);
                }
            }
            CommandId::ToggleQueue => self.toggle_queue_expanded(),
            CommandId::QueueCancel => self.cancel_selected_task().await,
            CommandId::QueueRetry => self.retry_selected_task().await,
            CommandId::QueueLogOlder => self.queue_log_scroll_up(),
            CommandId::QueueLogNewer => self.queue_log_scroll_down(),
        }
    }

    async fn handle_palette_key(&mut self, key: KeyEvent) {
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.should_quit = true;
            return;
        }

        match key.code {
            KeyCode::Esc => self.close_palette(),
            KeyCode::Enter => {
                let Some(entry) = self.palette_selected_entry() else {
                    return;
                };

                if !entry.enabled {
                    if let Some(reason) = entry.disabled_reason {
                        self.set_status(reason, true);
                    }
                    return;
                }

                self.close_palette();
                self.execute_command(entry.id).await;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.palette_cursor = self.palette_cursor.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let len = self.palette_entries().len();
                if len > 0 {
                    self.palette_cursor = (self.palette_cursor + 1).min(len - 1);
                }
            }
            KeyCode::Home => {
                self.palette_cursor = 0;
            }
            KeyCode::End => {
                let len = self.palette_entries().len();
                if len > 0 {
                    self.palette_cursor = len - 1;
                }
            }
            KeyCode::Backspace | KeyCode::Delete => {
                self.palette_query.pop();
                self.clamp_palette_cursor();
            }
            KeyCode::Char(ch)
                if !ch.is_control()
                    && !key.modifiers.contains(KeyModifiers::CONTROL)
                    && !key.modifiers.contains(KeyModifiers::ALT) =>
            {
                self.palette_query.push(ch);
                self.clamp_palette_cursor();
            }
            _ => {}
        }

        self.clamp_palette_cursor();
    }

    async fn handle_normal_key(&mut self, key: KeyEvent) {
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.should_quit = true;
            return;
        }

        if key.code == KeyCode::Char(':')
            || (key.code == KeyCode::Char('p') && key.modifiers.contains(KeyModifiers::CONTROL))
        {
            self.execute_command(CommandId::OpenPalette).await;
            return;
        }

        match key.code {
            KeyCode::Char('q') => {
                self.execute_command(CommandId::Quit).await;
                return;
            }
            KeyCode::Char('?') => {
                self.execute_command(CommandId::ShowHelp).await;
                return;
            }
            _ => {}
        }

        if self.queue_expanded && self.focus == Focus::Queue {
            match key.code {
                KeyCode::Esc | KeyCode::Char('l') => {
                    self.execute_command(CommandId::ToggleQueue).await;
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
                    self.execute_command(CommandId::QueueLogOlder).await;
                    return;
                }
                KeyCode::Char(']') => {
                    self.execute_command(CommandId::QueueLogNewer).await;
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
            KeyCode::Tab => self.execute_command(CommandId::CycleFocus).await,
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
            KeyCode::Char('1') => self.execute_command(CommandId::FilterAll).await,
            KeyCode::Char('2') => self.execute_command(CommandId::FilterInstalled).await,
            KeyCode::Char('3') => self.execute_command(CommandId::FilterUpdates).await,
            KeyCode::Char('4') => self.execute_command(CommandId::FilterFavorites).await,
            KeyCode::Char('f') => self.execute_command(CommandId::ToggleFavorite).await,
            KeyCode::Char('F') => self.execute_command(CommandId::BulkToggleFavorite).await,
            KeyCode::Char('v') => {
                self.execute_command(CommandId::ToggleFavoritesUpdatesOnly)
                    .await;
            }
            KeyCode::Char(' ') => self.execute_command(CommandId::ToggleSelection).await,
            KeyCode::Char('a') => self.execute_command(CommandId::SelectAll).await,
            KeyCode::Char('i') => self.execute_command(CommandId::Install).await,
            KeyCode::Char('x') => self.execute_command(CommandId::Remove).await,
            KeyCode::Char('u') => self.execute_command(CommandId::Update).await,
            KeyCode::Char('/') => self.execute_command(CommandId::Search).await,
            KeyCode::Char('r') => self.execute_command(CommandId::Refresh).await,
            KeyCode::Char('l') => self.execute_command(CommandId::ToggleQueue).await,
            KeyCode::Esc => {
                if self.queue_expanded {
                    self.execute_command(CommandId::ToggleQueue).await;
                } else if !self.search.is_empty() {
                    self.search.clear();
                    self.apply_filters();
                    self.set_status("Search cleared", true);
                } else if !self.selected.is_empty() {
                    self.clear_selection();
                    self.set_status("Selection cleared", true);
                }
            }
            KeyCode::Char('C') => self.execute_command(CommandId::QueueCancel).await,
            KeyCode::Char('R') => self.execute_command(CommandId::QueueRetry).await,
            _ => {}
        }
    }

    pub async fn handle_key(&mut self, key: KeyEvent) {
        self.clear_status_if_needed();

        if self.showing_palette {
            self.handle_palette_key(key).await;
            return;
        }
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

    pub async fn handle_mouse(&mut self, event: MouseEvent, regions: &ui::LayoutRegions) {
        const SCROLL_STEP: usize = 3;
        const DOUBLE_CLICK_MS: u128 = 400;

        let pos = (event.column, event.row);

        if self.showing_palette {
            match event.kind {
                MouseEventKind::ScrollUp => {
                    self.palette_cursor = self.palette_cursor.saturating_sub(1);
                    self.clamp_palette_cursor();
                }
                MouseEventKind::ScrollDown => {
                    let len = self.palette_entries().len();
                    if len > 0 {
                        self.palette_cursor = (self.palette_cursor + 1).min(len - 1);
                    }
                }
                MouseEventKind::Down(MouseButton::Left) => {
                    let col = event.column;
                    let row = event.row;
                    let is_double = self.last_click.take().is_some_and(|(lc, lr, lt)| {
                        lc == col && lr == row && lt.elapsed().as_millis() < DOUBLE_CLICK_MS
                    });
                    self.last_click = Some((col, row, Instant::now()));
                    self.handle_mouse_palette_click(col, row, is_double, &regions.palette)
                        .await;
                }
                _ => {}
            }
            return;
        }

        match event.kind {
            MouseEventKind::ScrollUp => {
                if rect_contains(regions.expanded_queue_logs, pos) {
                    self.focus = Focus::Queue;
                    self.queue_log_scroll_up();
                } else if rect_contains(regions.expanded_queue_tasks, pos) {
                    self.focus = Focus::Queue;
                    self.queue_prev();
                } else if rect_contains(regions.packages, pos) {
                    self.focus = Focus::Packages;
                    for _ in 0..SCROLL_STEP {
                        self.prev_package();
                    }
                } else if rect_contains(regions.sources, pos) {
                    self.focus = Focus::Sources;
                    let idx = self.source_index();
                    if idx > 0 {
                        self.set_source_by_index(idx - 1);
                    }
                }
            }
            MouseEventKind::ScrollDown => {
                if rect_contains(regions.expanded_queue_logs, pos) {
                    self.focus = Focus::Queue;
                    self.queue_log_scroll_down();
                } else if rect_contains(regions.expanded_queue_tasks, pos) {
                    self.focus = Focus::Queue;
                    self.queue_next();
                } else if rect_contains(regions.packages, pos) {
                    self.focus = Focus::Packages;
                    for _ in 0..SCROLL_STEP {
                        self.next_package();
                    }
                } else if rect_contains(regions.sources, pos) {
                    self.focus = Focus::Sources;
                    let idx = self.source_index();
                    if idx + 1 < self.source_count() {
                        self.set_source_by_index(idx + 1);
                    }
                }
            }
            MouseEventKind::Down(MouseButton::Left) => {
                let col = event.column;
                let row = event.row;

                let is_double = self.last_click.take().is_some_and(|(lc, lr, lt)| {
                    lc == col && lr == row && lt.elapsed().as_millis() < DOUBLE_CLICK_MS
                });
                self.last_click = Some((col, row, Instant::now()));

                if self.showing_help {
                    self.showing_help = false;
                    return;
                }
                if self.confirming.is_some() {
                    self.handle_mouse_confirm(col, row, &regions.footer).await;
                    return;
                }

                if rect_contains(regions.header_filter_row, pos) {
                    self.handle_mouse_header(col, row, regions);
                } else if rect_contains(regions.sources, pos) {
                    self.handle_mouse_sources(row, &regions.sources);
                } else if rect_contains(regions.packages, pos) {
                    self.handle_mouse_packages_click(col, row, is_double, &regions.packages);
                } else if rect_contains(regions.details, pos) {
                    self.focus = Focus::Packages;
                } else if rect_contains(regions.queue_bar, pos) {
                    self.toggle_queue_expanded();
                } else if rect_contains(regions.expanded_queue, pos) {
                    self.handle_mouse_expanded_queue_click(col, row, regions)
                        .await;
                }
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                if rect_contains(regions.packages, pos) {
                    self.handle_mouse_packages_drag(event.row, &regions.packages);
                }
            }
            MouseEventKind::Up(MouseButton::Left) => {
                self.drag_select_anchor = None;
            }
            MouseEventKind::Down(MouseButton::Right) => {
                if rect_contains(regions.packages, pos) {
                    self.handle_mouse_packages_right_click(event.row, &regions.packages);
                }
            }
            _ => {}
        }
    }

    fn palette_index_from_mouse_row(&self, row: u16, palette_rect: &Rect) -> Option<usize> {
        if palette_rect.width <= 2 || palette_rect.height <= 4 {
            return None;
        }

        let inner_y = palette_rect.y.saturating_add(1);
        let inner_height = palette_rect.height.saturating_sub(2);
        if inner_height < 3 {
            return None;
        }

        let list_top = inner_y.saturating_add(1);
        let visible_rows = inner_height.saturating_sub(2) as usize;
        if visible_rows == 0 {
            return None;
        }
        if row < list_top || row >= list_top.saturating_add(visible_rows as u16) {
            return None;
        }

        let entries = self.palette_entries();
        if entries.is_empty() {
            return None;
        }

        let start = ui::window_start(entries.len(), visible_rows, self.palette_cursor);
        let clicked = start + row.saturating_sub(list_top) as usize;
        (clicked < entries.len()).then_some(clicked)
    }

    async fn handle_mouse_palette_click(
        &mut self,
        col: u16,
        row: u16,
        is_double: bool,
        palette_rect: &Rect,
    ) {
        if !rect_contains(*palette_rect, (col, row)) {
            self.close_palette();
            return;
        }

        let Some(index) = self.palette_index_from_mouse_row(row, palette_rect) else {
            return;
        };

        self.palette_cursor = index;

        if !is_double {
            return;
        }

        let Some(entry) = self.palette_entries().get(index).cloned() else {
            return;
        };

        if !entry.enabled {
            if let Some(reason) = entry.disabled_reason {
                self.set_status(reason, true);
            }
            return;
        }

        self.close_palette();
        self.execute_command(entry.id).await;
    }

    fn handle_mouse_header(&mut self, col: u16, row: u16, regions: &ui::LayoutRegions) {
        if let Some(filter) = ui::header_filter_hit_test(self, regions.header_filter_row, col, row)
        {
            self.filter = filter;
            self.apply_filters();
            return;
        }

        if !self.searching {
            self.searching = true;
        }
    }

    fn source_index_from_mouse_row(&self, row: u16, sources_rect: &Rect) -> Option<usize> {
        if sources_rect.width <= 2 || sources_rect.height <= 2 {
            return None;
        }

        let top = sources_rect.y.saturating_add(1);
        let visible_rows = sources_rect.height.saturating_sub(2) as usize;
        if visible_rows == 0 {
            return None;
        }

        if row < top || row >= top.saturating_add(visible_rows as u16) {
            return None;
        }

        let total = self.visible_sources().len() + 1;
        let start = ui::window_start(total, visible_rows, self.source_index());
        let clicked_index = start + row.saturating_sub(top) as usize;
        (clicked_index < total).then_some(clicked_index)
    }

    fn handle_mouse_sources(&mut self, row: u16, sources_rect: &Rect) {
        self.focus = Focus::Sources;
        if let Some(clicked_index) = self.source_index_from_mouse_row(row, sources_rect) {
            self.set_source_by_index(clicked_index);
        }
    }

    fn package_index_from_mouse_row(&self, row: u16, packages_rect: &Rect) -> Option<usize> {
        if packages_rect.width <= 2 || packages_rect.height <= 4 || self.filtered.is_empty() {
            return None;
        }

        let first_row = packages_rect.y.saturating_add(2);
        let visible_rows = packages_rect.height.saturating_sub(4) as usize;
        if visible_rows == 0 {
            return None;
        }

        if row < first_row || row >= first_row.saturating_add(visible_rows as u16) {
            return None;
        }

        let start = ui::window_start(self.filtered.len(), visible_rows.max(1), self.cursor);
        let clicked_index = start + row.saturating_sub(first_row) as usize;
        (clicked_index < self.filtered.len()).then_some(clicked_index)
    }

    fn prepare_default_action_for_cursor(&mut self) {
        let Some(package) = self.current_package() else {
            return;
        };
        let action = match package.status {
            PackageStatus::NotInstalled => Some(TaskQueueAction::Install),
            PackageStatus::UpdateAvailable => Some(TaskQueueAction::Update),
            PackageStatus::Installed => Some(TaskQueueAction::Remove),
            _ => None,
        };

        if let Some(action) = action {
            self.prepare_action(action);
        } else {
            self.set_status("No primary action for this package", true);
        }
    }

    fn handle_mouse_packages_click(
        &mut self,
        col: u16,
        row: u16,
        is_double: bool,
        packages_rect: &Rect,
    ) {
        let Some(clicked_index) = self.package_index_from_mouse_row(row, packages_rect) else {
            return;
        };

        self.focus = Focus::Packages;
        self.cursor = clicked_index;
        self.drag_select_anchor = Some(clicked_index);

        let inner_col = col.saturating_sub(packages_rect.x.saturating_add(1));
        if inner_col < 2 {
            self.toggle_selection_on_cursor();
            return;
        }
        if (3..5).contains(&inner_col) {
            self.toggle_favorite_on_cursor();
            return;
        }

        if is_double {
            self.prepare_default_action_for_cursor();
        }
    }

    fn select_package_range(&mut self, start: usize, end: usize) {
        let (from, to) = if start <= end {
            (start, end)
        } else {
            (end, start)
        };
        for row_index in from..=to {
            let Some(package_index) = self.filtered.get(row_index).copied() else {
                continue;
            };
            let Some(package) = self.packages.get(package_index) else {
                continue;
            };
            self.selected.insert(package.id());
        }
    }

    fn handle_mouse_packages_drag(&mut self, row: u16, packages_rect: &Rect) {
        let Some(anchor) = self.drag_select_anchor else {
            return;
        };
        let Some(clicked_index) = self.package_index_from_mouse_row(row, packages_rect) else {
            return;
        };

        self.focus = Focus::Packages;
        self.cursor = clicked_index;
        self.select_package_range(anchor, clicked_index);
    }

    fn handle_mouse_packages_right_click(&mut self, row: u16, packages_rect: &Rect) {
        let Some(clicked_index) = self.package_index_from_mouse_row(row, packages_rect) else {
            return;
        };
        self.focus = Focus::Packages;
        self.cursor = clicked_index;
        self.prepare_default_action_for_cursor();
    }

    fn task_index_from_mouse_row(&self, row: u16, task_rect: &Rect) -> Option<usize> {
        if task_rect.width == 0 || task_rect.height == 0 || self.tasks.is_empty() {
            return None;
        }
        if row < task_rect.y || row >= task_rect.y + task_rect.height {
            return None;
        }

        let visible = task_rect.height as usize;
        let start = ui::window_start(self.tasks.len(), visible.max(1), self.task_cursor);
        let clicked = start + row.saturating_sub(task_rect.y) as usize;
        (clicked < self.tasks.len()).then_some(clicked)
    }

    async fn handle_mouse_expanded_queue_click(
        &mut self,
        col: u16,
        row: u16,
        regions: &ui::LayoutRegions,
    ) {
        if let Some(action) = ui::queue_hint_hit_test(
            regions.expanded_queue_hints,
            regions.expanded_queue_logs.width > 0,
            col,
            row,
        ) {
            self.focus = Focus::Queue;
            match action {
                ui::QueueHintAction::Cancel => self.cancel_selected_task().await,
                ui::QueueHintAction::Retry => self.retry_selected_task().await,
                ui::QueueHintAction::LogOlder => self.queue_log_scroll_up(),
                ui::QueueHintAction::LogNewer => self.queue_log_scroll_down(),
            }
            return;
        }

        if let Some(clicked_index) =
            self.task_index_from_mouse_row(row, &regions.expanded_queue_tasks)
        {
            self.focus = Focus::Queue;
            self.set_task_cursor(clicked_index);
            return;
        }

        if rect_contains(regions.expanded_queue_logs, (col, row)) {
            self.focus = Focus::Queue;
        }
    }

    async fn handle_mouse_confirm(&mut self, col: u16, row: u16, footer_rect: &Rect) {
        let Some(label) = self.confirming.as_ref().map(|action| action.label.clone()) else {
            return;
        };

        match ui::confirm_footer_hit_test(&label, *footer_rect, col, row) {
            Some(true) => {
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
            Some(false) => {
                self.confirming = None;
                self.set_status("Cancelled", true);
            }
            None => {}
        }
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
        app.poll_task_events();
        app.maybe_autohide_queue();

        terminal.draw(|frame| ui::draw(frame, app))?;

        if event::poll(Duration::from_millis(50))? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    app.handle_key(key).await;
                }
                Event::Mouse(mouse) => {
                    let regions = ui::compute_layout(app, Rect::new(0, 0, size.width, size.height));
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
        assert!(!app.command_enabled(CommandId::ToggleFavoritesUpdatesOnly));
        assert!(app.command_enabled(CommandId::Install));
        assert!(!app.command_enabled(CommandId::Remove));
        assert!(!app.command_enabled(CommandId::Update));

        app.packages[0].status = PackageStatus::UpdateAvailable;
        app.apply_filters();

        assert!(app.command_enabled(CommandId::Remove));
        assert!(app.command_enabled(CommandId::Update));

        app.filter = Filter::Favorites;
        app.apply_filters();
        assert!(app.command_enabled(CommandId::ToggleFavoritesUpdatesOnly));
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
        });
        app.tasks.push(TaskQueueEntry::new(
            TaskQueueAction::Install,
            "APT:seed".into(),
            "seed".into(),
            PackageSource::Apt,
        ));

        let regions = layout_regions(&app);
        let before = app.tasks.len();
        let row = regions.footer.y;
        let yes_col = (regions.footer.x..regions.footer.x + regions.footer.width)
            .find(|col| {
                ui::confirm_footer_hit_test("Install pkg?", regions.footer, *col, row) == Some(true)
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
    async fn mouse_queue_hint_clicks_cancel_and_retry() {
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
        app.tasks = vec![queued, failed];

        let regions = layout_regions(&app);
        let hint_row = regions.expanded_queue_hints.y;

        app.task_cursor = 0;
        app.handle_mouse(
            mouse(
                MouseEventKind::Down(MouseButton::Left),
                regions.expanded_queue_hints.x,
                hint_row,
            ),
            &regions,
        )
        .await;
        assert_eq!(app.tasks[0].status, TaskQueueStatus::Cancelled);

        app.task_cursor = 1;
        let before = app.tasks.len();
        let retry_col = regions.expanded_queue_hints.x + 10;
        app.handle_mouse(
            mouse(MouseEventKind::Down(MouseButton::Left), retry_col, hint_row),
            &regions,
        )
        .await;
        assert_eq!(app.tasks.len(), before + 1);
        assert_eq!(app.tasks.last().unwrap().status, TaskQueueStatus::Queued);
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
}
