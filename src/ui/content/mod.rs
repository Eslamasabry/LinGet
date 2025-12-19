#![allow(dead_code)]

mod discover_view;
mod favorites_view;
mod library_view;
mod updates_view;

pub use discover_view::DiscoverView;
pub use favorites_view::FavoritesView;
pub use library_view::LibraryView;
pub use updates_view::UpdatesView;

use crate::models::PackageSource;
use gtk4::prelude::*;
use gtk4::{self as gtk};
use std::collections::HashMap;

#[derive(Clone)]
pub struct SourceFilterWidgets {
    pub menu_btn: gtk::MenuButton,
    pub all_btn: gtk::CheckButton,
    pub source_btns: HashMap<PackageSource, gtk::CheckButton>,
    pub source_box: gtk::Box,
}

impl SourceFilterWidgets {
    pub fn new() -> Self {
        let menu_btn = gtk::MenuButton::builder().label("Source: All").build();
        menu_btn.add_css_class("flat");
        menu_btn.add_css_class("chip-btn");

        let popover = gtk::Popover::new();
        let popover_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(8)
            .margin_top(8)
            .margin_bottom(8)
            .margin_start(10)
            .margin_end(10)
            .build();

        let all_btn = gtk::CheckButton::builder()
            .label("All sources")
            .active(true)
            .build();
        popover_box.append(&all_btn);
        popover_box.append(&gtk::Separator::new(gtk::Orientation::Horizontal));

        let source_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(2)
            .build();

        let mut source_btns = HashMap::new();
        for source in PackageSource::ALL {
            let btn = gtk::CheckButton::with_label(&source.to_string());
            btn.set_group(Some(&all_btn));
            source_box.append(&btn);
            source_btns.insert(source, btn);
        }

        popover_box.append(&source_box);
        popover.set_child(Some(&popover_box));
        menu_btn.set_popover(Some(&popover));

        Self {
            menu_btn,
            all_btn,
            source_btns,
            source_box,
        }
    }

    pub fn set_selected(&self, source: Option<PackageSource>) {
        match source {
            None => {
                self.all_btn.set_active(true);
                self.menu_btn.set_label("Source: All");
            }
            Some(s) => {
                if let Some(btn) = self.source_btns.get(&s) {
                    btn.set_active(true);
                }
                self.menu_btn.set_label(&format!("Source: {}", s));
            }
        }
    }
}

pub struct ContentArea {
    pub widget: gtk::Box,
    pub content_stack: gtk::Stack,
    pub discover: DiscoverView,
    pub library: LibraryView,
    pub updates: UpdatesView,
    pub favorites: FavoritesView,
    pub sort_dropdown: gtk::DropDown,
}

impl ContentArea {
    pub fn new() -> Self {
        let widget = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .hexpand(true)
            .build();

        let content_stack = gtk::Stack::builder()
            .transition_type(gtk::StackTransitionType::SlideLeftRight)
            .transition_duration(200)
            .hexpand(true)
            .build();

        let discover = DiscoverView::new();
        let library = LibraryView::new();
        let updates = UpdatesView::new();
        let favorites = FavoritesView::new();

        content_stack.add_named(&discover.widget, Some("discover"));
        content_stack.add_named(&library.widget, Some("all"));
        content_stack.add_named(&updates.widget, Some("updates"));
        content_stack.add_named(&favorites.widget, Some("favorites"));
        content_stack.set_visible_child_name("all");

        widget.append(&content_stack);

        Self {
            widget,
            content_stack,
            sort_dropdown: library.sort_dropdown.clone(),
            discover,
            library,
            updates,
            favorites,
        }
    }

    pub fn show_view(&self, name: &str) {
        self.content_stack.set_visible_child_name(name);
    }
}
