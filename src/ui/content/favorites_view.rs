use crate::ui::{EmptyState, SkeletonList};
use gtk4::prelude::*;
use gtk4::{self as gtk, gio, glib};
use libadwaita as adw;

pub struct FavoritesView {
    pub widget: gtk::Box,
    pub stack: gtk::Stack,
    pub store: gio::ListStore,
    pub list_view: gtk::ListView,
}

impl FavoritesView {
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
        header.set_title_widget(Some(
            &adw::WindowTitle::builder().title("Favorites").build(),
        ));

        let list_area = adw::Clamp::builder()
            .maximum_size(1600)
            .tightening_threshold(1200)
            .child(&scrolled)
            .margin_top(8)
            .margin_bottom(24)
            .margin_start(24)
            .margin_end(24)
            .build();

        let empty = EmptyState::no_favorites().widget;

        let skeleton = SkeletonList::new(4).widget;
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
        }
    }

    pub fn show_empty(&self) {
        self.stack.set_visible_child_name("empty");
    }

    pub fn show_list(&self) {
        self.stack.set_visible_child_name("list");
    }

    pub fn show_skeleton(&self) {
        self.stack.set_visible_child_name("skeleton");
    }
}
