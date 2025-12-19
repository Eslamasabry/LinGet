#![allow(dead_code)]

use gtk4::prelude::*;
use gtk4::{self as gtk};

pub struct ProgressOverlay {
    pub widget: gtk::Box,
    pub progress_bar: gtk::ProgressBar,
    pub label: gtk::Label,
}

impl ProgressOverlay {
    pub fn new() -> Self {
        let widget = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .valign(gtk::Align::Fill)
            .halign(gtk::Align::Fill)
            .vexpand(true)
            .hexpand(true)
            .visible(false)
            .build();
        widget.add_css_class("progress-scrim");

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

        let progress_bar = gtk::ProgressBar::builder().show_text(true).build();
        progress_bar.add_css_class("osd");
        progress_bar.set_height_request(10);

        card.append(&label);
        card.append(&progress_bar);
        widget.append(&card);

        Self {
            widget,
            progress_bar,
            label,
        }
    }

    pub fn show(&self) {
        self.widget.set_visible(true);
    }

    pub fn hide(&self) {
        self.widget.set_visible(false);
    }

    pub fn set_progress(&self, fraction: f64, text: Option<&str>) {
        self.progress_bar.set_fraction(fraction);
        if let Some(t) = text {
            self.progress_bar.set_text(Some(t));
        }
    }

    pub fn set_label(&self, text: &str) {
        self.label.set_label(text);
    }
}

impl Default for ProgressOverlay {
    fn default() -> Self {
        Self::new()
    }
}
