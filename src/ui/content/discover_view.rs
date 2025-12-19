use super::SourceFilterWidgets;
use crate::ui::{EmptyState, SkeletonList};
use gtk4::prelude::*;
use gtk4::{self as gtk};
use libadwaita as adw;

pub struct DiscoverView {
    pub widget: gtk::Box,
    pub stack: gtk::Stack,
    pub list_box: gtk::ListBox,
    pub source_filter: SourceFilterWidgets,
    pub search_chip: gtk::Button,
    pub spinner: gtk::Spinner,
}

impl DiscoverView {
    pub fn new() -> Self {
        let list_box = gtk::ListBox::builder()
            .selection_mode(gtk::SelectionMode::None)
            .css_classes(vec!["boxed-list"])
            .build();

        let scrolled = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vexpand(true)
            .child(&list_box)
            .build();

        let header = adw::HeaderBar::builder()
            .show_start_title_buttons(false)
            .show_end_title_buttons(false)
            .build();
        header.add_css_class("view-toolbar");
        header.set_title_widget(Some(&adw::WindowTitle::builder().title("Discover").build()));

        let source_filter = SourceFilterWidgets::new();
        let search_chip = create_search_chip();

        let filters = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(6)
            .build();
        filters.append(&source_filter.menu_btn);
        filters.append(&search_chip);
        header.pack_start(&filters);

        let spinner = gtk::Spinner::builder().visible(false).build();
        header.pack_end(&spinner);

        let list_area = adw::Clamp::builder()
            .maximum_size(1600)
            .tightening_threshold(1200)
            .child(&scrolled)
            .margin_top(8)
            .margin_bottom(24)
            .margin_start(24)
            .margin_end(24)
            .build();

        let empty = EmptyState::search_packages().widget;

        let skeleton = SkeletonList::new(6).widget;
        let skeleton_clamp = adw::Clamp::builder()
            .maximum_size(1600)
            .tightening_threshold(1200)
            .child(&skeleton)
            .margin_top(8)
            .margin_start(24)
            .margin_end(24)
            .build();

        let stack = gtk::Stack::builder()
            .transition_type(gtk::StackTransitionType::Crossfade)
            .transition_duration(150)
            .build();
        stack.add_named(&list_area, Some("list"));
        stack.add_named(&empty, Some("empty"));
        stack.add_named(&skeleton_clamp, Some("skeleton"));
        stack.set_visible_child_name("empty");

        let widget = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();
        widget.append(&header);
        widget.append(&stack);

        Self {
            widget,
            stack,
            list_box,
            source_filter,
            search_chip,
            spinner,
        }
    }

    pub fn show_empty(&self) {
        self.stack.set_visible_child_name("empty");
    }

    pub fn show_list(&self) {
        self.stack.set_visible_child_name("list");
    }

    pub fn set_loading(&self, loading: bool) {
        self.spinner.set_visible(loading);
        if loading {
            self.spinner.start();
            self.stack.set_visible_child_name("skeleton");
        } else {
            self.spinner.stop();
        }
    }

    pub fn show_skeleton(&self) {
        self.stack.set_visible_child_name("skeleton");
    }

    pub fn clear(&self) {
        while let Some(child) = self.list_box.first_child() {
            self.list_box.remove(&child);
        }
    }
}

fn create_search_chip() -> gtk::Button {
    let b = gtk::Button::builder().label("").build();
    b.add_css_class("flat");
    b.add_css_class("chip-btn");
    b.set_visible(false);
    b
}
