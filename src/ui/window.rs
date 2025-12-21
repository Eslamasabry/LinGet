use crate::app::with_tray;
use crate::backend::PackageManager;
use crate::models::{
    Config, OperationHistory, OperationRecord, OperationType, Package, PackageCache, PackageSource,
    PackageStatus,
};
use crate::ui::content::ContentArea;
use crate::ui::header::Header;
use crate::ui::operations::{execute_bulk_operation, BulkOpContext, BulkOpKind};
use crate::ui::shortcuts::{setup_keyboard_shortcuts, ShortcutContext};
use crate::ui::sidebar::{set_enabled_in_config, Sidebar};
use crate::ui::widgets::{ProgressOverlay, SelectionBar};
use crate::ui::{
    notify_updates_available, show_about_dialog, CommandCenter, CommandEventKind,
    DiagnosticsDialog, OnboardingWindow, PackageDetailsPanel, PackageOp, PackageRow,
    PreferencesDialog, RetrySpec, TrayAction, View,
};
use gtk4::prelude::*;
use gtk4::{self as gtk, gio, glib};
use humansize::{format_size, BINARY};
use libadwaita as adw;
use libadwaita::prelude::*;
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

fn parse_suggestion(message: &str) -> Option<(String, String)> {
    let marker = crate::backend::SUGGEST_PREFIX;
    let idx = message.find(marker)?;
    let command = message[idx + marker.len()..].trim();
    if command.is_empty() {
        return None;
    }
    Some((message[..idx].trim().to_string(), command.to_string()))
}

type ActiveSource = Rc<RefCell<Option<glib::SourceId>>>;

fn cancel_active_source(source: &ActiveSource) {
    if let Some(id) = source.borrow_mut().take() {
        id.remove();
    }
}

fn remove_source(id: glib::SourceId) {
    id.remove();
}

/// Filter state for the package list
#[derive(Clone, Default)]
struct FilterState {
    source: Option<PackageSource>,
    search_query: String,
}

type LocalFn = Rc<dyn Fn()>;
type LocalFnHolder = Rc<RefCell<Option<LocalFn>>>;
type SourceClickHolder = Rc<RefCell<Option<Rc<dyn Fn(PackageSource)>>>>;
type ShowDetailsFn = Rc<dyn Fn(Package)>;
type ShowDetailsFnHolder = Rc<RefCell<Option<ShowDetailsFn>>>;

pub struct LinGetWindow {
    pub window: adw::ApplicationWindow,
    package_manager: Arc<Mutex<PackageManager>>,
    available_sources: Rc<RefCell<HashSet<PackageSource>>>,
    enabled_sources: Rc<RefCell<HashSet<PackageSource>>>,
    packages: Rc<RefCell<Vec<Package>>>,
    config: Rc<RefCell<Config>>,
    filter_state: Rc<RefCell<FilterState>>,
    selection_mode: Rc<RefCell<bool>>,
    sidebar: Sidebar,
    content: ContentArea,
    // Discover view
    discover_rows: Rc<RefCell<Vec<PackageRow>>>,
    // All packages view
    last_filtered_all: Rc<RefCell<Vec<Package>>>,
    // Updates view
    last_filtered_updates: Rc<RefCell<Vec<Package>>>,
    // Bulk selection
    selected_ids: Rc<RefCell<HashSet<String>>>,
    // UI elements
    search_entry: gtk::SearchEntry,
    main_stack: gtk::Stack,
    spinner: gtk::Spinner,
    toast_overlay: adw::ToastOverlay,
    current_view: Rc<RefCell<View>>,
    // Command center
    command_center: CommandCenter,
    command_center_split: adw::OverlaySplitView,
    command_center_btn: gtk::ToggleButton,
    // Details panel
    details_panel: PackageDetailsPanel,
    details_split: adw::OverlaySplitView,
    // Progress overlay
    progress_overlay: gtk::Box,
    progress_bar: gtk::ProgressBar,
    progress_label: gtk::Label,
    // Selection action bar
    selection_bar: gtk::ActionBar,
    selected_count_label: gtk::Label,
    // Operation history for undo
    operation_history: Rc<RefCell<OperationHistory>>,
    #[allow(dead_code)] // Used via clone in setup_signals
    undo_button: gtk::Button,
}

impl LinGetWindow {
    fn populate_list_store(
        list_view: &gtk::ListView,
        store: &gio::ListStore,
        packages: &[Package],
    ) {
        const CHUNK_SIZE: usize = 250;

        unsafe {
            if let Some(prev) = list_view.steal_data::<ActiveSource>("populate_source") {
                cancel_active_source(&prev);
            }
        }

        store.remove_all();
        if packages.is_empty() {
            return;
        }

        let store = store.clone();
        let packages: Vec<Package> = packages.to_vec();
        let index = Rc::new(RefCell::new(0usize));
        let active_source: ActiveSource = Rc::new(RefCell::new(None));
        let active_source_for_callback = active_source.clone();

        let source_id = glib::idle_add_local(move || {
            let mut start = *index.borrow();
            let end = (start + CHUNK_SIZE).min(packages.len());
            while start < end {
                store.append(&glib::BoxedAnyObject::new(packages[start].clone()));
                start += 1;
            }
            *index.borrow_mut() = start;
            if start >= packages.len() {
                *active_source_for_callback.borrow_mut() = None;
                glib::ControlFlow::Break
            } else {
                glib::ControlFlow::Continue
            }
        });

        *active_source.borrow_mut() = Some(source_id);

        unsafe {
            list_view.set_data("populate_source", active_source);
        }
    }

    pub fn new(app: &adw::Application) -> Self {
        let config = Rc::new(RefCell::new(Config::load()));
        let manager = PackageManager::new();
        let available_sources = Rc::new(RefCell::new(manager.available_sources()));
        let package_manager = Arc::new(Mutex::new(manager));

        let enabled_from_config = config.borrow().enabled_sources.to_sources();
        let enabled_sources = Rc::new(RefCell::new(
            enabled_from_config
                .into_iter()
                .filter(|s| available_sources.borrow().contains(s))
                .collect::<HashSet<_>>(),
        ));
        {
            let sources = enabled_sources.borrow().clone();
            let pm = package_manager.clone();
            glib::spawn_future_local(async move {
                pm.lock().await.set_enabled_sources(sources);
            });
        }
        let packages: Rc<RefCell<Vec<Package>>> = Rc::new(RefCell::new(Vec::new()));
        let discover_rows: Rc<RefCell<Vec<PackageRow>>> = Rc::new(RefCell::new(Vec::new()));
        let current_view = Rc::new(RefCell::new(View::Library));
        // Load saved source filter from config
        let initial_source_filter = config
            .borrow()
            .last_source_filter
            .as_ref()
            .and_then(|s| PackageSource::from_str(s));
        let filter_state = Rc::new(RefCell::new(FilterState {
            source: initial_source_filter,
            search_query: String::new(),
        }));
        let selection_mode = Rc::new(RefCell::new(false));
        let selected_ids: Rc<RefCell<HashSet<String>>> = Rc::new(RefCell::new(HashSet::new()));
        let last_filtered_all: Rc<RefCell<Vec<Package>>> = Rc::new(RefCell::new(Vec::new()));
        let last_filtered_updates: Rc<RefCell<Vec<Package>>> = Rc::new(RefCell::new(Vec::new()));

        let header = Header::new();

        let sidebar = Sidebar::new();
        let content = ContentArea::new();

        let progress = ProgressOverlay::new();
        let progress_overlay = progress.widget;
        let progress_bar = progress.progress_bar;
        let progress_label = progress.label;

        let selection = SelectionBar::new();
        let selection_bar = selection.widget;
        let select_all_btn = selection.select_all_btn;
        let deselect_all_btn = selection.deselect_all_btn;
        let update_selected_btn = selection.update_selected_btn;
        let remove_selected_btn = selection.remove_selected_btn;
        let selected_count_label = selection.count_label;

        // Loading view
        let spinner = gtk::Spinner::builder().visible(false).build();

        let loading_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(16)
            .valign(gtk::Align::Center)
            .halign(gtk::Align::Center)
            .vexpand(true)
            .hexpand(true)
            .margin_start(24)
            .margin_end(24)
            .build();

        let loading_label = gtk::Label::builder()
            .label("Loading your packages…")
            .wrap(true)
            .justify(gtk::Justification::Center)
            .build();
        loading_label.add_css_class("title-2");
        loading_label.add_css_class("dim-label");

        let skeleton_list = gtk::ListBox::builder()
            .selection_mode(gtk::SelectionMode::None)
            .css_classes(vec!["boxed-list", "skeleton-list"])
            .build();
        for _ in 0..10 {
            let row = gtk::ListBoxRow::new();
            row.add_css_class("skeleton-row");
            let r = gtk::Box::builder()
                .orientation(gtk::Orientation::Horizontal)
                .spacing(12)
                .margin_top(12)
                .margin_bottom(12)
                .margin_start(16)
                .margin_end(16)
                .build();
            let icon = gtk::Box::builder()
                .width_request(36)
                .height_request(36)
                .build();
            icon.add_css_class("skeleton-block");
            let lines = gtk::Box::builder()
                .orientation(gtk::Orientation::Vertical)
                .spacing(8)
                .hexpand(true)
                .build();
            let line1 = gtk::Box::builder().height_request(12).hexpand(true).build();
            line1.add_css_class("skeleton-block");
            let line2 = gtk::Box::builder().height_request(10).hexpand(true).build();
            line2.add_css_class("skeleton-block");
            lines.append(&line1);
            lines.append(&line2);
            r.append(&icon);
            r.append(&lines);
            row.set_child(Some(&r));
            skeleton_list.append(&row);
        }

        loading_box.append(&spinner);
        loading_box.append(&loading_label);
        loading_box.append(
            &adw::Clamp::builder()
                .maximum_size(1600)
                .tightening_threshold(1200)
                .margin_start(24)
                .margin_end(24)
                .child(&skeleton_list)
                .build(),
        );

        // Main Stack
        let main_stack = gtk::Stack::builder()
            .transition_type(gtk::StackTransitionType::Crossfade)
            .transition_duration(150)
            .build();

        // Assemble main content
        let content_with_bars = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();

        content_with_bars.append(&content.widget);
        content_with_bars.append(&selection_bar);

        main_stack.add_named(&content_with_bars, Some("content"));
        main_stack.add_named(&loading_box, Some("loading"));

        // Overlay
        let overlay = gtk::Overlay::new();
        overlay.set_child(Some(&main_stack));
        overlay.add_overlay(&progress_overlay);

        // Toast Overlay
        let toast_overlay = adw::ToastOverlay::new();
        toast_overlay.set_child(Some(&overlay));

        // Details Panel (right-side slide-in)
        let details_panel = PackageDetailsPanel::new();
        let details_split = adw::OverlaySplitView::builder()
            .content(&toast_overlay)
            .sidebar(details_panel.widget())
            .sidebar_position(gtk::PackType::End)
            .collapsed(true)
            .show_sidebar(false)
            .enable_hide_gesture(true)
            .enable_show_gesture(false)
            .build();

        // Command Center (right-side panel)
        let command_center = CommandCenter::new();
        command_center.attach_badge(header.command_center_badge.clone());
        let command_center_widget = command_center.widget();
        let command_center_split = adw::OverlaySplitView::builder()
            .content(&details_split)
            .sidebar(&command_center_widget)
            .sidebar_position(gtk::PackType::End)
            .show_sidebar(false)
            .build();
        command_center_split.set_hexpand(true);

        // Main Layout
        let main_paned = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .vexpand(true)
            .build();

        main_paned.append(&sidebar.widget);
        main_paned.append(&gtk::Separator::new(gtk::Orientation::Vertical));
        main_paned.append(&command_center_split);

        let toolbar_view = adw::ToolbarView::new();
        toolbar_view.add_top_bar(&header.widget);
        toolbar_view.set_content(Some(&main_paned));

        let window = adw::ApplicationWindow::builder()
            .application(app)
            .title("LinGet")
            .icon_name("io.github.linget")
            .default_width(1000)
            .default_height(700)
            .width_request(360)
            .height_request(294)
            .content(&toolbar_view)
            .build();

        if let Ok(startup_id) = std::env::var("DESKTOP_STARTUP_ID") {
            window.set_startup_id(&startup_id);
        }

        if config.borrow().window_maximized {
            window.maximize();
        }

        let win = Self {
            window,
            package_manager,
            available_sources,
            enabled_sources,
            packages,
            config,
            filter_state,
            selection_mode,
            sidebar,
            content,
            discover_rows,
            last_filtered_all,
            last_filtered_updates,
            selected_ids,
            search_entry: header.search_entry.clone(),
            main_stack,
            spinner,
            toast_overlay,
            current_view,
            command_center,
            command_center_split: command_center_split.clone(),
            command_center_btn: header.command_center_btn.clone(),
            details_panel,
            details_split: details_split.clone(),
            progress_overlay,
            progress_bar,
            progress_label,
            selection_bar,
            selected_count_label,
            operation_history: Rc::new(RefCell::new(OperationHistory::load())),
            undo_button: header.undo_button.clone(),
        };

        let reload_packages = win.setup_signals(
            header.refresh_button.clone(),
            header.undo_button.clone(),
            header.select_button.clone(),
            win.sidebar.navigation.nav_list.clone(),
            win.content.updates.update_all_btn.clone(),
            win.content.discover.stack.clone(),
            win.content.library.stack.clone(),
            win.content.updates.stack.clone(),
            win.content.favorites.stack.clone(),
            select_all_btn,
            deselect_all_btn,
            update_selected_btn,
            remove_selected_btn,
            win.content.sort_dropdown.clone(),
        );
        win.setup_actions(app, reload_packages.clone());

        // Track window visibility for tray state
        win.window
            .connect_notify_local(Some("visible"), |window, _| {
                let visible = window.is_visible();
                with_tray(|tray| {
                    tray.state.set_window_visible(visible);
                });
            });

        let window_for_tray = win.window.clone();
        let app_for_tray = app.clone();
        let reload_for_tray = reload_packages.clone();
        glib::timeout_add_local(Duration::from_millis(250), move || {
            with_tray(|tray| {
                while let Ok(action) = tray.action_receiver.try_recv() {
                    match action {
                        TrayAction::ShowWindow => {
                            if window_for_tray.is_visible() {
                                window_for_tray.set_visible(false);
                                tray.state.set_window_visible(false);
                            } else {
                                window_for_tray.set_visible(true);
                                window_for_tray.present();
                                tray.state.set_window_visible(true);
                            }
                        }
                        TrayAction::CheckUpdates => {
                            window_for_tray.set_visible(true);
                            window_for_tray.present();
                            tray.state.set_window_visible(true);
                            reload_for_tray();
                        }
                        TrayAction::Quit => {
                            app_for_tray.quit();
                        }
                    }
                }
            });
            glib::ControlFlow::Continue
        });

        if !win.config.borrow().onboarding_completed {
            let onboarding =
                OnboardingWindow::new(app, win.config.clone(), win.package_manager.clone(), {
                    let reload = reload_packages.clone();
                    move || reload()
                });
            onboarding.present();
        }

        win
    }

    fn setup_actions(&self, app: &adw::Application, reload_packages: Rc<dyn Fn()>) {
        // Import action
        let import_action = gio::SimpleAction::new("import", None);
        let window_import = self.window.clone();
        let pm_import = self.package_manager.clone();
        let toast_import = self.toast_overlay.clone();
        let progress_overlay_import = self.progress_overlay.clone();
        let progress_bar_import = self.progress_bar.clone();
        let progress_label_import = self.progress_label.clone();
        let reload_import = reload_packages.clone();

        import_action.connect_activate(move |_, _| {
            let dialog = gtk::FileChooserDialog::builder()
                .title("Import Packages")
                .action(gtk::FileChooserAction::Open)
                .modal(true)
                .transient_for(&window_import)
                .build();

            dialog.add_button("Cancel", gtk::ResponseType::Cancel);
            dialog.add_button("Import", gtk::ResponseType::Accept);

            let filter = gtk::FileFilter::new();
            filter.set_name(Some("JSON Files"));
            filter.add_pattern("*.json");
            dialog.add_filter(&filter);

            let pm = pm_import.clone();
            let toast = toast_import.clone();
            let progress_overlay = progress_overlay_import.clone();
            let progress_bar = progress_bar_import.clone();
            let progress_label = progress_label_import.clone();
            let window_parent = window_import.clone();
            let reload_import = reload_import.clone();

            dialog.connect_response(move |d, response| {
                if response == gtk::ResponseType::Accept {
                    if let Some(file) = d.file() {
                        if let Some(path) = file.path() {
                            if let Ok(content) = std::fs::read_to_string(path) {
                                if let Ok(list) = serde_json::from_str::<crate::models::PackageList>(&content) {
                                    d.close();

                                    // Start import process
                                    let packages_to_install = list.packages.clone();
                                    let total = packages_to_install.len();
                                    if total == 0 {
                                        let t = adw::Toast::new("No packages found in backup file");
                                        toast.add_toast(t);
                                        return;
                                    }

                                    let preview: String = packages_to_install
                                        .iter()
                                        .take(10)
                                        .map(|p| format!("• {} ({})", p.name, p.source))
                                        .collect::<Vec<_>>()
                                        .join("\n");

                                    let body = format!(
                                        "This will install {} packages using their respective package managers.\n\n{}\n\nYou may be prompted for admin privileges.",
                                        total,
                                        preview
                                    );

                                    let confirm = gtk::MessageDialog::builder()
                                        .transient_for(&window_parent)
                                        .modal(true)
                                        .message_type(gtk::MessageType::Question)
                                        .text("Install imported packages?")
                                        .secondary_text(&body)
                                        .build();

                                    confirm.add_button("Cancel", gtk::ResponseType::Cancel);
                                    confirm.add_button("Install", gtk::ResponseType::Accept);

                                    let pm = pm.clone();
                                    let toast = toast.clone();
                                    let progress_overlay = progress_overlay.clone();
                                    let progress_bar = progress_bar.clone();
                                    let progress_label = progress_label.clone();
                                    let reload = reload_import.clone();

                                    confirm.connect_response(move |dlg: &gtk::MessageDialog, resp| {
                                        dlg.close();
                                        if resp != gtk::ResponseType::Accept {
                                            return;
                                        }

                                        progress_overlay.set_visible(true);
                                        progress_bar.set_fraction(0.0);
                                        progress_label.set_label(&format!("Importing {} packages...", total));

                                        let pm = pm.clone();
                                        let toast = toast.clone();
                                        let progress_overlay = progress_overlay.clone();
                                        let progress_bar = progress_bar.clone();
                                        let packages_to_install = packages_to_install.clone();
                                        let reload = reload.clone();

                                        glib::spawn_future_local(async move {
                                            let mut success = 0;
                                            let mut failed = 0;

                                            let manager = pm.lock().await;
                                            for (i, backup_pkg) in packages_to_install.iter().enumerate() {
                                                progress_bar.set_fraction(((i + 1) as f64) / (total as f64));
                                                progress_bar.set_text(Some(&format!(
                                                    "{}/{} - {}",
                                                    i + 1,
                                                    total,
                                                    backup_pkg.name
                                                )));

                                                let pkg = crate::models::Package {
                                                    name: backup_pkg.name.clone(),
                                                    version: String::new(),
                                                    available_version: None,
                                                    description: String::new(),
                                                    source: backup_pkg.source,
                                                    status: crate::models::PackageStatus::NotInstalled,
                                                    size: None,
                                                    homepage: None,
                                                    license: None,
                                                    maintainer: None,
                                                    dependencies: Vec::new(),
                                                    install_date: None,
                                                    enrichment: None,
                                                };

                                                if manager.install(&pkg).await.is_ok() {
                                                    success += 1;
                                                } else {
                                                    failed += 1;
                                                }
                                            }
                                            drop(manager);

                                            progress_overlay.set_visible(false);
                                            let msg = format!(
                                                "Import complete: {} installed, {} failed",
                                                success, failed
                                            );
                                            let t = adw::Toast::new(&msg);
                                            t.set_timeout(5);
                                            toast.add_toast(t);
                                            reload();
                                        });
                                    });

                                    confirm.show();
                                    return;
                                }
                            }
                        }
                    }
                }
                d.close();
            });

            dialog.show();
        });
        app.add_action(&import_action);

        // Export action
        let export_action = gio::SimpleAction::new("export", None);
        let window_export = self.window.clone();
        let packages_export = self.packages.clone();
        let toast_export = self.toast_overlay.clone();

        export_action.connect_activate(move |_, _| {
            let dialog = gtk::FileChooserDialog::builder()
                .title("Export Packages")
                .action(gtk::FileChooserAction::Save)
                .modal(true)
                .transient_for(&window_export)
                .build();

            dialog.add_button("Cancel", gtk::ResponseType::Cancel);
            dialog.add_button("Export", gtk::ResponseType::Accept);
            dialog.set_current_name("linget-backup.json");

            let packages = packages_export.clone();
            let toast = toast_export.clone();

            dialog.connect_response(move |d, response| {
                if response == gtk::ResponseType::Accept {
                    if let Some(file) = d.file() {
                        if let Some(path) = file.path() {
                            let pkgs = packages.borrow();
                            let list = crate::models::PackageList::new(&pkgs);

                            match serde_json::to_string_pretty(&list) {
                                Ok(json) => {
                                    if let Err(e) = std::fs::write(path, json) {
                                        let t = adw::Toast::new(&format!("Export failed: {}", e));
                                        toast.add_toast(t);
                                    } else {
                                        let t = adw::Toast::new("Packages exported successfully");
                                        toast.add_toast(t);
                                    }
                                }
                                Err(e) => {
                                    let t =
                                        adw::Toast::new(&format!("Serialization failed: {}", e));
                                    toast.add_toast(t);
                                }
                            }
                        }
                    }
                }
                d.close();
            });

            dialog.show();
        });
        app.add_action(&export_action);

        let prefs_action = gio::SimpleAction::new("preferences", None);
        let config = self.config.clone();
        let pm = self.package_manager.clone();
        let enabled_sources = self.enabled_sources.clone();
        let available_sources = self.available_sources.clone();
        let window = self.window.clone();
        let reload = reload_packages.clone();
        prefs_action.connect_activate(move |_, _| {
            PreferencesDialog::show(
                config.clone(),
                pm.clone(),
                enabled_sources.clone(),
                available_sources.clone(),
                reload.clone(),
                &window,
            )
        });
        app.add_action(&prefs_action);

        let diagnostics_action = gio::SimpleAction::new("diagnostics", None);
        let window_diag = self.window.clone();
        let config_diag = self.config.clone();
        let enabled_sources_diag = self.enabled_sources.clone();
        let available_sources_diag = self.available_sources.clone();
        diagnostics_action.connect_activate(move |_, _| {
            DiagnosticsDialog::show(
                config_diag.clone(),
                enabled_sources_diag.clone(),
                available_sources_diag.clone(),
                &window_diag,
            )
        });
        app.add_action(&diagnostics_action);

        let about_action = gio::SimpleAction::new("about", None);
        let window = self.window.clone();
        about_action.connect_activate(move |_, _| show_about_dialog(&window));
        app.add_action(&about_action);

        let shortcuts_action = gio::SimpleAction::new("shortcuts", None);
        let window_shortcuts = self.window.clone();
        shortcuts_action
            .connect_activate(move |_, _| Self::show_shortcuts_dialog(&window_shortcuts));
        app.add_action(&shortcuts_action);

        let quit_action = gio::SimpleAction::new("quit", None);
        let app_clone = app.clone();
        quit_action.connect_activate(move |_, _| app_clone.quit());
        app.add_action(&quit_action);

        app.set_accels_for_action("app.quit", &["<Ctrl>q"]);
        app.set_accels_for_action("app.preferences", &["<Ctrl>comma"]);
        app.set_accels_for_action("app.diagnostics", &["<Ctrl>d"]);
        app.set_accels_for_action("app.shortcuts", &["<Ctrl>question", "F1"]);
    }

    fn show_shortcuts_dialog(window: &adw::ApplicationWindow) {
        let dialog = gtk::ShortcutsWindow::builder()
            .transient_for(window)
            .modal(true)
            .build();
        let section = gtk::ShortcutsSection::builder()
            .section_name("shortcuts")
            .visible(true)
            .build();
        let group = gtk::ShortcutsGroup::builder().title("General").build();

        for (title, accel) in [
            ("Search", "<Ctrl>f"),
            ("Quick Search", "slash"),
            ("Refresh", "<Ctrl>r"),
            ("Selection Mode", "<Ctrl>s"),
            ("Open Details", "Return"),
            ("Update Selected", "u"),
            ("Remove Selected", "Delete"),
            ("Preferences", "<Ctrl>comma"),
            ("Cancel / Clear", "Escape"),
            ("Quit", "<Ctrl>q"),
        ] {
            group.append(
                &gtk::ShortcutsShortcut::builder()
                    .title(title)
                    .accelerator(accel)
                    .build(),
            );
        }
        section.append(&group);
        dialog.set_child(Some(&section));
        dialog.present();
    }

    #[allow(clippy::too_many_arguments)]
    fn setup_signals(
        &self,
        refresh_button: gtk::Button,
        undo_button: gtk::Button,
        select_button: gtk::ToggleButton,
        nav_list: gtk::ListBox,
        update_all_btn: gtk::Button,
        discover_stack: gtk::Stack,
        all_stack: gtk::Stack,
        updates_stack: gtk::Stack,
        favorites_stack: gtk::Stack,
        select_all_btn: gtk::Button,
        deselect_all_btn: gtk::Button,
        update_selected_btn: gtk::Button,
        remove_selected_btn: gtk::Button,
        sort_dropdown: gtk::DropDown,
    ) -> Rc<dyn Fn()> {
        let config = self.config.clone();
        let pm = self.package_manager.clone();
        let enabled_sources = self.enabled_sources.clone();
        let available_sources = self.available_sources.clone();
        let packages = self.packages.clone();
        let discover_rows = self.discover_rows.clone();
        let discover_list_box = self.content.discover.list_box.clone();
        let all_list_view = self.content.library.list_view.clone();
        let all_store = self.content.library.store.clone();
        let updates_list_view = self.content.updates.list_view.clone();
        let updates_store = self.content.updates.store.clone();
        let favorites_list_view = self.content.favorites.list_view.clone();
        let favorites_store = self.content.favorites.store.clone();
        let favorites_count_label = self.sidebar.navigation.favorites_count_label.clone();
        let last_filtered_all = self.last_filtered_all.clone();
        let last_filtered_updates = self.last_filtered_updates.clone();
        let selected_ids = self.selected_ids.clone();
        let main_stack = self.main_stack.clone();
        let spinner = self.spinner.clone();
        let toast_overlay = self.toast_overlay.clone();
        let content_stack = self.content.content_stack.clone();
        let current_view = self.current_view.clone();
        let search_entry = self.search_entry.clone();
        let filter_state = self.filter_state.clone();
        let selection_mode = self.selection_mode.clone();
        let all_count_label = self.sidebar.navigation.all_count_label.clone();
        let update_count_label = self.sidebar.navigation.update_count_label.clone();
        let total_size_label = self.sidebar.total_size_label.clone();
        let source_count_labels = self.sidebar.providers.provider_counts.clone();
        let provider_rows = self.sidebar.providers.provider_rows.clone();
        let providers_box = self.sidebar.providers.providers_box.clone();
        let toolbar_source_filters = vec![
            self.content.discover.source_filter.clone(),
            self.content.library.source_filter.clone(),
            self.content.updates.source_filter.clone(),
        ];
        let toolbar_search_chips = vec![
            self.content.discover.search_chip.clone(),
            self.content.library.search_chip.clone(),
            self.content.updates.search_chip.clone(),
        ];
        let command_center = self.command_center.clone();
        let command_center_split = self.command_center_split.clone();
        let command_center_btn = self.command_center_btn.clone();
        let details_panel = self.details_panel.clone();
        let details_split = self.details_split.clone();
        let progress_overlay = self.progress_overlay.clone();
        let progress_bar = self.progress_bar.clone();
        let progress_label = self.progress_label.clone();
        let selection_bar = self.selection_bar.clone();
        let selected_count_label = self.selected_count_label.clone();
        let enable_detected_btn = self.sidebar.providers.enable_detected_btn.clone();
        let view_spinners: Rc<Vec<gtk::Spinner>> = Rc::new(vec![
            self.content.discover.spinner.clone(),
            self.content.library.spinner.clone(),
            self.content.updates.spinner.clone(),
        ]);
        let operation_history = self.operation_history.clone();

        let reveal_command_center: Rc<dyn Fn(bool)> = Rc::new({
            let command_center_split = command_center_split.clone();
            let command_center_btn = command_center_btn.clone();
            let command_center = command_center.clone();
            move |reveal| {
                command_center_split.set_show_sidebar(reveal);
                command_center_btn.set_active(reveal);
                if reveal {
                    command_center.mark_read();
                }
            }
        });

        command_center_btn.connect_toggled({
            let command_center_split = command_center_split.clone();
            let command_center = command_center.clone();
            move |btn| {
                let reveal = btn.is_active();
                command_center_split.set_show_sidebar(reveal);
                if reveal {
                    command_center.mark_read();
                }
            }
        });

        let close_details_panel: Rc<dyn Fn()> = Rc::new({
            let details_split = details_split.clone();
            let details_panel = details_panel.clone();
            move || {
                details_split.set_show_sidebar(false);
                details_panel.clear();
            }
        });

        details_panel.set_on_close({
            let close_details_panel = close_details_panel.clone();
            move || close_details_panel()
        });

        let show_details_panel_holder: ShowDetailsFnHolder = Rc::new(RefCell::new(None));

        // Helper to update undo button state
        let update_undo_button: Rc<dyn Fn()> = Rc::new({
            let operation_history = operation_history.clone();
            let undo_button = undo_button.clone();
            move || {
                let history = operation_history.borrow();
                if let Some(record) = history.last_undoable() {
                    let label = format!("Undo: {}", record.description());
                    undo_button.set_tooltip_text(Some(&label));
                    undo_button.set_sensitive(true);
                    undo_button.set_visible(true);
                } else {
                    undo_button.set_sensitive(false);
                    undo_button.set_visible(false);
                }
            }
        });

        // Initial undo button state
        update_undo_button();

        let update_top_chips: Rc<dyn Fn()> = Rc::new({
            let filter_state = filter_state.clone();
            let search_entry = search_entry.clone();
            let toolbar_source_filters = toolbar_source_filters.clone();
            let toolbar_search_chips = toolbar_search_chips.clone();

            move || {
                let source = filter_state.borrow().source;
                let query = search_entry.text().trim().to_string();

                let sources_label = match source {
                    None => "Source: All".to_string(),
                    Some(s) => format!("Source: {}", s),
                };

                let mut query_label = query.clone();
                if query_label.chars().count() > 30 {
                    query_label = format!("{}…", query_label.chars().take(29).collect::<String>());
                }
                let search_label = if query.is_empty() {
                    None
                } else {
                    Some(format!("Search: {}", query_label))
                };

                for f in &toolbar_source_filters {
                    f.menu_btn.set_label(&sources_label);
                    f.menu_btn.set_tooltip_text(Some(&sources_label));
                }

                for b in &toolbar_search_chips {
                    if let Some(ref label) = search_label {
                        b.set_label(label);
                        b.set_tooltip_text(Some(&query));
                        b.set_visible(true);
                    } else {
                        b.set_visible(false);
                    }
                }
            }
        });

        let reload_holder: LocalFnHolder = Rc::new(RefCell::new(None));

        *show_details_panel_holder.borrow_mut() = Some(Rc::new({
            let details_split = details_split.clone();
            let details_panel = details_panel.clone();
            let pm = pm.clone();
            let toast_overlay = toast_overlay.clone();
            let config = config.clone();
            let reload_holder = reload_holder.clone();
            let command_center = command_center.clone();
            let reveal_command_center = reveal_command_center.clone();
            let close_details_panel = close_details_panel.clone();
            move |pkg: Package| {
                details_panel.show_package(
                    &pkg,
                    pm.clone(),
                    toast_overlay.clone(),
                    config.clone(),
                    reload_holder.borrow().clone(),
                    Some(command_center.clone()),
                    Some(reveal_command_center.clone()),
                    close_details_panel.clone(),
                );
                details_split.set_show_sidebar(true);
            }
        }));

        // Command Center retry handler
        command_center.set_retry_handler(Rc::new({
            let pm = pm.clone();
            let toast_overlay = toast_overlay.clone();
            let progress_overlay = progress_overlay.clone();
            let progress_bar = progress_bar.clone();
            let progress_label = progress_label.clone();
            let reload_holder = reload_holder.clone();
            let command_center = command_center.clone();
            let reveal_command_center = reveal_command_center.clone();

            move |spec: RetrySpec| {
                let pm = pm.clone();
                let toast = toast_overlay.clone();
                let progress_overlay = progress_overlay.clone();
                let progress_bar = progress_bar.clone();
                let progress_label = progress_label.clone();
                let reload_holder = reload_holder.clone();
                let command_center = command_center.clone();
                let reveal_command_center = reveal_command_center.clone();

                glib::spawn_future_local(async move {
                    match spec.clone() {
                        RetrySpec::Package { package, op } => {
                            let package = *package;
                            let title = match &op {
                                PackageOp::Install => format!("Retrying install: {}", package.name),
                                PackageOp::Update => format!("Retrying update: {}", package.name),
                                PackageOp::Remove => format!("Retrying remove: {}", package.name),
                                PackageOp::Downgrade => {
                                    format!("Retrying downgrade: {}", package.name)
                                }
                                PackageOp::DowngradeTo(v) => {
                                    format!("Retrying downgrade: {} → {}", package.name, v)
                                }
                            };
                            let task = command_center.begin_task(
                                &title,
                                format!("Source: {}", package.source),
                                Some(spec.clone()),
                            );

                            let result = {
                                let manager = pm.lock().await;
                                match &op {
                                    PackageOp::Install => manager.install(&package).await,
                                    PackageOp::Update => manager.update(&package).await,
                                    PackageOp::Remove => manager.remove(&package).await,
                                    PackageOp::Downgrade => manager.downgrade(&package).await,
                                    PackageOp::DowngradeTo(v) => manager.downgrade_to(&package, v).await,
                                }
                            };

                            match result {
                                Ok(_) => {
                                    let done_title = match &op {
                                        PackageOp::Install => format!("Installed {}", package.name),
                                        PackageOp::Update => format!("Updated {}", package.name),
                                        PackageOp::Remove => format!("Removed {}", package.name),
                                        PackageOp::Downgrade => format!("Downgraded {}", package.name),
                                        PackageOp::DowngradeTo(v) => format!("Downgraded {} to {}", package.name, v),
                                    };
                                    task.finish(
                                        CommandEventKind::Success,
                                        done_title,
                                        format!("Source: {}", package.source),
                                        None,
                                        true,
                                    );
                                    if let Some(reload) = reload_holder.borrow().as_ref() {
                                        reload();
                                    }
                                }
                                Err(e) => {
                                    let raw = format!("Error: {}", e);
                                    let (title, details, command) =
                                        if let Some((details, command)) = parse_suggestion(&raw) {
                                            ("Action required".to_string(), details, Some(command))
                                        } else {
                                            ("Retry failed".to_string(), raw, None)
                                        };
                                    task.finish(CommandEventKind::Error, title, details, command, true);
                                    reveal_command_center(true);
                                    let t = adw::Toast::new("Retry failed (see Command Center)");
                                    t.set_timeout(5);
                                    toast.add_toast(t);
                                }
                            }
                        }
                        RetrySpec::BulkUpdate { packages } => {
                            if packages.is_empty() {
                                return;
                            }
                            let total = packages.len();
                            let task = command_center.begin_task(
                                "Retrying bulk update",
                                format!("{} packages", total),
                                Some(spec.clone()),
                            );

                            progress_overlay.set_visible(true);
                            progress_bar.set_fraction(0.0);
                            progress_label.set_label(&format!("Updating {} packages...", total));

                            let mut success = 0usize;
                            let mut blocked_snaps: Vec<String> = Vec::new();
                            let manager = pm.lock().await;
                            for (i, pkg) in packages.iter().enumerate() {
                                progress_bar.set_fraction((i as f64) / (total as f64));
                                progress_bar.set_text(Some(&format!("{}/{} - {}", i + 1, total, pkg.name)));
                                match manager.update(pkg).await {
                                    Ok(_) => success += 1,
                                    Err(e) => {
                                        let msg = e.to_string();
                                        if pkg.source == PackageSource::Snap
                                            && msg.contains("because it is running")
                                        {
                                            blocked_snaps.push(pkg.name.clone());
                                        }
                                    }
                                }
                            }
                            drop(manager);

                            progress_overlay.set_visible(false);

                            let base = format!("Updated {}/{} packages", success, total);
                            let msg = if blocked_snaps.is_empty() {
                                base
                            } else {
                                blocked_snaps.sort();
                                blocked_snaps.dedup();
                                let shown = blocked_snaps
                                    .iter()
                                    .take(3)
                                    .cloned()
                                    .collect::<Vec<_>>()
                                    .join(", ");
                                let suffix = if blocked_snaps.len() > 3 { ", …" } else { "" };
                                format!(
                                    "{base}. Blocked snaps: {shown}{suffix} (close running apps and retry)."
                                )
                            };

                            let kind = if success == total && blocked_snaps.is_empty() {
                                CommandEventKind::Success
                            } else {
                                CommandEventKind::Info
                            };
                            task.finish(kind, "Bulk update finished", msg, None, true);
                            if kind != CommandEventKind::Success {
                                reveal_command_center(true);
                            }
                            if let Some(reload) = reload_holder.borrow().as_ref() {
                                reload();
                            }
                        }
                        RetrySpec::BulkRemove { packages } => {
                            if packages.is_empty() {
                                return;
                            }
                            let total = packages.len();
                            let task = command_center.begin_task(
                                "Retrying bulk remove",
                                format!("{} packages", total),
                                Some(spec.clone()),
                            );

                            progress_overlay.set_visible(true);
                            progress_bar.set_fraction(0.0);
                            progress_label.set_label(&format!("Removing {} packages...", total));

                            let mut success = 0usize;
                            let manager = pm.lock().await;
                            for (i, pkg) in packages.iter().enumerate() {
                                progress_bar.set_fraction((i as f64) / (total as f64));
                                progress_bar.set_text(Some(&format!("{}/{} - {}", i + 1, total, pkg.name)));
                                if manager.remove(pkg).await.is_ok() {
                                    success += 1;
                                }
                            }
                            drop(manager);

                            progress_overlay.set_visible(false);

                            let msg = format!("Removed {}/{} packages", success, total);
                            let kind = if success == total {
                                CommandEventKind::Success
                            } else {
                                CommandEventKind::Info
                            };
                            task.finish(kind, "Bulk remove finished", msg, None, true);
                            if kind != CommandEventKind::Success {
                                reveal_command_center(true);
                            }
                            if let Some(reload) = reload_holder.borrow().as_ref() {
                                reload();
                            }
                        }
                    }
                });
            }
        }));

        // Helpers
        let enabled_sources_for_counts = enabled_sources.clone();
        let provider_rows_for_counts = provider_rows.clone();
        let update_source_counts = move |packages: &[Package]| {
            let enabled = enabled_sources_for_counts.borrow();
            let enabled_packages: Vec<&Package> = packages
                .iter()
                .filter(|p| enabled.contains(&p.source))
                .collect();

            let total = enabled_packages.len();
            let updates = enabled_packages.iter().filter(|p| p.has_update()).count();

            all_count_label.set_label(&format!("{}", total));
            if updates > 0 {
                update_count_label.set_label(&format!("{}", updates));
                update_count_label.set_visible(true);
            } else {
                update_count_label.set_visible(false);
            }

            // Update system tray with update count
            with_tray(|tray| {
                tray.state.set_updates_count(updates as u32);
            });

            for (source, label) in &source_count_labels {
                let count: usize = enabled_packages
                    .iter()
                    .filter(|p| p.source == *source)
                    .count();
                label.set_label(&count.to_string());
            }

            // Update disk space per source
            for (source, row_widgets) in &provider_rows_for_counts {
                let source_size: u64 = packages
                    .iter()
                    .filter(|p| p.source == *source)
                    .filter_map(|p| p.size)
                    .sum();
                if source_size > 0 {
                    row_widgets
                        .size_label
                        .set_label(&format_size(source_size, BINARY));
                    row_widgets.size_label.set_visible(true);
                } else {
                    row_widgets.size_label.set_visible(false);
                }
            }

            // Update total disk space used
            let total_disk_size: u64 = enabled_packages.iter().filter_map(|p| p.size).sum();
            if total_disk_size > 0 {
                total_size_label
                    .set_label(&format!("Disk: {}", format_size(total_disk_size, BINARY)));
                total_size_label.set_visible(true);
            } else {
                total_size_label.set_visible(false);
            }
        };

        let skip_filter = Rc::new(RefCell::new(false));
        let apply_filters_holder: LocalFnHolder = Rc::new(RefCell::new(None));
        let on_source_click_holder: SourceClickHolder = Rc::new(RefCell::new(None));

        // ListView factories (All + Updates) - set up once.
        {
            let make_factory = |list_view: &gtk::ListView| {
                let factory = gtk::SignalListItemFactory::new();

                factory.connect_setup({
                    let pm = pm.clone();
                    let toast_overlay = toast_overlay.clone();
                    let config = config.clone();
                    let selection_mode = selection_mode.clone();
                    let selected_ids = selected_ids.clone();
                    let selected_count_label = selected_count_label.clone();
                    let reload_holder = reload_holder.clone();
                    let command_center = command_center.clone();
                    let reveal_command_center = reveal_command_center.clone();
                    let on_source_click_holder = on_source_click_holder.clone();
                    let apply_filters_holder = apply_filters_holder.clone();
                    let show_details_panel_holder = show_details_panel_holder.clone();
                    move |_, item| {
                        let placeholder = Package {
                            name: "".to_string(),
                            version: "".to_string(),
                            available_version: None,
                            description: "".to_string(),
                            source: PackageSource::Apt,
                            status: PackageStatus::Installed,
                            size: None,
                            homepage: None,
                            license: None,
                            maintainer: None,
                            dependencies: Vec::new(),
                            install_date: None,
                            enrichment: None,
                        };

                        let row = PackageRow::new(placeholder, None, true);
                        let wrapper = gtk::Box::builder()
                            .orientation(gtk::Orientation::Vertical)
                            .hexpand(true)
                            .css_classes(vec!["boxed-row"])
                            .build();
                        wrapper.append(&row.widget);
                        item.set_child(Some(&wrapper));

                        let skip_check = Rc::new(RefCell::new(false));

                        let click_gesture = gtk::GestureClick::new();
                        click_gesture.connect_released({
                            let row_pkg = row.package.clone();
                            let row_widget = row.widget.clone();
                            let show_details_panel_holder = show_details_panel_holder.clone();
                            move |gesture, _, x, y| {
                                if gesture.current_button() != gtk::gdk::BUTTON_PRIMARY {
                                    return;
                                }
                                let Some(widget) = gesture.widget() else {
                                    return;
                                };
                                let width = widget.width() as f64;
                                let height = widget.height() as f64;
                                if x < 0.0 || y < 0.0 || x > width || y > height {
                                    return;
                                }
                                if let Some(target) = row_widget.pick(x, y, gtk::PickFlags::DEFAULT)
                                {
                                    let mut current: Option<gtk::Widget> = Some(target);
                                    while let Some(w) = current {
                                        if w.downcast_ref::<gtk::Button>().is_some()
                                            || w.downcast_ref::<gtk::ToggleButton>().is_some()
                                            || w.downcast_ref::<gtk::CheckButton>().is_some()
                                        {
                                            return;
                                        }
                                        current = w.parent();
                                    }
                                }
                                let pkg = row_pkg.borrow().clone();
                                if let Some(show_fn) = show_details_panel_holder.borrow().as_ref() {
                                    show_fn(pkg);
                                }
                            }
                        });
                        row.widget.add_controller(click_gesture);

                        // Filter by source
                        row.source_button.connect_clicked({
                            let row_pkg = row.package.clone();
                            let on_source_click_holder = on_source_click_holder.clone();
                            move |_| {
                                if let Some(cb) = on_source_click_holder.borrow().as_ref() {
                                    cb(row_pkg.borrow().source);
                                }
                            }
                        });

                        // Selection toggle
                        row.checkbox.connect_toggled({
                            let row_pkg = row.package.clone();
                            let selected_ids = selected_ids.clone();
                            let selection_mode = selection_mode.clone();
                            let selected_count_label = selected_count_label.clone();
                            let skip_check = skip_check.clone();
                            move |cb| {
                                if !*selection_mode.borrow() || *skip_check.borrow() {
                                    return;
                                }
                                let id = row_pkg.borrow().id();
                                if cb.is_active() {
                                    selected_ids.borrow_mut().insert(id);
                                } else {
                                    selected_ids.borrow_mut().remove(&id);
                                }
                                selected_count_label.set_label(&format!(
                                    "{} selected",
                                    selected_ids.borrow().len()
                                ));
                            }
                        });

                        // Favorite toggle
                        let skip_fav = Rc::new(RefCell::new(false));
                        row.favorite_button.connect_toggled({
                            let row_pkg = row.package.clone();
                            let row_fav_btn = row.favorite_button.clone();
                            let config = config.clone();
                            let skip_fav = skip_fav.clone();
                            let apply_filters_holder = apply_filters_holder.clone();
                            move |btn| {
                                if *skip_fav.borrow() {
                                    return;
                                }
                                let pkg_id = row_pkg.borrow().id();
                                let is_favorite = btn.is_active();

                                // Update button appearance
                                if is_favorite {
                                    row_fav_btn.set_icon_name("starred-symbolic");
                                    row_fav_btn.set_tooltip_text(Some("Remove from favorites"));
                                    row_fav_btn.add_css_class("favorited");
                                } else {
                                    row_fav_btn.set_icon_name("non-starred-symbolic");
                                    row_fav_btn.set_tooltip_text(Some("Add to favorites"));
                                    row_fav_btn.remove_css_class("favorited");
                                }

                                // Update config
                                {
                                    let mut cfg = config.borrow_mut();
                                    if is_favorite {
                                        if !cfg.favorite_packages.contains(&pkg_id) {
                                            cfg.favorite_packages.push(pkg_id);
                                        }
                                    } else {
                                        cfg.favorite_packages
                                            .retain(|id| id != &row_pkg.borrow().id());
                                    }
                                    if let Err(e) = cfg.save() {
                                        tracing::warn!("Failed to save favorites: {}", e);
                                    }
                                }

                                // Refresh favorites view
                                if let Some(apply) = apply_filters_holder.borrow().as_ref() {
                                    apply();
                                }
                            }
                        });

                        unsafe {
                            item.set_data("pkg_skip_fav", skip_fav);
                        }

                        // Action button
                        row.action_button.connect_clicked({
                            let pm = pm.clone();
                            let toast_overlay = toast_overlay.clone();
                            let row_widget = row.widget.clone();
                            let row_progress = row.progress_bar.clone();
                            let row_pkg = row.package.clone();
                            let row_update_icon = row.update_icon.clone();
                            let spinner = row.spinner.clone();
                            let btn = row.action_button.clone();
                            let reload_holder = reload_holder.clone();
                            let command_center = command_center.clone();
                            let reveal_command_center = reveal_command_center.clone();
                            move |_| {
                                let pkg = row_pkg.borrow().clone();
                                btn.set_visible(false);
                                spinner.set_visible(true);
                                spinner.start();

                                let task = command_center.begin_task(
                                    format!("Working on {}", pkg.name),
                                    format!("Source: {}", pkg.source),
                                    Some(RetrySpec::Package {
                                        package: Box::new(pkg.clone()),
                                        op: match pkg.status {
                                            PackageStatus::UpdateAvailable => PackageOp::Update,
                                            PackageStatus::Installed => PackageOp::Remove,
                                            PackageStatus::NotInstalled => PackageOp::Install,
                                            _ => PackageOp::Update,
                                        },
                                    }),
                                );

                                row_widget.set_subtitle("Working…");
                                row_progress.set_fraction(0.0);
                                row_progress.set_visible(true);

                                let progress_bar_pulse = row_progress.clone();
                                let pulser_id = glib::timeout_add_local(
                                    Duration::from_millis(120),
                                    move || {
                                        progress_bar_pulse.pulse();
                                        glib::ControlFlow::Continue
                                    },
                                );

                                glib::spawn_future_local({
                                    let pm = pm.clone();
                                    let toast_overlay = toast_overlay.clone();
                                    let row_widget = row_widget.clone();
                                    let row_progress = row_progress.clone();
                                    let row_pkg = row_pkg.clone();
                                    let row_update_icon = row_update_icon.clone();
                                    let spinner = spinner.clone();
                                    let btn = btn.clone();
                                    let reload_holder = reload_holder.clone();
                                    let reveal_command_center = reveal_command_center.clone();
                                    async move {
                                        let handle = tokio::spawn(async move {
                                            let manager = pm.lock().await;
                                            let result = match pkg.status {
                                                PackageStatus::UpdateAvailable => {
                                                    manager.update(&pkg).await
                                                }
                                                PackageStatus::Installed => {
                                                    manager.remove(&pkg).await
                                                }
                                                PackageStatus::NotInstalled => {
                                                    manager.install(&pkg).await
                                                }
                                                _ => Ok(()),
                                            };
                                            (pkg, result)
                                        });

                                        let (pkg, result) = match handle.await {
                                            Ok(v) => v,
                                            Err(e) => {
                                                remove_source(pulser_id);
                                                row_progress.set_visible(false);
                                                spinner.stop();
                                                spinner.set_visible(false);
                                                btn.set_visible(true);
                                                row_widget.set_subtitle("");
                                                task.finish(
                                                    CommandEventKind::Error,
                                                    "Operation failed",
                                                    format!("Task join error: {}", e),
                                                    None,
                                                    true,
                                                );
                                                reveal_command_center(true);
                                                let t = adw::Toast::new(
                                                    "Operation failed (see Command Center)",
                                                );
                                                t.set_timeout(5);
                                                toast_overlay.add_toast(t);
                                                return;
                                            }
                                        };

                                        remove_source(pulser_id);
                                        row_progress.set_visible(false);
                                        spinner.stop();
                                        spinner.set_visible(false);
                                        btn.set_visible(true);

                                        let ok = result.is_ok();
                                        match result {
                                            Ok(_) => {
                                                task.finish(
                                                    CommandEventKind::Success,
                                                    "Done",
                                                    format!("{}: {}", pkg.source, pkg.name),
                                                    None,
                                                    true,
                                                );
                                                {
                                                    let mut p = row_pkg.borrow_mut();
                                                    p.status = match p.status {
                                                        PackageStatus::UpdateAvailable => {
                                                            PackageStatus::Installed
                                                        }
                                                        PackageStatus::Installed => {
                                                            PackageStatus::NotInstalled
                                                        }
                                                        PackageStatus::NotInstalled => {
                                                            PackageStatus::Installed
                                                        }
                                                        other => other,
                                                    };
                                                    p.available_version = None;
                                                    row_update_icon.set_visible(
                                                        p.status == PackageStatus::UpdateAvailable,
                                                    );
                                                    PackageRow::apply_action_button_style(
                                                        &btn, p.status,
                                                    );
                                                }
                                            }
                                            Err(e) => {
                                                let raw = format!("Error: {}", e);
                                                let (details, command) = parse_suggestion(&raw)
                                                    .map(|(d, c)| (d, Some(c)))
                                                    .unwrap_or((raw, None));
                                                task.finish(
                                                    CommandEventKind::Error,
                                                    "Operation failed",
                                                    details.clone(),
                                                    command.clone(),
                                                    true,
                                                );
                                                reveal_command_center(true);
                                                let t = adw::Toast::new(
                                                    "Operation failed (see Command Center)",
                                                );
                                                t.set_timeout(5);
                                                toast_overlay.add_toast(t);
                                            }
                                        }

                                        if ok {
                                            if let Some(reload) = reload_holder.borrow().as_ref() {
                                                reload();
                                            }
                                        }
                                    }
                                });
                            }
                        });

                        unsafe {
                            item.set_data("pkg_row", row);
                            item.set_data("pkg_wrapper", wrapper);
                            item.set_data("pkg_skip_check", skip_check);
                        }
                    }
                });

                factory.connect_bind({
                    let config = config.clone();
                    let selection_mode = selection_mode.clone();
                    let selected_ids = selected_ids.clone();
                    move |_, item| {
                        let obj = item
                            .item()
                            .and_then(|o| o.downcast::<glib::BoxedAnyObject>().ok());
                        let Some(obj) = obj else { return };
                        let pkg = obj.borrow::<Package>().clone();

                        let row = unsafe {
                            item.data::<PackageRow>("pkg_row")
                                .expect("pkg_row missing")
                                .as_ref()
                        };
                        let wrapper = unsafe {
                            item.data::<gtk::Box>("pkg_wrapper")
                                .expect("pkg_wrapper missing")
                                .as_ref()
                        };
                        let skip_check = unsafe {
                            item.data::<Rc<RefCell<bool>>>("pkg_skip_check")
                                .expect("pkg_skip_check missing")
                                .as_ref()
                        };
                        let skip_fav = unsafe {
                            item.data::<Rc<RefCell<bool>>>("pkg_skip_fav")
                                .expect("pkg_skip_fav missing")
                                .as_ref()
                        };

                        let cfg = config.borrow();
                        row.update_from_package(&pkg, cfg.ui_show_icons);

                        if cfg.ui_compact {
                            wrapper.add_css_class("compact-row");
                        } else {
                            wrapper.remove_css_class("compact-row");
                        }

                        row.checkbox.set_visible(*selection_mode.borrow());
                        *skip_check.borrow_mut() = true;
                        row.checkbox
                            .set_active(selected_ids.borrow().contains(&pkg.id()));
                        *skip_check.borrow_mut() = false;

                        // Update favorite state from config
                        let is_favorite = cfg.favorite_packages.contains(&pkg.id());
                        *skip_fav.borrow_mut() = true;
                        row.set_favorite(is_favorite);
                        *skip_fav.borrow_mut() = false;
                    }
                });

                list_view.set_factory(Some(&factory));
            };

            make_factory(&all_list_view);
            make_factory(&updates_list_view);
            make_factory(&favorites_list_view);
        }

        let apply_filters: Rc<dyn Fn()> = Rc::new({
            let packages = packages.clone();
            let filter_state = filter_state.clone();
            let config = config.clone();
            let all_list_view = all_list_view.clone();
            let all_store = all_store.clone();
            let updates_list_view = updates_list_view.clone();
            let updates_store = updates_store.clone();
            let favorites_list_view = favorites_list_view.clone();
            let favorites_store = favorites_store.clone();
            let favorites_stack = favorites_stack.clone();
            let favorites_count_label = favorites_count_label.clone();
            let all_stack = all_stack.clone();
            let updates_stack = updates_stack.clone();
            let selection_mode = selection_mode.clone();
            let skip_filter = skip_filter.clone();
            let apply_filters_holder = apply_filters_holder.clone();
            let enabled_sources = enabled_sources.clone();
            let update_top_chips = update_top_chips.clone();
            let toolbar_source_filters = toolbar_source_filters.clone();
            let on_source_click_holder = on_source_click_holder.clone();
            let last_filtered_all = last_filtered_all.clone();
            let last_filtered_updates = last_filtered_updates.clone();
            let selected_ids = selected_ids.clone();
            let selected_count_label = selected_count_label.clone();

            move || {
                if *skip_filter.borrow() {
                    return;
                }
                update_top_chips();
                let enabled = enabled_sources.borrow();

                let all_packages = packages.borrow();
                let filter = filter_state.borrow();
                let sel_mode = *selection_mode.borrow();

                // Sync source-filter popovers with current state.
                {
                    let active_source = filter.source;
                    *skip_filter.borrow_mut() = true;
                    for f in &toolbar_source_filters {
                        f.all_btn.set_active(active_source.is_none());
                        for (s, btn) in &f.source_btns {
                            btn.set_active(Some(*s) == active_source);
                        }
                    }
                    *skip_filter.borrow_mut() = false;
                }

                let filtered_all: Vec<Package> = all_packages
                    .iter()
                    .filter(|p| {
                        enabled.contains(&p.source)
                            && filter.source.is_none_or(|s| p.source == s)
                            && (filter.search_query.is_empty()
                                || p.name.to_lowercase().contains(&filter.search_query)
                                || p.description.to_lowercase().contains(&filter.search_query))
                    })
                    .cloned()
                    .collect();

                let filtered_updates: Vec<Package> = filtered_all
                    .iter()
                    .filter(|p| p.has_update())
                    .cloned()
                    .collect();

                // Filter favorites from all packages (ignoring search/source filters)
                let favorite_ids = config.borrow().favorite_packages.clone();
                let filtered_favorites: Vec<Package> = all_packages
                    .iter()
                    .filter(|p| enabled.contains(&p.source) && favorite_ids.contains(&p.id()))
                    .cloned()
                    .collect();

                let on_source_click = {
                    let filter_state = filter_state.clone();
                    let skip_filter = skip_filter.clone();
                    let apply_filters_holder = apply_filters_holder.clone();
                    let enabled_sources = enabled_sources.clone();
                    let toolbar_source_filters = toolbar_source_filters.clone();

                    move |source: PackageSource| {
                        if !enabled_sources.borrow().contains(&source) {
                            return;
                        }

                        *skip_filter.borrow_mut() = true;
                        filter_state.borrow_mut().source = Some(source);
                        for f in &toolbar_source_filters {
                            f.all_btn.set_active(false);
                            for (s, btn) in &f.source_btns {
                                btn.set_active(*s == source);
                            }
                        }
                        *skip_filter.borrow_mut() = false;
                        if let Some(apply) = apply_filters_holder.borrow().as_ref() {
                            apply();
                        }
                    }
                };

                *on_source_click_holder.borrow_mut() = Some(Rc::new(on_source_click.clone()));

                *last_filtered_all.borrow_mut() = filtered_all.clone();
                *last_filtered_updates.borrow_mut() = filtered_updates.clone();

                // Keep selection count label accurate.
                if sel_mode {
                    let count = selected_ids.borrow().len();
                    selected_count_label.set_label(&format!("{} selected", count));
                }

                Self::populate_list_store(&all_list_view, &all_store, &filtered_all);
                Self::populate_list_store(&updates_list_view, &updates_store, &filtered_updates);
                Self::populate_list_store(
                    &favorites_list_view,
                    &favorites_store,
                    &filtered_favorites,
                );

                // Update favorites count in sidebar
                favorites_count_label.set_label(&filtered_favorites.len().to_string());

                if filtered_all.is_empty() {
                    all_stack.set_visible_child_name("empty");
                } else {
                    all_stack.set_visible_child_name("list");
                }
                if filtered_updates.is_empty() {
                    updates_stack.set_visible_child_name("empty");
                } else {
                    updates_stack.set_visible_child_name("list");
                }
                if filtered_favorites.is_empty() {
                    favorites_stack.set_visible_child_name("empty");
                } else {
                    favorites_stack.set_visible_child_name("list");
                }
            }
        });

        *apply_filters_holder.borrow_mut() = Some(apply_filters.clone());

        // Source filter popovers (top toolbar).
        for filter_ui in toolbar_source_filters.iter() {
            let all_btn = filter_ui.all_btn.clone();
            let source_btns = filter_ui.source_btns.clone();
            let filter_state = filter_state.clone();
            let skip_filter = skip_filter.clone();
            let apply_filters = apply_filters.clone();

            let filter_state_all = filter_state.clone();
            let skip_filter_all = skip_filter.clone();
            let apply_filters_all = apply_filters.clone();
            all_btn.connect_toggled(move |btn| {
                if *skip_filter_all.borrow() || !btn.is_active() {
                    return;
                }
                *skip_filter_all.borrow_mut() = true;
                filter_state_all.borrow_mut().source = None;
                *skip_filter_all.borrow_mut() = false;
                apply_filters_all();
            });

            for (source, btn) in source_btns {
                let btn = btn.clone();
                let filter_state = filter_state.clone();
                let skip_filter = skip_filter.clone();
                let apply_filters = apply_filters.clone();
                let enabled_sources = enabled_sources.clone();

                btn.connect_toggled(move |b| {
                    if *skip_filter.borrow() || !b.is_active() {
                        return;
                    }
                    if !enabled_sources.borrow().contains(&source) {
                        return;
                    }
                    *skip_filter.borrow_mut() = true;
                    filter_state.borrow_mut().source = Some(source);
                    *skip_filter.borrow_mut() = false;
                    apply_filters();
                });
            }
        }

        for chip in toolbar_search_chips.iter() {
            let chip = chip.clone();
            let search_entry = search_entry.clone();
            chip.connect_clicked(move |_| {
                search_entry.set_text("");
            });
        }

        update_top_chips();

        let skip_provider_ui = Rc::new(RefCell::new(false));

        let apply_enabled_sources_ui: Rc<dyn Fn()> = Rc::new({
            let enabled_sources = enabled_sources.clone();
            let available_sources = available_sources.clone();
            let filter_state = filter_state.clone();
            let provider_rows = provider_rows.clone();
            let providers_box = providers_box.clone();
            let toolbar_source_filters = toolbar_source_filters.clone();
            let skip_filter = skip_filter.clone();
            let skip_provider_ui = skip_provider_ui.clone();
            let apply_filters = apply_filters.clone();

            move || {
                let enabled = enabled_sources.borrow().clone();
                let available = available_sources.borrow().clone();

                let mut sources = PackageSource::ALL.to_vec();
                sources.sort_by(|a, b| {
                    let a_key = (!available.contains(a), !enabled.contains(a), a.to_string());
                    let b_key = (!available.contains(b), !enabled.contains(b), b.to_string());
                    a_key.cmp(&b_key)
                });

                // Reorder sidebar provider rows.
                let mut prev: Option<gtk::Widget> = None;
                for source in &sources {
                    if let Some(row) = provider_rows.get(source) {
                        let w = row.row.clone().upcast::<gtk::Widget>();
                        providers_box.reorder_child_after(&w, prev.as_ref());
                        prev = Some(w);
                    }
                }

                *skip_provider_ui.borrow_mut() = true;
                for source in PackageSource::ALL {
                    if let Some(row) = provider_rows.get(&source) {
                        let is_available = available.contains(&source);
                        let is_enabled = enabled.contains(&source) && is_available;

                        row.enabled_switch.set_sensitive(is_available);
                        row.enabled_switch.set_active(is_enabled);

                        if is_available {
                            row.row.remove_css_class("provider-unavailable");
                            row.status_label.set_visible(false);
                        } else {
                            row.row.add_css_class("provider-unavailable");
                            row.status_label.set_visible(true);
                            let label = match source.install_hint() {
                                Some(hint) => format!("Not detected • {hint}"),
                                None => "Not detected".to_string(),
                            };
                            row.status_label.set_label(&label);
                        }
                    }
                }
                *skip_provider_ui.borrow_mut() = false;

                // Update popover filter list (disable items that can't be filtered).
                *skip_filter.borrow_mut() = true;
                for filter_ui in &toolbar_source_filters {
                    let mut prev: Option<gtk::Widget> = None;
                    for source in &sources {
                        if let Some(btn) = filter_ui.source_btns.get(source) {
                            let w = btn.clone().upcast::<gtk::Widget>();
                            filter_ui.source_box.reorder_child_after(&w, prev.as_ref());
                            prev = Some(w);

                            let can_filter = available.contains(source) && enabled.contains(source);
                            btn.set_sensitive(can_filter);
                            if !can_filter && btn.is_active() {
                                btn.set_active(false);
                            }
                        }
                    }
                }

                // If the currently selected filter becomes unavailable/disabled, fall back to All.
                {
                    let mut state = filter_state.borrow_mut();
                    if let Some(s) = state.source {
                        if !enabled.contains(&s) || !available.contains(&s) {
                            state.source = None;
                        }
                    }
                }
                *skip_filter.borrow_mut() = false;

                apply_filters();
            }
        });

        for source in PackageSource::ALL {
            if let Some(row) = provider_rows.get(&source) {
                let enabled_switch = row.enabled_switch.clone();
                let enabled_sources = enabled_sources.clone();
                let available_sources = available_sources.clone();
                let config = config.clone();
                let pm = pm.clone();
                let filter_state = filter_state.clone();
                let apply_enabled_sources_ui = apply_enabled_sources_ui.clone();
                let reload_holder = reload_holder.clone();
                let skip_provider_ui = skip_provider_ui.clone();

                enabled_switch.connect_state_set(move |_, state| {
                    if *skip_provider_ui.borrow() {
                        return glib::Propagation::Proceed;
                    }

                    let available = available_sources.borrow().contains(&source);
                    if !available {
                        return glib::Propagation::Stop;
                    }

                    if state {
                        enabled_sources.borrow_mut().insert(source);
                    } else {
                        enabled_sources.borrow_mut().remove(&source);
                        if filter_state.borrow().source == Some(source) {
                            filter_state.borrow_mut().source = None;
                        }
                    }

                    {
                        let mut cfg = config.borrow_mut();
                        set_enabled_in_config(&mut cfg, source, state);
                        let _ = cfg.save();
                    }

                    let sources = enabled_sources.borrow().clone();
                    let pm = pm.clone();
                    glib::spawn_future_local(async move {
                        pm.lock().await.set_enabled_sources(sources);
                    });

                    apply_enabled_sources_ui();
                    if state {
                        if let Some(reload) = reload_holder.borrow().as_ref() {
                            reload();
                        }
                    }

                    glib::Propagation::Proceed
                });
            }
        }

        apply_enabled_sources_ui();

        let refresh_in_progress = Rc::new(RefCell::new(false));
        let load_packages = {
            let pm = pm.clone();
            let packages = packages.clone();
            let main_stack = main_stack.clone();
            let spinner = spinner.clone();
            let toast_overlay = toast_overlay.clone();
            let update_source_counts = update_source_counts.clone();
            let apply_filters = apply_filters.clone();
            let refresh_button = refresh_button.clone();
            let config = config.clone(); // Capture config
            let apply_enabled_sources_ui = apply_enabled_sources_ui.clone();
            let refresh_in_progress = refresh_in_progress.clone();

            move || {
                if *refresh_in_progress.borrow() {
                    return;
                }
                *refresh_in_progress.borrow_mut() = true;
                apply_enabled_sources_ui();

                let pm = pm.clone();
                let packages = packages.clone();
                let main_stack = main_stack.clone();
                let spinner = spinner.clone();
                let toast_overlay = toast_overlay.clone();
                let update_source_counts = update_source_counts.clone();
                let apply_filters = apply_filters.clone();
                let refresh_button = refresh_button.clone();
                let config = config.clone();
                let refresh_in_progress = refresh_in_progress.clone();
                let view_spinners = view_spinners.clone();

                glib::spawn_future_local(async move {
                    let initial_load = packages.borrow().is_empty();

                    if initial_load {
                        main_stack.set_visible_child_name("loading");
                        spinner.start();
                    } else {
                        for s in view_spinners.iter() {
                            s.set_visible(true);
                            s.start();
                        }
                        refresh_button.set_sensitive(false);
                        // Optional: Show a small toast
                        let t = adw::Toast::new("Checking for updates...");
                        t.set_timeout(1);
                        toast_overlay.add_toast(t);
                    }

                    let ignored = config.borrow().ignored_packages.clone();
                    let should_check_updates =
                        config.borrow().check_updates_on_startup || !initial_load;

                    let pm_task = pm.clone();
                    let handle = tokio::spawn(async move {
                        let manager = pm_task.lock().await;
                        let all_packages = manager.list_all_installed().await.unwrap_or_default();
                        let updates = if should_check_updates {
                            manager.check_all_updates().await.unwrap_or_default()
                        } else {
                            Vec::new()
                        };
                        (all_packages, updates)
                    });

                    let (mut all_packages, updates) = match handle.await {
                        Ok(v) => v,
                        Err(e) => {
                            let t = adw::Toast::new(&format!("Refresh failed: {}", e));
                            t.set_timeout(5);
                            toast_overlay.add_toast(t);
                            *refresh_in_progress.borrow_mut() = false;
                            (Vec::new(), Vec::new())
                        }
                    };

                    // Get ignored packages
                    for update in &updates {
                        // Skip if ignored
                        let update_id = update.id();
                        if ignored.contains(&update_id) {
                            continue;
                        }

                        if let Some(pkg) = all_packages
                            .iter_mut()
                            .find(|p| p.name == update.name && p.source == update.source)
                        {
                            pkg.status = PackageStatus::UpdateAvailable;
                            pkg.available_version = update.available_version.clone();
                        }
                    }

                    update_source_counts(&all_packages);
                    *packages.borrow_mut() = all_packages;
                    apply_filters();

                    if initial_load {
                        spinner.stop();
                        main_stack.set_visible_child_name("content");
                    } else {
                        for s in view_spinners.iter() {
                            s.stop();
                            s.set_visible(false);
                        }
                        refresh_button.set_sensitive(true);
                        let update_count =
                            packages.borrow().iter().filter(|p| p.has_update()).count();
                        let msg = if update_count > 0 {
                            format!("Refreshed: {} updates available", update_count)
                        } else {
                            "Refreshed: All up to date".to_string()
                        };
                        let t = adw::Toast::new(&msg);
                        t.set_timeout(3);
                        toast_overlay.add_toast(t);

                        // Send desktop notification if enabled
                        if update_count > 0 && config.borrow().show_notifications {
                            notify_updates_available(update_count);
                        }
                    }

                    // Save cache after UI is updated (best-effort)
                    let packages_for_cache = packages.borrow().clone();
                    tokio::task::spawn_blocking(move || {
                        PackageCache::save_packages(&packages_for_cache);
                    });

                    *refresh_in_progress.borrow_mut() = false;
                });
            }
        };

        *reload_holder.borrow_mut() = Some(Rc::new(load_packages.clone()));

        // Undo button click handler
        undo_button.connect_clicked({
            let operation_history = operation_history.clone();
            let pm = pm.clone();
            let toast_overlay = toast_overlay.clone();
            let command_center = command_center.clone();
            let reveal_command_center = reveal_command_center.clone();
            let reload_holder = reload_holder.clone();
            let update_undo_button = update_undo_button.clone();
            move |_| {
                let record = {
                    let mut history = operation_history.borrow_mut();
                    history.pop_undoable()
                };

                let Some(record) = record else {
                    return;
                };

                let reverse_op = record.reverse_operation();
                let Some(reverse_op) = reverse_op else {
                    let t = adw::Toast::new("Cannot undo this operation");
                    t.set_timeout(3);
                    toast_overlay.add_toast(t);
                    return;
                };

                let pkg_name = record.package_name.clone();
                let pkg_source = record.source;
                let running_title = match reverse_op {
                    OperationType::Install => format!("Undoing: reinstalling {}", pkg_name),
                    OperationType::Remove => format!("Undoing: removing {}", pkg_name),
                    OperationType::Update => format!("Undoing: reverting {}", pkg_name),
                };

                let task = command_center.begin_task(
                    &running_title,
                    format!("Source: {}", pkg_source),
                    None,
                );

                let pm = pm.clone();
                let toast_overlay = toast_overlay.clone();
                let reveal_command_center = reveal_command_center.clone();
                let reload_holder = reload_holder.clone();
                let operation_history = operation_history.clone();
                let update_undo_button_async = update_undo_button.clone();
                let update_undo_button_sync = update_undo_button.clone();

                glib::spawn_future_local(async move {
                    let update_undo_button = update_undo_button_async;
                    let result = {
                        let manager = pm.lock().await;
                        match reverse_op {
                            OperationType::Install => {
                                manager
                                    .install(&Package {
                                        name: pkg_name.clone(),
                                        version: String::new(),
                                        available_version: None,
                                        description: String::new(),
                                        source: pkg_source,
                                        status: PackageStatus::NotInstalled,
                                        size: None,
                                        homepage: None,
                                        license: None,
                                        maintainer: None,
                                        dependencies: Vec::new(),
                                        install_date: None,
                                        enrichment: None,
                                    })
                                    .await
                            }
                            OperationType::Remove => {
                                manager
                                    .remove(&Package {
                                        name: pkg_name.clone(),
                                        version: String::new(),
                                        available_version: None,
                                        description: String::new(),
                                        source: pkg_source,
                                        status: PackageStatus::Installed,
                                        size: None,
                                        homepage: None,
                                        license: None,
                                        maintainer: None,
                                        dependencies: Vec::new(),
                                        install_date: None,
                                        enrichment: None,
                                    })
                                    .await
                            }
                            OperationType::Update => Ok(()), // Can't undo updates
                        }
                    };

                    match result {
                        Ok(_) => {
                            let done_title = match reverse_op {
                                OperationType::Install => format!("Reinstalled {}", pkg_name),
                                OperationType::Remove => format!("Removed {}", pkg_name),
                                OperationType::Update => format!("Reverted {}", pkg_name),
                            };
                            task.finish(
                                CommandEventKind::Success,
                                done_title,
                                format!("Undo completed for {}", pkg_source),
                                None,
                                true,
                            );
                            let t = adw::Toast::new(&format!("Undo: {}", pkg_name));
                            t.set_timeout(3);
                            toast_overlay.add_toast(t);

                            // Save history
                            let _ = operation_history.borrow().save();

                            // Reload packages
                            if let Some(reload) = reload_holder.borrow().as_ref() {
                                reload();
                            }
                        }
                        Err(e) => {
                            task.finish(
                                CommandEventKind::Error,
                                "Undo failed",
                                format!("Error: {}", e),
                                None,
                                true,
                            );
                            reveal_command_center(true);
                            let t = adw::Toast::new("Undo failed (see Command Center)");
                            t.set_timeout(5);
                            toast_overlay.add_toast(t);
                        }
                    }

                    update_undo_button();
                });

                // Update button immediately
                update_undo_button_sync();
            }
        });

        // Load with cache
        let load_with_cache = {
            let load_packages = load_packages.clone();
            let packages = packages.clone();
            let update_source_counts = update_source_counts.clone();
            let apply_filters = apply_filters.clone();
            let main_stack = main_stack.clone();
            let config = config.clone(); // Capture config

            move || {
                if let Some(mut cache) = PackageCache::load() {
                    let is_stale = cache.is_stale();

                    // Filter out ignored updates from cached packages
                    let ignored = &config.borrow().ignored_packages;
                    for pkg in cache.packages.iter_mut() {
                        if pkg.has_update() && ignored.contains(&pkg.id()) {
                            // Revert status to Installed if update is ignored
                            pkg.status = PackageStatus::Installed;
                            pkg.available_version = None;
                        }
                    }

                    update_source_counts(&cache.packages);
                    *packages.borrow_mut() = cache.packages;
                    apply_filters();
                    main_stack.set_visible_child_name("content");
                    if !is_stale || !config.borrow().check_updates_on_startup {
                        return;
                    }
                }
                load_packages();
            }
        };

        // Enable all detected providers.
        enable_detected_btn.connect_clicked({
            let enabled_sources = enabled_sources.clone();
            let available_sources = available_sources.clone();
            let config = config.clone();
            let pm = pm.clone();
            let apply_enabled_sources_ui = apply_enabled_sources_ui.clone();
            let reload_holder = reload_holder.clone();
            move |_| {
                let available = available_sources.borrow().clone();
                *enabled_sources.borrow_mut() = available.clone();

                {
                    let mut cfg = config.borrow_mut();
                    for source in PackageSource::ALL {
                        set_enabled_in_config(&mut cfg, source, available.contains(&source));
                    }
                    let _ = cfg.save();
                }

                let sources = enabled_sources.borrow().clone();
                let pm = pm.clone();
                glib::spawn_future_local(async move {
                    pm.lock().await.set_enabled_sources(sources);
                });

                apply_enabled_sources_ui();
                if let Some(reload) = reload_holder.borrow().as_ref() {
                    reload();
                }
            }
        });

        let load_fn_refresh = load_packages.clone();
        refresh_button.connect_clicked(move |_| load_fn_refresh());
        glib::idle_add_local_once(load_with_cache);

        // Background refresh timer (hours; 0 disables)
        let load_fn_timer = load_packages.clone();
        let config_timer = config.clone();
        if config_timer.borrow().update_check_interval > 0 {
            let secs = (config_timer.borrow().update_check_interval as u64) * 60 * 60;
            glib::timeout_add_local(Duration::from_secs(secs), move || {
                if config_timer.borrow().update_check_interval == 0 {
                    return glib::ControlFlow::Break;
                }
                load_fn_timer();
                glib::ControlFlow::Continue
            });
        }

        // Connect other signals (Search, Sort, Filter, Selection, Updates)

        // Search
        let filter_state_search = filter_state.clone();
        let apply_filters_search = apply_filters.clone();
        let current_view_search = current_view.clone();
        let pm_search = pm.clone();
        let toast_search = toast_overlay.clone();

        let config_search = config.clone();
        let discover_stack_search = discover_stack.clone();
        let discover_list_box_search = discover_list_box.clone();
        let discover_rows_search = discover_rows.clone();
        let discover_debounce: ActiveSource = Rc::new(RefCell::new(None));
        let discover_debounce_holder = discover_debounce.clone();
        let reload_holder_search = reload_holder.clone();
        let update_top_chips_search = update_top_chips.clone();
        let command_center_search = command_center.clone();
        let reveal_command_center_search = reveal_command_center.clone();
        let operation_history_search = operation_history.clone();
        let update_undo_button_search = update_undo_button.clone();
        let show_details_panel_holder_search = show_details_panel_holder.clone();

        search_entry.connect_search_changed(move |entry| {
            let query = entry.text().to_lowercase();
            filter_state_search.borrow_mut().search_query = query.clone();
            update_top_chips_search();

            if *current_view_search.borrow() != View::Discover {
                apply_filters_search();
                return;
            }

            cancel_active_source(&discover_debounce_holder);

            let pm = pm_search.clone();
            let toast = toast_search.clone();

            let config = config_search.clone();
            let discover_stack = discover_stack_search.clone();
            let discover_list_box = discover_list_box_search.clone();
            let discover_rows = discover_rows_search.clone();
            let current_view = current_view_search.clone();

            if query.trim().is_empty() {
                discover_stack.set_visible_child_name("empty");
                while let Some(child) = discover_list_box.first_child() {
                    discover_list_box.remove(&child);
                }
                discover_rows.borrow_mut().clear();
                return;
            }

            let reload_holder_for_timeout = reload_holder_search.clone();
            let command_center_for_timeout = command_center_search.clone();
            let reveal_command_center_for_timeout = reveal_command_center_search.clone();
            let operation_history_for_timeout = operation_history_search.clone();
            let update_undo_button_for_timeout = update_undo_button_search.clone();
            let show_details_panel_holder_for_timeout = show_details_panel_holder_search.clone();
            let debounce_clear = discover_debounce_holder.clone();

            let id = glib::timeout_add_local_once(Duration::from_millis(300), move || {
                *debounce_clear.borrow_mut() = None;
                if *current_view.borrow() != View::Discover {
                    return;
                }
                glib::spawn_future_local(async move {
                    let pm_task = pm.clone();
                    let query_task = query.clone();
                    let handle = tokio::spawn(async move {
                        let manager = pm_task.lock().await;
                        manager.search(&query_task).await.unwrap_or_default()
                    });

                    let results = match handle.await {
                        Ok(v) => v,
                        Err(e) => {
                            let t = adw::Toast::new(&format!("Search failed: {}", e));
                            t.set_timeout(5);
                            toast.add_toast(t);
                            Vec::new()
                        }
                    };

                    let on_source_click = |_src: PackageSource| {};
                    let reload_packages = reload_holder_for_timeout.borrow().clone();
                    Self::populate_list(
                        &discover_list_box,
                        &results,
                        &discover_rows,
                        &config,
                        false,
                        on_source_click,
                        reload_packages,
                        &command_center_for_timeout,
                        reveal_command_center_for_timeout.clone(),
                        operation_history_for_timeout.clone(),
                        update_undo_button_for_timeout.clone(),
                        show_details_panel_holder_for_timeout.clone(),
                        &pm,
                        &toast,
                    );

                    if results.is_empty() {
                        discover_stack.set_visible_child_name("empty");
                    } else {
                        discover_stack.set_visible_child_name("list");
                    }
                });
            });
            *discover_debounce_holder.borrow_mut() = Some(id);
        });

        let config_search_activate = config.clone();
        search_entry.connect_activate(move |entry| {
            let query = entry.text().trim().to_string();
            if !query.is_empty() {
                let mut cfg = config_search_activate.borrow_mut();
                cfg.recent_searches.retain(|s| s != &query);
                cfg.recent_searches.insert(0, query);
                cfg.recent_searches.truncate(5);
                let _ = cfg.save();
            }
        });

        // Source filter lives in the top toolbar popover (not the sidebar).

        // Sort
        let packages_sort = packages.clone();
        let apply_filters_sort = apply_filters.clone();
        sort_dropdown.connect_selected_notify(move |dropdown| {
            let mut pkgs = packages_sort.borrow_mut();
            match dropdown.selected() {
                0 => pkgs.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase())),
                1 => pkgs.sort_by(|a, b| b.name.to_lowercase().cmp(&a.name.to_lowercase())),
                2 => pkgs.sort_by(|a, b| a.source.cmp(&b.source)),
                _ => {}
            }
            drop(pkgs);
            apply_filters_sort();
        });

        // Selection Toggle
        let selection_bar_toggle = selection_bar.clone();
        let selection_mode_toggle = selection_mode.clone();
        let selected_ids_toggle = selected_ids.clone();
        let selected_count_toggle = selected_count_label.clone();
        let nav_list_toggle = nav_list.clone();
        let current_view_toggle = current_view.clone();
        let apply_filters_toggle = apply_filters.clone();
        select_button.connect_toggled(move |btn| {
            let active = btn.is_active();

            if active && *current_view_toggle.borrow() != View::Library {
                if let Some(row) = nav_list_toggle.row_at_index(1) {
                    nav_list_toggle.select_row(Some(&row));
                }
            }

            *selection_mode_toggle.borrow_mut() = active;
            selection_bar_toggle.set_visible(active);

            if !active {
                selected_ids_toggle.borrow_mut().clear();
                selected_count_toggle.set_label("0 selected");
            }
            apply_filters_toggle();
        });

        // Select/Deselect All
        let selected_ids_select_all = selected_ids.clone();
        let selected_count = selected_count_label.clone();
        let selection_mode_select_all = selection_mode.clone();
        let last_filtered_all_select_all = last_filtered_all.clone();
        let apply_filters_select_all = apply_filters.clone();
        select_all_btn.connect_clicked(move |_| {
            if !*selection_mode_select_all.borrow() {
                return;
            }
            let ids: HashSet<String> = last_filtered_all_select_all
                .borrow()
                .iter()
                .map(|p| p.id())
                .collect();
            let count = ids.len();
            *selected_ids_select_all.borrow_mut() = ids;
            selected_count.set_label(&format!("{} selected", count));
            apply_filters_select_all();
        });

        let selected_ids_desel = selected_ids.clone();
        let selected_count_desel = selected_count_label.clone();
        let selection_mode_desel = selection_mode.clone();
        let apply_filters_desel = apply_filters.clone();
        deselect_all_btn.connect_clicked(move |_| {
            if !*selection_mode_desel.borrow() {
                return;
            }
            selected_ids_desel.borrow_mut().clear();
            selected_count_desel.set_label("0 selected");
            apply_filters_desel();
        });

        // Navigation
        let content_stack_nav = content_stack.clone();
        let current_view_nav = current_view.clone();
        nav_list.connect_row_selected(move |_, row| {
            if let Some(row) = row {
                match row.index() {
                    0 => {
                        content_stack_nav.set_visible_child_name("discover");
                        *current_view_nav.borrow_mut() = View::Discover;
                    }
                    1 => {
                        content_stack_nav.set_visible_child_name("all");
                        *current_view_nav.borrow_mut() = View::Library;
                    }
                    2 => {
                        content_stack_nav.set_visible_child_name("updates");
                        *current_view_nav.borrow_mut() = View::Updates;
                    }
                    3 => {
                        content_stack_nav.set_visible_child_name("favorites");
                        *current_view_nav.borrow_mut() = View::Favorites;
                    }
                    _ => {}
                }
            }
        });

        // Update All
        let pm_all = pm.clone();
        let packages_all = packages.clone();
        let toast_all = toast_overlay.clone();
        let progress_overlay_all = progress_overlay.clone();
        let progress_bar_all = progress_bar.clone();
        let progress_label_all = progress_label.clone();
        let load_fn_all = load_packages.clone();
        let command_center_all = command_center.clone();
        let reveal_command_center_all = reveal_command_center.clone();

        update_all_btn.connect_clicked(move |btn| {
            btn.set_sensitive(false);
            let updates: Vec<Package> = packages_all
                .borrow()
                .iter()
                .filter(|p| p.has_update())
                .cloned()
                .collect();
            if updates.is_empty() {
                btn.set_sensitive(true);
                return;
            }

            let ctx = BulkOpContext {
                pm: pm_all.clone(),
                progress_overlay: progress_overlay_all.clone(),
                progress_bar: progress_bar_all.clone(),
                progress_label: progress_label_all.clone(),
                command_center: command_center_all.clone(),
                toast_overlay: toast_all.clone(),
                reveal_command_center: reveal_command_center_all.clone(),
                reload_packages: Rc::new(load_fn_all.clone()),
            };
            let btn = btn.clone();
            glib::spawn_future_local(async move {
                execute_bulk_operation(ctx, updates, BulkOpKind::Update, Some(btn)).await;
            });
        });

        // Update Selected
        let pm_upd = pm.clone();
        let toast_upd = toast_overlay.clone();
        let progress_overlay_upd = progress_overlay.clone();
        let progress_bar_upd = progress_bar.clone();
        let progress_label_upd = progress_label.clone();
        let load_fn_upd = load_packages.clone();
        let command_center_upd = command_center.clone();
        let reveal_command_center_upd = reveal_command_center.clone();
        let selected_ids_upd = selected_ids.clone();
        let packages_upd = packages.clone();

        update_selected_btn.connect_clicked(move |btn| {
            let selected_set = selected_ids_upd.borrow().clone();
            let selected: Vec<Package> = packages_upd
                .borrow()
                .iter()
                .filter(|p| selected_set.contains(&p.id()) && p.has_update())
                .cloned()
                .collect();
            if selected.is_empty() {
                let toast = adw::Toast::new("No updatable packages selected");
                toast.set_timeout(2);
                toast_upd.add_toast(toast);
                return;
            }
            btn.set_sensitive(false);
            let ctx = BulkOpContext {
                pm: pm_upd.clone(),
                progress_overlay: progress_overlay_upd.clone(),
                progress_bar: progress_bar_upd.clone(),
                progress_label: progress_label_upd.clone(),
                command_center: command_center_upd.clone(),
                toast_overlay: toast_upd.clone(),
                reveal_command_center: reveal_command_center_upd.clone(),
                reload_packages: Rc::new(load_fn_upd.clone()),
            };
            let btn = btn.clone();
            glib::spawn_future_local(async move {
                execute_bulk_operation(ctx, selected, BulkOpKind::Update, Some(btn)).await;
            });
        });

        // Remove Selected
        let pm_rem = pm.clone();
        let toast_rem = toast_overlay.clone();
        let progress_overlay_rem = progress_overlay.clone();
        let progress_bar_rem = progress_bar.clone();
        let progress_label_rem = progress_label.clone();
        let load_fn_rem = load_packages.clone();
        let command_center_rem = command_center.clone();
        let reveal_command_center_rem = reveal_command_center.clone();
        let selected_ids_rem = selected_ids.clone();
        let packages_rem = packages.clone();

        remove_selected_btn.connect_clicked(move |btn| {
            let selected_set = selected_ids_rem.borrow().clone();
            let selected: Vec<Package> = packages_rem
                .borrow()
                .iter()
                .filter(|p| selected_set.contains(&p.id()))
                .cloned()
                .collect();

            if selected.is_empty() {
                let toast = adw::Toast::new("No packages selected");
                toast.set_timeout(2);
                toast_rem.add_toast(toast);
                return;
            }

            btn.set_sensitive(false);
            let ctx = BulkOpContext {
                pm: pm_rem.clone(),
                progress_overlay: progress_overlay_rem.clone(),
                progress_bar: progress_bar_rem.clone(),
                progress_label: progress_label_rem.clone(),
                command_center: command_center_rem.clone(),
                toast_overlay: toast_rem.clone(),
                reveal_command_center: reveal_command_center_rem.clone(),
                reload_packages: Rc::new(load_fn_rem.clone()),
            };
            let btn = btn.clone();
            glib::spawn_future_local(async move {
                execute_bulk_operation(ctx, selected, BulkOpKind::Remove, Some(btn)).await;
            });
        });

        // Window Close
        let config_state = self.config.clone();
        let filter_state_save = filter_state.clone();
        self.window.connect_close_request(move |window| {
            let mut cfg = config_state.borrow_mut();
            cfg.window_maximized = window.is_maximized();
            if !cfg.window_maximized {
                cfg.window_width = window.width();
                cfg.window_height = window.height();
            }
            // Save source filter state
            cfg.last_source_filter = filter_state_save
                .borrow()
                .source
                .map(|s| s.as_config_str().to_string());
            let _ = cfg.save();
            glib::Propagation::Proceed
        });

        setup_keyboard_shortcuts(
            &self.window,
            ShortcutContext {
                search_entry: search_entry.clone(),
                select_button: select_button.clone(),
                update_selected_btn: update_selected_btn.clone(),
                remove_selected_btn: remove_selected_btn.clone(),
                refresh_fn: Rc::new(load_packages.clone()),
                close_details_panel: close_details_panel.clone(),
                details_split: details_split.clone(),
            },
        );

        Rc::new(load_packages)
    }

    #[allow(clippy::too_many_arguments)]
    fn populate_list<F>(
        list_box: &gtk::ListBox,
        packages: &[Package],
        rows: &Rc<RefCell<Vec<PackageRow>>>,
        config: &Rc<RefCell<Config>>,
        selection_mode: bool,
        on_source_click: F,
        reload_packages: Option<LocalFn>,
        command_center: &CommandCenter,
        reveal_command_center: Rc<dyn Fn(bool)>,
        operation_history: Rc<RefCell<OperationHistory>>,
        update_undo_button: Rc<dyn Fn()>,
        show_details_panel_holder: ShowDetailsFnHolder,
        pm: &Arc<Mutex<PackageManager>>,
        toast_overlay: &adw::ToastOverlay,
    ) where
        F: Fn(PackageSource) + Clone + 'static,
    {
        const CHUNK_SIZE: usize = 200;

        unsafe {
            if let Some(prev) = list_box.steal_data::<ActiveSource>("populate_source") {
                cancel_active_source(&prev);
            }
        }

        while let Some(child) = list_box.first_child() {
            list_box.remove(&child);
        }
        rows.borrow_mut().clear();

        let list_box_for_idle = list_box.clone();
        let active_source: ActiveSource = Rc::new(RefCell::new(None));
        let active_source_for_callback = active_source.clone();
        let pm = pm.clone();
        let toast_overlay = toast_overlay.clone();
        let config = config.clone();
        let rows = rows.clone();
        let reload_packages = reload_packages.clone();
        let command_center = command_center.clone();
        let reveal_command_center = reveal_command_center.clone();
        let operation_history = operation_history.clone();
        let update_undo_button = update_undo_button.clone();
        let show_details_panel_holder = show_details_panel_holder.clone();
        let (show_icons, compact) = {
            let cfg = config.borrow();
            (cfg.ui_show_icons, cfg.ui_compact)
        };
        let packages: Vec<Package> = packages.to_vec();
        let on_source_click = Rc::new(on_source_click);

        let index = Rc::new(RefCell::new(0usize));

        let source_id = glib::idle_add_local(move || {
            let mut start = *index.borrow();
            let end = (start + CHUNK_SIZE).min(packages.len());

            while start < end {
                let package = packages[start].clone();
                let row = PackageRow::new(package.clone(), None, show_icons);
                row.set_selection_mode(selection_mode);
                let list_row = gtk::ListBoxRow::new();
                if compact {
                    list_row.add_css_class("compact-row");
                }
                list_row.set_child(Some(&row.widget));

                let pkg = package.clone();
                let show_details_holder = show_details_panel_holder.clone();

                row.widget.connect_activated(move |_| {
                    if let Some(show_fn) = show_details_holder.borrow().as_ref() {
                        show_fn(pkg.clone());
                    }
                });

                let source = package.source;
                let on_source_click_clone = on_source_click.clone();
                row.source_button
                    .connect_clicked(move |_| on_source_click_clone(source));

                let pkg_action = package.clone();
                let pm_action = pm.clone();
                let toast_action = toast_overlay.clone();
                let spinner = row.spinner.clone();
                let action_btn = row.action_button.clone();
                let row_widget = row.widget.clone();
                let row_progress = row.progress_bar.clone();
                let row_pkg = row.package.clone();
                let row_update_icon = row.update_icon.clone();
                let reload_action = reload_packages.clone();
                let command_center_action = command_center.clone();
                let reveal_command_center_action = reveal_command_center.clone();
                let operation_history_action = operation_history.clone();
                let update_undo_button_action = update_undo_button.clone();

                row.action_button.connect_clicked(move |_| {
                    let pkg = pkg_action.clone();
                    let pm = pm_action.clone();
                    let toast = toast_action.clone();
                    let spinner = spinner.clone();
                    let btn = action_btn.clone();
                    let row_widget = row_widget.clone();
                    let row_progress = row_progress.clone();
                    let row_pkg = row_pkg.clone();
                    let row_update_icon = row_update_icon.clone();
                    let reload_action = reload_action.clone();
                    let command_center = command_center_action.clone();
                    let reveal_command_center = reveal_command_center_action.clone();
                    let operation_history = operation_history_action.clone();
                    let update_undo_button = update_undo_button_action.clone();

                    btn.set_visible(false);
                    spinner.set_visible(true);
                    spinner.start();

                    glib::spawn_future_local(async move {
                        let op = match pkg.status {
                            PackageStatus::UpdateAvailable => PackageOp::Update,
                            PackageStatus::Installed => PackageOp::Remove,
                            PackageStatus::NotInstalled => PackageOp::Install,
                            _ => PackageOp::Update,
                        };
                        let retry = RetrySpec::Package {
                            package: Box::new(pkg.clone()),
                            op: op.clone(),
                        };
                        let running_title = match op {
                            PackageOp::Update => format!("Updating {}", pkg.name),
                            PackageOp::Remove => format!("Removing {}", pkg.name),
                            PackageOp::Install => format!("Installing {}", pkg.name),
                            PackageOp::Downgrade => format!("Downgrading {}", pkg.name),
                            PackageOp::DowngradeTo(_) => format!("Downgrading {}", pkg.name),
                        };
                        let task = command_center.begin_task(
                            &running_title,
                            format!("Source: {}", pkg.source),
                            Some(retry),
                        );

                        let op_label = match pkg.status {
                            PackageStatus::UpdateAvailable => format!("Updating {}...", pkg.name),
                            PackageStatus::Installed => format!("Removing {}...", pkg.name),
                            PackageStatus::NotInstalled => format!("Installing {}...", pkg.name),
                            _ => "Working...".to_string(),
                        };
                        row_widget.set_subtitle(&op_label);
                        row_progress.set_fraction(0.0);
                        row_progress.set_visible(true);

                        let progress_bar_pulse = row_progress.clone();
                        let pulser_id =
                            glib::timeout_add_local(Duration::from_millis(120), move || {
                                progress_bar_pulse.pulse();
                                glib::ControlFlow::Continue
                            });

                        let handle = tokio::spawn(async move {
                            let manager = pm.lock().await;
                            let result = match pkg.status {
                                PackageStatus::UpdateAvailable => manager.update(&pkg).await,
                                PackageStatus::Installed => manager.remove(&pkg).await,
                                PackageStatus::NotInstalled => manager.install(&pkg).await,
                                _ => Ok(()),
                            };
                            (pkg, result)
                        });

                        let (pkg, result) = match handle.await {
                            Ok(v) => v,
                            Err(e) => {
                                remove_source(pulser_id);
                                row_progress.set_visible(false);
                                spinner.stop();
                                spinner.set_visible(false);
                                btn.set_visible(true);
                                row_widget.set_subtitle("");
                                task.finish(
                                    CommandEventKind::Error,
                                    "Operation failed",
                                    format!("Task join error: {}", e),
                                    None,
                                    true,
                                );
                                reveal_command_center(true);
                                let t = adw::Toast::new("Operation failed (see Command Center)");
                                t.set_timeout(5);
                                toast.add_toast(t);
                                return;
                            }
                        };

                        remove_source(pulser_id);
                        row_progress.set_visible(false);
                        spinner.stop();
                        spinner.set_visible(false);
                        btn.set_visible(true);
                        let restore = if pkg.description.is_empty() {
                            pkg.source.to_string()
                        } else {
                            pkg.description.clone()
                        };
                        row_widget.set_subtitle(&restore);

                        let ok = result.is_ok();
                        let (kind, title, details, command) = match result {
                            Ok(_) => {
                                let title = match pkg.status {
                                    PackageStatus::UpdateAvailable => {
                                        format!("Updated {}", pkg.name)
                                    }
                                    PackageStatus::Installed => format!("Removed {}", pkg.name),
                                    PackageStatus::NotInstalled => {
                                        format!("Installed {}", pkg.name)
                                    }
                                    _ => format!("Completed {}", pkg.name),
                                };
                                (
                                    CommandEventKind::Success,
                                    title,
                                    format!("Source: {}", pkg.source),
                                    None,
                                )
                            }
                            Err(e) => {
                                let raw = format!("Error: {}", e);
                                if let Some((details, command)) = parse_suggestion(&raw) {
                                    (
                                        CommandEventKind::Error,
                                        "Action required".to_string(),
                                        details,
                                        Some(command),
                                    )
                                } else {
                                    (
                                        CommandEventKind::Error,
                                        "Operation failed".to_string(),
                                        raw,
                                        None,
                                    )
                                }
                            }
                        };
                        task.finish(kind, title, details, command, true);

                        if !ok {
                            reveal_command_center(true);
                            let t = adw::Toast::new("Operation failed (see Command Center)");
                            t.set_timeout(5);
                            toast.add_toast(t);
                        }

                        if ok {
                            // Record operation in history for undo
                            let op_type = match pkg.status {
                                PackageStatus::UpdateAvailable => OperationType::Update,
                                PackageStatus::Installed => OperationType::Remove,
                                PackageStatus::NotInstalled => OperationType::Install,
                                _ => OperationType::Update,
                            };
                            let record = OperationRecord::new(
                                op_type,
                                pkg.name.clone(),
                                pkg.source,
                                pkg.status,
                                Some(pkg.version.clone()),
                                pkg.available_version.clone(),
                                true,
                            );
                            {
                                let mut history = operation_history.borrow_mut();
                                history.push(record);
                                let _ = history.save();
                            }
                            update_undo_button();

                            // Optimistic UI update (then reload to sync exact versions/counts).
                            {
                                let mut p = row_pkg.borrow_mut();
                                p.status = match p.status {
                                    PackageStatus::UpdateAvailable => PackageStatus::Installed,
                                    PackageStatus::Installed => PackageStatus::NotInstalled,
                                    PackageStatus::NotInstalled => PackageStatus::Installed,
                                    other => other,
                                };
                                p.available_version = None;
                                row_update_icon
                                    .set_visible(p.status == PackageStatus::UpdateAvailable);
                                PackageRow::apply_action_button_style(&btn, p.status);
                            }
                            if let Some(reload) = reload_action.as_ref() {
                                reload();
                            }
                        }
                    });
                });

                list_box_for_idle.append(&list_row);
                rows.borrow_mut().push(row);

                start += 1;
            }

            *index.borrow_mut() = start;
            if start >= packages.len() {
                *active_source_for_callback.borrow_mut() = None;
                glib::ControlFlow::Break
            } else {
                glib::ControlFlow::Continue
            }
        });

        *active_source.borrow_mut() = Some(source_id);

        unsafe {
            list_box.set_data("populate_source", active_source);
        }
    }

    pub fn present(&self) {
        self.window.present();
    }
}
