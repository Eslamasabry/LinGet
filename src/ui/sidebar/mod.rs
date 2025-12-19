mod navigation;
mod providers;

pub use navigation::NavigationSection;
#[allow(unused_imports)]
pub use providers::{set_enabled_in_config, ProviderRowWidgets, ProvidersSection};

use gtk4::prelude::*;
use gtk4::{self as gtk};

pub struct Sidebar {
    pub widget: gtk::Box,
    pub navigation: NavigationSection,
    pub providers: ProvidersSection,
    pub total_size_label: gtk::Label,
}

impl Sidebar {
    pub fn new() -> Self {
        let widget = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .width_request(220)
            .css_classes(vec!["sidebar"])
            .build();

        let header = gtk::Box::builder()
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

        header.append(&app_icon);
        header.append(&app_title);
        widget.append(&header);

        let navigation = NavigationSection::new();
        widget.append(&navigation.widget);

        let providers = ProvidersSection::new();
        widget.append(&providers.widget);

        widget.append(&gtk::Box::builder().vexpand(true).build());

        let stats_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .margin_start(16)
            .margin_end(16)
            .margin_bottom(16)
            .spacing(4)
            .build();

        let total_size_label = gtk::Label::builder()
            .label("")
            .xalign(0.0)
            .visible(false)
            .build();
        total_size_label.add_css_class("caption");
        total_size_label.add_css_class("dim-label");
        total_size_label.set_tooltip_text(Some("Total disk space used by installed packages"));
        stats_box.append(&total_size_label);

        let stats_label = gtk::Label::builder()
            .label("Last updated: Just now")
            .xalign(0.0)
            .build();
        stats_label.add_css_class("caption");
        stats_label.add_css_class("dim-label");
        stats_box.append(&stats_label);

        widget.append(&stats_box);

        Self {
            widget,
            navigation,
            providers,
            total_size_label,
        }
    }

    #[allow(dead_code)]
    pub fn set_total_size(&self, size_text: &str) {
        if size_text.is_empty() {
            self.total_size_label.set_visible(false);
        } else {
            self.total_size_label.set_label(size_text);
            self.total_size_label.set_visible(true);
        }
    }
}
