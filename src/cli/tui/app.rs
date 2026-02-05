use super::ui;
use crate::backend::{HistoryTracker, PackageManager, TaskQueueEvent, TaskQueueExecutor};
use crate::models::history::{TaskQueueAction, TaskQueueEntry, TaskQueueStatus};
use crate::models::{Package, PackageSource, PackageStatus};
use anyhow::Result;
use chrono;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use tokio::sync::{mpsc, Mutex};
use tracing::error;
use std::collections::HashSet;
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

const MAX_CONSOLE_LINES: usize = 100;
const COMPACT_WIDTH: u16 = 100;
const COMPACT_HEIGHT: u16 = 28;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivePanel {
    Sources,
    Packages,
    Details,
    Queue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    Normal,
    Search,
    Confirm,
}

#[derive(Clone, Debug)]
pub enum PendingAction {
    Install(Package),
    Remove(Package),
    UpdateAll(Vec<Package>),
    InstallSelected(Vec<Package>),
    RemoveSelected(Vec<Package>),
}

pub struct App {
    pub pm: Arc<Mutex<PackageManager>>,
    pub history_tracker: Arc<Mutex<Option<HistoryTracker>>>,
    pub packages: Vec<Package>,
    pub filtered_packages: Vec<Package>,
    pub available_sources: Vec<PackageSource>,
    pub selected_source: Option<PackageSource>,
    pub source_index: usize,
    pub package_index: usize,
    pub queue_index: usize,
    pub active_panel: ActivePanel,
    pub mode: AppMode,
    pub search_query: String,
    pub status_message: String,
    pub loading: bool,
    pub should_quit: bool,
    pub show_updates_only: bool,
    pub load_rx: Option<mpsc::Receiver<Result<Vec<Package>, String>>>,
    pub console_buffer: Vec<String>,
    pub pending_action: Option<PendingAction>,
    pub compact: bool,
    pub selected_packages: HashSet<String>,
    pub queued_tasks: Vec<TaskQueueEntry>,
    pub refresh_after_queue_idle: bool,
    pub task_events_rx: Option<mpsc::Receiver<TaskQueueEvent>>,
    pub task_events_tx: Option<mpsc::Sender<TaskQueueEvent>>,
    pub task_executor_running: Arc<AtomicBool>,
}

impl App {
    pub fn new(
        pm: Arc<Mutex<PackageManager>>,
        history_tracker: Arc<Mutex<Option<HistoryTracker>>>,
        task_events_rx: Option<mpsc::Receiver<TaskQueueEvent>>,
        task_events_tx: Option<mpsc::Sender<TaskQueueEvent>>,
    ) -> Self {
        Self {
            pm,
            history_tracker,
            packages: Vec::new(),
            filtered_packages: Vec::new(),
            available_sources: Vec::new(),
            selected_source: None,
            source_index: 0,
            package_index: 0,
            queue_index: 0,
            active_panel: ActivePanel::Sources,
            mode: AppMode::Normal,
            search_query: String::new(),
            status_message: String::from("Press 'h' for help, 'q' to quit"),
            loading: false,
            should_quit: false,
            show_updates_only: false,
            load_rx: None,
            console_buffer: Vec::new(),
            pending_action: None,
            compact: false,
            selected_packages: HashSet::new(),
            queued_tasks: Vec::new(),
            refresh_after_queue_idle: false,
            task_events_rx,
            task_events_tx,
            task_executor_running: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn append_to_console(&mut self, line: String) {
        self.console_buffer.push(line);
        if self.console_buffer.len() > MAX_CONSOLE_LINES {
            self.console_buffer.remove(0);
        }
    }

    pub async fn load_sources(&mut self) {
        let manager = self.pm.lock().await;
        self.available_sources = manager.available_sources().into_iter().collect();
        self.available_sources
            .sort_by(|a, b| format!("{:?}", a).cmp(&format!("{:?}", b)));
    }

    pub async fn initialize_history_tracker(&mut self) {
        match HistoryTracker::load().await {
            Ok(tracker) => {
                {
                    let mut guard = self.history_tracker.lock().await;
                    *guard = Some(tracker);
                }
                self.append_to_console(format!(
                    "[{}] History tracker initialized",
                    chrono::Local::now().format("%H:%M:%S")
                ));
            }
            Err(e) => {
                self.append_to_console(format!(
                    "[{}] WARN: failed to load history tracker - {}",
                    chrono::Local::now().format("%H:%M:%S"),
                    e
                ));
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

        self.queued_tasks = entries;
        self.clamp_queue_index();
    }

    /// Start loading packages in the background (non-blocking)
    pub fn start_loading(&mut self) {
        self.loading = true;
        self.status_message = if self.show_updates_only {
            String::from("Checking for updates...")
        } else {
            String::from("Loading packages...")
        };
        self.append_to_console(if self.show_updates_only {
            format!(
                "[{}] Checking for available updates...",
                chrono::Local::now().format("%H:%M:%S")
            )
        } else {
            format!(
                "[{}] Loading installed packages from system...",
                chrono::Local::now().format("%H:%M:%S")
            )
        });

        let (tx, rx) = mpsc::channel(1);
        self.load_rx = Some(rx);

        let pm = self.pm.clone();
        let show_updates = self.show_updates_only;

        tokio::spawn(async move {
            let result = {
                let manager = pm.lock().await;
                if show_updates {
                    manager.check_all_updates().await
                } else {
                    manager.list_all_installed().await
                }
            };

            let _ = tx.send(result.map_err(|e| e.to_string())).await;
        });
    }

    /// Check if background loading is complete and process results
    pub fn poll_loading(&mut self) {
        if let Some(ref mut rx) = self.load_rx {
            match rx.try_recv() {
                Ok(Ok(packages)) => {
                    self.packages = packages;
                    self.cleanup_stale_selections();
                    self.filter_packages();
                    self.status_message = if self.show_updates_only {
                        format!("{} updates available", self.filtered_packages.len())
                    } else {
                        format!("Loaded {} packages", self.filtered_packages.len())
                    };
                    self.append_to_console(format!(
                        "[{}] Loaded {} packages from {}",
                        chrono::Local::now().format("%H:%M:%S"),
                        self.filtered_packages.len(),
                        if self.show_updates_only {
                            "update check"
                        } else {
                            "system"
                        }
                    ));
                    self.loading = false;
                    self.load_rx = None;
                }
                Ok(Err(e)) => {
                    self.status_message = format!("Error: {}", e);
                    self.append_to_console(format!(
                        "[{}] ERROR: Failed to load packages - {}",
                        chrono::Local::now().format("%H:%M:%S"),
                        e
                    ));
                    self.loading = false;
                    self.load_rx = None;
                }
                Err(mpsc::error::TryRecvError::Empty) => {
                    // Still loading, do nothing
                }
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    self.status_message = String::from("Loading failed");
                    self.append_to_console(format!(
                        "[{}] ERROR: Loading channel disconnected",
                        chrono::Local::now().format("%H:%M:%S")
                    ));
                    self.loading = false;
                    self.load_rx = None;
                }
            }
        }
    }

    pub fn poll_task_events(&mut self) {
        let Some(mut rx) = self.task_events_rx.take() else {
            return;
        };

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

        if changed
            && self.refresh_after_queue_idle
            && !self.loading
            && !self
                .queued_tasks
                .iter()
                .any(|task| task.status == TaskQueueStatus::Running)
        {
            self.refresh_after_queue_idle = false;
            self.append_to_console(format!(
                "[{}] Queue idle - refreshing package state",
                chrono::Local::now().format("%H:%M:%S")
            ));
            self.start_loading();
        }
    }

    fn apply_task_event(&mut self, event: TaskQueueEvent) {
        match event {
            TaskQueueEvent::Started(entry)
            | TaskQueueEvent::Completed(entry)
            | TaskQueueEvent::Failed(entry) => {
                if matches!(
                    entry.status,
                    TaskQueueStatus::Completed | TaskQueueStatus::Failed
                ) {
                    self.refresh_after_queue_idle = true;
                }
                if let Some(pos) = self.queued_tasks.iter().position(|t| t.id == entry.id) {
                    self.queued_tasks[pos] = entry;
                } else {
                    self.queued_tasks.push(entry);
                    self.clamp_queue_index();
                }
            }
            TaskQueueEvent::Log { entry_id, line } => {
                let (label, content) = match line {
                    crate::backend::streaming::StreamLine::Stdout(text) => ("LOG", text),
                    crate::backend::streaming::StreamLine::Stderr(text) => ("ERR", text),
                };
                let short_id: String = entry_id.chars().take(8).collect();
                self.append_to_console(format!(
                    "[{}] {} {}: {}",
                    chrono::Local::now().format("%H:%M:%S"),
                    short_id,
                    label,
                    content
                ));
            }
        }
    }

    /// Blocking load for initial startup
    pub async fn load_packages(&mut self) {
        self.loading = true;
        self.status_message = String::from("Loading packages...");
        self.append_to_console(format!(
            "[{}] Initial package load started from {}...",
            chrono::Local::now().format("%H:%M:%S"),
            if self.show_updates_only {
                "update check"
            } else {
                "system"
            }
        ));

        let result = {
            let manager = self.pm.lock().await;
            if self.show_updates_only {
                manager.check_all_updates().await
            } else {
                manager.list_all_installed().await
            }
        };

        match result {
            Ok(packages) => {
                self.packages = packages;
                self.cleanup_stale_selections();
                self.filter_packages();
                self.status_message = format!("Loaded {} packages", self.filtered_packages.len());
                self.append_to_console(format!(
                    "[{}] Initial load complete: {} packages loaded",
                    chrono::Local::now().format("%H:%M:%S"),
                    self.filtered_packages.len()
                ));
            }
            Err(e) => {
                self.status_message = format!("Error: {}", e);
                self.append_to_console(format!(
                    "[{}] ERROR: Initial load failed - {}",
                    chrono::Local::now().format("%H:%M:%S"),
                    e
                ));
            }
        }

        self.loading = false;
    }

    pub fn filter_packages(&mut self) {
        self.filtered_packages = self
            .packages
            .iter()
            .filter(|p| {
                // Filter by source
                if let Some(src) = self.selected_source {
                    if p.source != src {
                        return false;
                    }
                }
                // Filter by search query
                if !self.search_query.is_empty() {
                    let query = self.search_query.to_lowercase();
                    if !p.name.to_lowercase().contains(&query)
                        && !p.description.to_lowercase().contains(&query)
                    {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect();

        // Reset package index if out of bounds
        if self.package_index >= self.filtered_packages.len() {
            self.package_index = self.filtered_packages.len().saturating_sub(1);
        }
    }

    pub fn selected_package(&self) -> Option<&Package> {
        self.filtered_packages.get(self.package_index)
    }

    pub fn selected_queue_task(&self) -> Option<&TaskQueueEntry> {
        self.queued_tasks.get(self.queue_index)
    }

    pub fn next_source(&mut self) {
        // Index 0 is "All", rest are sources
        let total = self.available_sources.len() + 1;
        self.source_index = (self.source_index + 1) % total;
        self.selected_source = if self.source_index == 0 {
            None
        } else {
            Some(self.available_sources[self.source_index - 1])
        };
        self.filter_packages();
    }

    pub fn prev_source(&mut self) {
        let total = self.available_sources.len() + 1;
        self.source_index = if self.source_index == 0 {
            total - 1
        } else {
            self.source_index - 1
        };
        self.selected_source = if self.source_index == 0 {
            None
        } else {
            Some(self.available_sources[self.source_index - 1])
        };
        self.filter_packages();
    }

    pub fn next_package(&mut self) {
        if !self.filtered_packages.is_empty() {
            self.package_index = (self.package_index + 1) % self.filtered_packages.len();
        }
    }

    pub fn prev_package(&mut self) {
        if !self.filtered_packages.is_empty() {
            self.package_index = if self.package_index == 0 {
                self.filtered_packages.len() - 1
            } else {
                self.package_index - 1
            };
        }
    }

    pub fn next_queue_task(&mut self) {
        if !self.queued_tasks.is_empty() {
            self.queue_index = (self.queue_index + 1) % self.queued_tasks.len();
        }
    }

    pub fn prev_queue_task(&mut self) {
        if !self.queued_tasks.is_empty() {
            self.queue_index = if self.queue_index == 0 {
                self.queued_tasks.len() - 1
            } else {
                self.queue_index - 1
            };
        }
    }

    pub fn page_down(&mut self) {
        if !self.filtered_packages.is_empty() {
            self.package_index = (self.package_index + 10).min(self.filtered_packages.len() - 1);
        }
    }

    pub fn page_up(&mut self) {
        self.package_index = self.package_index.saturating_sub(10);
    }

    pub fn is_selected(&self, package: &Package) -> bool {
        self.selected_packages.contains(&package.id())
    }

    #[allow(dead_code)]
    pub fn toggle_selection(&mut self, package: &Package) {
        let id = package.id();
        if self.selected_packages.contains(&id) {
            self.selected_packages.remove(&id);
        } else {
            self.selected_packages.insert(id);
        }
    }

    #[allow(dead_code)]
    pub fn select_all(&mut self) {
        for pkg in &self.filtered_packages {
            self.selected_packages.insert(pkg.id());
        }
    }

    #[allow(dead_code)]
    pub fn clear_selection(&mut self) {
        self.selected_packages.clear();
    }

    #[allow(dead_code)]
    pub fn selected_count(&self) -> usize {
        self.selected_packages.len()
    }

    #[allow(dead_code)]
    pub fn get_selected_packages(&self) -> Vec<Package> {
        self.packages
            .iter()
            .filter(|p| self.selected_packages.contains(&p.id()))
            .cloned()
            .collect()
    }

    fn cleanup_stale_selections(&mut self) {
        let valid_ids: HashSet<String> = self.packages.iter().map(|p| p.id()).collect();
        self.selected_packages.retain(|id| valid_ids.contains(id));
    }

    async fn queue_tasks(&mut self, packages: Vec<Package>, action: TaskQueueAction) -> usize {
        let mut queued = 0;

        for pkg in packages {
            let entry = TaskQueueEntry::new(action, pkg.id(), pkg.name.clone(), pkg.source);
            self.enqueue_task_entry(entry).await;
            queued += 1;
        }

        if queued > 0 {
            self.spawn_task_executor();
        }

        queued
    }

    async fn enqueue_task_entry(&mut self, entry: TaskQueueEntry) {
        let enqueue_result = {
            let mut guard = self.history_tracker.lock().await;
            if let Some(tracker) = guard.as_mut() {
                tracker.enqueue_task(entry.clone()).await.map(|_| true)
            } else {
                Ok(false)
            }
        };

        match enqueue_result {
            Ok(true) => {}
            Ok(false) => {
                self.append_to_console(format!(
                    "[{}] WARN: history tracker unavailable; queue not persisted",
                    chrono::Local::now().format("%H:%M:%S")
                ));
            }
            Err(e) => {
                self.append_to_console(format!(
                    "[{}] ERROR: failed to persist task - {}",
                    chrono::Local::now().format("%H:%M:%S"),
                    e
                ));
            }
        }

        self.queued_tasks.push(entry);
        self.clamp_queue_index();
    }

    fn spawn_task_executor(&self) {
        if self.task_executor_running.swap(true, Ordering::SeqCst) {
            return;
        }

        let running = self.task_executor_running.clone();
        let pm = self.pm.clone();
        let history_tracker = self.history_tracker.clone();
        let sender = self.task_events_tx.clone();

        tokio::spawn(async move {
            let executor = TaskQueueExecutor::new(pm, history_tracker);
            if let Err(e) = executor.run(sender).await {
                error!(error = %e, "Task queue executor stopped");
            }
            running.store(false, Ordering::SeqCst);
        });
    }

    fn clamp_queue_index(&mut self) {
        if self.queued_tasks.is_empty() {
            self.queue_index = 0;
        } else if self.queue_index >= self.queued_tasks.len() {
            self.queue_index = self.queued_tasks.len() - 1;
        }
    }
}

pub async fn run() -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let pm = Arc::new(Mutex::new(PackageManager::new()));
    let history_tracker = Arc::new(Mutex::new(None));
    let (task_tx, task_rx) = mpsc::channel(200);
    let mut app = App::new(
        pm.clone(),
        history_tracker.clone(),
        Some(task_rx),
        Some(task_tx.clone()),
    );

    // Initial load
    app.initialize_history_tracker().await;
    app.sync_task_queue_from_history().await;
    app.load_sources().await;
    app.load_packages().await;

    app.spawn_task_executor();

    // Main loop
    let result = run_app(&mut terminal, &mut app).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> Result<()> {
    loop {
        let size = terminal.size()?;
        app.compact = size.width < COMPACT_WIDTH || size.height < COMPACT_HEIGHT;

        // Check for completed background loading
        app.poll_loading();
        app.poll_task_events();

        terminal.draw(|f| ui::draw(f, app))?;

        // Poll for events with a small timeout to allow async operations
        if event::poll(std::time::Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match app.mode {
                        AppMode::Normal => handle_normal_mode(app, key.code).await,
                        AppMode::Search => handle_search_mode(app, key.code),
                        AppMode::Confirm => handle_confirm_mode(app, key.code).await,
                    }
                }
            }
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}

async fn handle_normal_mode(app: &mut App, key: KeyCode) {
    // Don't process keys while loading (except quit)
    if app.loading && key != KeyCode::Char('q') && key != KeyCode::Esc {
        return;
    }

    match key {
        KeyCode::Char('q') | KeyCode::Esc => {
            app.should_quit = true;
        }
        KeyCode::Char('h') => {
            app.append_to_console(String::from(
                "Help displayed - keyboard shortcuts available",
            ));
            app.status_message = String::from(
                "j/k:nav | Tab:switch panel | Enter:focus details | /:search | Space:select | a:select-all | c:clear | u:updates | U:queue updates (filtered) | I:queue installs (selected) | X:queue removals (selected) | C:cancel task | R:retry failed | r:refresh | i:install | x:remove | q:quit"
            );
        }
        KeyCode::Tab => {
            app.active_panel = match app.active_panel {
                ActivePanel::Sources => ActivePanel::Packages,
                ActivePanel::Packages => ActivePanel::Details,
                ActivePanel::Details => ActivePanel::Queue,
                ActivePanel::Queue => ActivePanel::Sources,
            };
        }
        KeyCode::Char('j') | KeyCode::Down => match app.active_panel {
            ActivePanel::Sources => app.next_source(),
            ActivePanel::Packages => app.next_package(),
            ActivePanel::Details => app.next_package(),
            ActivePanel::Queue => app.next_queue_task(),
        },
        KeyCode::Char('k') | KeyCode::Up => match app.active_panel {
            ActivePanel::Sources => app.prev_source(),
            ActivePanel::Packages => app.prev_package(),
            ActivePanel::Details => app.prev_package(),
            ActivePanel::Queue => app.prev_queue_task(),
        },
        KeyCode::Char('g') | KeyCode::Home => {
            app.package_index = 0;
        }
        KeyCode::Char('G') | KeyCode::End => {
            if !app.filtered_packages.is_empty() {
                app.package_index = app.filtered_packages.len() - 1;
            }
        }
        KeyCode::PageDown | KeyCode::Char('d') => {
            app.page_down();
        }
        KeyCode::PageUp | KeyCode::Char('b') => {
            app.page_up();
        }
        KeyCode::Char('/') | KeyCode::Char('s') => {
            app.mode = AppMode::Search;
            app.search_query.clear();
            app.status_message =
                String::from("Search: type query, Enter to confirm, Esc to cancel");
        }
        KeyCode::Char('u') => {
            app.show_updates_only = !app.show_updates_only;
            app.start_loading();
        }
        KeyCode::Char('U') => {
            let updates: Vec<Package> = app
                .filtered_packages
                .iter()
                .filter(|pkg| pkg.has_update())
                .cloned()
                .collect();

            if updates.is_empty() {
                app.status_message = String::from("No updates available");
            } else {
                app.status_message =
                    format!("Queue updates for {} packages? (y/n)", updates.len());
                app.append_to_console(format!(
                    "[{}] Confirming queued updates for {} packages",
                    chrono::Local::now().format("%H:%M:%S"),
                    updates.len()
                ));
                app.pending_action = Some(PendingAction::UpdateAll(updates));
                app.mode = AppMode::Confirm;
            }
        }
        KeyCode::Char('I') => {
            let install_targets: Vec<Package> = app
                .get_selected_packages()
                .into_iter()
                .filter(|pkg| pkg.status == PackageStatus::NotInstalled)
                .collect();

            if install_targets.is_empty() {
                app.status_message = String::from("No installable packages selected");
            } else {
                app.status_message =
                    format!("Queue installs for {} packages? (y/n)", install_targets.len());
                app.append_to_console(format!(
                    "[{}] Confirming queued installs for {} packages",
                    chrono::Local::now().format("%H:%M:%S"),
                    install_targets.len()
                ));
                app.pending_action = Some(PendingAction::InstallSelected(install_targets));
                app.mode = AppMode::Confirm;
            }
        }
        KeyCode::Char('X') => {
            let remove_targets: Vec<Package> = app
                .get_selected_packages()
                .into_iter()
                .filter(|pkg| {
                    matches!(
                        pkg.status,
                        PackageStatus::Installed | PackageStatus::UpdateAvailable
                    )
                })
                .collect();

            if remove_targets.is_empty() {
                app.status_message = String::from("No removable packages selected");
            } else {
                app.status_message =
                    format!("Queue removals for {} packages? (y/n)", remove_targets.len());
                app.append_to_console(format!(
                    "[{}] Confirming queued removals for {} packages",
                    chrono::Local::now().format("%H:%M:%S"),
                    remove_targets.len()
                ));
                app.pending_action = Some(PendingAction::RemoveSelected(remove_targets));
                app.mode = AppMode::Confirm;
            }
        }
        KeyCode::Char('r') => {
            app.append_to_console(format!(
                "[{}] Manual refresh triggered - reloading package list...",
                chrono::Local::now().format("%H:%M:%S")
            ));
            app.start_loading();
        }
        KeyCode::Char('C') => {
            if app.active_panel != ActivePanel::Queue {
                return;
            }
            let task = app.selected_queue_task().cloned();
            match task {
                Some(task) => {
                    match task.status {
                        TaskQueueStatus::Queued => {
                            if let Some(pos) =
                                app.queued_tasks.iter().position(|t| t.id == task.id)
                            {
                                app.queued_tasks[pos].mark_cancelled();
                                app.clamp_queue_index();
                            }

                            let cancel_result = {
                                let mut guard = app.history_tracker.lock().await;
                                if let Some(tracker) = guard.as_mut() {
                                    tracker.mark_task_cancelled(&task.id).await
                                } else {
                                    Ok(None)
                                }
                            };

                            if let Err(e) = cancel_result {
                                app.append_to_console(format!(
                                    "[{}] ERROR: failed to cancel task - {}",
                                    chrono::Local::now().format("%H:%M:%S"),
                                    e
                                ));
                            }

                            app.status_message = format!(
                                "Cancelled {} for {}",
                                action_display_name(task.action),
                                task.package_name
                            );
                            app.append_to_console(format!(
                                "[{}] CANCELLED: {} {}",
                                chrono::Local::now().format("%H:%M:%S"),
                                action_display_name(task.action),
                                task.package_name
                            ));
                        }
                        TaskQueueStatus::Running => {
                            app.status_message = String::from("Cannot cancel running task");
                        }
                        TaskQueueStatus::Completed | TaskQueueStatus::Failed => {
                            app.status_message = String::from("Cannot cancel completed task");
                        }
                        TaskQueueStatus::Cancelled => {
                            app.status_message = String::from("Task already cancelled");
                        }
                    }
                }
                None => {
                    app.status_message = String::from("No queued task selected");
                }
            }
        }
        KeyCode::Char('R') => {
            if app.active_panel != ActivePanel::Queue {
                return;
            }
            let task = app.selected_queue_task().cloned();
            match task {
                Some(task) => {
                    if task.status != TaskQueueStatus::Failed {
                        app.status_message = String::from("Only failed tasks can be retried");
                        return;
                    }

                    let retry_task = TaskQueueEntry::new(
                        task.action,
                        task.package_id.clone(),
                        task.package_name.clone(),
                        task.package_source,
                    );
                    app.enqueue_task_entry(retry_task).await;
                    app.spawn_task_executor();
                    app.status_message = format!(
                        "Re-queued {} for {}",
                        action_display_name(task.action),
                        task.package_name
                    );
                    app.append_to_console(format!(
                        "[{}] RETRY: {} {}",
                        chrono::Local::now().format("%H:%M:%S"),
                        action_display_name(task.action),
                        task.package_name
                    ));
                }
                None => {
                    app.status_message = String::from("No queued task selected");
                }
            }
        }
        KeyCode::Char(' ') => {
            if let Some(pkg) = app.selected_package().cloned() {
                app.toggle_selection(&pkg);
                app.status_message = format!("Selected: {}", app.selected_count());
            } else {
                app.status_message = String::from("No package selected");
            }
        }
        KeyCode::Char('a') => {
            app.select_all();
            app.status_message = format!("Selected: {}", app.selected_count());
        }
        KeyCode::Char('c') => {
            app.clear_selection();
            app.status_message = String::from("Selection cleared");
        }
        KeyCode::Char('i') => {
            if let Some(pkg) = app.selected_package().cloned() {
                app.status_message = format!("Install {}? (y/n)", pkg.name);
                app.append_to_console(format!(
                    "[{}] Confirming installation of {} v{} from {:?}",
                    chrono::Local::now().format("%H:%M:%S"),
                    pkg.name,
                    pkg.version,
                    pkg.source
                ));
                app.pending_action = Some(PendingAction::Install(pkg));
                app.mode = AppMode::Confirm;
            }
        }
        KeyCode::Char('x') => {
            if let Some(pkg) = app.selected_package().cloned() {
                app.status_message = format!("Remove {}? (y/n)", pkg.name);
                app.append_to_console(format!(
                    "[{}] Confirming removal of {} v{}",
                    chrono::Local::now().format("%H:%M:%S"),
                    pkg.name,
                    pkg.version
                ));
                app.pending_action = Some(PendingAction::Remove(pkg));
                app.mode = AppMode::Confirm;
            }
        }
        KeyCode::Enter => match app.active_panel {
            ActivePanel::Queue => {
                if let Some(task) = app.selected_queue_task() {
                    let status = match task.status {
                        TaskQueueStatus::Queued => "Queued",
                        TaskQueueStatus::Running => "Running",
                        TaskQueueStatus::Completed => "Completed",
                        TaskQueueStatus::Failed => "Failed",
                        TaskQueueStatus::Cancelled => "Cancelled",
                    };
                    app.status_message = format!(
                        "{} {} - {}",
                        action_display_name(task.action),
                        task.package_name,
                        status
                    );
                }
            }
            _ => {
                if let Some(pkg) = app.selected_package() {
                    app.status_message = format!(
                        "{} v{} ({:?}) - {}",
                        pkg.name, pkg.version, pkg.source, pkg.description
                    );
                    app.active_panel = ActivePanel::Details;
                }
            }
        },
        _ => {}
    }
}

fn action_display_name(action: TaskQueueAction) -> &'static str {
    match action {
        TaskQueueAction::Install => "Install",
        TaskQueueAction::Remove => "Remove",
        TaskQueueAction::Update => "Update",
    }
}

fn handle_search_mode(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc => {
            app.mode = AppMode::Normal;
            app.search_query.clear();
            app.filter_packages();
            app.status_message = String::from("Search cancelled");
        }
        KeyCode::Enter => {
            app.mode = AppMode::Normal;
            app.filter_packages();
            app.status_message = format!(
                "Found {} packages matching '{}'",
                app.filtered_packages.len(),
                app.search_query
            );
        }
        KeyCode::Backspace => {
            app.search_query.pop();
            app.filter_packages();
        }
        KeyCode::Char(c) => {
            app.search_query.push(c);
            app.filter_packages();
        }
        _ => {}
    }
}

async fn handle_confirm_mode(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            let action = app.pending_action.take();
            let mut should_refresh = false;
            match action {
                Some(PendingAction::Install(pkg)) => {
                    let result = {
                        let manager = app.pm.lock().await;
                        manager.install(&pkg).await
                    };
                    match result {
                        Ok(_) => {
                            app.status_message = format!("Success: {}", pkg.name);
                            app.append_to_console(format!(
                                "[{}] INSTALLED: {} v{} from {:?}",
                                chrono::Local::now().format("%H:%M:%S"),
                                pkg.name,
                                pkg.version,
                                pkg.source
                            ));
                        }
                        Err(e) => {
                            app.status_message = format!("Error: {}", e);
                            app.append_to_console(format!(
                                "[{}] FAILED: {} - {}",
                                chrono::Local::now().format("%H:%M:%S"),
                                pkg.name,
                                e
                            ));
                        }
                    }
                    should_refresh = true;
                }
                Some(PendingAction::Remove(pkg)) => {
                    let result = {
                        let manager = app.pm.lock().await;
                        manager.remove(&pkg).await
                    };
                    match result {
                        Ok(_) => {
                            app.status_message = format!("Success: {}", pkg.name);
                            app.append_to_console(format!(
                                "[{}] REMOVED: {} v{}",
                                chrono::Local::now().format("%H:%M:%S"),
                                pkg.name,
                                pkg.version
                            ));
                        }
                        Err(e) => {
                            app.status_message = format!("Error: {}", e);
                            app.append_to_console(format!(
                                "[{}] FAILED: {} - {}",
                                chrono::Local::now().format("%H:%M:%S"),
                                pkg.name,
                                e
                            ));
                        }
                    }
                    should_refresh = true;
                }
                Some(PendingAction::UpdateAll(packages)) => {
                    let queued = app.queue_tasks(packages, TaskQueueAction::Update).await;
                    app.status_message = format!("Queued {} update tasks", queued);
                    app.append_to_console(format!(
                        "[{}] QUEUED: {} update tasks",
                        chrono::Local::now().format("%H:%M:%S"),
                        queued
                    ));
                }
                Some(PendingAction::InstallSelected(packages)) => {
                    let queued = app.queue_tasks(packages, TaskQueueAction::Install).await;
                    app.status_message = format!("Queued {} install tasks", queued);
                    app.append_to_console(format!(
                        "[{}] QUEUED: {} install tasks",
                        chrono::Local::now().format("%H:%M:%S"),
                        queued
                    ));
                }
                Some(PendingAction::RemoveSelected(packages)) => {
                    let queued = app.queue_tasks(packages, TaskQueueAction::Remove).await;
                    app.status_message = format!("Queued {} remove tasks", queued);
                    app.append_to_console(format!(
                        "[{}] QUEUED: {} remove tasks",
                        chrono::Local::now().format("%H:%M:%S"),
                        queued
                    ));
                }
                None => {}
            }
            app.mode = AppMode::Normal;
            if should_refresh {
                app.start_loading();
            }
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            app.pending_action = None;
            app.mode = AppMode::Normal;
            app.status_message = String::from("Cancelled");
            app.append_to_console(format!(
                "[{}] Operation cancelled by user",
                chrono::Local::now().format("%H:%M:%S")
            ));
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::PackageStatus;

    fn create_test_package(name: &str, source: PackageSource) -> Package {
        Package {
            name: name.to_string(),
            version: "1.0.0".to_string(),
            available_version: None,
            description: "Test package".to_string(),
            source,
            status: PackageStatus::Installed,
            size: None,
            homepage: None,
            license: None,
            maintainer: None,
            dependencies: vec![],
            install_date: None,
            update_category: None,
            enrichment: None,
        }
    }

    #[test]
    fn test_selection_methods() {
        let pm = Arc::new(Mutex::new(PackageManager::new()));
        let history_tracker = Arc::new(Mutex::new(None));
        let mut app = App::new(pm, history_tracker, None, None);

        let pkg1 = create_test_package("pkg1", PackageSource::Apt);
        let pkg2 = create_test_package("pkg2", PackageSource::Apt);
        let pkg3 = create_test_package("pkg3", PackageSource::Dnf);

        app.packages = vec![pkg1.clone(), pkg2.clone(), pkg3.clone()];
        app.filter_packages();

        assert!(!app.is_selected(&pkg1));
        assert_eq!(app.selected_count(), 0);

        app.toggle_selection(&pkg1);
        assert!(app.is_selected(&pkg1));
        assert_eq!(app.selected_count(), 1);

        app.toggle_selection(&pkg1);
        assert!(!app.is_selected(&pkg1));
        assert_eq!(app.selected_count(), 0);

        app.select_all();
        assert_eq!(app.selected_count(), 3);
        assert!(app.is_selected(&pkg1));
        assert!(app.is_selected(&pkg2));
        assert!(app.is_selected(&pkg3));

        app.clear_selection();
        assert_eq!(app.selected_count(), 0);

        app.select_all();
        let selected = app.get_selected_packages();
        assert_eq!(selected.len(), 3);
    }

    #[test]
    fn test_cleanup_stale_selections() {
        let pm = Arc::new(Mutex::new(PackageManager::new()));
        let history_tracker = Arc::new(Mutex::new(None));
        let mut app = App::new(pm, history_tracker, None, None);

        let pkg1 = create_test_package("pkg1", PackageSource::Apt);
        let pkg2 = create_test_package("pkg2", PackageSource::Apt);

        app.packages = vec![pkg1.clone(), pkg2.clone()];
        app.filter_packages();

        app.select_all();
        assert_eq!(app.selected_count(), 2);

        app.packages = vec![pkg1.clone()];
        app.cleanup_stale_selections();
        assert_eq!(app.selected_count(), 1);
        assert!(app.is_selected(&pkg1));
    }
}
