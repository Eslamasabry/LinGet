use crate::backend::PackageManager;
use crate::models::{Package, PackageSource};
use anyhow::Result;
use chrono;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::collections::HashSet;
use std::io;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

use super::ui;

const MAX_CONSOLE_LINES: usize = 100;
const COMPACT_WIDTH: u16 = 100;
const COMPACT_HEIGHT: u16 = 28;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivePanel {
    Sources,
    Packages,
    Details,
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
}

pub struct App {
    pub pm: Arc<Mutex<PackageManager>>,
    pub packages: Vec<Package>,
    pub filtered_packages: Vec<Package>,
    pub available_sources: Vec<PackageSource>,
    pub selected_source: Option<PackageSource>,
    pub source_index: usize,
    pub package_index: usize,
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
}

impl App {
    pub fn new(pm: Arc<Mutex<PackageManager>>) -> Self {
        Self {
            pm,
            packages: Vec::new(),
            filtered_packages: Vec::new(),
            available_sources: Vec::new(),
            selected_source: None,
            source_index: 0,
            package_index: 0,
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
    let mut app = App::new(pm);

    // Initial load
    app.load_sources().await;
    app.load_packages().await;

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

        terminal.draw(|f| ui::draw(f, app))?;

        // Poll for events with a small timeout to allow async operations
        if event::poll(std::time::Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match app.mode {
                        AppMode::Normal => handle_normal_mode(app, key.code),
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

fn handle_normal_mode(app: &mut App, key: KeyCode) {
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
                "j/k:nav | Tab:switch panel | Enter:focus details | /: search | u:updates | U:update all (filtered) | r:refresh | i:install | x:remove | q:quit"
            );
        }
        KeyCode::Tab => {
            app.active_panel = match app.active_panel {
                ActivePanel::Sources => ActivePanel::Packages,
                ActivePanel::Packages => ActivePanel::Details,
                ActivePanel::Details => ActivePanel::Sources,
            };
        }
        KeyCode::Char('j') | KeyCode::Down => match app.active_panel {
            ActivePanel::Sources => app.next_source(),
            ActivePanel::Packages => app.next_package(),
            ActivePanel::Details => app.next_package(),
        },
        KeyCode::Char('k') | KeyCode::Up => match app.active_panel {
            ActivePanel::Sources => app.prev_source(),
            ActivePanel::Packages => app.prev_package(),
            ActivePanel::Details => app.prev_package(),
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
                app.status_message = format!("Update all {} packages? (y/n)", updates.len());
                app.append_to_console(format!(
                    "[{}] Confirming bulk update of {} packages",
                    chrono::Local::now().format("%H:%M:%S"),
                    updates.len()
                ));
                app.pending_action = Some(PendingAction::UpdateAll(updates));
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
        KeyCode::Enter => {
            if let Some(pkg) = app.selected_package() {
                app.status_message = format!(
                    "{} v{} ({:?}) - {}",
                    pkg.name, pkg.version, pkg.source, pkg.description
                );
                app.active_panel = ActivePanel::Details;
            }
        }
        _ => {}
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
                    let mut ok_count = 0;
                    let mut failed_count = 0;
                    let mut logs: Vec<String> = Vec::new();
                    let now = chrono::Local::now().format("%H:%M:%S").to_string();

                    {
                        let manager = app.pm.lock().await;

                        for pkg in &packages {
                            match manager.update(pkg).await {
                                Ok(_) => {
                                    ok_count += 1;
                                    logs.push(format!(
                                        "[{}] UPDATED: {} v{} -> v{}",
                                        now,
                                        pkg.name,
                                        pkg.version,
                                        pkg.available_version.as_ref().unwrap_or(&pkg.version)
                                    ));
                                }
                                Err(e) => {
                                    failed_count += 1;
                                    logs.push(format!("[{}] FAILED: {} - {}", now, pkg.name, e));
                                }
                            }
                        }
                    };

                    for log in logs {
                        app.append_to_console(log);
                    }

                    if failed_count == 0 {
                        app.status_message = format!("Updated {} packages", ok_count);
                        app.append_to_console(format!(
                            "[{}] BULK UPDATE COMPLETE: {} succeeded, 0 failed",
                            now, ok_count
                        ));
                    } else {
                        app.status_message =
                            format!("Update all: {} ok, {} failed", ok_count, failed_count);
                        app.append_to_console(format!(
                            "[{}] BULK UPDATE COMPLETE: {} succeeded, {} failed",
                            now, ok_count, failed_count
                        ));
                    }
                    should_refresh = true;
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
        let mut app = App::new(pm);

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
        let mut app = App::new(pm);

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
