use crate::backend::PackageManager;
use crate::models::{Config, Package, PackageCache, PackageSource, PackageStatus};
use crate::ui::{
    show_about_dialog, DiagnosticsDialog, PackageDetailsDialog, PackageRow, PreferencesDialog,
};
use gtk4::prelude::*;
use gtk4::{self as gtk, gio, glib};
use libadwaita as adw;
use libadwaita::prelude::*;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
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

fn show_action_required_dialog(parent: &adw::ApplicationWindow, details: &str, command: &str) {
    let win = gtk::Window::builder()
        .title("Action required")
        .modal(true)
        .transient_for(parent)
        .default_width(640)
        .default_height(240)
        .build();

    let header = adw::HeaderBar::new();
    let copy_btn = gtk::Button::builder().label("Copy command").build();
    copy_btn.add_css_class("suggested-action");
    header.pack_end(&copy_btn);

    let close_btn = gtk::Button::builder().label("Close").build();
    header.pack_start(&close_btn);

    let details_label = gtk::Label::builder()
        .label(details)
        .wrap(true)
        .xalign(0.0)
        .build();
    details_label.add_css_class("dim-label");

    let command_entry = gtk::Entry::builder().text(command).editable(false).build();
    command_entry.add_css_class("monospace");

    let content = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(12)
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .build();
    content.append(&details_label);
    content.append(&command_entry);

    let root = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .build();
    root.append(&header);
    root.append(&content);
    win.set_child(Some(&root));

    let cmd = command.to_string();
    copy_btn.connect_clicked({
        let command_entry = command_entry.clone();
        move |_| {
            command_entry.select_region(0, -1);
            if let Some(display) = gtk::gdk::Display::default() {
                display.clipboard().set_text(&cmd);
                display.primary_clipboard().set_text(&cmd);
            }
        }
    });
    close_btn.connect_clicked({
        let win = win.clone();
        move |_| win.close()
    });

    win.present();
}

fn try_remove_source(id: glib::SourceId) {
    unsafe {
        glib::ffi::g_source_remove(id.as_raw());
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ViewMode {
    Discover,
    AllPackages,
    Updates,
}

/// Filter state for the package list
#[derive(Clone, Default)]
struct FilterState {
    sources: Vec<PackageSource>,
    search_query: String,
}

type LocalFn = Rc<dyn Fn()>;
type LocalFnHolder = Rc<RefCell<Option<LocalFn>>>;

struct SidebarWidgets {
    sidebar: gtk::Box,
    nav_list: gtk::ListBox,
    all_count_label: gtk::Label,
    update_count_label: gtk::Label,
    sources_box: gtk::Box,
    sources_filter_badge: gtk::Label,
    sources_reset_btn: gtk::Button,
    sources_all_btn: gtk::ToggleButton,
    source_buttons: HashMap<PackageSource, gtk::ToggleButton>,
    source_counts: HashMap<PackageSource, gtk::Label>,
    sources_select_all_btn: gtk::Button,
    sources_unselect_all_btn: gtk::Button,
}

struct ContentWidgets {
    content_area: gtk::Box,
    discover_stack: gtk::Stack,
    all_stack: gtk::Stack,
    updates_stack: gtk::Stack,
    discover_list_box: gtk::ListBox,
    all_list_box: gtk::ListBox,
    updates_list_box: gtk::ListBox,
    content_stack: gtk::Stack,
    sort_dropdown: gtk::DropDown,
    update_all_btn: gtk::Button,
}

pub struct LinGetWindow {
    pub window: adw::ApplicationWindow,
    package_manager: Arc<Mutex<PackageManager>>,
    available_sources: Rc<RefCell<HashSet<PackageSource>>>,
    enabled_sources: Rc<RefCell<HashSet<PackageSource>>>,
    packages: Rc<RefCell<Vec<Package>>>,
    config: Rc<RefCell<Config>>,
    filter_state: Rc<RefCell<FilterState>>,
    selection_mode: Rc<RefCell<bool>>,
    // Discover view
    discover_list_box: gtk::ListBox,
    discover_rows: Rc<RefCell<Vec<PackageRow>>>,
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
    source_count_labels: HashMap<PackageSource, gtk::Label>,
    source_filter_buttons: HashMap<PackageSource, gtk::ToggleButton>,
    sources_box: gtk::Box,
    sources_filter_badge: gtk::Label,
    sources_all_btn: gtk::ToggleButton,
    sources_reset_btn: gtk::Button,
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
        let all_rows: Rc<RefCell<Vec<PackageRow>>> = Rc::new(RefCell::new(Vec::new()));
        let updates_rows: Rc<RefCell<Vec<PackageRow>>> = Rc::new(RefCell::new(Vec::new()));
        let current_view = Rc::new(RefCell::new(ViewMode::AllPackages));
        let filter_state = Rc::new(RefCell::new(FilterState::default()));
        let selection_mode = Rc::new(RefCell::new(false));

        // Create UI components
        let (header, search_entry, refresh_button, select_button) = Self::build_header();

        let sidebar_widgets = Self::build_sidebar();

        let content_widgets = Self::build_content_area();

        let (progress_overlay, progress_bar, progress_label) = Self::build_progress_overlay();

        let (
            selection_bar,
            select_all_btn,
            deselect_all_btn,
            update_selected_btn,
            remove_selected_btn,
            selected_count_label,
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
            .label("Loading your packages…")
            .wrap(true)
            .justify(gtk::Justification::Center)
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

        content_with_bars.append(&content_widgets.content_area);
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

        main_paned.append(&sidebar_widgets.sidebar);
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
            available_sources,
            enabled_sources,
            packages,
            config,
            filter_state,
            selection_mode,
            discover_list_box: content_widgets.discover_list_box.clone(),
            discover_rows,
            all_list_box: content_widgets.all_list_box.clone(),
            all_rows,
            updates_list_box: content_widgets.updates_list_box.clone(),
            updates_rows,
            search_entry,
            content_stack: content_widgets.content_stack.clone(),
            main_stack,
            spinner,
            toast_overlay,
            current_view,
            all_count_label: sidebar_widgets.all_count_label.clone(),
            update_count_label: sidebar_widgets.update_count_label.clone(),
            source_count_labels: sidebar_widgets.source_counts.clone(),
            source_filter_buttons: sidebar_widgets.source_buttons.clone(),
            sources_box: sidebar_widgets.sources_box.clone(),
            sources_filter_badge: sidebar_widgets.sources_filter_badge.clone(),
            sources_all_btn: sidebar_widgets.sources_all_btn.clone(),
            sources_reset_btn: sidebar_widgets.sources_reset_btn.clone(),
            progress_overlay,
            progress_bar,
            progress_label,
            selection_bar,
            selected_count_label,
        };

        let reload_packages = win.setup_signals(
            refresh_button,
            select_button,
            sidebar_widgets.nav_list.clone(),
            content_widgets.update_all_btn.clone(),
            content_widgets.discover_stack.clone(),
            content_widgets.all_stack.clone(),
            content_widgets.updates_stack.clone(),
            sidebar_widgets.sources_select_all_btn.clone(),
            sidebar_widgets.sources_unselect_all_btn.clone(),
            select_all_btn,
            deselect_all_btn,
            update_selected_btn,
            remove_selected_btn,
            content_widgets.sort_dropdown.clone(),
        );
        win.setup_actions(app, reload_packages);

        win
    }

    fn build_header() -> (
        adw::HeaderBar,
        gtk::SearchEntry,
        gtk::Button,
        gtk::ToggleButton,
    ) {
        let header = adw::HeaderBar::new();

        // Menu
        let menu = gio::Menu::new();

        let backup_section = gio::Menu::new();
        backup_section.append(Some("Import Packages..."), Some("app.import"));
        backup_section.append(Some("Export Packages..."), Some("app.export"));
        menu.append_section(Some("Backup"), &backup_section);

        let app_section = gio::Menu::new();
        app_section.append(Some("Preferences"), Some("app.preferences"));
        app_section.append(Some("Diagnostics"), Some("app.diagnostics"));
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

    fn build_sidebar() -> SidebarWidgets {
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

        let app_title = gtk::Label::builder().label("LinGet").xalign(0.0).build();
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

        // Discover
        let discover_row = gtk::ListBoxRow::new();
        discover_row.add_css_class("nav-row");
        let discover_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(12)
            .margin_top(10)
            .margin_bottom(10)
            .margin_start(12)
            .margin_end(12)
            .build();
        discover_box.append(&gtk::Image::from_icon_name("system-search-symbolic"));
        discover_box.append(
            &gtk::Label::builder()
                .label("Discover")
                .hexpand(true)
                .xalign(0.0)
                .build(),
        );
        discover_row.set_child(Some(&discover_box));
        nav_list.append(&discover_row);

        // All Packages
        let all_row = gtk::ListBoxRow::new();
        all_row.add_css_class("nav-row");
        let all_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(12)
            .margin_top(10)
            .margin_bottom(10)
            .margin_start(12)
            .margin_end(12)
            .build();
        let all_count_label = gtk::Label::builder()
            .label("0")
            .css_classes(vec!["dim-label", "caption"])
            .build();
        all_box.append(&gtk::Image::from_icon_name("view-grid-symbolic"));
        all_box.append(
            &gtk::Label::builder()
                .label("Library")
                .hexpand(true)
                .xalign(0.0)
                .build(),
        );
        all_box.append(&all_count_label);
        all_row.set_child(Some(&all_box));
        nav_list.append(&all_row);

        // Updates
        let updates_row = gtk::ListBoxRow::new();
        updates_row.add_css_class("nav-row");
        let updates_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(12)
            .margin_top(10)
            .margin_bottom(10)
            .margin_start(12)
            .margin_end(12)
            .build();
        let update_count_label = gtk::Label::builder()
            .label("0")
            .css_classes(vec!["badge-accent"])
            .visible(false)
            .build();
        updates_box.append(&gtk::Image::from_icon_name(
            "software-update-available-symbolic",
        ));
        updates_box.append(
            &gtk::Label::builder()
                .label("Updates")
                .hexpand(true)
                .xalign(0.0)
                .build(),
        );
        updates_box.append(&update_count_label);
        updates_row.set_child(Some(&updates_box));
        nav_list.append(&updates_row);

        nav_list.select_row(Some(&all_row));
        sidebar_box.append(&nav_list);

        // Sources header + actions
        let sources_header = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(8)
            .margin_top(24)
            .margin_start(16)
            .margin_end(16)
            .margin_bottom(8)
            .build();

        let sources_label = gtk::Label::builder()
            .label("Sources")
            .xalign(0.0)
            .hexpand(true)
            .build();
        sources_label.add_css_class("caption");
        sources_label.add_css_class("dim-label");

        let sources_filter_badge = gtk::Label::builder().label("All").build();
        sources_filter_badge.add_css_class("chip");
        sources_filter_badge.add_css_class("chip-muted");

        let sources_reset_btn = gtk::Button::builder()
            .label("Reset")
            .tooltip_text("Clear search and source filters")
            .build();
        sources_reset_btn.add_css_class("flat");
        sources_reset_btn.add_css_class("caption");

        let sources_unselect_all_btn = gtk::Button::builder()
            .label("Unselect")
            .tooltip_text("Clear selected source filters (show all)")
            .build();
        sources_unselect_all_btn.add_css_class("flat");
        sources_unselect_all_btn.add_css_class("caption");

        let sources_select_all_btn = gtk::Button::builder()
            .label("Select All")
            .tooltip_text("Select all available sources (filter to all)")
            .build();
        sources_select_all_btn.add_css_class("flat");
        sources_select_all_btn.add_css_class("caption");

        sources_header.append(&sources_label);
        sources_header.append(&sources_filter_badge);
        sources_header.append(&sources_reset_btn);
        sources_header.append(&sources_unselect_all_btn);
        sources_header.append(&sources_select_all_btn);
        sidebar_box.append(&sources_header);

        let sources_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(2)
            .margin_start(8)
            .margin_end(8)
            .build();

        let mut source_buttons = HashMap::new();
        let mut source_counts = HashMap::new();

        let (sources_all_btn, _) =
            Self::create_source_filter_btn("All", "view-grid-symbolic", "source-all");
        sources_all_btn.set_active(true);
        sources_all_btn.set_sensitive(true);
        sources_box.append(&sources_all_btn);

        let mut add_source = |source: PackageSource, label: &str, icon: &str, css: &str| {
            let (btn, count) = Self::create_source_filter_btn(label, icon, css);
            sources_box.append(&btn);
            source_buttons.insert(source, btn);
            source_counts.insert(source, count);
        };

        add_source(
            PackageSource::Apt,
            "APT",
            "package-x-generic-symbolic",
            "source-apt",
        );
        add_source(
            PackageSource::Dnf,
            "DNF",
            "system-software-install-symbolic",
            "source-dnf",
        );
        add_source(
            PackageSource::Pacman,
            "Pacman",
            "package-x-generic-symbolic",
            "source-pacman",
        );
        add_source(
            PackageSource::Zypper,
            "Zypper",
            "system-software-install-symbolic",
            "source-zypper",
        );
        add_source(
            PackageSource::Flatpak,
            "Flatpak",
            "system-software-install-symbolic",
            "source-flatpak",
        );
        add_source(
            PackageSource::Snap,
            "Snap",
            "package-x-generic-symbolic",
            "source-snap",
        );
        add_source(
            PackageSource::Npm,
            "npm",
            "text-x-script-symbolic",
            "source-npm",
        );
        add_source(
            PackageSource::Pip,
            "pip",
            "text-x-python-symbolic",
            "source-pip",
        );
        add_source(
            PackageSource::Pipx,
            "pipx",
            "text-x-python-symbolic",
            "source-pipx",
        );
        add_source(
            PackageSource::Cargo,
            "cargo",
            "applications-development-symbolic",
            "source-cargo",
        );
        add_source(
            PackageSource::Brew,
            "brew",
            "application-x-executable-symbolic",
            "source-brew",
        );
        add_source(
            PackageSource::Aur,
            "AUR",
            "package-x-generic-symbolic",
            "source-aur",
        );
        add_source(
            PackageSource::Conda,
            "conda",
            "text-x-python-symbolic",
            "source-conda",
        );
        add_source(
            PackageSource::Mamba,
            "mamba",
            "text-x-python-symbolic",
            "source-mamba",
        );
        add_source(
            PackageSource::Dart,
            "dart",
            "applications-development-symbolic",
            "source-dart",
        );
        add_source(
            PackageSource::Deb,
            "Deb",
            "package-x-generic-symbolic",
            "source-deb",
        );
        add_source(
            PackageSource::AppImage,
            "AppImage",
            "application-x-executable-symbolic",
            "source-appimage",
        );

        sidebar_box.append(&sources_box);

        // Spacer and Footer
        sidebar_box.append(&gtk::Box::builder().vexpand(true).build());

        let stats_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .margin_start(16)
            .margin_end(16)
            .margin_bottom(16)
            .spacing(4)
            .build();
        let stats_label = gtk::Label::builder()
            .label("Last updated: Just now")
            .xalign(0.0)
            .build();
        stats_label.add_css_class("caption");
        stats_label.add_css_class("dim-label");
        stats_box.append(&stats_label);
        sidebar_box.append(&stats_box);

        SidebarWidgets {
            sidebar: sidebar_box,
            nav_list,
            all_count_label,
            update_count_label,
            sources_box,
            sources_filter_badge,
            sources_reset_btn,
            sources_all_btn,
            source_buttons,
            source_counts,
            sources_select_all_btn,
            sources_unselect_all_btn,
        }
    }

    fn build_content_area() -> ContentWidgets {
        let content_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .hexpand(true)
            .build();

        let content_stack = gtk::Stack::builder()
            .transition_type(gtk::StackTransitionType::SlideLeftRight)
            .transition_duration(200)
            .hexpand(true)
            .build();

        // Discover View
        let discover_list_box = gtk::ListBox::builder()
            .selection_mode(gtk::SelectionMode::None)
            .css_classes(vec!["boxed-list"])
            .build();
        let discover_scrolled = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vexpand(true)
            .child(&discover_list_box)
            .build();

        let discover_content = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();
        let discover_label = gtk::Label::builder()
            .label("Search for new packages across all enabled providers")
            .css_classes(vec!["dim-label"])
            .margin_top(24)
            .margin_bottom(24)
            .build();
        discover_content.append(&discover_label);
        discover_content.append(
            &adw::Clamp::builder()
                .maximum_size(1000)
                .child(&discover_scrolled)
                .margin_top(8)
                .margin_bottom(24)
                .margin_start(12)
                .margin_end(12)
                .build(),
        );

        let discover_empty = adw::StatusPage::builder()
            .icon_name("system-search-symbolic")
            .title("Discover Packages")
            .description("Type in the search bar to find new software")
            .build();

        let discover_stack = gtk::Stack::builder()
            .transition_type(gtk::StackTransitionType::Crossfade)
            .transition_duration(150)
            .build();
        discover_stack.add_named(&discover_content, Some("list"));
        discover_stack.add_named(&discover_empty, Some("empty"));
        discover_stack.set_visible_child_name("empty");

        // All Packages View
        let filter_bar = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(8)
            .margin_start(24)
            .margin_end(24)
            .margin_top(12)
            .margin_bottom(8)
            .build();
        filter_bar.append(&gtk::Label::new(Some("Filter:")));
        let sort_options =
            gtk::StringList::new(&["Name (A-Z)", "Name (Z-A)", "Source", "Recently Added"]);
        let sort_dropdown = gtk::DropDown::builder()
            .model(&sort_options)
            .tooltip_text("Sort by")
            .build();
        sort_dropdown.add_css_class("flat");
        let sort_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .hexpand(true)
            .halign(gtk::Align::End)
            .spacing(8)
            .build();
        sort_box.append(&gtk::Label::new(Some("Sort:")));
        sort_box.append(&sort_dropdown);
        filter_bar.append(&sort_box);

        let all_list_box = gtk::ListBox::builder()
            .selection_mode(gtk::SelectionMode::None)
            .css_classes(vec!["boxed-list"])
            .build();
        let all_scrolled = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vexpand(true)
            .child(&all_list_box)
            .build();
        let all_content = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();
        all_content.append(&filter_bar);
        all_content.append(
            &adw::Clamp::builder()
                .maximum_size(1000)
                .child(&all_scrolled)
                .margin_top(8)
                .margin_bottom(24)
                .margin_start(12)
                .margin_end(12)
                .build(),
        );

        let all_empty = adw::StatusPage::builder()
            .icon_name("package-x-generic-symbolic")
            .title("No Packages Found")
            .description("Try adjusting your search or filters")
            .build();
        let all_stack = gtk::Stack::builder()
            .transition_type(gtk::StackTransitionType::Crossfade)
            .transition_duration(150)
            .build();
        all_stack.add_named(&all_content, Some("list"));
        all_stack.add_named(&all_empty, Some("empty"));

        // Updates View
        let updates_list_box = gtk::ListBox::builder()
            .selection_mode(gtk::SelectionMode::None)
            .css_classes(vec!["boxed-list"])
            .build();
        let updates_scrolled = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vexpand(true)
            .child(&updates_list_box)
            .build();
        let updates_header = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .margin_start(24)
            .margin_end(24)
            .margin_top(16)
            .margin_bottom(8)
            .build();
        let updates_title = gtk::Label::builder()
            .label("Available Updates")
            .hexpand(true)
            .xalign(0.0)
            .build();
        updates_title.add_css_class("title-3");
        let update_all_btn = gtk::Button::builder()
            .label("Update All")
            .css_classes(vec!["suggested-action", "pill"])
            .build();
        updates_header.append(&updates_title);
        updates_header.append(&update_all_btn);

        let updates_content = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();
        updates_content.append(&updates_header);
        updates_content.append(
            &adw::Clamp::builder()
                .maximum_size(1000)
                .child(&updates_scrolled)
                .margin_top(8)
                .margin_bottom(24)
                .margin_start(12)
                .margin_end(12)
                .build(),
        );

        let updates_empty = adw::StatusPage::builder()
            .icon_name("emblem-ok-symbolic")
            .title("All Up to Date!")
            .description("All your packages are running the latest versions")
            .build();
        updates_empty.add_css_class("success-status");
        let updates_stack = gtk::Stack::builder()
            .transition_type(gtk::StackTransitionType::Crossfade)
            .transition_duration(150)
            .build();
        updates_stack.add_named(&updates_content, Some("list"));
        updates_stack.add_named(&updates_empty, Some("empty"));

        content_stack.add_named(&discover_stack, Some("discover"));
        content_stack.add_named(&all_stack, Some("all"));
        content_stack.add_named(&updates_stack, Some("updates"));
        content_stack.set_visible_child_name("all");
        content_box.append(&content_stack);

        ContentWidgets {
            content_area: content_box,
            discover_stack,
            all_stack,
            updates_stack,
            discover_list_box,
            all_list_box,
            updates_list_box,
            content_stack,
            sort_dropdown,
            update_all_btn,
        }
    }

    fn build_progress_overlay() -> (gtk::Box, gtk::ProgressBar, gtk::Label) {
        // Full-screen scrim + centered card (modern "modal progress" look).
        let overlay = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .valign(gtk::Align::Fill)
            .halign(gtk::Align::Fill)
            .vexpand(true)
            .hexpand(true)
            .visible(false)
            .build();
        overlay.add_css_class("progress-scrim");

        let card = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .valign(gtk::Align::Center)
            .halign(gtk::Align::Center)
            .spacing(12)
            .margin_start(24)
            .margin_end(24)
            .build();
        card.add_css_class("progress-card");

        let label = gtk::Label::builder().label("Working…").wrap(true).build();
        label.add_css_class("title-3");
        label.set_max_width_chars(60);
        label.set_wrap_mode(gtk::pango::WrapMode::WordChar);

        let bar = gtk::ProgressBar::builder().show_text(true).build();
        bar.add_css_class("osd");
        bar.set_height_request(10);

        card.append(&label);
        card.append(&bar);
        overlay.append(&card);
        (overlay, bar, label)
    }

    fn build_selection_bar() -> (
        gtk::ActionBar,
        gtk::Button,
        gtk::Button,
        gtk::Button,
        gtk::Button,
        gtk::Label,
    ) {
        let bar = gtk::ActionBar::builder().visible(false).build();
        bar.add_css_class("selection-bar");

        let select_all = gtk::Button::builder().label("Select All").build();
        select_all.add_css_class("flat");
        let deselect_all = gtk::Button::builder().label("Deselect All").build();
        deselect_all.add_css_class("flat");
        let count_label = gtk::Label::builder()
            .label("0 selected")
            .hexpand(true)
            .build();
        let update_btn = gtk::Button::builder().label("Update Selected").build();
        update_btn.add_css_class("suggested-action");
        let remove_btn = gtk::Button::builder().label("Remove Selected").build();
        remove_btn.add_css_class("destructive-action");

        bar.pack_start(&select_all);
        bar.pack_start(&deselect_all);
        bar.set_center_widget(Some(&count_label));
        bar.pack_end(&remove_btn);
        bar.pack_end(&update_btn);

        (
            bar,
            select_all,
            deselect_all,
            update_btn,
            remove_btn,
            count_label,
        )
    }

    fn create_source_filter_btn(
        label: &str,
        icon: &str,
        css_class: &str,
    ) -> (gtk::ToggleButton, gtk::Label) {
        let btn_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(8)
            .build();
        let count_label = gtk::Label::new(Some("0"));
        count_label.add_css_class("dim-label");
        count_label.add_css_class("caption");

        if css_class != "source-all" {
            let dot = gtk::Box::builder()
                .width_request(10)
                .height_request(10)
                .valign(gtk::Align::Center)
                .build();
            dot.add_css_class("source-dot");
            dot.add_css_class(css_class);
            btn_box.append(&dot);
        }
        btn_box.append(&gtk::Image::from_icon_name(icon));
        btn_box.append(
            &gtk::Label::builder()
                .label(label)
                .hexpand(true)
                .xalign(0.0)
                .build(),
        );
        btn_box.append(&count_label);

        let btn = gtk::ToggleButton::builder()
            .child(&btn_box)
            .active(false)
            .build();
        btn.add_css_class("flat");
        btn.add_css_class("source-filter-btn");

        (btn, count_label)
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
            .build();
        let group = gtk::ShortcutsGroup::builder().title("General").build();

        for (title, accel) in [
            ("Search", "<Ctrl>f"),
            ("Refresh", "<Ctrl>r"),
            ("Selection Mode", "<Ctrl>s"),
            ("Preferences", "<Ctrl>comma"),
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
        dialog.present();
    }

    #[allow(clippy::too_many_arguments)]
    fn setup_signals(
        &self,
        refresh_button: gtk::Button,
        select_button: gtk::ToggleButton,
        nav_list: gtk::ListBox,
        update_all_btn: gtk::Button,
        discover_stack: gtk::Stack,
        all_stack: gtk::Stack,
        updates_stack: gtk::Stack,
        sources_select_all_btn: gtk::Button,
        sources_unselect_all_btn: gtk::Button,
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
        let discover_list_box = self.discover_list_box.clone();
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
        let source_count_labels = self.source_count_labels.clone();
        let source_filter_buttons = self.source_filter_buttons.clone();
        let sources_box = self.sources_box.clone();
        let sources_filter_badge = self.sources_filter_badge.clone();
        let sources_all_btn = self.sources_all_btn.clone();
        let sources_reset_btn = self.sources_reset_btn.clone();
        let progress_overlay = self.progress_overlay.clone();
        let progress_bar = self.progress_bar.clone();
        let progress_label = self.progress_label.clone();
        let selection_bar = self.selection_bar.clone();
        let selected_count_label = self.selected_count_label.clone();

        // Helpers
        let enabled_sources_for_counts = enabled_sources.clone();
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

            for (source, label) in &source_count_labels {
                let count = enabled_packages
                    .iter()
                    .filter(|p| p.source == *source)
                    .count();
                label.set_label(&count.to_string());
            }
        };

        let skip_filter = Rc::new(RefCell::new(false));
        let apply_filters_holder: LocalFnHolder = Rc::new(RefCell::new(None));
        let reload_holder: LocalFnHolder = Rc::new(RefCell::new(None));

        let apply_filters: Rc<dyn Fn()> = Rc::new({
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
            let source_filter_buttons = source_filter_buttons.clone();
            let enabled_sources = enabled_sources.clone();
            let reload_holder = reload_holder.clone();
            let progress_overlay = progress_overlay.clone();
            let progress_bar = progress_bar.clone();
            let progress_label = progress_label.clone();
            let sources_filter_badge = sources_filter_badge.clone();
            let sources_all_btn = sources_all_btn.clone();

            move || {
                if *skip_filter.borrow() {
                    return;
                }
                let enabled = enabled_sources.borrow();

                let all_packages = packages.borrow();
                let filter = filter_state.borrow();
                let sel_mode = *selection_mode.borrow();

                let filtered_all: Vec<Package> = all_packages
                    .iter()
                    .filter(|p| {
                        enabled.contains(&p.source)
                            && (filter.sources.is_empty() || filter.sources.contains(&p.source))
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

                // Source filter badge
                {
                    let count = filter.sources.len();
                    let badge = if count == 0 {
                        "All".to_string()
                    } else {
                        count.to_string()
                    };
                    sources_filter_badge.set_label(&badge);

                    *skip_filter.borrow_mut() = true;
                    sources_all_btn.set_active(count == 0);
                    *skip_filter.borrow_mut() = false;
                }

                let on_source_click = {
                    let filter_state = filter_state.clone();
                    let skip_filter = skip_filter.clone();
                    let apply_filters_holder = apply_filters_holder.clone();
                    let source_filter_buttons = source_filter_buttons.clone();
                    let enabled_sources = enabled_sources.clone();
                    let sources_all_btn = sources_all_btn.clone();

                    move |source: PackageSource| {
                        if !enabled_sources.borrow().contains(&source) {
                            return;
                        }

                        *skip_filter.borrow_mut() = true;
                        filter_state.borrow_mut().sources = vec![source];
                        for (s, btn) in &source_filter_buttons {
                            btn.set_active(*s == source);
                        }
                        sources_all_btn.set_active(false);
                        *skip_filter.borrow_mut() = false;
                        if let Some(apply) = apply_filters_holder.borrow().as_ref() {
                            apply();
                        }
                    }
                };

                let reload_packages = reload_holder.borrow().clone();
                Self::populate_list(
                    &all_list_box,
                    &filtered_all,
                    &all_rows,
                    &window,
                    &pm,
                    &toast_overlay,
                    &config,
                    sel_mode,
                    on_source_click.clone(),
                    reload_packages.clone(),
                    &progress_overlay,
                    &progress_bar,
                    &progress_label,
                );
                Self::populate_list(
                    &updates_list_box,
                    &filtered_updates,
                    &updates_rows,
                    &window,
                    &pm,
                    &toast_overlay,
                    &config,
                    sel_mode,
                    on_source_click,
                    reload_packages,
                    &progress_overlay,
                    &progress_bar,
                    &progress_label,
                );

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
            }
        });

        *apply_filters_holder.borrow_mut() = Some(apply_filters.clone());

        let apply_enabled_sources_ui: Rc<dyn Fn()> = Rc::new({
            let enabled_sources = enabled_sources.clone();
            let available_sources = available_sources.clone();
            let filter_state = filter_state.clone();
            let source_filter_buttons = source_filter_buttons.clone();
            let sources_box = sources_box.clone();
            let skip_filter = skip_filter.clone();
            let apply_filters = apply_filters.clone();

            move || {
                let enabled = enabled_sources.borrow().clone();
                let available = available_sources.borrow().clone();

                *skip_filter.borrow_mut() = true;
                let mut sources: Vec<PackageSource> =
                    source_filter_buttons.keys().copied().collect();
                sources.sort_by(|a, b| {
                    let a_key = (!available.contains(a), !enabled.contains(a), a.to_string());
                    let b_key = (!available.contains(b), !enabled.contains(b), b.to_string());
                    a_key.cmp(&b_key)
                });

                while let Some(child) = sources_box.first_child() {
                    sources_box.remove(&child);
                }

                for source in sources {
                    if let Some(btn) = source_filter_buttons.get(&source) {
                        btn.set_visible(true);

                        let is_available = available.contains(&source);
                        let is_enabled = enabled.contains(&source);
                        btn.set_sensitive(is_available && is_enabled);

                        if is_available {
                            btn.remove_css_class("source-unavailable");
                        } else {
                            btn.add_css_class("source-unavailable");
                        }

                        if is_enabled {
                            btn.remove_css_class("source-disabled");
                        } else {
                            btn.add_css_class("source-disabled");
                        }

                        if !is_available || !is_enabled {
                            btn.set_active(false);
                        }

                        sources_box.append(btn);
                    }
                }

                let mut state = filter_state.borrow_mut();
                state.sources.retain(|s| enabled.contains(s));
                drop(state);
                *skip_filter.borrow_mut() = false;

                apply_filters();
            }
        });

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
        let window_search = window.clone();
        let config_search = config.clone();
        let discover_stack_search = discover_stack.clone();
        let discover_list_box_search = discover_list_box.clone();
        let discover_rows_search = discover_rows.clone();
        let discover_debounce: Rc<RefCell<Option<glib::SourceId>>> = Rc::new(RefCell::new(None));
        let discover_debounce_holder = discover_debounce.clone();
        let reload_holder_search = reload_holder.clone();
        let progress_overlay_search = progress_overlay.clone();
        let progress_bar_search = progress_bar.clone();
        let progress_label_search = progress_label.clone();

        search_entry.connect_search_changed(move |entry| {
            let query = entry.text().to_lowercase();
            filter_state_search.borrow_mut().search_query = query.clone();

            if *current_view_search.borrow() != ViewMode::Discover {
                apply_filters_search();
                return;
            }

            if let Some(id) = discover_debounce_holder.borrow_mut().take() {
                try_remove_source(id);
            }

            let pm = pm_search.clone();
            let toast = toast_search.clone();
            let window = window_search.clone();
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
            let progress_overlay_for_timeout = progress_overlay_search.clone();
            let progress_bar_for_timeout = progress_bar_search.clone();
            let progress_label_for_timeout = progress_label_search.clone();

            let id = glib::timeout_add_local_once(Duration::from_millis(300), move || {
                if *current_view.borrow() != ViewMode::Discover {
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
                        &window,
                        &pm,
                        &toast,
                        &config,
                        false,
                        on_source_click,
                        reload_packages,
                        &progress_overlay_for_timeout,
                        &progress_bar_for_timeout,
                        &progress_label_for_timeout,
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

        // Source Filter Buttons
        let create_source_handler = |source: PackageSource| {
            let filter_state = filter_state.clone();
            let apply_filters = apply_filters.clone();
            let skip_filter = skip_filter.clone();
            let sources_all_btn = sources_all_btn.clone();
            move |btn: &gtk::ToggleButton| {
                if *skip_filter.borrow() {
                    return;
                }
                let mut state = filter_state.borrow_mut();
                if btn.is_active() {
                    if !state.sources.contains(&source) {
                        state.sources.push(source);
                    }
                } else {
                    state.sources.retain(|&s| s != source);
                }
                let empty = state.sources.is_empty();
                drop(state);
                *skip_filter.borrow_mut() = true;
                sources_all_btn.set_active(empty);
                *skip_filter.borrow_mut() = false;
                apply_filters();
            }
        };
        for (source, btn) in &source_filter_buttons {
            btn.connect_toggled(create_source_handler(*source));
        }

        // All sources filter
        let source_filter_buttons_clear = source_filter_buttons.clone();
        let filter_state_all_toggle = filter_state.clone();
        let skip_filter_all_toggle = skip_filter.clone();
        let apply_filters_all_toggle = apply_filters.clone();
        sources_all_btn.connect_toggled(move |btn| {
            if *skip_filter_all_toggle.borrow() {
                return;
            }
            if !btn.is_active() {
                return;
            }

            *skip_filter_all_toggle.borrow_mut() = true;
            for b in source_filter_buttons_clear.values() {
                b.set_active(false);
            }
            filter_state_all_toggle.borrow_mut().sources.clear();
            *skip_filter_all_toggle.borrow_mut() = false;
            apply_filters_all_toggle();
        });

        // Source Select All / Unselect
        let source_filter_buttons_all = source_filter_buttons.clone();
        let enabled_sources_all = enabled_sources.clone();
        let available_sources_all = available_sources.clone();
        let filter_state_all = filter_state.clone();
        let skip_filter_all = skip_filter.clone();
        let apply_filters_all = apply_filters.clone();
        let sources_all_btn_all = sources_all_btn.clone();
        sources_select_all_btn.connect_clicked(move |_| {
            let enabled = enabled_sources_all.borrow();
            let available = available_sources_all.borrow();
            *skip_filter_all.borrow_mut() = true;
            for (source, btn) in &source_filter_buttons_all {
                if enabled.contains(source) && available.contains(source) {
                    btn.set_active(true);
                } else {
                    btn.set_active(false);
                }
            }
            filter_state_all.borrow_mut().sources = enabled
                .iter()
                .copied()
                .filter(|s| available.contains(s))
                .collect();
            sources_all_btn_all.set_active(false);
            *skip_filter_all.borrow_mut() = false;
            apply_filters_all();
        });

        let source_filter_buttons_none = source_filter_buttons.clone();
        let filter_state_none = filter_state.clone();
        let skip_filter_none = skip_filter.clone();
        let apply_filters_none = apply_filters.clone();
        let sources_all_btn_none = sources_all_btn.clone();
        sources_unselect_all_btn.connect_clicked(move |_| {
            *skip_filter_none.borrow_mut() = true;
            for btn in source_filter_buttons_none.values() {
                btn.set_active(false);
            }
            filter_state_none.borrow_mut().sources.clear();
            sources_all_btn_none.set_active(true);
            *skip_filter_none.borrow_mut() = false;
            apply_filters_none();
        });

        // Reset (clear search + source filters)
        let search_entry_reset = search_entry.clone();
        let source_filter_buttons_reset = source_filter_buttons.clone();
        let filter_state_reset = filter_state.clone();
        let skip_filter_reset = skip_filter.clone();
        let apply_filters_reset = apply_filters.clone();
        let sources_all_btn_reset = sources_all_btn.clone();
        sources_reset_btn.connect_clicked(move |_| {
            *skip_filter_reset.borrow_mut() = true;
            search_entry_reset.set_text("");
            for btn in source_filter_buttons_reset.values() {
                btn.set_active(false);
            }
            sources_all_btn_reset.set_active(true);
            filter_state_reset.borrow_mut().sources.clear();
            *skip_filter_reset.borrow_mut() = false;
            apply_filters_reset();
        });

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
        let all_rows_toggle = all_rows.clone();
        let selected_count_toggle = selected_count_label.clone();
        let nav_list_toggle = nav_list.clone();
        let current_view_toggle = current_view.clone();
        select_button.connect_toggled(move |btn| {
            let active = btn.is_active();

            if active && *current_view_toggle.borrow() != ViewMode::AllPackages {
                if let Some(row) = nav_list_toggle.row_at_index(1) {
                    nav_list_toggle.select_row(Some(&row));
                }
            }

            *selection_mode_toggle.borrow_mut() = active;
            selection_bar_toggle.set_visible(active);

            for row in all_rows_toggle.borrow().iter() {
                row.set_selection_mode(active);
                if !active {
                    row.checkbox.set_active(false);
                }
            }
            if !active {
                selected_count_toggle.set_label("0 selected");
            }
        });

        // Select/Deselect All
        let all_rows_sel = all_rows.clone();
        let selected_count = selected_count_label.clone();
        select_all_btn.connect_clicked(move |_| {
            let rows = all_rows_sel.borrow();
            for row in rows.iter() {
                row.checkbox.set_active(true);
            }
            selected_count.set_label(&format!("{} selected", rows.len()));
        });

        let all_rows_desel = all_rows.clone();
        let selected_count_desel = selected_count_label.clone();
        deselect_all_btn.connect_clicked(move |_| {
            for row in all_rows_desel.borrow().iter() {
                row.checkbox.set_active(false);
            }
            selected_count_desel.set_label("0 selected");
        });

        // Navigation
        let content_stack_nav = content_stack.clone();
        let current_view_nav = current_view.clone();
        nav_list.connect_row_selected(move |_, row| {
            if let Some(row) = row {
                match row.index() {
                    0 => {
                        content_stack_nav.set_visible_child_name("discover");
                        *current_view_nav.borrow_mut() = ViewMode::Discover;
                    }
                    1 => {
                        content_stack_nav.set_visible_child_name("all");
                        *current_view_nav.borrow_mut() = ViewMode::AllPackages;
                    }
                    2 => {
                        content_stack_nav.set_visible_child_name("updates");
                        *current_view_nav.borrow_mut() = ViewMode::Updates;
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

            let updates: Vec<Package> = packages
                .borrow()
                .iter()
                .filter(|p| p.has_update())
                .cloned()
                .collect();
            if updates.is_empty() {
                btn.set_sensitive(true);
                return;
            }

            progress_overlay.set_visible(true);
            progress_label.set_label(&format!("Updating {} packages...", updates.len()));

            glib::spawn_future_local(async move {
                let total = updates.len();
                let mut success = 0;
                let mut blocked_snaps: Vec<String> = Vec::new();
                let manager = pm.lock().await;
                for (i, pkg) in updates.iter().enumerate() {
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
                btn.set_sensitive(true);
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
                let t = adw::Toast::new(&msg);
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
            let selected: Vec<Package> = all_rows_upd
                .borrow()
                .iter()
                .filter(|r| r.checkbox.is_active() && r.package.borrow().has_update())
                .map(|r| r.package.borrow().clone())
                .collect();
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
                let mut blocked_snaps: Vec<String> = Vec::new();
                let manager = pm.lock().await;
                for (i, pkg) in selected.iter().enumerate() {
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
                btn.set_sensitive(true);
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
                let t = adw::Toast::new(&msg);
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
            let selected: Vec<Package> = all_rows_rem
                .borrow()
                .iter()
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
                    if manager.remove(pkg).await.is_ok() {
                        success += 1;
                    }
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
                    gtk::gdk::Key::f => {
                        search_entry_focus.grab_focus();
                        return glib::Propagation::Stop;
                    }
                    gtk::gdk::Key::r => {
                        refresh_fn();
                        return glib::Propagation::Stop;
                    }
                    gtk::gdk::Key::s => {
                        select_btn.set_active(!select_btn.is_active());
                        return glib::Propagation::Stop;
                    }
                    _ => {}
                }
            }
            match key {
                gtk::gdk::Key::slash => {
                    search_entry_focus.grab_focus();
                    return glib::Propagation::Stop;
                }
                gtk::gdk::Key::Escape => {
                    // ESC: exit selection mode first; otherwise clear search.
                    if select_btn.is_active() {
                        select_btn.set_active(false);
                        return glib::Propagation::Stop;
                    }
                    if !search_entry_focus.text().is_empty() {
                        search_entry_focus.set_text("");
                        return glib::Propagation::Stop;
                    }
                }
                _ => {}
            }
            glib::Propagation::Proceed
        });
        self.window.add_controller(controller);

        Rc::new(load_packages)
    }

    #[allow(clippy::too_many_arguments)]
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
        reload_packages: Option<LocalFn>,
        progress_overlay: &gtk::Box,
        progress_bar: &gtk::ProgressBar,
        progress_label: &gtk::Label,
    ) where
        F: Fn(PackageSource) + Clone + 'static,
    {
        const CHUNK_SIZE: usize = 200;

        unsafe {
            if let Some(prev) = list_box.steal_data::<glib::SourceId>("populate_source") {
                try_remove_source(prev);
            }
        }

        while let Some(child) = list_box.first_child() {
            list_box.remove(&child);
        }
        rows.borrow_mut().clear();

        let list_box_for_idle = list_box.clone();
        let list_box_for_data = list_box.clone();
        let window = window.clone();
        let pm = pm.clone();
        let toast_overlay = toast_overlay.clone();
        let config = config.clone();
        let rows = rows.clone();
        let reload_packages = reload_packages.clone();
        let progress_overlay = progress_overlay.clone();
        let progress_bar = progress_bar.clone();
        let progress_label = progress_label.clone();
        let packages: Vec<Package> = packages.to_vec();
        let on_source_click = Rc::new(on_source_click);

        let index = Rc::new(RefCell::new(0usize));

        let source_id = glib::idle_add_local(move || {
            let mut start = *index.borrow();
            let end = (start + CHUNK_SIZE).min(packages.len());

            while start < end {
                let package = packages[start].clone();
                let row = PackageRow::new(package.clone(), None);
                row.set_selection_mode(selection_mode);

                let pkg = package.clone();
                let win = window.clone();
                let pm_details = pm.clone();
                let toast_details = toast_overlay.clone();
                let config_details = config.clone();
                let reload_details = reload_packages.clone();

                row.widget.connect_activated(move |_| {
                    PackageDetailsDialog::show(
                        &pkg,
                        &win,
                        pm_details.clone(),
                        toast_details.clone(),
                        config_details.clone(),
                        reload_details.clone(),
                    );
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
                let row_pkg = row.package.clone();
                let row_update_icon = row.update_icon.clone();
                let progress_overlay = progress_overlay.clone();
                let progress_bar = progress_bar.clone();
                let progress_label = progress_label.clone();
                let reload_action = reload_packages.clone();
                let window_for_action = window.clone();

                row.action_button.connect_clicked(move |_| {
                    let pkg = pkg_action.clone();
                    let pm = pm_action.clone();
                    let toast = toast_action.clone();
                    let spinner = spinner.clone();
                    let btn = action_btn.clone();
                    let row_pkg = row_pkg.clone();
                    let row_update_icon = row_update_icon.clone();
                    let progress_overlay = progress_overlay.clone();
                    let progress_bar = progress_bar.clone();
                    let progress_label = progress_label.clone();
                    let reload_action = reload_action.clone();
                    let parent_window = window_for_action.clone();

                    btn.set_visible(false);
                    spinner.set_visible(true);
                    spinner.start();

                    glib::spawn_future_local(async move {
                        progress_overlay.set_visible(true);
                        progress_bar.set_fraction(0.0);
                        progress_bar.set_text(None);
                        let op_label = match pkg.status {
                            PackageStatus::UpdateAvailable => format!("Updating {}...", pkg.name),
                            PackageStatus::Installed => format!("Removing {}...", pkg.name),
                            PackageStatus::NotInstalled => format!("Installing {}...", pkg.name),
                            _ => "Working...".to_string(),
                        };
                        progress_label.set_label(&op_label);

                        let progress_bar_pulse = progress_bar.clone();
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
                                try_remove_source(pulser_id);
                                progress_overlay.set_visible(false);
                                spinner.stop();
                                spinner.set_visible(false);
                                btn.set_visible(true);
                                let t = adw::Toast::new(&format!("Error: {}", e));
                                t.set_timeout(5);
                                toast.add_toast(t);
                                return;
                            }
                        };

                        try_remove_source(pulser_id);
                        progress_overlay.set_visible(false);
                        spinner.stop();
                        spinner.set_visible(false);
                        btn.set_visible(true);

                        let ok = result.is_ok();
                        let msg = match result {
                            Ok(_) => match pkg.status {
                                PackageStatus::UpdateAvailable => format!("Updated {}", pkg.name),
                                PackageStatus::Installed => format!("Removed {}", pkg.name),
                                PackageStatus::NotInstalled => format!("Installed {}", pkg.name),
                                _ => "Operation completed".to_string(),
                            },
                            Err(e) => format!("Error: {}", e),
                        };

                        if !ok {
                            if let Some((details, command)) = parse_suggestion(&msg) {
                                show_action_required_dialog(&parent_window, &details, &command);
                                let t =
                                    adw::Toast::new("Action required (copied command available)");
                                t.set_timeout(5);
                                toast.add_toast(t);
                                return;
                            }
                        }

                        let t = adw::Toast::new(&msg);
                        t.set_timeout(3);
                        toast.add_toast(t);

                        if ok {
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

                list_box_for_idle.append(&row.widget);
                rows.borrow_mut().push(row);

                start += 1;
            }

            *index.borrow_mut() = start;
            if start >= packages.len() {
                glib::ControlFlow::Break
            } else {
                glib::ControlFlow::Continue
            }
        });

        unsafe {
            list_box_for_data.set_data("populate_source", source_id);
        }
    }

    pub fn present(&self) {
        self.window.present();
    }
}
