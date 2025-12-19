use super::SourceFilterWidgets;
use crate::ui::{EmptyState, SkeletonList};
use gtk4::prelude::*;
use gtk4::{self as gtk, gio, glib};
use libadwaita as adw;

pub struct UpdatesView {
    pub widget: gtk::Box,
    pub stack: gtk::Stack,
    pub store: gio::ListStore,
    pub list_view: gtk::ListView,
    pub source_filter: SourceFilterWidgets,
    pub search_chip: gtk::Button,
    pub update_all_btn: gtk::Button,
    pub spinner: gtk::Spinner,
}

impl UpdatesView {
    pub fn new() -> Self {
        let store = gio::ListStore::new::<glib::BoxedAnyObject>();
        let model = gtk::NoSelection::new(Some(store.clone()));
        let list_view = gtk::ListView::new(Some(model), None::<gtk::ListItemFactory>);
        list_view.add_css_class("boxed-list");

        let scrolled = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vexpand(true)
            .child(&list_view)
            .build();

        let header = adw::HeaderBar::builder()
            .show_start_title_buttons(false)
            .show_end_title_buttons(false)
            .build();
        header.add_css_class("view-toolbar");
        header.set_title_widget(Some(&adw::WindowTitle::builder().title("Updates").build()));

        let source_filter = SourceFilterWidgets::new();
        let search_chip = create_search_chip();

        let filters = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(6)
            .build();
        filters.append(&source_filter.menu_btn);
        filters.append(&search_chip);
        header.pack_start(&filters);

        let update_all_btn = gtk::Button::builder()
            .label("Update All")
            .css_classes(vec!["suggested-action", "pill"])
            .build();
        header.pack_end(&update_all_btn);

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

        let empty = EmptyState::all_up_to_date().widget;
        empty.add_css_class("success-status");

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

        let widget = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();
        widget.append(&header);
        widget.append(&stack);

        Self {
            widget,
            stack,
            store,
            list_view,
            source_filter,
            search_chip,
            update_all_btn,
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
}

fn create_search_chip() -> gtk::Button {
    let b = gtk::Button::builder().label("").build();
    b.add_css_class("flat");
    b.add_css_class("chip-btn");
    b.set_visible(false);
    b
}
