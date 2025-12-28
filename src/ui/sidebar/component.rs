use crate::models::PackageSource;

use gtk4::prelude::*;
use gtk4::{self as gtk, glib};
use relm4::prelude::*;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NavItem {
    Home,
    Library,
    Updates,
    Favorites,
    Storage,
    Health,
    History,
    Tasks,
    Aliases,
    Collection(String),
}

impl NavItem {
    pub fn icon_name(&self) -> &'static str {
        match self {
            NavItem::Home => "go-home-symbolic",
            NavItem::Library => "view-grid-symbolic",
            NavItem::Updates => "software-update-available-symbolic",
            NavItem::Favorites => "starred-symbolic",
            NavItem::Storage => "drive-harddisk-symbolic",
            NavItem::Health => "emblem-ok-symbolic",
            NavItem::History => "document-open-recent-symbolic",
            NavItem::Tasks => "alarm-symbolic",
            NavItem::Aliases => "utilities-terminal-symbolic",
            NavItem::Collection(_) => "folder-symbolic",
        }
    }

    pub fn label(&self) -> String {
        match self {
            NavItem::Home => "Home".to_string(),
            NavItem::Library => "Library".to_string(),
            NavItem::Updates => "Updates".to_string(),
            NavItem::Favorites => "Favorites".to_string(),
            NavItem::Storage => "Storage".to_string(),
            NavItem::Health => "Health".to_string(),
            NavItem::History => "History".to_string(),
            NavItem::Tasks => "Tasks".to_string(),
            NavItem::Aliases => "Aliases".to_string(),
            NavItem::Collection(name) => name.clone(),
        }
    }

    pub fn from_index(index: i32) -> Option<Self> {
        match index {
            0 => Some(NavItem::Home),
            1 => Some(NavItem::Library),
            2 => Some(NavItem::Updates),
            3 => Some(NavItem::Favorites),
            4 => Some(NavItem::Storage),
            5 => Some(NavItem::Health),
            6 => Some(NavItem::History),
            7 => Some(NavItem::Tasks),
            8 => Some(NavItem::Aliases),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SidebarInit {
    pub available_sources: HashSet<PackageSource>,
    pub enabled_sources: HashSet<PackageSource>,
    pub library_count: usize,
    pub updates_count: usize,
    pub favorites_count: usize,
    pub collections: HashMap<String, usize>,
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum SidebarInput {
    SetCounts {
        library: usize,
        updates: usize,
        favorites: usize,
    },
    SetProviderCount(PackageSource, usize),
    SetAllProviderCounts(HashMap<PackageSource, usize>),
    UpdateAvailability {
        available: HashSet<PackageSource>,
        enabled: HashSet<PackageSource>,
    },
    UpdateCollections(HashMap<String, usize>),
    SelectView(NavItem),
    SetActiveFilter(Option<PackageSource>),
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum SidebarOutput {
    ViewChanged(NavItem),
    SourceToggled(PackageSource),
    EnableDetectedSources,
    NewCollection,
    FilterBySource(Option<PackageSource>),
}

#[allow(dead_code)]
pub struct SidebarModel {
    available_sources: HashSet<PackageSource>,
    enabled_sources: HashSet<PackageSource>,
    library_count: usize,
    updates_count: usize,
    favorites_count: usize,
    collections: HashMap<String, usize>,
    collection_names: Vec<String>,
    provider_counts: HashMap<PackageSource, usize>,
    providers_expanded: bool,
    active_filter: Option<PackageSource>,
    show_unavailable: bool,
    last_sorted_sources: RefCell<Vec<PackageSource>>,
}

#[allow(dead_code)]
pub struct SidebarWidgets {
    nav_list: gtk::ListBox,
    library_count_label: gtk::Label,
    updates_count_label: gtk::Label,
    favorites_count_label: gtk::Label,
    collections_list: gtk::ListBox,
    collection_rows: HashMap<String, gtk::ListBoxRow>,
    provider_rows: HashMap<PackageSource, ProviderRowWidgets>,
    providers_box: gtk::Box,
    providers_revealer: gtk::Revealer,
    updating_switches: Rc<RefCell<bool>>,
    filter_indicator: gtk::Box,
    filter_label: gtk::Label,
    show_unavailable_btn: gtk::ToggleButton,
}

#[derive(Debug, Clone)]
struct ProviderRowWidgets {
    row: gtk::Box,
    enabled_switch: gtk::Switch,
    count_label: gtk::Label,
    status_label: gtk::Label,
}

impl SidebarModel {
    fn create_nav_row(item: NavItem, count_label: Option<&gtk::Label>) -> gtk::ListBoxRow {
        let row = gtk::ListBoxRow::new();
        row.add_css_class("nav-row");

        let content = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(12)
            .margin_top(10)
            .margin_bottom(10)
            .margin_start(12)
            .margin_end(12)
            .build();

        content.append(&gtk::Image::from_icon_name(item.icon_name()));
        content.append(
            &gtk::Label::builder()
                .label(item.label())
                .hexpand(true)
                .xalign(0.0)
                .build(),
        );

        if let Some(label) = count_label {
            content.append(label);
        }

        row.set_child(Some(&content));
        row
    }

    fn create_collection_row(name: &str, count: usize) -> gtk::ListBoxRow {
        let count_label = gtk::Label::builder()
            .label(count.to_string())
            .css_classes(vec!["dim-label", "caption"])
            .build();

        let nav_item = NavItem::Collection(name.to_string());
        let row = Self::create_nav_row(nav_item, Some(&count_label));
        row.set_widget_name(&format!("collection:{}", name));
        row
    }

    fn create_provider_row(
        source: PackageSource,
        sender: &ComponentSender<Self>,
        updating_flag: Rc<RefCell<bool>>,
    ) -> ProviderRowWidgets {
        let row = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(10)
            .build();
        row.add_css_class("provider-row");

        let click_area = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(10)
            .hexpand(true)
            .build();

        let dot = gtk::Box::builder()
            .width_request(10)
            .height_request(10)
            .valign(gtk::Align::Center)
            .build();
        dot.add_css_class("source-dot");
        dot.add_css_class(source.color_class());

        let labels = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(2)
            .hexpand(true)
            .build();

        let title = gtk::Label::builder()
            .label(source.to_string())
            .xalign(0.0)
            .build();
        title.add_css_class("provider-title");

        let status = gtk::Label::builder()
            .label("Not detected")
            .xalign(0.0)
            .visible(false)
            .build();
        status.add_css_class("caption");
        status.add_css_class("dim-label");

        labels.append(&title);
        labels.append(&status);

        let count_label = gtk::Label::new(Some("0"));
        count_label.add_css_class("dim-label");
        count_label.add_css_class("caption");

        click_area.append(&dot);
        click_area.append(&labels);
        click_area.append(&count_label);

        let gesture = gtk::GestureClick::new();
        let sender_filter = sender.clone();
        gesture.connect_released(move |_, _, _, _| {
            sender_filter
                .output(SidebarOutput::FilterBySource(Some(source)))
                .ok();
        });
        click_area.add_controller(gesture);

        let enabled_switch = gtk::Switch::builder().valign(gtk::Align::Center).build();

        let sender_clone = sender.clone();
        enabled_switch.connect_state_set(move |_, _state| {
            if !*updating_flag.borrow() {
                sender_clone
                    .output(SidebarOutput::SourceToggled(source))
                    .ok();
            }
            glib::Propagation::Proceed
        });

        row.append(&click_area);
        row.append(&enabled_switch);

        ProviderRowWidgets {
            row,
            enabled_switch,
            count_label,
            status_label: status,
        }
    }
}

impl SimpleComponent for SidebarModel {
    type Init = SidebarInit;
    type Input = SidebarInput;
    type Output = SidebarOutput;
    type Root = gtk::Box;
    type Widgets = SidebarWidgets;

    fn init_root() -> Self::Root {
        gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .width_request(200)
            .css_classes(vec!["sidebar"])
            .build()
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = SidebarModel {
            available_sources: init.available_sources.clone(),
            enabled_sources: init.enabled_sources.clone(),
            library_count: init.library_count,
            updates_count: init.updates_count,
            favorites_count: init.favorites_count,
            collections: init.collections.clone(),
            collection_names: Vec::new(),
            provider_counts: HashMap::new(),
            providers_expanded: true,
            active_filter: None,
            show_unavailable: false,
            last_sorted_sources: RefCell::new(Vec::new()),
        };

        let header = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(12)
            .margin_top(16)
            .margin_bottom(8)
            .margin_start(16)
            .margin_end(16)
            .build();

        let app_icon = gtk::Image::builder()
            .icon_name("io.github.linget")
            .pixel_size(32)
            .build();
        app_icon.add_css_class("app-icon");

        let app_title = gtk::Label::builder().label("LinGet").xalign(0.0).build();
        app_title.add_css_class("title-1");

        header.append(&app_icon);
        header.append(&app_title);
        root.append(&header);

        let scroll_content = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();

        let nav_label = gtk::Label::builder()
            .label("Library")
            .xalign(0.0)
            .margin_top(16)
            .margin_start(16)
            .margin_bottom(4)
            .build();
        nav_label.add_css_class("caption");
        nav_label.add_css_class("dim-label");
        scroll_content.append(&nav_label);

        let nav_list = gtk::ListBox::builder()
            .selection_mode(gtk::SelectionMode::Single)
            .css_classes(vec!["navigation-sidebar"])
            .build();

        let home_row = Self::create_nav_row(NavItem::Home, None);
        nav_list.append(&home_row);

        let library_count_label = gtk::Label::builder()
            .label(init.library_count.to_string())
            .css_classes(vec!["dim-label", "caption"])
            .build();
        let library_row = Self::create_nav_row(NavItem::Library, Some(&library_count_label));
        nav_list.append(&library_row);

        let updates_count_label = gtk::Label::builder()
            .label(init.updates_count.to_string())
            .css_classes(vec!["badge-accent"])
            .visible(init.updates_count > 0)
            .build();
        let updates_row = Self::create_nav_row(NavItem::Updates, Some(&updates_count_label));
        nav_list.append(&updates_row);

        let favorites_count_label = gtk::Label::builder()
            .label(init.favorites_count.to_string())
            .css_classes(vec!["dim-label", "caption"])
            .visible(init.favorites_count > 0)
            .build();
        let favorites_row = Self::create_nav_row(NavItem::Favorites, Some(&favorites_count_label));
        nav_list.append(&favorites_row);

        let storage_row = Self::create_nav_row(NavItem::Storage, None);
        nav_list.append(&storage_row);

        let health_row = Self::create_nav_row(NavItem::Health, None);
        nav_list.append(&health_row);

        let history_row = Self::create_nav_row(NavItem::History, None);
        nav_list.append(&history_row);

        let tasks_row = Self::create_nav_row(NavItem::Tasks, None);
        nav_list.append(&tasks_row);

        let aliases_row = Self::create_nav_row(NavItem::Aliases, None);
        nav_list.append(&aliases_row);

        nav_list.select_row(Some(&library_row));

        let collections_header = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(8)
            .margin_top(24)
            .margin_start(16)
            .margin_end(16)
            .margin_bottom(8)
            .build();

        let collections_label = gtk::Label::builder()
            .label("Collections")
            .xalign(0.0)
            .hexpand(true)
            .build();
        collections_label.add_css_class("caption");
        collections_label.add_css_class("dim-label");

        let add_collection_btn = gtk::Button::builder()
            .icon_name("list-add-symbolic")
            .tooltip_text("New Collection")
            .css_classes(vec!["flat", "circular"])
            .build();

        {
            let sender = sender.clone();
            add_collection_btn.connect_clicked(move |_| {
                sender.output(SidebarOutput::NewCollection).ok();
            });
        }

        collections_header.append(&collections_label);
        collections_header.append(&add_collection_btn);
        scroll_content.append(&collections_header);

        let collections_list = gtk::ListBox::builder()
            .selection_mode(gtk::SelectionMode::Single)
            .css_classes(vec!["navigation-sidebar"])
            .build();

        let collection_rows = HashMap::new();

        let sender_nav = sender.clone();
        nav_list.connect_row_selected({
            let collections_list = collections_list.clone();
            move |_, row| {
                if let Some(row) = row {
                    collections_list.select_row(None::<&gtk::ListBoxRow>);
                    if let Some(item) = NavItem::from_index(row.index()) {
                        sender_nav.output(SidebarOutput::ViewChanged(item)).ok();
                    }
                }
            }
        });

        let sender_coll = sender.clone();
        collections_list.connect_row_selected({
            let nav_list = nav_list.clone();
            move |_, row| {
                if let Some(row) = row {
                    nav_list.select_row(None::<&gtk::ListBoxRow>);
                    if let Some(name) = row.widget_name().strip_prefix("collection:") {
                        sender_coll
                            .output(SidebarOutput::ViewChanged(NavItem::Collection(
                                name.to_string(),
                            )))
                            .ok();
                    }
                }
            }
        });

        scroll_content.append(&nav_list);
        scroll_content.append(&collections_list);

        let filter_indicator = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(8)
            .margin_top(16)
            .margin_start(16)
            .margin_end(16)
            .margin_bottom(8)
            .visible(false)
            .build();
        filter_indicator.add_css_class("filter-indicator");

        let filter_icon = gtk::Image::from_icon_name("funnel-symbolic");
        filter_icon.add_css_class("dim-label");

        let filter_label = gtk::Label::builder()
            .label("Filtering by: Flatpak")
            .hexpand(true)
            .xalign(0.0)
            .build();
        filter_label.add_css_class("caption");

        let clear_filter_btn = gtk::Button::builder()
            .icon_name("window-close-symbolic")
            .tooltip_text("Clear filter")
            .build();
        clear_filter_btn.add_css_class("flat");
        clear_filter_btn.add_css_class("circular");

        let sender_clear = sender.clone();
        clear_filter_btn.connect_clicked(move |_| {
            sender_clear
                .output(SidebarOutput::FilterBySource(None))
                .ok();
        });

        filter_indicator.append(&filter_icon);
        filter_indicator.append(&filter_label);
        filter_indicator.append(&clear_filter_btn);
        scroll_content.append(&filter_indicator);

        let providers_header = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(8)
            .margin_top(16)
            .margin_start(16)
            .margin_end(16)
            .margin_bottom(4)
            .build();

        let providers_label = gtk::Label::builder()
            .label("Providers")
            .xalign(0.0)
            .hexpand(true)
            .build();
        providers_label.add_css_class("caption");
        providers_label.add_css_class("dim-label");

        let show_unavailable_btn = gtk::ToggleButton::builder()
            .icon_name("view-more-symbolic")
            .tooltip_text("Show unavailable providers")
            .build();
        show_unavailable_btn.add_css_class("flat");
        show_unavailable_btn.add_css_class("circular");

        let toggle_btn = gtk::ToggleButton::builder()
            .icon_name("pan-down-symbolic")
            .active(true)
            .tooltip_text("Show/hide providers")
            .build();
        toggle_btn.add_css_class("flat");
        toggle_btn.add_css_class("circular");

        providers_header.append(&providers_label);
        providers_header.append(&show_unavailable_btn);
        providers_header.append(&toggle_btn);
        scroll_content.append(&providers_header);

        let providers_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(2)
            .margin_start(8)
            .margin_end(8)
            .build();

        let updating_switches = Rc::new(RefCell::new(true));

        let mut provider_rows = HashMap::new();
        for source in PackageSource::ALL {
            let row_widgets = Self::create_provider_row(source, &sender, updating_switches.clone());

            let is_available = init.available_sources.contains(&source);
            let is_enabled = init.enabled_sources.contains(&source);

            row_widgets.enabled_switch.set_sensitive(is_available);
            row_widgets
                .enabled_switch
                .set_active(is_enabled && is_available);

            if is_available {
                row_widgets.status_label.set_visible(false);
                row_widgets.row.remove_css_class("provider-unavailable");
                row_widgets.row.set_visible(true);
            } else {
                row_widgets.status_label.set_visible(true);
                row_widgets.row.add_css_class("provider-unavailable");
                row_widgets.row.set_visible(false);
            }

            providers_box.append(&row_widgets.row);
            provider_rows.insert(source, row_widgets);
        }

        *updating_switches.borrow_mut() = false;

        let providers_revealer = gtk::Revealer::builder()
            .reveal_child(true)
            .transition_type(gtk::RevealerTransitionType::SlideDown)
            .child(&providers_box)
            .build();

        toggle_btn.connect_toggled({
            let revealer = providers_revealer.clone();
            move |btn| revealer.set_reveal_child(btn.is_active())
        });

        {
            let provider_rows_clone = provider_rows.clone();
            let available_sources = init.available_sources.clone();
            show_unavailable_btn.connect_toggled(move |btn| {
                let show_all = btn.is_active();
                for (source, row) in &provider_rows_clone {
                    let is_available = available_sources.contains(source);
                    row.row.set_visible(is_available || show_all);
                }
            });
        }

        scroll_content.append(&providers_revealer);

        let scrolled = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .propagate_natural_height(false)
            .propagate_natural_width(false)
            .vexpand(true)
            .child(&scroll_content)
            .build();
        root.append(&scrolled);

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

        root.append(&stats_box);

        let widgets = SidebarWidgets {
            nav_list,
            library_count_label,
            updates_count_label,
            favorites_count_label,
            collections_list,
            collection_rows,
            provider_rows,
            providers_box,
            providers_revealer,
            updating_switches,
            filter_indicator,
            filter_label,
            show_unavailable_btn,
        };

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            SidebarInput::SetCounts {
                library,
                updates,
                favorites,
            } => {
                self.library_count = library;
                self.updates_count = updates;
                self.favorites_count = favorites;
            }
            SidebarInput::SetProviderCount(source, count) => {
                self.provider_counts.insert(source, count);
            }
            SidebarInput::SetAllProviderCounts(counts) => {
                self.provider_counts = counts;
            }
            SidebarInput::UpdateAvailability { available, enabled } => {
                self.available_sources = available;
                self.enabled_sources = enabled;
            }
            SidebarInput::SelectView(_item) => {}
            SidebarInput::UpdateCollections(collections) => {
                self.collections = collections;
                self.collection_names = self.collections.keys().cloned().collect();
                self.collection_names.sort();
            }
            SidebarInput::SetActiveFilter(filter) => {
                self.active_filter = filter;
            }
        }
    }

    fn update_view(&self, widgets: &mut Self::Widgets, _sender: ComponentSender<Self>) {
        let start = Instant::now();

        widgets
            .library_count_label
            .set_label(&self.library_count.to_string());
        widgets
            .updates_count_label
            .set_label(&self.updates_count.to_string());
        widgets
            .updates_count_label
            .set_visible(self.updates_count > 0);
        widgets
            .favorites_count_label
            .set_label(&self.favorites_count.to_string());
        widgets
            .favorites_count_label
            .set_visible(self.favorites_count > 0);

        let current_names: HashSet<String> = widgets.collection_rows.keys().cloned().collect();
        let new_names: HashSet<&String> = self.collections.keys().collect();

        let names_to_remove: Vec<String> = current_names
            .iter()
            .filter(|n| !new_names.contains(n))
            .cloned()
            .collect();

        for name in names_to_remove {
            if let Some(row) = widgets.collection_rows.remove(&name) {
                widgets.collections_list.remove(&row);
            }
        }

        for name in &self.collection_names {
            let count = self.collections.get(name).copied().unwrap_or(0);
            if let Some(row) = widgets.collection_rows.get(name) {
                if let Some(content) = row.child().and_downcast::<gtk::Box>() {
                    if let Some(label) = content.last_child().and_downcast::<gtk::Label>() {
                        label.set_label(&count.to_string());
                    }
                }
            } else {
                let row = Self::create_collection_row(name, count);
                widgets.collections_list.append(&row);
                widgets.collection_rows.insert(name.clone(), row);
            }
        }

        *widgets.updating_switches.borrow_mut() = true;

        let mut sorted_sources: Vec<PackageSource> = PackageSource::ALL.to_vec();
        sorted_sources.sort_by(|a, b| {
            let count_a = self.provider_counts.get(a).copied().unwrap_or(0);
            let count_b = self.provider_counts.get(b).copied().unwrap_or(0);
            count_b.cmp(&count_a)
        });

        let last_sorted = self.last_sorted_sources.borrow();
        let order_changed = *last_sorted != sorted_sources;
        drop(last_sorted);

        if order_changed {
            crate::ui::set_ui_marker("SidebarReorderProviders");
            let reorder_start = Instant::now();

            *self.last_sorted_sources.borrow_mut() = sorted_sources.clone();
            for source in sorted_sources.iter().rev() {
                if let Some(row) = widgets.provider_rows.get(source) {
                    row.row.unparent();
                    widgets.providers_box.prepend(&row.row);
                }
            }

            let reorder_elapsed = reorder_start.elapsed();
            if reorder_elapsed > Duration::from_millis(10) {
                tracing::debug!(
                    reorder_ms = reorder_elapsed.as_millis() as u64,
                    "Reordered provider rows"
                );
            }
        }

        for (source, row) in &widgets.provider_rows {
            let is_available = self.available_sources.contains(source);
            let is_enabled = self.enabled_sources.contains(source);
            let should_be_active = is_enabled && is_available;

            row.enabled_switch.set_sensitive(is_available);
            if row.enabled_switch.is_active() != should_be_active {
                row.enabled_switch.set_active(should_be_active);
            }

            if is_available {
                row.status_label.set_visible(false);
                row.row.remove_css_class("provider-unavailable");
            } else {
                row.status_label.set_visible(true);
                row.row.add_css_class("provider-unavailable");
            }

            let count = self.provider_counts.get(source).copied().unwrap_or(0);
            row.count_label.set_label(&count.to_string());

            let is_active_filter = self.active_filter == Some(*source);
            if is_active_filter {
                row.row.add_css_class("provider-active-filter");
            } else {
                row.row.remove_css_class("provider-active-filter");
            }

            let show_unavailable = widgets.show_unavailable_btn.is_active();
            row.row.set_visible(is_available || show_unavailable);
        }
        *widgets.updating_switches.borrow_mut() = false;

        if let Some(source) = self.active_filter {
            widgets.filter_indicator.set_visible(true);
            widgets
                .filter_label
                .set_label(&format!("Filtering: {}", source));
        } else {
            widgets.filter_indicator.set_visible(false);
        }

        let elapsed = start.elapsed();
        if elapsed > Duration::from_millis(30) {
            tracing::warn!(
                elapsed_ms = elapsed.as_millis() as u64,
                "Sidebar update_view slow"
            );
        }
    }
}
