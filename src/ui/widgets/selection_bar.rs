#![allow(dead_code)]

use gtk4::prelude::*;
use gtk4::{self as gtk};

pub struct SelectionBar {
    pub widget: gtk::ActionBar,
    pub select_all_btn: gtk::Button,
    pub deselect_all_btn: gtk::Button,
    pub update_selected_btn: gtk::Button,
    pub remove_selected_btn: gtk::Button,
    pub count_label: gtk::Label,
}

impl SelectionBar {
    pub fn new() -> Self {
        let widget = gtk::ActionBar::builder().visible(false).build();
        widget.add_css_class("selection-bar");

        let select_all_btn = gtk::Button::builder().label("Select All").build();
        select_all_btn.add_css_class("flat");

        let deselect_all_btn = gtk::Button::builder().label("Deselect All").build();
        deselect_all_btn.add_css_class("flat");

        let count_label = gtk::Label::builder()
            .label("0 selected")
            .hexpand(true)
            .build();

        let update_selected_btn = gtk::Button::builder().label("Update Selected").build();
        update_selected_btn.add_css_class("suggested-action");

        let remove_selected_btn = gtk::Button::builder().label("Remove Selected").build();
        remove_selected_btn.add_css_class("destructive-action");

        widget.pack_start(&select_all_btn);
        widget.pack_start(&deselect_all_btn);
        widget.set_center_widget(Some(&count_label));
        widget.pack_end(&remove_selected_btn);
        widget.pack_end(&update_selected_btn);

        Self {
            widget,
            select_all_btn,
            deselect_all_btn,
            update_selected_btn,
            remove_selected_btn,
            count_label,
        }
    }

    pub fn show(&self) {
        self.widget.set_visible(true);
    }

    pub fn hide(&self) {
        self.widget.set_visible(false);
    }

    pub fn update_count(&self, count: usize) {
        self.count_label.set_label(&format!("{} selected", count));
    }
}

impl Default for SelectionBar {
    fn default() -> Self {
        Self::new()
    }
}
