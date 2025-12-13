use crate::backend::PackageManager;
use crate::models::{Config, Package, PackageCache, PackageSource, PackageStatus};
use crate::ui::{show_about_dialog, PackageDetailsDialog, PackageRow, PreferencesDialog};
use gtk4::prelude::*;
use gtk4::{self as gtk, gio, glib};
use libadwaita::prelude::*;
use libadwaita as adw;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone, Copy, PartialEq, Eq)]
enum ViewMode {
    AllPackages,
    Updates,
}

/// Filter state for the package list
#[derive(Clone)]
struct FilterState {
    sources: Vec<PackageSource>,
    search_query: String,
}

impl Default for FilterState {
    fn default() -> Self {
        Self {
            sources: vec![
                PackageSource::Apt,
                PackageSource::Flatpak,
                PackageSource::Snap,
                PackageSource::Npm,
                PackageSource::Pip,
                PackageSource::Deb,
                PackageSource::AppImage,
            ],
            search_query: String::new(),
        }
    }
}

pub struct LinGetWindow {
    pub window: adw::ApplicationWindow,
    package_manager: Arc<Mutex<PackageManager>>,
    packages: Rc<RefCell<Vec<Package>>>,
    config: Rc<RefCell<Config>>,
    filter_state: Rc<RefCell<FilterState>>,
    selection_mode: Rc<RefCell<bool>>,
    // All packages view
    all_list_box: gtk::ListBox,
    all_rows: Rc<RefCell<Vec<PackageRow>>>,
    // Updates view
    updates_list_box: gtk::ListBox,
    updates_rows: Rc<RefCell<Vec<PackageRow>>>,
    // UI elements
    search_entry: gtk::SearchEntry,
    content_stack: gtk::Stack,
    main_stack: gtk::Stack,
    spinner: gtk::Spinner,
    toast_overlay: adw::ToastOverlay,
    current_view: Rc<RefCell<ViewMode>>,
    // Sidebar labels
    all_count_label: gtk::Label,
    update_count_label: gtk::Label,
    // Source count labels
    apt_count_label: gtk::Label,
    flatpak_count_label: gtk::Label,
    snap_count_label: gtk::Label,
    npm_count_label: gtk::Label,
    pip_count_label: gtk::Label,
    deb_count_label: gtk::Label,
    appimage_count_label: gtk::Label,
    // Progress overlay
    progress_overlay: gtk::Box,
    progress_bar: gtk::ProgressBar,
    progress_label: gtk::Label,
    // Selection action bar
    selection_bar: gtk::ActionBar,
    selected_count_label: gtk::Label,
}

impl LinGetWindow {
    pub fn new(app: &adw::Application) -> Self {
        let config = Rc::new(RefCell::new(Config::load()));
        let package_manager = Arc::new(Mutex::new(PackageManager::new()));
        let packages: Rc<RefCell<Vec<Package>>> = Rc::new(RefCell::new(Vec::new()));
        let all_rows: Rc<RefCell<Vec<PackageRow>>> = Rc::new(RefCell::new(Vec::new()));
        let updates_rows: Rc<RefCell<Vec<PackageRow>>> = Rc::new(RefCell::new(Vec::new()));
        let current_view = Rc::new(RefCell::new(ViewMode::AllPackages));
        let filter_state = Rc::new(RefCell::new(FilterState::default()));
        let selection_mode = Rc::new(RefCell::new(false));

        // Create UI components
        let (header, search_entry, refresh_button, select_button) = Self::build_header();
        
        let (
            sidebar, 
            nav_list, 
            all_count_label, 
            update_count_label,
            apt_btn, apt_count,
            flatpak_btn, flatpak_count,
            snap_btn, snap_count,
            npm_btn, npm_count,
            pip_btn, pip_count,
            deb_btn, deb_count,
            appimage_btn, appimage_count
        ) = Self::build_sidebar();

        let (
            content_area, 
            all_stack, 
            updates_stack, 
            all_list_box, 
            updates_list_box, 
            content_stack, 
            sort_dropdown, 
            update_all_btn
        ) = Self::build_content_area();

        let (progress_overlay, progress_bar, progress_label) = Self::build_progress_overlay();
        
        let (
            selection_bar, 
            select_all_btn, 
            deselect_all_btn, 
            update_selected_btn, 
            remove_selected_btn, 
            selected_count_label
        ) = Self::build_selection_bar();

        // Loading view
        let spinner = gtk::Spinner::builder()
            .width_request(48)
            .height_request(48)
            .build();

        let loading_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(16)
            .valign(gtk::Align::Center)
            .halign(gtk::Align::Center)
            .vexpand(true)
            .hexpand(true)
            .build();

        let loading_label = gtk::Label::builder()
            .label("Loading packages...")
            .build();
        loading_label.add_css_class("title-2");
        loading_label.add_css_class("dim-label");

        loading_box.append(&spinner);
        loading_box.append(&loading_label);

        // Main Stack
        let main_stack = gtk::Stack::builder()
            .transition_type(gtk::StackTransitionType::Crossfade)
            .transition_duration(150)
            .build();

        // Assemble main content
        let content_with_bars = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();
        
        content_with_bars.append(&content_area);
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

        // Main Layout
        let main_paned = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .build();

        main_paned.append(&sidebar);
        main_paned.append(&gtk::Separator::new(gtk::Orientation::Vertical));
        main_paned.append(&toast_overlay);

        let main_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();

        main_box.append(&header);
        main_box.append(&main_paned);

        // Window
        let window = adw::ApplicationWindow::builder()
            .application(app)
            .title("LinGet")
            .default_width(config.borrow().window_width.max(1100))
            .default_height(config.borrow().window_height.max(700))
            .content(&main_box)
            .build();

        if config.borrow().window_maximized {
            window.maximize();
        }

        let win = Self {
            window,
            package_manager,
            packages,
            config,
            filter_state,
            selection_mode,
            all_list_box,
            all_rows,
            updates_list_box,
            updates_rows,
            search_entry,
            content_stack,
            main_stack,
            spinner,
            toast_overlay,
            current_view,
            all_count_label,
            update_count_label,
            apt_count_label: apt_count,
            flatpak_count_label: flatpak_count,
            snap_count_label: snap_count,
            npm_count_label: npm_count,
            pip_count_label: pip_count,
            deb_count_label: deb_count,
            appimage_count_label: appimage_count,
            progress_overlay,
            progress_bar,
            progress_label,
            selection_bar,
            selected_count_label,
        };

        win.setup_actions(app);
        win.setup_signals(
            refresh_button,
            select_button,
            nav_list,
            update_all_btn,
            all_stack,
            updates_stack,
            apt_btn,
            flatpak_btn,
            snap_btn,
            npm_btn,
            pip_btn,
            deb_btn,
            appimage_btn,
            select_all_btn,
            deselect_all_btn,
            update_selected_btn,
            remove_selected_btn,
            sort_dropdown,
        );

        win
    }

    fn build_header() -> (adw::HeaderBar, gtk::SearchEntry, gtk::Button, gtk::ToggleButton) {
        let header = adw::HeaderBar::new();

        // Menu
        let menu = gio::Menu::new();
        
        let backup_section = gio::Menu::new();
        backup_section.append(Some("Import Packages..."), Some("app.import"));
        backup_section.append(Some("Export Packages..."), Some("app.export"));
        menu.append_section(Some("Backup"), &backup_section);

        let app_section = gio::Menu::new();
        app_section.append(Some("Preferences"), Some("app.preferences"));
        app_section.append(Some("Keyboard Shortcuts"), Some("app.shortcuts"));
        app_section.append(Some("About LinGet"), Some("app.about"));
        menu.append_section(None, &app_section);

        let menu_button = gtk::MenuButton::builder()
            .icon_name("open-menu-symbolic")
            .menu_model(&menu)
            .tooltip_text("Main Menu (F10)")
            .build();
        header.pack_end(&menu_button);

        // Refresh
        let refresh_button = gtk::Button::builder()
            .icon_name("view-refresh-symbolic")
            .tooltip_text("Refresh (Ctrl+R)")
            .build();
        refresh_button.add_css_class("flat");
        header.pack_end(&refresh_button);

        // Selection Mode
        let select_button = gtk::ToggleButton::builder()
            .icon_name("selection-mode-symbolic")
            .tooltip_text("Selection Mode (Ctrl+S)")
            .build();
        select_button.add_css_class("flat");
        header.pack_end(&select_button);

        // Search
        let search_entry = gtk::SearchEntry::builder()
            .placeholder_text("Search packages... (Ctrl+F)")
            .hexpand(true)
            .build();
        search_entry.add_css_class("search-entry-large");

        let search_clamp = adw::Clamp::builder()
            .maximum_size(500)
            .child(&search_entry)
            .build();

        header.set_title_widget(Some(&search_clamp));

        (header, search_entry, refresh_button, select_button)
    }

    fn build_sidebar() -> (
        gtk::Box, gtk::ListBox, gtk::Label, gtk::Label,
        gtk::ToggleButton, gtk::Label,
        gtk::ToggleButton, gtk::Label,
        gtk::ToggleButton, gtk::Label,
        gtk::ToggleButton, gtk::Label,
        gtk::ToggleButton, gtk::Label,
        gtk::ToggleButton, gtk::Label,
        gtk::ToggleButton, gtk::Label,
    ) {
        let sidebar_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .width_request(220)
            .css_classes(vec!["sidebar"])
            .build();

        // Header
        let sidebar_header = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(12)
            .margin_top(16)
            .margin_bottom(8)
            .margin_start(16)
            .margin_end(16)
            .build();

        let app_icon = gtk::Image::builder()
            .icon_name("package-x-generic")
            .pixel_size(32)
            .build();
        app_icon.add_css_class("app-icon");

        let app_title = gtk::Label::builder()
            .label("LinGet")
            .xalign(0.0)
            .build();
        app_title.add_css_class("title-1");

        sidebar_header.append(&app_icon);
        sidebar_header.append(&app_title);
        sidebar_box.append(&sidebar_header);

        // Nav List
        let nav_label = gtk::Label::builder()
            .label("Library")
            .xalign(0.0)
            .margin_top(16)
            .margin_start(16)
            .margin_bottom(4)
            .build();
        nav_label.add_css_class("caption");
        nav_label.add_css_class("dim-label");
        sidebar_box.append(&nav_label);

        let nav_list = gtk::ListBox::builder()
            .selection_mode(gtk::SelectionMode::Single)
            .css_classes(vec!["navigation-sidebar"])
            .build();

        // All Packages
        let all_row = gtk::ListBoxRow::new();
        all_row.add_css_class("nav-row");
        let all_box = gtk::Box::builder().orientation(gtk::Orientation::Horizontal).spacing(12).margin_top(10).margin_bottom(10).margin_start(12).margin_end(12).build();
        let all_count_label = gtk::Label::builder().label("0").css_classes(vec!["dim-label", "caption"]).build();
        all_box.append(&gtk::Image::from_icon_name("view-grid-symbolic"));
        all_box.append(&gtk::Label::builder().label("All Packages").hexpand(true).xalign(0.0).build());
        all_box.append(&all_count_label);
        all_row.set_child(Some(&all_box));
        nav_list.append(&all_row);

        // Updates
        let updates_row = gtk::ListBoxRow::new();
        updates_row.add_css_class("nav-row");
        let updates_box = gtk::Box::builder().orientation(gtk::Orientation::Horizontal).spacing(12).margin_top(10).margin_bottom(10).margin_start(12).margin_end(12).build();
        let update_count_label = gtk::Label::builder().label("0").css_classes(vec!["badge-accent"]).visible(false).build();
        updates_box.append(&gtk::Image::from_icon_name("software-update-available-symbolic"));
        updates_box.append(&gtk::Label::builder().label("Updates").hexpand(true).xalign(0.0).build());
        updates_box.append(&update_count_label);
        updates_row.set_child(Some(&updates_box));
        nav_list.append(&updates_row);

        nav_list.select_row(Some(&all_row));
        sidebar_box.append(&nav_list);

        // Sources
        let sources_label = gtk::Label::builder().label("Sources").xalign(0.0).margin_top(24).margin_start(16).margin_bottom(8).build();
        sources_label.add_css_class("caption");
        sources_label.add_css_class("dim-label");
        sidebar_box.append(&sources_label);

        let sources_box = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(2).margin_start(8).margin_end(8).build();

        let (apt_btn, apt_count) = Self::create_source_filter_btn("APT", "package-x-generic-symbolic", "source-apt");
        let (flatpak_btn, flatpak_count) = Self::create_source_filter_btn("Flatpak", "system-software-install-symbolic", "source-flatpak");
        let (snap_btn, snap_count) = Self::create_source_filter_btn("Snap", "package-x-generic-symbolic", "source-snap");
        let (npm_btn, npm_count) = Self::create_source_filter_btn("npm", "text-x-script-symbolic", "source-npm");
        let (pip_btn, pip_count) = Self::create_source_filter_btn("pip", "text-x-python-symbolic", "source-pip");
        let (deb_btn, deb_count) = Self::create_source_filter_btn("Deb", "package-x-generic-symbolic", "source-deb");
        let (appimage_btn, appimage_count) = Self::create_source_filter_btn("AppImage", "application-x-executable-symbolic", "source-appimage");

        sources_box.append(&apt_btn);
        sources_box.append(&flatpak_btn);
        sources_box.append(&snap_btn);
        sources_box.append(&npm_btn);
        sources_box.append(&pip_btn);
        sources_box.append(&deb_btn);
        sources_box.append(&appimage_btn);
        sidebar_box.append(&sources_box);
        
        // Spacer and Footer
        sidebar_box.append(&gtk::Box::builder().vexpand(true).build());
        
        let stats_box = gtk::Box::builder().orientation(gtk::Orientation::Vertical).margin_start(16).margin_end(16).margin_bottom(16).spacing(4).build();
        let stats_label = gtk::Label::builder().label("Last updated: Just now").xalign(0.0).build();
        stats_label.add_css_class("caption");
        stats_label.add_css_class("dim-label");
        stats_box.append(&stats_label);
        sidebar_box.append(&stats_box);

        (
            sidebar_box, nav_list, all_count_label, update_count_label,
            apt_btn, apt_count,
            flatpak_btn, flatpak_count,
            snap_btn, snap_count,
            npm_btn, npm_count,
            pip_btn, pip_count,
            deb_btn, deb_count,
            appimage_btn, appimage_count
        )
    }

    fn build_content_area() -> (
        gtk::Box, gtk::Stack, gtk::Stack, gtk::ListBox, gtk::ListBox, gtk::Stack, gtk::DropDown, gtk::Button
    ) {
        let content_box = gtk::Box::builder().orientation(gtk::Orientation::Vertical).hexpand(true).build();
        
        let content_stack = gtk::Stack::builder().transition_type(gtk::StackTransitionType::SlideLeftRight).transition_duration(200).hexpand(true).build();
        
        // All Packages View
        let filter_bar = gtk::Box::builder().orientation(gtk::Orientation::Horizontal).spacing(8).margin_start(24).margin_end(24).margin_top(12).margin_bottom(8).build();
        filter_bar.append(&gtk::Label::new(Some("Filter:")));
        let sort_options = gtk::StringList::new(&["Name (A-Z)", "Name (Z-A)", "Source", "Recently Added"]);
        let sort_dropdown = gtk::DropDown::builder().model(&sort_options).tooltip_text("Sort by").build();
        sort_dropdown.add_css_class("flat");
        let sort_box = gtk::Box::builder().orientation(gtk::Orientation::Horizontal).hexpand(true).halign(gtk::Align::End).spacing(8).build();
        sort_box.append(&gtk::Label::new(Some("Sort:")));
        sort_box.append(&sort_dropdown);
        filter_bar.append(&sort_box);

        let all_list_box = gtk::ListBox::builder().selection_mode(gtk::SelectionMode::None).css_classes(vec!["boxed-list"]).build();
        let all_scrolled = gtk::ScrolledWindow::builder().hscrollbar_policy(gtk::PolicyType::Never).vexpand(true).child(&all_list_box).build();
        let all_content = gtk::Box::builder().orientation(gtk::Orientation::Vertical).build();
        all_content.append(&filter_bar);
        all_content.append(&adw::Clamp::builder().maximum_size(1000).child(&all_scrolled).margin_top(8).margin_bottom(24).margin_start(12).margin_end(12).build());

        let all_empty = adw::StatusPage::builder().icon_name("package-x-generic-symbolic").title("No Packages Found").description("Try adjusting your search or filters").build();
        let all_stack = gtk::Stack::builder().transition_type(gtk::StackTransitionType::Crossfade).transition_duration(150).build();
        all_stack.add_named(&all_content, Some("list"));
        all_stack.add_named(&all_empty, Some("empty"));

        // Updates View
        let updates_list_box = gtk::ListBox::builder().selection_mode(gtk::SelectionMode::None).css_classes(vec!["boxed-list"]).build();
        let updates_scrolled = gtk::ScrolledWindow::builder().hscrollbar_policy(gtk::PolicyType::Never).vexpand(true).child(&updates_list_box).build();
        let updates_header = gtk::Box::builder().orientation(gtk::Orientation::Horizontal).margin_start(24).margin_end(24).margin_top(16).margin_bottom(8).build();
        let updates_title = gtk::Label::builder().label("Available Updates").hexpand(true).xalign(0.0).build();
        updates_title.add_css_class("title-3");
        let update_all_btn = gtk::Button::builder().label("Update All").css_classes(vec!["suggested-action", "pill"]).build();
        updates_header.append(&updates_title);
        updates_header.append(&update_all_btn);
        
        let updates_content = gtk::Box::builder().orientation(gtk::Orientation::Vertical).build();
        updates_content.append(&updates_header);
        updates_content.append(&adw::Clamp::builder().maximum_size(1000).child(&updates_scrolled).margin_top(8).margin_bottom(24).margin_start(12).margin_end(12).build());

        let updates_empty = adw::StatusPage::builder().icon_name("emblem-ok-symbolic").title("All Up to Date!").description("All your packages are running the latest versions").build();
        updates_empty.add_css_class("success-status");
        let updates_stack = gtk::Stack::builder().transition_type(gtk::StackTransitionType::Crossfade).transition_duration(150).build();
        updates_stack.add_named(&updates_content, Some("list"));
        updates_stack.add_named(&updates_empty, Some("empty"));

        content_stack.add_named(&all_stack, Some("all"));
        content_stack.add_named(&updates_stack, Some("updates"));
        content_box.append(&content_stack);

        (content_box, all_stack, updates_stack, all_list_box, updates_list_box, content_stack, sort_dropdown, update_all_btn)
    }

    fn build_progress_overlay() -> (gtk::Box, gtk::ProgressBar, gtk::Label) {
        let overlay = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .valign(gtk::Align::Center)
            .halign(gtk::Align::Center)
            .spacing(16)
            .margin_start(48)
            .margin_end(48)
            .visible(false)
            .build();
        overlay.add_css_class("progress-overlay");

        let label = gtk::Label::builder().label("Updating packages...").build();
        label.add_css_class("title-3");
        let bar = gtk::ProgressBar::builder().show_text(true).build();
        bar.add_css_class("osd");

        overlay.append(&label);
        overlay.append(&bar);
        (overlay, bar, label)
    }

    fn build_selection_bar() -> (gtk::ActionBar, gtk::Button, gtk::Button, gtk::Button, gtk::Button, gtk::Label) {
        let bar = gtk::ActionBar::builder().visible(false).build();
        bar.add_css_class("selection-bar");
        
        let select_all = gtk::Button::builder().label("Select All").build();
        select_all.add_css_class("flat");
        let deselect_all = gtk::Button::builder().label("Deselect All").build();
        deselect_all.add_css_class("flat");
        let count_label = gtk::Label::builder().label("0 selected").hexpand(true).build();
        let update_btn = gtk::Button::builder().label("Update Selected").build();
        update_btn.add_css_class("suggested-action");
        let remove_btn = gtk::Button::builder().label("Remove Selected").build();
        remove_btn.add_css_class("destructive-action");

        bar.pack_start(&select_all);
        bar.pack_start(&deselect_all);
        bar.set_center_widget(Some(&count_label));
        bar.pack_end(&remove_btn);
        bar.pack_end(&update_btn);

        (bar, select_all, deselect_all, update_btn, remove_btn, count_label)
    }

    fn create_source_filter_btn(label: &str, icon: &str, css_class: &str) -> (gtk::ToggleButton, gtk::Label) {
        let btn_box = gtk::Box::builder().orientation(gtk::Orientation::Horizontal).spacing(8).build();
        let count_label = gtk::Label::new(Some("0"));
        count_label.add_css_class("dim-label");
        count_label.add_css_class("caption");

        btn_box.append(&gtk::Image::from_icon_name(icon));
        btn_box.append(&gtk::Label::builder().label(label).hexpand(true).xalign(0.0).build());
        btn_box.append(&count_label);

        let btn = gtk::ToggleButton::builder().child(&btn_box).active(true).build();
        btn.add_css_class("flat");
        btn.add_css_class("source-filter-btn");
        btn.add_css_class(css_class);

        (btn, count_label)
    }

    fn setup_actions(&self, app: &adw::Application) {
        // Import action
        let import_action = gio::SimpleAction::new("import", None);
        let window_import = self.window.clone();
        let pm_import = self.package_manager.clone();
        let toast_import = self.toast_overlay.clone();
        let progress_overlay_import = self.progress_overlay.clone();
        let progress_bar_import = self.progress_bar.clone();
        let progress_label_import = self.progress_label.clone();
        
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
            
            let _window = window_import.clone();
            let pm = pm_import.clone();
            let toast = toast_import.clone();
            let progress_overlay = progress_overlay_import.clone();
            let progress_bar = progress_bar_import.clone();
            let progress_label = progress_label_import.clone();
            
            dialog.connect_response(move |d, response| {
                if response == gtk::ResponseType::Accept {
                    if let Some(file) = d.file() {
                        if let Some(path) = file.path() {
                            if let Ok(content) = std::fs::read_to_string(path) {
                                if let Ok(list) = serde_json::from_str::<crate::models::PackageList>(&content) {
                                    d.close();
                                    
                                    // Start import process
                                    let total = list.packages.len();
                                    if total == 0 {
                                        let t = adw::Toast::new("No packages found in backup file");
                                        toast.add_toast(t);
                                        return;
                                    }
                                    
                                    progress_overlay.set_visible(true);
                                    progress_label.set_label(&format!("Importing {} packages...", total));
                                    
                                    let pm = pm.clone();
                                    let toast = toast.clone();
                                    let progress_overlay = progress_overlay.clone();
                                    let progress_bar = progress_bar.clone();
                                    
                                    glib::spawn_future_local(async move {
                                        let mut success = 0;
                                        let mut failed = 0;
                                        
                                        let manager = pm.lock().await;
                                        
                                        for (i, backup_pkg) in list.packages.iter().enumerate() {
                                            progress_bar.set_fraction((i as f64) / (total as f64));
                                            progress_bar.set_text(Some(&format!("{}/{} - {}", i + 1, total, backup_pkg.name)));
                                            
                                            // Construct a temporary package object to pass to install
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
                                            };
                                            
                                            if manager.install(&pkg).await.is_ok() {
                                                success += 1;
                                            } else {
                                                failed += 1;
                                            }
                                        }
                                        drop(manager);
                                        
                                        progress_overlay.set_visible(false);
                                        
                                        let msg = format!("Import complete: {} installed, {} failed", success, failed);
                                        let t = adw::Toast::new(&msg);
                                        t.set_timeout(5);
                                        toast.add_toast(t);
                                    });
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
                                    let t = adw::Toast::new(&format!("Serialization failed: {}", e));
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
        let window = self.window.clone();
        prefs_action.connect_activate(move |_, _| PreferencesDialog::show(config.clone(), &window));
        app.add_action(&prefs_action);

        let about_action = gio::SimpleAction::new("about", None);
        let window = self.window.clone();
        about_action.connect_activate(move |_, _| show_about_dialog(&window));
        app.add_action(&about_action);

        let shortcuts_action = gio::SimpleAction::new("shortcuts", None);
        let window_shortcuts = self.window.clone();
        shortcuts_action.connect_activate(move |_, _| Self::show_shortcuts_dialog(&window_shortcuts));
        app.add_action(&shortcuts_action);

        let quit_action = gio::SimpleAction::new("quit", None);
        let app_clone = app.clone();
        quit_action.connect_activate(move |_, _| app_clone.quit());
        app.add_action(&quit_action);

        app.set_accels_for_action("app.quit", &["<Ctrl>q"]);
        app.set_accels_for_action("app.preferences", &["<Ctrl>comma"]);
        app.set_accels_for_action("app.shortcuts", &["<Ctrl>question", "F1"]);
    }

    fn show_shortcuts_dialog(window: &adw::ApplicationWindow) {
        let dialog = gtk::ShortcutsWindow::builder().transient_for(window).modal(true).build();
        let section = gtk::ShortcutsSection::builder().section_name("shortcuts").build();
        let group = gtk::ShortcutsGroup::builder().title("General").build();

        for (title, accel) in [("Search", "<Ctrl>f"), ("Refresh", "<Ctrl>r"), ("Selection Mode", "<Ctrl>s"), ("Preferences", "<Ctrl>comma"), ("Quit", "<Ctrl>q")] {
            group.append(&gtk::ShortcutsShortcut::builder().title(title).accelerator(accel).build());
        }
        section.append(&group);
        dialog.present();
    }

    #[allow(clippy::too_many_arguments)]
    fn setup_signals(
        &self,
        refresh_button: gtk::Button,
        select_button: gtk::ToggleButton,
        nav_list: gtk::ListBox,
        update_all_btn: gtk::Button,
        all_stack: gtk::Stack,
        updates_stack: gtk::Stack,
        apt_btn: gtk::ToggleButton,
        flatpak_btn: gtk::ToggleButton,
        snap_btn: gtk::ToggleButton,
        npm_btn: gtk::ToggleButton,
        pip_btn: gtk::ToggleButton,
        deb_btn: gtk::ToggleButton,
        appimage_btn: gtk::ToggleButton,
        select_all_btn: gtk::Button,
        deselect_all_btn: gtk::Button,
        update_selected_btn: gtk::Button,
        remove_selected_btn: gtk::Button,
        sort_dropdown: gtk::DropDown,
    ) {
        let config = self.config.clone();
        let pm = self.package_manager.clone();
        let packages = self.packages.clone();
        let all_rows = self.all_rows.clone();
        let updates_rows = self.updates_rows.clone();
        let all_list_box = self.all_list_box.clone();
        let updates_list_box = self.updates_list_box.clone();
        let main_stack = self.main_stack.clone();
        let spinner = self.spinner.clone();
        let toast_overlay = self.toast_overlay.clone();
        let window = self.window.clone();
        let content_stack = self.content_stack.clone();
        let current_view = self.current_view.clone();
        let search_entry = self.search_entry.clone();
        let filter_state = self.filter_state.clone();
        let selection_mode = self.selection_mode.clone();
        let all_count_label = self.all_count_label.clone();
        let update_count_label = self.update_count_label.clone();
        let apt_count = self.apt_count_label.clone();
        let flatpak_count = self.flatpak_count_label.clone();
        let snap_count = self.snap_count_label.clone();
        let npm_count = self.npm_count_label.clone();
        let pip_count = self.pip_count_label.clone();
        let deb_count = self.deb_count_label.clone();
        let appimage_count = self.appimage_count_label.clone();
        let progress_overlay = self.progress_overlay.clone();
        let progress_bar = self.progress_bar.clone();
        let progress_label = self.progress_label.clone();
        let selection_bar = self.selection_bar.clone();
        let selected_count_label = self.selected_count_label.clone();

        // Helpers
        let update_source_counts = move |packages: &[Package]| {
            let total = packages.len();
            let updates = packages.iter().filter(|p| p.has_update()).count();

            all_count_label.set_label(&format!("{}", total));
            if updates > 0 {
                update_count_label.set_label(&format!("{}", updates));
                update_count_label.set_visible(true);
            } else {
                update_count_label.set_visible(false);
            }

            let count_by_source = |src| packages.iter().filter(|p| p.source == src).count();
            apt_count.set_label(&format!("{}", count_by_source(PackageSource::Apt)));
            flatpak_count.set_label(&format!("{}", count_by_source(PackageSource::Flatpak)));
            snap_count.set_label(&format!("{}", count_by_source(PackageSource::Snap)));
            npm_count.set_label(&format!("{}", count_by_source(PackageSource::Npm)));
            pip_count.set_label(&format!("{}", count_by_source(PackageSource::Pip)));
            deb_count.set_label(&format!("{}", count_by_source(PackageSource::Deb)));
            appimage_count.set_label(&format!("{}", count_by_source(PackageSource::AppImage)));
        };

        let skip_filter = Rc::new(RefCell::new(false));
        let apply_filters_holder: Rc<RefCell<Option<Rc<dyn Fn()>>>> = Rc::new(RefCell::new(None));

        let apply_filters = {
            let packages = packages.clone();
            let filter_state = filter_state.clone();
            let all_list_box = all_list_box.clone();
            let all_rows = all_rows.clone();
            let updates_list_box = updates_list_box.clone();
            let updates_rows = updates_rows.clone();
            let window = window.clone();
            let pm = pm.clone();
            let toast_overlay = toast_overlay.clone();
            let config = config.clone();
            let all_stack = all_stack.clone();
            let updates_stack = updates_stack.clone();
            let selection_mode = selection_mode.clone();
            let skip_filter = skip_filter.clone();
            let apply_filters_holder = apply_filters_holder.clone();
            // Buttons
            let apt_btn = apt_btn.clone();
            let flatpak_btn = flatpak_btn.clone();
            let snap_btn = snap_btn.clone();
            let npm_btn = npm_btn.clone();
            let pip_btn = pip_btn.clone();
            let deb_btn = deb_btn.clone();
            let appimage_btn = appimage_btn.clone();

            move || {
                if *skip_filter.borrow() { return; }

                let all_packages = packages.borrow();
                let filter = filter_state.borrow();
                let sel_mode = *selection_mode.borrow();

                let filtered_all: Vec<Package> = all_packages.iter().filter(|p| {
                    filter.sources.contains(&p.source) &&
                    (filter.search_query.is_empty() || p.name.to_lowercase().contains(&filter.search_query) || p.description.to_lowercase().contains(&filter.search_query))
                }).cloned().collect();

                let filtered_updates: Vec<Package> = filtered_all.iter().filter(|p| p.has_update()).cloned().collect();

                let on_source_click = {
                    let filter_state = filter_state.clone();
                    let skip_filter = skip_filter.clone();
                    let apply_filters_holder = apply_filters_holder.clone();
                    let apt_btn = apt_btn.clone();
                    let flatpak_btn = flatpak_btn.clone();
                    let snap_btn = snap_btn.clone();
                    let npm_btn = npm_btn.clone();
                    let pip_btn = pip_btn.clone();
                    let deb_btn = deb_btn.clone();
                    let appimage_btn = appimage_btn.clone();

                    move |source: PackageSource| {
                        *skip_filter.borrow_mut() = true;
                        filter_state.borrow_mut().sources = vec![source];
                        apt_btn.set_active(source == PackageSource::Apt);
                        flatpak_btn.set_active(source == PackageSource::Flatpak);
                        snap_btn.set_active(source == PackageSource::Snap);
                        npm_btn.set_active(source == PackageSource::Npm);
                        pip_btn.set_active(source == PackageSource::Pip);
                        deb_btn.set_active(source == PackageSource::Deb);
                        appimage_btn.set_active(source == PackageSource::AppImage);
                        *skip_filter.borrow_mut() = false;
                        if let Some(apply) = apply_filters_holder.borrow().as_ref() { apply(); }
                    }
                };

                Self::populate_list(&all_list_box, &filtered_all, &all_rows, &window, &pm, &toast_overlay, &config, sel_mode, on_source_click.clone());
                Self::populate_list(&updates_list_box, &filtered_updates, &updates_rows, &window, &pm, &toast_overlay, &config, sel_mode, on_source_click);

                if filtered_all.is_empty() { all_stack.set_visible_child_name("empty"); } else { all_stack.set_visible_child_name("list"); }
                if filtered_updates.is_empty() { updates_stack.set_visible_child_name("empty"); } else { updates_stack.set_visible_child_name("list"); }
            }
        };

        *apply_filters_holder.borrow_mut() = Some(Rc::new({
            let apply = apply_filters.clone();
            move || apply()
        }));

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

            move || {
                let pm = pm.clone();
                let packages = packages.clone();
                let main_stack = main_stack.clone();
                let spinner = spinner.clone();
                let toast_overlay = toast_overlay.clone();
                let update_source_counts = update_source_counts.clone();
                let apply_filters = apply_filters.clone();
                let refresh_button = refresh_button.clone();
                let config = config.clone();

                glib::spawn_future_local(async move {
                    let initial_load = packages.borrow().is_empty();
                    
                    if initial_load {
                        main_stack.set_visible_child_name("loading");
                        spinner.start();
                    } else {
                        refresh_button.set_sensitive(false);
                        // Optional: Show a small toast
                        let t = adw::Toast::new("Checking for updates...");
                        t.set_timeout(1);
                        toast_overlay.add_toast(t);
                    }

                    let manager = pm.lock().await;
                    let mut all_packages = manager.list_all_installed().await.unwrap_or_default();
                    let updates = manager.check_all_updates().await.unwrap_or_default();
                    drop(manager);

                    // Get ignored packages
                    let ignored = &config.borrow().ignored_packages;

                    for update in &updates {
                        // Skip if ignored
                        let update_id = update.id();
                        if ignored.contains(&update_id) {
                            continue;
                        }

                        if let Some(pkg) = all_packages.iter_mut().find(|p| p.name == update.name && p.source == update.source) {
                            pkg.status = PackageStatus::UpdateAvailable;
                            pkg.available_version = update.available_version.clone();
                        }
                    }

                    PackageCache::save_packages(&all_packages);
                    update_source_counts(&all_packages);
                    *packages.borrow_mut() = all_packages.clone();
                    apply_filters();

                    if initial_load {
                        spinner.stop();
                        main_stack.set_visible_child_name("content");
                    } else {
                        refresh_button.set_sensitive(true);
                        let update_count = all_packages.iter().filter(|p| p.has_update()).count();
                        let msg = if update_count > 0 {
                             format!("Refreshed: {} updates available", update_count)
                        } else {
                             "Refreshed: All up to date".to_string()
                        };
                        let t = adw::Toast::new(&msg);
                        t.set_timeout(3);
                        toast_overlay.add_toast(t);
                    }
                });
            }
        };

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
                     if !is_stale { return; }
                 }
                 load_packages();
             }
        };

        let load_fn_refresh = load_packages.clone();
        refresh_button.connect_clicked(move |_| load_fn_refresh());
        glib::idle_add_local_once(move || load_with_cache());

        // Connect other signals (Search, Sort, Filter, Selection, Updates)
        
        // Search
        let filter_state_search = filter_state.clone();
        let apply_filters_search = apply_filters.clone();
        search_entry.connect_search_changed(move |entry| {
            filter_state_search.borrow_mut().search_query = entry.text().to_lowercase();
            apply_filters_search();
        });

        // Source Filter Buttons
        let create_source_handler = |source: PackageSource| {
            let filter_state = filter_state.clone();
            let apply_filters = apply_filters.clone();
            move |btn: &gtk::ToggleButton| {
                let mut state = filter_state.borrow_mut();
                if btn.is_active() {
                    if !state.sources.contains(&source) { state.sources.push(source); }
                } else {
                    state.sources.retain(|&s| s != source);
                }
                drop(state);
                apply_filters();
            }
        };
        apt_btn.connect_toggled(create_source_handler(PackageSource::Apt));
        flatpak_btn.connect_toggled(create_source_handler(PackageSource::Flatpak));
        snap_btn.connect_toggled(create_source_handler(PackageSource::Snap));
        npm_btn.connect_toggled(create_source_handler(PackageSource::Npm));
        pip_btn.connect_toggled(create_source_handler(PackageSource::Pip));
        deb_btn.connect_toggled(create_source_handler(PackageSource::Deb));
        appimage_btn.connect_toggled(create_source_handler(PackageSource::AppImage));

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
        let apply_filters_sel = apply_filters.clone();
        select_button.connect_toggled(move |btn| {
            let active = btn.is_active();
            *selection_mode_toggle.borrow_mut() = active;
            selection_bar_toggle.set_visible(active);
            apply_filters_sel();
        });

        // Select/Deselect All
        let all_rows_sel = all_rows.clone();
        let selected_count = selected_count_label.clone();
        select_all_btn.connect_clicked(move |_| {
            let rows = all_rows_sel.borrow();
            for row in rows.iter() { row.checkbox.set_active(true); }
            selected_count.set_label(&format!("{} selected", rows.len()));
        });
        
        let all_rows_desel = all_rows.clone();
        let selected_count_desel = selected_count_label.clone();
        deselect_all_btn.connect_clicked(move |_| {
            for row in all_rows_desel.borrow().iter() { row.checkbox.set_active(false); }
            selected_count_desel.set_label("0 selected");
        });

        // Navigation
        let content_stack_nav = content_stack.clone();
        let current_view_nav = current_view.clone();
        nav_list.connect_row_selected(move |_, row| {
            if let Some(row) = row {
                match row.index() {
                    0 => { content_stack_nav.set_visible_child_name("all"); *current_view_nav.borrow_mut() = ViewMode::AllPackages; }
                    1 => { content_stack_nav.set_visible_child_name("updates"); *current_view_nav.borrow_mut() = ViewMode::Updates; }
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

        update_all_btn.connect_clicked(move |btn| {
            btn.set_sensitive(false);
            let pm = pm_all.clone();
            let packages = packages_all.clone();
            let toast = toast_all.clone();
            let progress_overlay = progress_overlay_all.clone();
            let progress_bar = progress_bar_all.clone();
            let progress_label = progress_label_all.clone();
            let btn = btn.clone();
            let load_fn = load_fn_all.clone();

            let updates: Vec<Package> = packages.borrow().iter().filter(|p| p.has_update()).cloned().collect();
            if updates.is_empty() { btn.set_sensitive(true); return; }

            progress_overlay.set_visible(true);
            progress_label.set_label(&format!("Updating {} packages...", updates.len()));

            glib::spawn_future_local(async move {
                let total = updates.len();
                let mut success = 0;
                let manager = pm.lock().await;
                for (i, pkg) in updates.iter().enumerate() {
                    progress_bar.set_fraction((i as f64) / (total as f64));
                    progress_bar.set_text(Some(&format!("{}/{} - {}", i + 1, total, pkg.name)));
                    if manager.update(pkg).await.is_ok() { success += 1; }
                }
                drop(manager);
                progress_overlay.set_visible(false);
                btn.set_sensitive(true);
                let t = adw::Toast::new(&format!("Updated {}/{} packages", success, total));
                t.set_timeout(5);
                toast.add_toast(t);
                load_fn();
            });
        });

        // Update Selected
        let all_rows_upd = all_rows.clone();
        let pm_upd = pm.clone();
        let toast_upd = toast_overlay.clone();
        let progress_overlay_upd = progress_overlay.clone();
        let progress_bar_upd = progress_bar.clone();
        let progress_label_upd = progress_label.clone();
        let load_fn_upd = load_packages.clone();

        update_selected_btn.connect_clicked(move |btn| {
            let selected: Vec<Package> = all_rows_upd.borrow().iter().filter(|r| r.checkbox.is_active() && r.package.borrow().has_update()).map(|r| r.package.borrow().clone()).collect();
            if selected.is_empty() {
                let toast = adw::Toast::new("No updatable packages selected");
                toast.set_timeout(2);
                toast_upd.add_toast(toast);
                return;
            }
            btn.set_sensitive(false);
            let pm = pm_upd.clone();
            let toast = toast_upd.clone();
            let progress_overlay = progress_overlay_upd.clone();
            let progress_bar = progress_bar_upd.clone();
            let progress_label = progress_label_upd.clone();
            let btn = btn.clone();
            let load_fn = load_fn_upd.clone();

            progress_overlay.set_visible(true);
            progress_label.set_label(&format!("Updating {} packages...", selected.len()));

            glib::spawn_future_local(async move {
                let total = selected.len();
                let mut success = 0;
                let manager = pm.lock().await;
                for (i, pkg) in selected.iter().enumerate() {
                    progress_bar.set_fraction((i as f64) / (total as f64));
                    progress_bar.set_text(Some(&format!("{}/{} - {}", i + 1, total, pkg.name)));
                    if manager.update(pkg).await.is_ok() { success += 1; }
                }
                drop(manager);
                progress_overlay.set_visible(false);
                btn.set_sensitive(true);
                let t = adw::Toast::new(&format!("Updated {}/{} packages", success, total));
                t.set_timeout(5);
                toast.add_toast(t);
                load_fn();
            });
        });

        // Remove Selected
        let all_rows_rem = all_rows.clone();
        let pm_rem = pm.clone();
        let toast_rem = toast_overlay.clone();
        let progress_overlay_rem = progress_overlay.clone();
        let progress_bar_rem = progress_bar.clone();
        let progress_label_rem = progress_label.clone();
        let load_fn_rem = load_packages.clone();

        remove_selected_btn.connect_clicked(move |btn| {
            let selected: Vec<Package> = all_rows_rem.borrow().iter()
                .filter(|r| r.checkbox.is_active())
                .map(|r| r.package.borrow().clone())
                .collect();

            if selected.is_empty() {
                let toast = adw::Toast::new("No packages selected");
                toast.set_timeout(2);
                toast_rem.add_toast(toast);
                return;
            }

            btn.set_sensitive(false);
            let pm = pm_rem.clone();
            let toast = toast_rem.clone();
            let progress_overlay = progress_overlay_rem.clone();
            let progress_bar = progress_bar_rem.clone();
            let progress_label = progress_label_rem.clone();
            let btn = btn.clone();
            let load_fn = load_fn_rem.clone();

            progress_overlay.set_visible(true);
            progress_label.set_label(&format!("Removing {} packages...", selected.len()));

            glib::spawn_future_local(async move {
                let total = selected.len();
                let mut success = 0;
                let manager = pm.lock().await;
                for (i, pkg) in selected.iter().enumerate() {
                    progress_bar.set_fraction((i as f64) / (total as f64));
                    progress_bar.set_text(Some(&format!("{}/{} - {}", i + 1, total, pkg.name)));
                    if manager.remove(pkg).await.is_ok() { success += 1; }
                }
                drop(manager);
                progress_overlay.set_visible(false);
                btn.set_sensitive(true);
                let t = adw::Toast::new(&format!("Removed {}/{} packages", success, total));
                t.set_timeout(5);
                toast.add_toast(t);
                load_fn();
            });
        });

        // Window Close
        let config_state = self.config.clone();
        self.window.connect_close_request(move |window| {
            let mut cfg = config_state.borrow_mut();
            cfg.window_maximized = window.is_maximized();
            if !cfg.window_maximized {
                cfg.window_width = window.width();
                cfg.window_height = window.height();
            }
            let _ = cfg.save();
            glib::Propagation::Proceed
        });

        // Shortcuts
        let search_entry_focus = search_entry.clone();
        let controller = gtk::EventControllerKey::new();
        let refresh_fn = load_packages.clone();
        let select_btn = select_button.clone();
        controller.connect_key_pressed(move |_, key, _, modifier| {
            if modifier.contains(gtk::gdk::ModifierType::CONTROL_MASK) {
                match key {
                    gtk::gdk::Key::f => { search_entry_focus.grab_focus(); return glib::Propagation::Stop; }
                    gtk::gdk::Key::r => { refresh_fn(); return glib::Propagation::Stop; }
                    gtk::gdk::Key::s => { select_btn.set_active(!select_btn.is_active()); return glib::Propagation::Stop; }
                    _ => {}
                }
            }
            glib::Propagation::Proceed
        });
        self.window.add_controller(controller);
    }

    fn populate_list<F>(
        list_box: &gtk::ListBox,
        packages: &[Package],
        rows: &Rc<RefCell<Vec<PackageRow>>>,
        window: &adw::ApplicationWindow,
        pm: &Arc<Mutex<PackageManager>>,
        toast_overlay: &adw::ToastOverlay,
        config: &Rc<RefCell<Config>>,
        selection_mode: bool,
        on_source_click: F,
    ) where
        F: Fn(PackageSource) + Clone + 'static,
    {
        while let Some(child) = list_box.first_child() {
            list_box.remove(&child);
        }
        rows.borrow_mut().clear();

        for package in packages {
            let row = PackageRow::new(package.clone(), None);
            row.set_selection_mode(selection_mode);

            let pkg = package.clone();
            let win = window.clone();
            let pm_details = pm.clone();
            let toast_details = toast_overlay.clone();
            let config_details = config.clone();
            
            row.widget.connect_activated(move |_| {
                PackageDetailsDialog::show(&pkg, &win, pm_details.clone(), toast_details.clone(), config_details.clone());
            });

            let source = package.source;
            let on_source_click_clone = on_source_click.clone();
            row.source_button.connect_clicked(move |_| {
                on_source_click_clone(source);
            });

            let pkg_action = package.clone();
            let pm_action = pm.clone();
            let toast_action = toast_overlay.clone();
            let spinner = row.spinner.clone();
            let action_btn = row.action_button.clone();

            row.action_button.connect_clicked(move |_| {
                let pkg = pkg_action.clone();
                let pm = pm_action.clone();
                let toast = toast_action.clone();
                let spinner = spinner.clone();
                let btn = action_btn.clone();

                btn.set_visible(false);
                spinner.set_visible(true);
                spinner.start();

                glib::spawn_future_local(async move {
                    let manager = pm.lock().await;
                    let result = match pkg.status {
                        PackageStatus::UpdateAvailable => manager.update(&pkg).await,
                        PackageStatus::Installed => manager.remove(&pkg).await,
                        PackageStatus::NotInstalled => manager.install(&pkg).await,
                        _ => Ok(()),
                    };
                    drop(manager);

                    spinner.stop();
                    spinner.set_visible(false);
                    btn.set_visible(true);

                    let msg = match result {
                        Ok(_) => match pkg.status {
                            PackageStatus::UpdateAvailable => format!("Updated {}", pkg.name),
                            PackageStatus::Installed => format!("Removed {}", pkg.name),
                            PackageStatus::NotInstalled => format!("Installed {}", pkg.name),
                            _ => "Operation completed".to_string(),
                        },
                        Err(e) => format!("Error: {}", e),
                    };

                    let t = adw::Toast::new(&msg);
                    t.set_timeout(3);
                    toast.add_toast(t);
                });
            });

            list_box.append(&row.widget);
            rows.borrow_mut().push(row);
        }
    }

    pub fn present(&self) {
        self.window.present();
    }
}