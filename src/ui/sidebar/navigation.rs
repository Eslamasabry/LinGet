#![allow(dead_code)]

use gtk4::prelude::*;
use gtk4::{self as gtk};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NavItem {
    Discover,
    Library,
    Updates,
    Favorites,
}

impl NavItem {
    pub fn icon_name(&self) -> &'static str {
        match self {
            NavItem::Discover => "system-search-symbolic",
            NavItem::Library => "view-grid-symbolic",
            NavItem::Updates => "software-update-available-symbolic",
            NavItem::Favorites => "starred-symbolic",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            NavItem::Discover => "Discover",
            NavItem::Library => "Library",
            NavItem::Updates => "Updates",
            NavItem::Favorites => "Favorites",
        }
    }

    pub fn stack_name(&self) -> &'static str {
        match self {
            NavItem::Discover => "discover",
            NavItem::Library => "all",
            NavItem::Updates => "updates",
            NavItem::Favorites => "favorites",
        }
    }
}

pub struct NavigationSection {
    pub widget: gtk::Box,
    pub nav_list: gtk::ListBox,
    pub all_count_label: gtk::Label,
    pub update_count_label: gtk::Label,
    pub favorites_count_label: gtk::Label,
    discover_row: gtk::ListBoxRow,
    library_row: gtk::ListBoxRow,
    updates_row: gtk::ListBoxRow,
    favorites_row: gtk::ListBoxRow,
}

impl NavigationSection {
    pub fn new() -> Self {
        let widget = gtk::Box::builder()
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
        widget.append(&nav_label);

        let nav_list = gtk::ListBox::builder()
            .selection_mode(gtk::SelectionMode::Single)
            .css_classes(vec!["navigation-sidebar"])
            .build();

        let discover_row = create_nav_row(NavItem::Discover, None);
        nav_list.append(&discover_row);

        let all_count_label = gtk::Label::builder()
            .label("0")
            .css_classes(vec!["dim-label", "caption"])
            .build();
        let library_row = create_nav_row(NavItem::Library, Some(&all_count_label));
        nav_list.append(&library_row);

        let update_count_label = gtk::Label::builder()
            .label("0")
            .css_classes(vec!["badge-accent"])
            .visible(false)
            .build();
        let updates_row = create_nav_row(NavItem::Updates, Some(&update_count_label));
        nav_list.append(&updates_row);

        let favorites_count_label = gtk::Label::builder()
            .label("0")
            .css_classes(vec!["dim-label", "caption"])
            .visible(false)
            .build();
        let favorites_row = create_nav_row(NavItem::Favorites, Some(&favorites_count_label));
        nav_list.append(&favorites_row);

        nav_list.select_row(Some(&library_row));
        widget.append(&nav_list);

        Self {
            widget,
            nav_list,
            all_count_label,
            update_count_label,
            favorites_count_label,
            discover_row,
            library_row,
            updates_row,
            favorites_row,
        }
    }

    pub fn item_at_index(&self, index: i32) -> Option<NavItem> {
        match index {
            0 => Some(NavItem::Discover),
            1 => Some(NavItem::Library),
            2 => Some(NavItem::Updates),
            3 => Some(NavItem::Favorites),
            _ => None,
        }
    }

    pub fn select(&self, item: NavItem) {
        let row = match item {
            NavItem::Discover => &self.discover_row,
            NavItem::Library => &self.library_row,
            NavItem::Updates => &self.updates_row,
            NavItem::Favorites => &self.favorites_row,
        };
        self.nav_list.select_row(Some(row));
    }

    pub fn connect_row_selected<F>(&self, callback: F)
    where
        F: Fn(NavItem) + 'static,
    {
        self.nav_list.connect_row_selected(move |_, row| {
            if let Some(row) = row {
                let index = row.index();
                let item = match index {
                    0 => NavItem::Discover,
                    1 => NavItem::Library,
                    2 => NavItem::Updates,
                    3 => NavItem::Favorites,
                    _ => return,
                };
                callback(item);
            }
        });
    }

    pub fn set_library_count(&self, count: usize) {
        self.all_count_label.set_label(&count.to_string());
    }

    pub fn set_updates_count(&self, count: usize) {
        self.update_count_label.set_label(&count.to_string());
        self.update_count_label.set_visible(count > 0);
    }

    pub fn set_favorites_count(&self, count: usize) {
        self.favorites_count_label.set_label(&count.to_string());
        self.favorites_count_label.set_visible(count > 0);
    }
}

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
