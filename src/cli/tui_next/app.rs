//! State and view-model for the reimagined TUI ("Radar").
//!
//! Design contract:
//! - One column of focus: a single list. No rails, no tabs, no persistent
//!   inspector. Detail expands inline (or docks right on wide terminals).
//! - Sections are urgency tiers (Security -> Updates) or sources (Installed
//!   view), never a second list.
//! - 13 keys total. Everything else lives in the `:` palette.

use crate::backend::history_tracker::HistoryTracker;
use crate::backend::{PackageLoadProgress, PackageManager, TaskQueueEvent};
use crate::models::history::{TaskQueueAction, TaskQueueEntry, TaskQueueStatus};
use crate::models::{Package, PackageSource, PackageStatus, UpdateCategory};
use anyhow::{Context, Result};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, Mutex, RwLock};

pub const MIN_WIDTH: u16 = 60;
pub const MIN_HEIGHT: u16 = 16;
/// Running tasks older than this are considered orphaned (the process that
/// owned them is long dead, the queue just never noticed).
pub const ORPHAN_AGE: Duration = Duration::from_secs(2 * 60 * 60);
/// Terminal-wide breakpoint for docking the expansion to the right.
pub const DOCK_WIDTH: u16 = 140;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Filter {
    Updates,
    Security,
    Installed,
}

impl Filter {
    pub fn label(self) -> &'static str {
        match self {
            Self::Updates => "updates",
            Self::Security => "security",
            Self::Installed => "installed",
        }
    }
}

/// A flattened, scrollable view-model row. The UI never walks packages
/// directly; it walks this list, so cursor math and rendering stay trivial.
#[derive(Debug, Clone)]
pub enum Row {
    Header {
        key: String,
        label: String,
        count: usize,
    },
    Item {
        id: String,
    },
}

#[derive(Debug, Default)]
pub struct Search {
    pub text: String,
    pub focused: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadPhase {
    Installed,
    Updates,
    Done,
}

#[derive(Debug)]
pub enum LoadMsg {
    Progress(PackageLoadProgress),
    InstalledDone,
    UpdatesDone(Vec<Package>),
    UpdatesFailed(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmAction {
    RemoveSelected,
    Quit,
}

#[derive(Debug)]
pub enum Overlay {
    Palette,
    Help,
    Confirm {
        title: String,
        body: String,
        action: ConfirmAction,
    },
    Changelog {
        title: String,
        lines: Vec<String>,
        scroll: usize,
    },
}

pub struct App {
    pub pm: Arc<RwLock<PackageManager>>,
    pub history: Arc<Mutex<Option<HistoryTracker>>>,
    queue_tx: mpsc::Sender<TaskQueueEvent>,
    queue_rx: mpsc::Receiver<TaskQueueEvent>,
    executor_done_tx: mpsc::Sender<()>,
    executor_done_rx: mpsc::Receiver<()>,
    load_rx: mpsc::Receiver<LoadMsg>,

    pub packages: Vec<Package>,
    pub pkg_by_id: HashMap<String, usize>,
    pub load_phase: LoadPhase,
    pub sources_total: usize,
    pub sources_done: usize,

    pub filter: Filter,
    pub rows: Vec<Row>,
    pub cursor: usize,
    pub scroll: usize,
    pub collapsed: HashSet<String>,
    pub expanded: Option<String>,
    pub selected: HashSet<String>,

    pub search: Search,

    pub queue: Vec<TaskQueueEntry>,
    pub queue_open: bool,
    pub live_logs: HashMap<String, String>,
    pub executor_running: bool,

    pub overlay: Option<Overlay>,
    pub palette_cursor: usize,
    /// Rows visible in the list viewport, refreshed on every draw. Paging
    /// keys use this instead of a magic constant.
    pub visible_rows: usize,
    pub status: Option<(String, Instant)>,
    pub spinner: usize,
    pub should_quit: bool,
}

impl App {
    pub fn new(
        pm: Arc<RwLock<PackageManager>>,
        history: Arc<Mutex<Option<HistoryTracker>>>,
        load_rx: mpsc::Receiver<LoadMsg>,
        queue_tx: mpsc::Sender<TaskQueueEvent>,
        queue_rx: mpsc::Receiver<TaskQueueEvent>,
        executor_done_tx: mpsc::Sender<()>,
        executor_done_rx: mpsc::Receiver<()>,
    ) -> Self {
        Self {
            pm,
            history,
            queue_tx,
            queue_rx,
            executor_done_tx,
            executor_done_rx,
            load_rx,
            packages: Vec::new(),
            pkg_by_id: HashMap::new(),
            load_phase: LoadPhase::Installed,
            sources_total: 0,
            sources_done: 0,
            filter: Filter::Updates,
            rows: Vec::new(),
            cursor: 0,
            scroll: 0,
            collapsed: HashSet::new(),
            expanded: None,
            selected: HashSet::new(),
            search: Search::default(),
            queue: Vec::new(),
            queue_open: false,
            live_logs: HashMap::new(),
            executor_running: false,
            overlay: None,
            palette_cursor: 0,
            visible_rows: 20,
            status: None,
            spinner: 0,
            should_quit: false,
        }
    }

    // ------------------------------------------------------------------
    // Catalog loading
    // ------------------------------------------------------------------

    pub fn spawn_loader(pm: Arc<RwLock<PackageManager>>, tx: mpsc::Sender<LoadMsg>) {
        tokio::spawn(async move {
            {
                let guard = pm.read().await;
                let (progress_tx, mut progress_rx) = mpsc::channel(200);
                let forward = tokio::spawn(async move {
                    while let Some(event) = progress_rx.recv().await {
                        let _ = tx.send(LoadMsg::Progress(event)).await;
                    }
                    tx
                });
                let _ = guard.list_all_installed_progressive(progress_tx).await;
                let tx = match forward.await {
                    Ok(tx) => tx,
                    Err(_) => return,
                };
                let _ = tx.send(LoadMsg::InstalledDone).await;

                let (progress_tx2, mut progress_rx2) = mpsc::channel(200);
                let tx2 = tx.clone();
                let forward2 = tokio::spawn(async move {
                    while let Some(event) = progress_rx2.recv().await {
                        let _ = tx2.send(LoadMsg::Progress(event)).await;
                    }
                });
                let updates = guard.check_all_updates_progressive(progress_tx2).await;
                drop(guard);
                let _ = forward2.await;
                match updates {
                    Ok(updates) => {
                        let _ = tx.send(LoadMsg::UpdatesDone(updates)).await;
                    }
                    Err(error) => {
                        let _ = tx.send(LoadMsg::UpdatesFailed(error.to_string())).await;
                    }
                }
            }
        });
    }

    pub fn refresh(&mut self) {
        if self.load_phase != LoadPhase::Done {
            return;
        }
        self.packages.clear();
        self.pkg_by_id.clear();
        self.load_phase = LoadPhase::Installed;
        self.sources_done = 0;
        self.expanded = None;
        let tx = self.take_load_replacement();
        Self::spawn_loader(self.pm.clone(), tx);
        self.rebuild_rows();
    }

    fn handle_load_msg(&mut self, msg: LoadMsg) {
        match msg {
            LoadMsg::Progress(PackageLoadProgress::SourceLoaded { source, packages }) => {
                self.sources_done += 1;
                self.merge_packages(packages, false);
                let _ = source;
            }
            LoadMsg::Progress(PackageLoadProgress::SourceFailed { .. }) => {
                self.sources_done += 1;
            }
            LoadMsg::Progress(PackageLoadProgress::UpdateChecked { .. }) => {
                self.sources_done += 1;
            }
            LoadMsg::Progress(PackageLoadProgress::UpdateFailed { .. }) => {
                self.sources_done += 1;
            }
            LoadMsg::InstalledDone => {
                self.load_phase = LoadPhase::Updates;
                self.sources_done = 0;
            }
            LoadMsg::UpdatesDone(updates) => {
                self.load_phase = LoadPhase::Done;
                self.merge_packages(updates, true);
            }
            LoadMsg::UpdatesFailed(error) => {
                self.load_phase = LoadPhase::Done;
                self.set_status(format!("update check failed: {error}"));
            }
        }
        self.rebuild_rows();
    }

    fn merge_packages(&mut self, incoming: Vec<Package>, are_updates: bool) {
        for package in incoming {
            let id = package.id();
            if let Some(&index) = self.pkg_by_id.get(&id) {
                if are_updates {
                    let existing = &mut self.packages[index];
                    existing.status = PackageStatus::UpdateAvailable;
                    existing.available_version = package
                        .available_version
                        .clone()
                        .or_else(|| Some(package.version.clone()));
                    existing.update_category = package.update_category;
                }
            } else if !are_updates {
                self.pkg_by_id.insert(id, self.packages.len());
                self.packages.push(package);
            }
        }
    }

    // ------------------------------------------------------------------
    // View model
    // ------------------------------------------------------------------

    pub fn rebuild_rows(&mut self) {
        let needle = self.search.text.trim().to_lowercase();
        let matches = |package: &Package| -> bool {
            if needle.is_empty() {
                return true;
            }
            package.name.to_lowercase().contains(&needle)
                || package.description.to_lowercase().contains(&needle)
        };

        let mut rows = Vec::new();
        match self.filter {
            Filter::Updates | Filter::Security => {
                let mut security: Vec<&Package> = Vec::new();
                let mut regular: Vec<&Package> = Vec::new();
                for package in &self.packages {
                    if package.status != PackageStatus::UpdateAvailable || !matches(package) {
                        continue;
                    }
                    if package.detect_update_category() == UpdateCategory::Security {
                        security.push(package);
                    } else {
                        regular.push(package);
                    }
                }
                security.sort_by(|a, b| a.name.cmp(&b.name));
                regular.sort_by(|a, b| a.name.cmp(&b.name));
                if self.filter == Filter::Security {
                    push_section(
                        &mut rows,
                        "security",
                        "SECURITY",
                        &security,
                        &self.collapsed,
                    );
                } else {
                    push_section(
                        &mut rows,
                        "security",
                        "SECURITY",
                        &security,
                        &self.collapsed,
                    );
                    push_section(&mut rows, "updates", "UPDATES", &regular, &self.collapsed);
                }
            }
            Filter::Installed => {
                let mut by_source: HashMap<PackageSource, Vec<&Package>> = HashMap::new();
                for package in &self.packages {
                    if !matches(package) {
                        continue;
                    }
                    by_source.entry(package.source).or_default().push(package);
                }
                let mut sources: Vec<(PackageSource, Vec<&Package>)> =
                    by_source.into_iter().collect();
                sources.sort_by(|a, b| {
                    b.1.len()
                        .cmp(&a.1.len())
                        .then(a.0.to_string().cmp(&b.0.to_string()))
                });
                for (source, mut packages) in sources {
                    packages.sort_by(|a, b| a.name.cmp(&b.name));
                    let key = format!("src:{source}");
                    let label = source.to_string().to_lowercase();
                    push_section(&mut rows, &key, &label, &packages, &self.collapsed);
                }
            }
        }
        self.rows = rows;
        self.clamp_cursor();
    }

    fn clamp_cursor(&mut self) {
        if self.rows.is_empty() {
            self.cursor = 0;
            self.scroll = 0;
            return;
        }
        self.cursor = self.cursor.min(self.rows.len() - 1);
    }

    pub fn cursor_row(&self) -> Option<&Row> {
        self.rows.get(self.cursor)
    }

    pub fn cursor_package(&self) -> Option<&Package> {
        match self.cursor_row()? {
            Row::Item { id } => self.package(id),
            Row::Header { .. } => None,
        }
    }

    pub fn package(&self, id: &str) -> Option<&Package> {
        self.pkg_by_id.get(id).map(|&index| &self.packages[index])
    }

    pub fn counts(&self) -> (usize, usize, usize) {
        let mut updates = 0;
        let mut security = 0;
        let mut installed = 0;
        for package in &self.packages {
            match package.status {
                PackageStatus::UpdateAvailable => {
                    updates += 1;
                    if package.detect_update_category() == UpdateCategory::Security {
                        security += 1;
                    }
                }
                PackageStatus::Installed => installed += 1,
                _ => {}
            }
        }
        (updates, security, installed)
    }

    // ------------------------------------------------------------------
    // Input
    // ------------------------------------------------------------------

    pub async fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.request_quit();
            return Ok(());
        }

        if let Some(overlay) = self.overlay.take() {
            return self.handle_overlay_key(key, overlay).await;
        }

        if self.search.focused {
            self.handle_search_key(key);
            return Ok(());
        }

        match key.code {
            KeyCode::Char('q') => self.request_quit(),
            KeyCode::Up | KeyCode::Char('k') => self.move_cursor(-1),
            KeyCode::Down | KeyCode::Char('j') => self.move_cursor(1),
            KeyCode::PageUp => self.move_cursor(-(self.page_size() as isize)),
            KeyCode::PageDown => self.move_cursor(self.page_size() as isize),
            KeyCode::Home | KeyCode::Char('g') => self.set_cursor(0),
            KeyCode::End | KeyCode::Char('G') => self.set_cursor(self.rows.len().saturating_sub(1)),
            KeyCode::Enter => self.activate_cursor(),
            KeyCode::Char(' ') => self.toggle_select(),
            KeyCode::Char('/') => {
                self.search.focused = true;
            }
            KeyCode::Char(':') => {
                self.palette_cursor = 0;
                self.overlay = Some(Overlay::Palette);
            }
            KeyCode::Char('?') => {
                self.overlay = Some(Overlay::Help);
            }
            KeyCode::Tab => {
                self.queue_open = !self.queue_open;
            }
            KeyCode::Char('u') => self.queue_selected_updates().await?,
            KeyCode::Char('a') => self.queue_all_updates().await?,
            KeyCode::Char('r') => {
                if self.queue_open {
                    self.retry_failed().await?;
                } else {
                    self.refresh();
                }
            }
            KeyCode::Char('x') if self.queue_open => {
                self.reap_orphans().await?;
            }
            KeyCode::Char('1') => self.set_filter(Filter::Updates),
            KeyCode::Char('2') => self.set_filter(Filter::Security),
            KeyCode::Char('3') => self.set_filter(Filter::Installed),
            KeyCode::Esc => {
                if !self.search.text.is_empty() {
                    self.search.text.clear();
                    self.rebuild_rows();
                } else if self.expanded.is_some() {
                    self.expanded = None;
                } else if !self.selected.is_empty() {
                    self.selected.clear();
                } else if self.queue_open {
                    self.queue_open = false;
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// The load channel is consumed by the event loop; a refresh replaces the
    /// receiver with a fresh one and hands the matching sender to the loader.
    fn take_load_replacement(&mut self) -> mpsc::Sender<LoadMsg> {
        let (tx, rx) = mpsc::channel(200);
        self.load_rx = rx;
        tx
    }

    fn handle_search_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.search.focused = false;
                if self.search.text.is_empty() {
                    // Second Esc on an empty field: nothing to clear, stay blurred.
                } else {
                    self.search.text.clear();
                }
                self.rebuild_rows();
            }
            KeyCode::Enter => {
                self.search.focused = false;
            }
            KeyCode::Backspace => {
                self.search.text.pop();
                self.rebuild_rows();
            }
            KeyCode::Char(ch) => {
                self.search.text.push(ch);
                self.rebuild_rows();
            }
            _ => {}
        }
    }

    async fn handle_overlay_key(&mut self, key: KeyEvent, overlay: Overlay) -> Result<()> {
        match overlay {
            Overlay::Palette => match key.code {
                KeyCode::Esc => {}
                KeyCode::Up | KeyCode::Char('k') => {
                    self.palette_cursor = self.palette_cursor.saturating_sub(1);
                    self.overlay = Some(Overlay::Palette);
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.palette_cursor = self.palette_cursor.saturating_add(1).min(
                        crate::cli::tui_next::palette::commands_for(self)
                            .len()
                            .saturating_sub(1),
                    );
                    self.overlay = Some(Overlay::Palette);
                }
                KeyCode::Enter => {
                    let commands = crate::cli::tui_next::palette::commands_for(self);
                    if let Some(command) = commands.get(self.palette_cursor) {
                        let action = command.action;
                        crate::cli::tui_next::palette::run(self, action).await?;
                    }
                }
                KeyCode::Char(ch) => {
                    let index = usize::try_from(ch.to_digit(10).unwrap_or(0)).unwrap_or(0);
                    if (1..=9).contains(&index) {
                        let commands = crate::cli::tui_next::palette::commands_for(self);
                        if let Some(command) = commands.get(index - 1) {
                            let action = command.action;
                            crate::cli::tui_next::palette::run(self, action).await?;
                        } else {
                            self.overlay = Some(Overlay::Palette);
                        }
                    } else {
                        self.overlay = Some(Overlay::Palette);
                    }
                }
                _ => {
                    self.overlay = Some(Overlay::Palette);
                }
            },
            // Help fits on one screen by design; any key closes it.
            Overlay::Help => {}
            Overlay::Confirm { action, .. } => match key.code {
                KeyCode::Char('y') | KeyCode::Enter => match action {
                    ConfirmAction::RemoveSelected => {
                        self.remove_selected().await?;
                    }
                    ConfirmAction::Quit => {
                        self.should_quit = true;
                    }
                },
                _ => {}
            },
            Overlay::Changelog {
                title,
                lines,
                mut scroll,
            } => {
                let max_scroll = lines.len().saturating_sub(1);
                match key.code {
                    KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('c') => {}
                    KeyCode::Down | KeyCode::Char('j') => {
                        scroll = (scroll + 1).min(max_scroll);
                        self.overlay = Some(Overlay::Changelog {
                            title,
                            lines,
                            scroll,
                        });
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        scroll = scroll.saturating_sub(1);
                        self.overlay = Some(Overlay::Changelog {
                            title,
                            lines,
                            scroll,
                        });
                    }
                    KeyCode::PageDown => {
                        scroll = (scroll + 20).min(max_scroll);
                        self.overlay = Some(Overlay::Changelog {
                            title,
                            lines,
                            scroll,
                        });
                    }
                    KeyCode::PageUp => {
                        scroll = scroll.saturating_sub(20);
                        self.overlay = Some(Overlay::Changelog {
                            title,
                            lines,
                            scroll,
                        });
                    }
                    _ => {
                        self.overlay = Some(Overlay::Changelog {
                            title,
                            lines,
                            scroll,
                        });
                    }
                }
            }
        }
        Ok(())
    }

    fn set_filter(&mut self, filter: Filter) {
        if self.filter != filter {
            self.filter = filter;
            self.expanded = None;
            self.rebuild_rows();
            self.set_cursor(0);
        }
    }

    fn move_cursor(&mut self, delta: isize) {
        if self.rows.is_empty() {
            return;
        }
        let len = self.rows.len() as isize;
        let next = (self.cursor as isize + delta).clamp(0, len - 1);
        self.set_cursor(next as usize);
    }

    fn set_cursor(&mut self, index: usize) {
        if self.rows.is_empty() {
            self.cursor = 0;
            return;
        }
        self.cursor = index.min(self.rows.len() - 1);
    }

    fn page_size(&self) -> usize {
        self.visible_rows.max(5)
    }

    fn activate_cursor(&mut self) {
        match self.cursor_row().cloned() {
            Some(Row::Header { key, .. }) => {
                if !self.collapsed.remove(&key) {
                    self.collapsed.insert(key);
                }
                self.rebuild_rows();
            }
            Some(Row::Item { id }) => {
                if self.expanded.as_deref() == Some(id.as_str()) {
                    self.expanded = None;
                } else {
                    self.expanded = Some(id);
                }
            }
            None => {}
        }
    }

    fn toggle_select(&mut self) {
        if let Some(Row::Item { id }) = self.cursor_row().cloned() {
            if !self.selected.remove(&id) {
                self.selected.insert(id);
            }
            self.move_cursor(1);
        }
    }

    // ------------------------------------------------------------------
    // Queue
    // ------------------------------------------------------------------

    pub async fn sync_queue_from_history(&mut self) {
        let guard = self.history.lock().await;
        if let Some(tracker) = guard.as_ref() {
            self.queue = tracker.history().task_queue.entries.clone();
        }
    }

    /// (queued, live-running, failed, done). Orphaned "running" entries —
    /// tasks whose owning process died long ago — are deliberately excluded
    /// from the running count; showing zombies as live work erodes trust.
    pub fn queue_counts(&self) -> (usize, usize, usize, usize) {
        let mut queued = 0;
        let mut running = 0;
        let mut failed = 0;
        let mut done = 0;
        for entry in &self.queue {
            match entry.status {
                TaskQueueStatus::Queued => queued += 1,
                TaskQueueStatus::Running if !Self::is_orphan(entry) => running += 1,
                TaskQueueStatus::Running => {}
                TaskQueueStatus::Failed => failed += 1,
                TaskQueueStatus::Completed | TaskQueueStatus::Cancelled => done += 1,
            }
        }
        (queued, running, failed, done)
    }

    pub fn orphan_count(&self) -> usize {
        self.queue
            .iter()
            .filter(|entry| Self::is_orphan(entry))
            .count()
    }

    pub fn request_quit(&mut self) {
        if self.any_task_running() {
            self.overlay = Some(Overlay::Confirm {
                title: "Operation in progress".to_string(),
                body: "A package operation is running. Quitting now may leave your package \
                       database in an inconsistent state."
                    .to_string(),
                action: ConfirmAction::Quit,
            });
        } else {
            self.should_quit = true;
        }
    }

    pub fn retryable_failed_count(&self) -> usize {
        retryable_failed_entries(&self.queue).len()
    }

    fn any_task_running(&self) -> bool {
        self.queue
            .iter()
            .any(|entry| entry.status == TaskQueueStatus::Running && !Self::is_orphan(entry))
    }

    pub fn is_orphan(entry: &TaskQueueEntry) -> bool {
        if entry.status != TaskQueueStatus::Running {
            return false;
        }
        let Some(started) = entry.started_at else {
            return true;
        };
        let age = chrono::Local::now().signed_duration_since(started);
        age.to_std().map(|age| age > ORPHAN_AGE).unwrap_or(true)
    }

    pub async fn queue_action_for(
        &mut self,
        ids: Vec<String>,
        action: TaskQueueAction,
    ) -> Result<usize> {
        let mut queued = 0;
        let mut entries = Vec::new();
        for id in ids {
            let Some(package) = self.package(&id) else {
                continue;
            };
            let allowed = match action {
                TaskQueueAction::Update => package.status == PackageStatus::UpdateAvailable,
                TaskQueueAction::Remove => {
                    matches!(
                        package.status,
                        PackageStatus::Installed | PackageStatus::UpdateAvailable
                    )
                }
                TaskQueueAction::Install => package.status == PackageStatus::NotInstalled,
            };
            if !allowed {
                continue;
            }
            let already_queued = self.queue.iter().any(|entry| {
                entry.package_id == id
                    && matches!(
                        entry.status,
                        TaskQueueStatus::Queued | TaskQueueStatus::Running
                    )
            });
            if already_queued {
                continue;
            }
            entries.push(TaskQueueEntry::new(
                action,
                id,
                package.name.clone(),
                package.source,
            ));
            queued += 1;
        }

        if queued == 0 {
            self.set_status("nothing to queue");
            return Ok(0);
        }

        {
            let mut guard = self.history.lock().await;
            let tracker = guard.as_mut().context("history tracker not initialized")?;
            for entry in entries {
                tracker
                    .enqueue_task(entry)
                    .await
                    .context("failed to enqueue task")?;
            }
            tracker.save().await.context("failed to save task queue")?;
        }
        self.sync_queue_from_history().await;
        self.ensure_executor();
        Ok(queued)
    }

    async fn queue_selected_updates(&mut self) -> Result<()> {
        let ids: Vec<String> = if self.selected.is_empty() {
            self.cursor_package().map(|p| p.id()).into_iter().collect()
        } else {
            self.selected.iter().cloned().collect()
        };
        let queued = self.queue_action_for(ids, TaskQueueAction::Update).await?;
        if queued > 0 {
            self.set_status(format!("queued {queued} update(s)"));
            self.selected.clear();
        }
        Ok(())
    }

    async fn queue_all_updates(&mut self) -> Result<()> {
        let ids: Vec<String> = self
            .packages
            .iter()
            .filter(|package| package.status == PackageStatus::UpdateAvailable)
            .map(|package| package.id())
            .collect();
        let total = ids.len();
        let queued = self.queue_action_for(ids, TaskQueueAction::Update).await?;
        if queued > 0 {
            self.set_status(format!("queued {queued} of {total} updates"));
        }
        Ok(())
    }

    pub async fn remove_selected(&mut self) -> Result<()> {
        let ids: Vec<String> = if self.selected.is_empty() {
            self.cursor_package().map(|p| p.id()).into_iter().collect()
        } else {
            self.selected.iter().cloned().collect()
        };
        let queued = self.queue_action_for(ids, TaskQueueAction::Remove).await?;
        if queued > 0 {
            self.set_status(format!("queued {queued} removal(s)"));
            self.selected.clear();
        }
        Ok(())
    }

    pub async fn retry_failed(&mut self) -> Result<()> {
        let failed: Vec<(String, TaskQueueAction, String, PackageSource)> =
            retryable_failed_entries(&self.queue)
                .into_iter()
                .map(|entry| {
                    (
                        entry.package_id.clone(),
                        entry.action,
                        entry.package_name.clone(),
                        entry.package_source,
                    )
                })
                .collect();
        if failed.is_empty() {
            self.set_status("no failed tasks to retry");
            return Ok(());
        }
        let mut queued = 0;
        {
            let mut guard = self.history.lock().await;
            let tracker = guard.as_mut().context("history tracker not initialized")?;
            for (id, action, name, source) in failed {
                tracker
                    .enqueue_task(TaskQueueEntry::new(action, id, name, source))
                    .await
                    .context("failed to re-enqueue task")?;
                queued += 1;
            }
            tracker.save().await.context("failed to save task queue")?;
        }
        self.sync_queue_from_history().await;
        self.ensure_executor();
        self.set_status(format!("re-queued {queued} failed task(s)"));
        Ok(())
    }

    pub async fn reap_orphans(&mut self) -> Result<()> {
        let orphan_ids: Vec<String> = self
            .queue
            .iter()
            .filter(|entry| Self::is_orphan(entry))
            .map(|entry| entry.id.clone())
            .collect();
        if orphan_ids.is_empty() {
            self.set_status("no orphaned tasks");
            return Ok(());
        }
        let count = orphan_ids.len();
        {
            let mut guard = self.history.lock().await;
            let tracker = guard.as_mut().context("history tracker not initialized")?;
            for id in orphan_ids {
                let _ = tracker.mark_task_cancelled(&id).await;
            }
            tracker.save().await.context("failed to save task queue")?;
        }
        self.sync_queue_from_history().await;
        self.set_status(format!("reaped {count} orphaned task(s)"));
        Ok(())
    }

    fn ensure_executor(&mut self) {
        if self.executor_running {
            return;
        }
        self.executor_running = true;
        let executor =
            crate::backend::TaskQueueExecutor::new(self.pm.clone(), self.history.clone());
        let tx = self.queue_tx.clone();
        let done_tx = self.executor_done_tx.clone();
        tokio::spawn(async move {
            let _ = executor.run(Some(tx)).await;
            let _ = done_tx.send(()).await;
        });
    }

    pub fn handle_queue_event(&mut self, event: TaskQueueEvent) {
        match event {
            TaskQueueEvent::Started(entry) => {
                self.upsert_entry(entry);
            }
            TaskQueueEvent::Log { entry_id, line } => {
                let text = match line {
                    crate::backend::streaming::StreamLine::Stdout(text)
                    | crate::backend::streaming::StreamLine::Stderr(text) => text,
                };
                self.live_logs.insert(entry_id, text);
            }
            TaskQueueEvent::Completed(entry) => {
                self.live_logs.remove(&entry.id);
                self.upsert_entry(entry);
            }
            TaskQueueEvent::Failed(entry) => {
                self.live_logs.remove(&entry.id);
                self.upsert_entry(entry);
            }
        }
    }

    fn upsert_entry(&mut self, entry: TaskQueueEntry) {
        if let Some(existing) = self.queue.iter_mut().find(|e| e.id == entry.id) {
            *existing = entry;
        } else {
            self.queue.push(entry);
        }
    }

    // ------------------------------------------------------------------
    // Event pumps
    // ------------------------------------------------------------------

    pub async fn poll_backend(&mut self) {
        while let Ok(msg) = self.load_rx.try_recv() {
            self.handle_load_msg(msg);
        }
        while let Ok(event) = self.queue_rx.try_recv() {
            self.handle_queue_event(event);
        }
        while self.executor_done_rx.try_recv().is_ok() {
            self.executor_running = false;
            self.set_status("queue finished");
        }
    }

    pub fn tick(&mut self) {
        self.spinner = self.spinner.wrapping_add(1);
        if let Some((_, at)) = &self.status {
            if at.elapsed() > Duration::from_secs(4) {
                self.status = None;
            }
        }
    }

    pub fn spinner_frame(&self) -> &'static str {
        const FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⦎", "⦏", "⧗", "⏳"];
        FRAMES[self.spinner % FRAMES.len()]
    }

    pub fn is_loading(&self) -> bool {
        self.load_phase != LoadPhase::Done
    }

    pub fn set_status(&mut self, message: impl Into<String>) {
        self.status = Some((message.into(), Instant::now()));
    }
}

fn push_section(
    rows: &mut Vec<Row>,
    key: &str,
    label: &str,
    packages: &[&Package],
    collapsed: &HashSet<String>,
) {
    if packages.is_empty() {
        return;
    }
    rows.push(Row::Header {
        key: key.to_string(),
        label: label.to_string(),
        count: packages.len(),
    });
    if collapsed.contains(key) {
        return;
    }
    for package in packages {
        rows.push(Row::Item { id: package.id() });
    }
}

fn action_key(action: TaskQueueAction) -> u8 {
    match action {
        TaskQueueAction::Install => 0,
        TaskQueueAction::Remove => 1,
        TaskQueueAction::Update => 2,
    }
}

fn retryable_failed_entries(queue: &[TaskQueueEntry]) -> Vec<&TaskQueueEntry> {
    let active: HashSet<(String, u8)> = queue
        .iter()
        .filter(|entry| {
            matches!(
                entry.status,
                TaskQueueStatus::Queued | TaskQueueStatus::Running
            )
        })
        .map(|entry| (entry.package_id.clone(), action_key(entry.action)))
        .collect();
    let mut seen = HashSet::new();
    let mut entries: Vec<&TaskQueueEntry> = queue
        .iter()
        .rev()
        .filter(|entry| entry.status == TaskQueueStatus::Failed)
        .filter(|entry| {
            let key = (entry.package_id.clone(), action_key(entry.action));
            !active.contains(&key) && seen.insert(key)
        })
        .collect();
    entries.reverse();
    entries
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Local;

    fn task(name: &str, action: TaskQueueAction, status: TaskQueueStatus) -> TaskQueueEntry {
        let mut entry = TaskQueueEntry::new(
            action,
            format!("apt:{name}"),
            name.to_string(),
            PackageSource::Apt,
        );
        entry.status = status;
        entry
    }

    #[test]
    fn retry_targets_deduplicate_package_actions() {
        let queue = vec![
            task("curl", TaskQueueAction::Update, TaskQueueStatus::Failed),
            task("curl", TaskQueueAction::Update, TaskQueueStatus::Failed),
            task("git", TaskQueueAction::Update, TaskQueueStatus::Failed),
        ];

        let targets = retryable_failed_entries(&queue);

        assert_eq!(targets.len(), 2);
        assert_eq!(targets[0].package_name, "curl");
        assert_eq!(targets[1].package_name, "git");
    }

    #[test]
    fn retry_targets_skip_actions_already_active() {
        let queue = vec![
            task("curl", TaskQueueAction::Update, TaskQueueStatus::Failed),
            task("curl", TaskQueueAction::Update, TaskQueueStatus::Queued),
        ];

        assert!(retryable_failed_entries(&queue).is_empty());
    }

    #[test]
    fn stale_running_tasks_are_orphaned() {
        let mut stale = task("curl", TaskQueueAction::Update, TaskQueueStatus::Running);
        stale.started_at = Some(Local::now() - chrono::Duration::hours(3));
        let mut recent = task("git", TaskQueueAction::Update, TaskQueueStatus::Running);
        recent.started_at = Some(Local::now() - chrono::Duration::minutes(1));

        assert!(App::is_orphan(&stale));
        assert!(!App::is_orphan(&recent));
    }
}
