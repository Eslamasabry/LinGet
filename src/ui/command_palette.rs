#![allow(dead_code)]

use gtk4::prelude::*;
use gtk4::{self as gtk, glib};
use libadwaita as adw;
use libadwaita::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

type CommandCallback = Rc<RefCell<Option<Box<dyn Fn(PaletteCommand)>>>>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaletteCommand {
    UpdateAll,
    ScheduleAllUpdates,
    CleanCaches,
    GoToHome,
    GoToLibrary,
    GoToUpdates,
    GoToStorage,
    GoToHealth,
    GoToHistory,
    GoToFavorites,
    GoToTasks,
    RefreshPackages,
    ToggleSelectionMode,
    OpenPreferences,
    ShowShortcuts,
    ExportPackages,
    ImportPackages,
    Search(String),
}

impl PaletteCommand {
    pub fn label(&self) -> &str {
        match self {
            PaletteCommand::UpdateAll => "Update all packages",
            PaletteCommand::ScheduleAllUpdates => "Schedule all updates for later",
            PaletteCommand::CleanCaches => "Clean all caches",
            PaletteCommand::GoToHome => "Go to Home",
            PaletteCommand::GoToLibrary => "Go to Library",
            PaletteCommand::GoToUpdates => "Go to Updates",
            PaletteCommand::GoToStorage => "Go to Storage",
            PaletteCommand::GoToHealth => "Go to Health",
            PaletteCommand::GoToHistory => "Go to History",
            PaletteCommand::GoToFavorites => "Go to Favorites",
            PaletteCommand::GoToTasks => "Go to Scheduled Tasks",
            PaletteCommand::RefreshPackages => "Refresh package list",
            PaletteCommand::ToggleSelectionMode => "Toggle selection mode",
            PaletteCommand::OpenPreferences => "Open preferences",
            PaletteCommand::ShowShortcuts => "Show keyboard shortcuts",
            PaletteCommand::ExportPackages => "Export packages",
            PaletteCommand::ImportPackages => "Import packages",
            PaletteCommand::Search(_) => "Search packages",
        }
    }

    pub fn icon(&self) -> &str {
        match self {
            PaletteCommand::UpdateAll => "software-update-available-symbolic",
            PaletteCommand::ScheduleAllUpdates => "alarm-symbolic",
            PaletteCommand::CleanCaches => "edit-clear-all-symbolic",
            PaletteCommand::GoToHome => "go-home-symbolic",
            PaletteCommand::GoToLibrary => "view-grid-symbolic",
            PaletteCommand::GoToUpdates => "software-update-available-symbolic",
            PaletteCommand::GoToStorage => "drive-harddisk-symbolic",
            PaletteCommand::GoToHealth => "heart-outline-thick-symbolic",
            PaletteCommand::GoToHistory => "document-open-recent-symbolic",
            PaletteCommand::GoToFavorites => "starred-symbolic",
            PaletteCommand::GoToTasks => "alarm-symbolic",
            PaletteCommand::RefreshPackages => "view-refresh-symbolic",
            PaletteCommand::ToggleSelectionMode => "selection-mode-symbolic",
            PaletteCommand::OpenPreferences => "emblem-system-symbolic",
            PaletteCommand::ShowShortcuts => "preferences-desktop-keyboard-shortcuts-symbolic",
            PaletteCommand::ExportPackages => "document-save-symbolic",
            PaletteCommand::ImportPackages => "document-open-symbolic",
            PaletteCommand::Search(_) => "system-search-symbolic",
        }
    }

    pub fn shortcut(&self) -> Option<&str> {
        match self {
            PaletteCommand::RefreshPackages => Some("Ctrl+R"),
            PaletteCommand::ToggleSelectionMode => Some("Ctrl+S"),
            PaletteCommand::OpenPreferences => Some("Ctrl+,"),
            _ => None,
        }
    }

    pub fn keywords(&self) -> &[&str] {
        match self {
            PaletteCommand::UpdateAll => &["update", "upgrade", "all", "packages"],
            PaletteCommand::ScheduleAllUpdates => &["schedule", "later", "defer", "timer", "queue", "all", "updates"],
            PaletteCommand::CleanCaches => &["clean", "cache", "cleanup", "clear", "free", "space"],
            PaletteCommand::GoToHome => &["home", "discover", "browse"],
            PaletteCommand::GoToLibrary => &["library", "installed", "packages", "list"],
            PaletteCommand::GoToUpdates => &["updates", "available", "pending"],
            PaletteCommand::GoToStorage => &["storage", "disk", "space", "size"],
            PaletteCommand::GoToHealth => &["health", "status", "score", "issues"],
            PaletteCommand::GoToHistory => &["history", "timeline", "log", "recent"],
            PaletteCommand::GoToFavorites => &["favorites", "starred", "bookmarks"],
            PaletteCommand::GoToTasks => &["tasks", "scheduled", "queue", "pending", "timer", "alarm"],
            PaletteCommand::RefreshPackages => &["refresh", "reload", "sync"],
            PaletteCommand::ToggleSelectionMode => &["select", "selection", "multi", "bulk"],
            PaletteCommand::OpenPreferences => &["preferences", "settings", "config", "options"],
            PaletteCommand::ShowShortcuts => &["shortcuts", "keyboard", "keys", "help"],
            PaletteCommand::ExportPackages => &["export", "backup", "save", "list"],
            PaletteCommand::ImportPackages => &["import", "restore", "load"],
            PaletteCommand::Search(_) => &["search", "find", "filter"],
        }
    }

    fn all_static() -> Vec<PaletteCommand> {
        vec![
            PaletteCommand::UpdateAll,
            PaletteCommand::ScheduleAllUpdates,
            PaletteCommand::CleanCaches,
            PaletteCommand::GoToHome,
            PaletteCommand::GoToLibrary,
            PaletteCommand::GoToUpdates,
            PaletteCommand::GoToStorage,
            PaletteCommand::GoToHealth,
            PaletteCommand::GoToHistory,
            PaletteCommand::GoToFavorites,
            PaletteCommand::GoToTasks,
            PaletteCommand::RefreshPackages,
            PaletteCommand::ToggleSelectionMode,
            PaletteCommand::OpenPreferences,
            PaletteCommand::ShowShortcuts,
            PaletteCommand::ExportPackages,
            PaletteCommand::ImportPackages,
        ]
    }
}

fn fuzzy_match(query: &str, target: &str) -> bool {
    if query.is_empty() {
        return true;
    }
    let query_lower = query.to_lowercase();
    let target_lower = target.to_lowercase();

    if target_lower.contains(&query_lower) {
        return true;
    }

    let mut query_chars = query_lower.chars().peekable();
    for c in target_lower.chars() {
        if query_chars.peek() == Some(&c) {
            query_chars.next();
        }
    }
    query_chars.peek().is_none()
}

fn score_match(query: &str, cmd: &PaletteCommand) -> Option<i32> {
    if query.is_empty() {
        return Some(0);
    }

    let label = cmd.label().to_lowercase();
    let query_lower = query.to_lowercase();

    if label.starts_with(&query_lower) {
        return Some(100);
    }
    if label.contains(&query_lower) {
        return Some(80);
    }

    for keyword in cmd.keywords() {
        if keyword.starts_with(&query_lower) {
            return Some(70);
        }
        if keyword.contains(&query_lower) {
            return Some(50);
        }
    }

    if fuzzy_match(&query_lower, &label) {
        return Some(30);
    }

    None
}

pub struct CommandPalette {
    window: adw::Window,
    search_entry: gtk::SearchEntry,
    results_list: gtk::ListBox,
    commands: Rc<RefCell<Vec<PaletteCommand>>>,
    on_command: CommandCallback,
}

impl CommandPalette {
    pub fn new(parent: &impl IsA<gtk::Widget>) -> Self {
        let window = adw::Window::builder()
            .title("")
            .default_width(500)
            .default_height(400)
            .modal(true)
            .build();
        window.add_css_class("command-palette-window");

        if let Some(root) = parent.root() {
            if let Some(app_window) = root.downcast_ref::<gtk::Window>() {
                window.set_transient_for(Some(app_window));
            }
        }

        let container = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(0)
            .build();

        let search_entry = gtk::SearchEntry::builder()
            .placeholder_text("Type a command...")
            .hexpand(true)
            .build();
        search_entry.add_css_class("command-palette-entry");

        let search_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(8)
            .margin_top(16)
            .margin_bottom(8)
            .margin_start(16)
            .margin_end(16)
            .build();
        search_box.append(&search_entry);

        container.append(&search_box);
        container.append(&gtk::Separator::new(gtk::Orientation::Horizontal));

        let results_list = gtk::ListBox::builder()
            .selection_mode(gtk::SelectionMode::Single)
            .build();
        results_list.add_css_class("command-palette-list");
        results_list.add_css_class("boxed-list");

        let scrolled = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vexpand(true)
            .child(&results_list)
            .build();

        let list_container = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .margin_top(8)
            .margin_bottom(16)
            .margin_start(16)
            .margin_end(16)
            .build();
        list_container.append(&scrolled);

        container.append(&list_container);
        window.set_content(Some(&container));

        let commands = Rc::new(RefCell::new(PaletteCommand::all_static()));
        let on_command: CommandCallback = Rc::new(RefCell::new(None));

        let palette = Self {
            window,
            search_entry,
            results_list,
            commands,
            on_command,
        };

        palette.setup_signals();
        palette.populate_results("");

        palette
    }

    fn setup_signals(&self) {
        let results_list = self.results_list.clone();
        let commands = self.commands.clone();

        self.search_entry.connect_search_changed({
            let results_list = results_list.clone();
            let commands = commands.clone();
            move |entry| {
                let query = entry.text().to_string();
                Self::update_results(&results_list, &commands.borrow(), &query);
            }
        });

        let window = self.window.clone();
        let on_command = self.on_command.clone();
        let commands_for_activate = self.commands.clone();

        self.results_list.connect_row_activated({
            let window = window.clone();
            move |_, row| {
                let idx = row.index() as usize;
                let cmds = commands_for_activate.borrow();
                if let Some(cmd) = cmds.get(idx) {
                    if let Some(ref callback) = *on_command.borrow() {
                        callback(cmd.clone());
                    }
                    window.close();
                }
            }
        });

        let key_controller = gtk::EventControllerKey::new();
        let window_for_key = self.window.clone();
        let results_list_for_key = self.results_list.clone();
        let on_command_for_key = self.on_command.clone();
        let commands_for_key = self.commands.clone();

        key_controller.connect_key_pressed(move |_, keyval, _keycode, _state| match keyval {
            gtk::gdk::Key::Escape => {
                window_for_key.close();
                glib::Propagation::Stop
            }
            gtk::gdk::Key::Return | gtk::gdk::Key::KP_Enter => {
                if let Some(row) = results_list_for_key.selected_row() {
                    let idx = row.index() as usize;
                    let cmds = commands_for_key.borrow();
                    if let Some(cmd) = cmds.get(idx) {
                        if let Some(ref callback) = *on_command_for_key.borrow() {
                            callback(cmd.clone());
                        }
                        window_for_key.close();
                    }
                }
                glib::Propagation::Stop
            }
            gtk::gdk::Key::Down => {
                Self::navigate_list(&results_list_for_key, 1);
                glib::Propagation::Stop
            }
            gtk::gdk::Key::Up => {
                Self::navigate_list(&results_list_for_key, -1);
                glib::Propagation::Stop
            }
            _ => glib::Propagation::Proceed,
        });
        self.window.add_controller(key_controller);
    }

    fn navigate_list(list: &gtk::ListBox, direction: i32) {
        let current = list.selected_row().map(|r| r.index()).unwrap_or(-1);
        let next = current + direction;
        if next >= 0 {
            if let Some(row) = list.row_at_index(next) {
                list.select_row(Some(&row));
                row.grab_focus();
            }
        }
    }

    fn populate_results(&self, query: &str) {
        Self::update_results(&self.results_list, &self.commands.borrow(), query);
    }

    fn update_results(list: &gtk::ListBox, commands: &[PaletteCommand], query: &str) {
        while let Some(child) = list.first_child() {
            list.remove(&child);
        }

        let mut scored: Vec<(i32, &PaletteCommand)> = commands
            .iter()
            .filter_map(|cmd| score_match(query, cmd).map(|score| (score, cmd)))
            .collect();

        scored.sort_by(|a, b| b.0.cmp(&a.0));

        for (_, cmd) in scored.iter().take(10) {
            let row = Self::create_command_row(cmd);
            list.append(&row);
        }

        if let Some(first_row) = list.row_at_index(0) {
            list.select_row(Some(&first_row));
        }
    }

    fn create_command_row(cmd: &PaletteCommand) -> adw::ActionRow {
        let row = adw::ActionRow::builder()
            .title(cmd.label())
            .activatable(true)
            .build();

        let icon = gtk::Image::builder().icon_name(cmd.icon()).build();
        row.add_prefix(&icon);

        if let Some(shortcut) = cmd.shortcut() {
            let shortcut_label = gtk::Label::builder().label(shortcut).build();
            shortcut_label.add_css_class("dim-label");
            shortcut_label.add_css_class("caption");
            row.add_suffix(&shortcut_label);
        }

        row
    }

    pub fn connect_command<F: Fn(PaletteCommand) + 'static>(&self, callback: F) {
        *self.on_command.borrow_mut() = Some(Box::new(callback));
    }

    pub fn present(&self) {
        self.search_entry.set_text("");
        self.populate_results("");
        self.window.present();
        self.search_entry.grab_focus();
    }

    pub fn close(&self) {
        self.window.close();
    }
}
