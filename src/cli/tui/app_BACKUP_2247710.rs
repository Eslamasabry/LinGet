use crate::backend::PackageManager;
use crate::models::{Package, PackageSource};
use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::collections::VecDeque;
use std::io;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

use super::ui;

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
    pub console_buffer: VecDeque<String>,
    pub console_scroll: usize,
}

impl App {
    pub fn new(pm: Arc<Mutex<PackageManager>>) -> Self {
        let mut app = Self {
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
            console_buffer: VecDeque::with_capacity(100),
            console_scroll: 0,
        };
        app.push_console_message("LinGet started");
        app
    }

    pub fn push_console_message(&mut self, message: &str) {
        self.console_buffer.push_back(message.to_string());
        if self.console_buffer.len() > 100 {
            self.console_buffer.pop_front();
        }
        self.console_scroll = self.console_buffer.len().saturating_sub(1).max(0);
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
        let msg = if self.show_updates_only {
            String::from("Checking for updates...")
        } else {
            String::from("Loading packages...")
        };
        self.status_message = msg.clone();
        self.push_console_message(&msg);

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
                    self.filter_packages();
                    let msg = if self.show_updates_only {
                        format!("{} updates available", self.filtered_packages.len())
                    } else {
                        format!("Loaded {} packages", self.filtered_packages.len())
                    };
                    self.status_message = msg.clone();
                    self.push_console_message(&msg);
                    self.loading = false;
                    self.load_rx = None;
                }
                Ok(Err(e)) => {
                    let msg = format!("Error: {}", e);
                    self.status_message = msg.clone();
                    self.push_console_message(&msg);
                    self.loading = false;
                    self.load_rx = None;
                }
                Err(mpsc::error::TryRecvError::Empty) => {
                    // Still loading, do nothing
                }
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    self.status_message = String::from("Loading failed");
                    self.loading = false;
                    self.load_rx = None;
                }
            }
        }
    }

    /// Blocking load for initial startup
    pub async fn load_packages(&mut self) {
        self.loading = true;
        let msg = String::from("Loading packages...");
        self.status_message = msg.clone();
        self.push_console_message(&msg);

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
                self.filter_packages();
                let msg = format!("Loaded {} packages", self.filtered_packages.len());
                self.status_message = msg.clone();
                self.push_console_message(&msg);
            }
            Err(e) => {
                let msg = format!("Error: {}", e);
                self.status_message = msg.clone();
                self.push_console_message(&msg);
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
            app.status_message = String::from(
                "j/k:nav | Tab:switch panel | Enter:focus details | /: search | u:updates | r:refresh | i:install | x:remove | q:quit"
            );
            app.push_console_message("Help displayed");
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
            app.push_console_message("Search mode activated");
        }
        KeyCode::Char('u') => {
            app.show_updates_only = !app.show_updates_only;
            app.start_loading();
        }
        KeyCode::Char('r') => {
            app.start_loading();
        }
        KeyCode::Char('i') => {
            if let Some(pkg) = app.selected_package().cloned() {
                let msg = format!("Install {}? (y/n)", pkg.name);
                app.status_message = msg;
                app.push_console_message(&format!("Install {} requested", pkg.name));
                app.mode = AppMode::Confirm;
            }
        }
        KeyCode::Char('x') => {
            if let Some(pkg) = app.selected_package().cloned() {
                let msg = format!("Remove {}? (y/n)", pkg.name);
                app.status_message = msg;
                app.push_console_message(&format!("Remove {} requested", pkg.name));
                app.mode = AppMode::Confirm;
            }
        }
        KeyCode::Enter => {
            if let Some(pkg) = app.selected_package().cloned() {
                let msg = format!(
                    "{} v{} ({:?}) - {}",
                    pkg.name, pkg.version, pkg.source, pkg.description
                );
<<<<<<< HEAD
                app.status_message = msg;
                app.push_console_message(&format!("Viewed package: {}", pkg.name));
=======
                app.active_panel = ActivePanel::Details;
>>>>>>> 2ccd7b0 (LinGet-nge.2: LinGet-nge.2 - US-002: Populate package details panel)
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
            app.push_console_message("Search cancelled");
        }
        KeyCode::Enter => {
            app.mode = AppMode::Normal;
            app.filter_packages();
            let msg = format!(
                "Found {} packages matching '{}'",
                app.filtered_packages.len(),
                app.search_query
            );
            app.status_message = msg.clone();
            app.push_console_message(&msg);
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
            if let Some(pkg) = app.selected_package().cloned() {
                let manager = app.pm.lock().await;
                let is_install = app.status_message.starts_with("Install");
                let result = if is_install {
                    manager.install(&pkg).await
                } else {
                    manager.remove(&pkg).await
                };
                drop(manager);

                match result {
                    Ok(_) => {
                        let msg = format!("Success: {}", pkg.name);
                        app.status_message = msg.clone();
                        app.push_console_message(&format!(
                            "{} {}: success",
                            if is_install { "Installed" } else { "Removed" },
                            pkg.name
                        ));
                    }
                    Err(e) => {
                        let msg = format!("Error: {}", e);
                        app.status_message = msg.clone();
                        app.push_console_message(&msg);
                    }
                }
            }
            app.mode = AppMode::Normal;
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            app.mode = AppMode::Normal;
            app.status_message = String::from("Cancelled");
            app.push_console_message("Action cancelled");
        }
        _ => {}
    }
}
