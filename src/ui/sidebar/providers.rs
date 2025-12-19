#![allow(dead_code)]

use crate::models::{Config, PackageSource};
use gtk4::prelude::*;
use gtk4::{self as gtk, glib};
use std::collections::{HashMap, HashSet};

#[derive(Clone)]
pub struct ProviderRowWidgets {
    pub row: gtk::Box,
    pub enabled_switch: gtk::Switch,
    pub count_label: gtk::Label,
    pub size_label: gtk::Label,
    pub status_label: gtk::Label,
}

pub struct ProvidersSection {
    pub widget: gtk::Box,
    pub providers_box: gtk::Box,
    pub provider_rows: HashMap<PackageSource, ProviderRowWidgets>,
    pub provider_counts: HashMap<PackageSource, gtk::Label>,
    pub enable_detected_btn: gtk::Button,
}

impl ProvidersSection {
    pub fn new() -> Self {
        let providers_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(2)
            .margin_start(8)
            .margin_end(8)
            .build();

        let mut provider_rows = HashMap::new();
        let mut provider_counts = HashMap::new();

        for source in PackageSource::ALL {
            let row = create_provider_row(source);
            provider_counts.insert(source, row.count_label.clone());
            providers_box.append(&row.row);
            provider_rows.insert(source, row);
        }

        let header = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(8)
            .margin_top(24)
            .margin_start(16)
            .margin_end(16)
            .margin_bottom(8)
            .build();

        let label = gtk::Label::builder()
            .label("Providers")
            .xalign(0.0)
            .hexpand(true)
            .build();
        label.add_css_class("caption");
        label.add_css_class("dim-label");

        let enable_detected_btn = gtk::Button::builder()
            .label("Enable detected")
            .tooltip_text("Enable all detected providers")
            .build();
        enable_detected_btn.add_css_class("flat");
        enable_detected_btn.add_css_class("pill");

        let toggle = gtk::ToggleButton::builder()
            .icon_name("pan-down-symbolic")
            .active(true)
            .tooltip_text("Show/hide providers")
            .build();
        toggle.add_css_class("flat");
        toggle.add_css_class("circular");

        header.append(&label);
        header.append(&toggle);
        header.append(&enable_detected_btn);

        let revealer = gtk::Revealer::builder()
            .reveal_child(true)
            .transition_type(gtk::RevealerTransitionType::SlideDown)
            .child(&providers_box)
            .build();

        toggle.connect_toggled({
            let revealer = revealer.clone();
            move |b| revealer.set_reveal_child(b.is_active())
        });

        let widget = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();
        widget.append(&header);
        widget.append(&revealer);

        Self {
            widget,
            providers_box,
            provider_rows,
            provider_counts,
            enable_detected_btn,
        }
    }

    pub fn update_availability(
        &self,
        available: &HashSet<PackageSource>,
        enabled: &HashSet<PackageSource>,
    ) {
        for (source, row) in &self.provider_rows {
            let is_available = available.contains(source);
            let is_enabled = enabled.contains(source);

            row.enabled_switch.set_sensitive(is_available);
            row.enabled_switch.set_active(is_enabled && is_available);

            if is_available {
                row.status_label.set_visible(false);
                row.row.remove_css_class("provider-unavailable");
            } else {
                row.status_label.set_label("Not detected");
                row.status_label.set_visible(true);
                row.row.add_css_class("provider-unavailable");
            }
        }
    }

    pub fn connect_switches<F>(&self, on_toggle: F)
    where
        F: Fn(PackageSource, bool) + Clone + 'static,
    {
        for (source, row) in &self.provider_rows {
            let source = *source;
            let on_toggle = on_toggle.clone();
            row.enabled_switch.connect_state_set(move |_, state| {
                on_toggle(source, state);
                glib::Propagation::Proceed
            });
        }
    }

    pub fn connect_enable_detected<F>(&self, on_click: F)
    where
        F: Fn() + 'static,
    {
        self.enable_detected_btn
            .connect_clicked(move |_| on_click());
    }

    pub fn set_count(&self, source: PackageSource, count: usize) {
        if let Some(label) = self.provider_counts.get(&source) {
            label.set_label(&count.to_string());
        }
    }

    pub fn set_size(&self, source: PackageSource, size_text: &str) {
        if let Some(row) = self.provider_rows.get(&source) {
            row.size_label.set_label(size_text);
        }
    }
}

pub fn create_provider_row(source: PackageSource) -> ProviderRowWidgets {
    let row = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(10)
        .build();
    row.add_css_class("provider-row");

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
        .build();
    status.add_css_class("caption");
    status.add_css_class("dim-label");
    status.set_visible(false);
    labels.append(&title);
    labels.append(&status);

    let count_label = gtk::Label::new(Some("0"));
    count_label.add_css_class("dim-label");
    count_label.add_css_class("caption");

    let size_label = gtk::Label::new(Some(""));
    size_label.add_css_class("dim-label");
    size_label.add_css_class("caption");
    size_label.set_tooltip_text(Some("Disk space used by this source"));

    let enabled_switch = gtk::Switch::builder().valign(gtk::Align::Center).build();

    row.append(&dot);
    row.append(&labels);
    row.append(&count_label);
    row.append(&size_label);
    row.append(&enabled_switch);

    ProviderRowWidgets {
        row,
        enabled_switch,
        count_label,
        size_label,
        status_label: status,
    }
}

pub fn set_enabled_in_config(config: &mut Config, source: PackageSource, enabled: bool) {
    match source {
        PackageSource::Apt => config.enabled_sources.apt = enabled,
        PackageSource::Dnf => config.enabled_sources.dnf = enabled,
        PackageSource::Pacman => config.enabled_sources.pacman = enabled,
        PackageSource::Zypper => config.enabled_sources.zypper = enabled,
        PackageSource::Flatpak => config.enabled_sources.flatpak = enabled,
        PackageSource::Snap => config.enabled_sources.snap = enabled,
        PackageSource::Npm => config.enabled_sources.npm = enabled,
        PackageSource::Pip => config.enabled_sources.pip = enabled,
        PackageSource::Pipx => config.enabled_sources.pipx = enabled,
        PackageSource::Cargo => config.enabled_sources.cargo = enabled,
        PackageSource::Brew => config.enabled_sources.brew = enabled,
        PackageSource::Aur => config.enabled_sources.aur = enabled,
        PackageSource::Conda => config.enabled_sources.conda = enabled,
        PackageSource::Mamba => config.enabled_sources.mamba = enabled,
        PackageSource::Dart => config.enabled_sources.dart = enabled,
        PackageSource::Deb => config.enabled_sources.deb = enabled,
        PackageSource::AppImage => config.enabled_sources.appimage = enabled,
    }
}
