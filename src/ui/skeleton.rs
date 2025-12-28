#![allow(dead_code)]

use gtk4::prelude::*;
use gtk4::{self as gtk};

pub struct SkeletonRow {
    pub widget: gtk::Box,
}

impl SkeletonRow {
    pub fn new() -> Self {
        let row = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(12)
            .margin_start(16)
            .margin_end(16)
            .margin_top(12)
            .margin_bottom(12)
            .build();

        let icon_placeholder = gtk::Box::builder()
            .width_request(48)
            .height_request(48)
            .build();
        icon_placeholder.add_css_class("skeleton-block");
        icon_placeholder.add_css_class("skeleton-shimmer");

        let text_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(8)
            .valign(gtk::Align::Center)
            .hexpand(true)
            .build();

        let title_placeholder = gtk::Box::builder()
            .width_request(180)
            .height_request(16)
            .halign(gtk::Align::Start)
            .build();
        title_placeholder.add_css_class("skeleton-block");
        title_placeholder.add_css_class("skeleton-shimmer");

        let subtitle_placeholder = gtk::Box::builder()
            .width_request(240)
            .height_request(12)
            .halign(gtk::Align::Start)
            .build();
        subtitle_placeholder.add_css_class("skeleton-block");
        subtitle_placeholder.add_css_class("skeleton-shimmer");

        text_box.append(&title_placeholder);
        text_box.append(&subtitle_placeholder);

        let chip_placeholder = gtk::Box::builder()
            .width_request(60)
            .height_request(20)
            .valign(gtk::Align::Center)
            .build();
        chip_placeholder.add_css_class("skeleton-block");
        chip_placeholder.add_css_class("skeleton-shimmer");

        row.append(&icon_placeholder);
        row.append(&text_box);
        row.append(&chip_placeholder);

        Self { widget: row }
    }

    pub fn new_compact() -> Self {
        let row = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(10)
            .margin_start(12)
            .margin_end(12)
            .margin_top(8)
            .margin_bottom(8)
            .build();

        let icon_placeholder = gtk::Box::builder()
            .width_request(32)
            .height_request(32)
            .build();
        icon_placeholder.add_css_class("skeleton-block");
        icon_placeholder.add_css_class("skeleton-shimmer");

        let text_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(6)
            .valign(gtk::Align::Center)
            .hexpand(true)
            .build();

        let title_placeholder = gtk::Box::builder()
            .width_request(140)
            .height_request(14)
            .halign(gtk::Align::Start)
            .build();
        title_placeholder.add_css_class("skeleton-block");
        title_placeholder.add_css_class("skeleton-shimmer");

        let subtitle_placeholder = gtk::Box::builder()
            .width_request(200)
            .height_request(10)
            .halign(gtk::Align::Start)
            .build();
        subtitle_placeholder.add_css_class("skeleton-block");
        subtitle_placeholder.add_css_class("skeleton-shimmer");

        text_box.append(&title_placeholder);
        text_box.append(&subtitle_placeholder);

        row.append(&icon_placeholder);
        row.append(&text_box);

        Self { widget: row }
    }
}

impl Default for SkeletonRow {
    fn default() -> Self {
        Self::new()
    }
}

pub struct SkeletonList {
    pub widget: gtk::ListBox,
}

impl SkeletonList {
    pub fn new(count: usize) -> Self {
        let list = gtk::ListBox::builder()
            .selection_mode(gtk::SelectionMode::None)
            .build();
        list.add_css_class("boxed-list");
        list.add_css_class("skeleton-list");

        for _ in 0..count {
            let row = SkeletonRow::new();
            list.append(&row.widget);
        }

        Self { widget: list }
    }

    pub fn new_compact(count: usize) -> Self {
        let list = gtk::ListBox::builder()
            .selection_mode(gtk::SelectionMode::None)
            .build();
        list.add_css_class("boxed-list");
        list.add_css_class("skeleton-list");

        for _ in 0..count {
            let row = SkeletonRow::new_compact();
            list.append(&row.widget);
        }

        Self { widget: list }
    }
}

pub struct SkeletonDetails {
    pub widget: gtk::Box,
}

impl SkeletonDetails {
    pub fn new() -> Self {
        let container = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(16)
            .margin_start(24)
            .margin_end(24)
            .margin_top(16)
            .margin_bottom(24)
            .build();

        let header = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(16)
            .build();

        let icon_placeholder = gtk::Box::builder()
            .width_request(64)
            .height_request(64)
            .build();
        icon_placeholder.add_css_class("skeleton-block");
        icon_placeholder.add_css_class("skeleton-shimmer");

        let title_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(8)
            .valign(gtk::Align::Center)
            .build();

        let title_placeholder = gtk::Box::builder()
            .width_request(200)
            .height_request(24)
            .halign(gtk::Align::Start)
            .build();
        title_placeholder.add_css_class("skeleton-block");
        title_placeholder.add_css_class("skeleton-shimmer");

        let badge_placeholder = gtk::Box::builder()
            .width_request(80)
            .height_request(20)
            .halign(gtk::Align::Start)
            .build();
        badge_placeholder.add_css_class("skeleton-block");
        badge_placeholder.add_css_class("skeleton-shimmer");

        title_box.append(&title_placeholder);
        title_box.append(&badge_placeholder);

        header.append(&icon_placeholder);
        header.append(&title_box);

        let desc_placeholder = gtk::Box::builder().height_request(48).hexpand(true).build();
        desc_placeholder.add_css_class("skeleton-block");
        desc_placeholder.add_css_class("skeleton-shimmer");

        let details_group = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(12)
            .build();

        for _ in 0..3 {
            let row = gtk::Box::builder()
                .orientation(gtk::Orientation::Horizontal)
                .spacing(12)
                .build();

            let label = gtk::Box::builder()
                .width_request(80)
                .height_request(16)
                .build();
            label.add_css_class("skeleton-block");
            label.add_css_class("skeleton-shimmer");

            let value = gtk::Box::builder()
                .width_request(120)
                .height_request(16)
                .hexpand(true)
                .build();
            value.add_css_class("skeleton-block");
            value.add_css_class("skeleton-shimmer");

            row.append(&label);
            row.append(&value);
            details_group.append(&row);
        }

        let button_placeholder = gtk::Box::builder()
            .width_request(120)
            .height_request(36)
            .halign(gtk::Align::Start)
            .margin_top(8)
            .build();
        button_placeholder.add_css_class("skeleton-block");
        button_placeholder.add_css_class("skeleton-shimmer");

        container.append(&header);
        container.append(&desc_placeholder);
        container.append(&details_group);
        container.append(&button_placeholder);

        Self { widget: container }
    }
}

impl Default for SkeletonDetails {
    fn default() -> Self {
        Self::new()
    }
}

pub struct SkeletonCard {
    pub widget: gtk::Box,
}

impl SkeletonCard {
    pub fn new() -> Self {
        let card = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(10)
            .width_request(200)
            .height_request(260)
            .margin_top(32)
            .margin_bottom(12)
            .margin_start(14)
            .margin_end(14)
            .build();
        card.add_css_class("pkg-card");
        card.add_css_class("skeleton-card");

        let icon_placeholder = gtk::Box::builder()
            .width_request(80)
            .height_request(80)
            .halign(gtk::Align::Center)
            .build();
        icon_placeholder.add_css_class("skeleton-block");
        icon_placeholder.add_css_class("skeleton-shimmer");
        icon_placeholder.add_css_class("card-icon-frame");

        let title_placeholder = gtk::Box::builder()
            .width_request(120)
            .height_request(16)
            .halign(gtk::Align::Center)
            .build();
        title_placeholder.add_css_class("skeleton-block");
        title_placeholder.add_css_class("skeleton-shimmer");

        let chips_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(6)
            .halign(gtk::Align::Center)
            .build();

        let version_chip = gtk::Box::builder()
            .width_request(50)
            .height_request(18)
            .build();
        version_chip.add_css_class("skeleton-block");
        version_chip.add_css_class("skeleton-shimmer");

        let source_chip = gtk::Box::builder()
            .width_request(45)
            .height_request(18)
            .build();
        source_chip.add_css_class("skeleton-block");
        source_chip.add_css_class("skeleton-shimmer");

        chips_box.append(&version_chip);
        chips_box.append(&source_chip);

        let desc_placeholder = gtk::Box::builder()
            .width_request(160)
            .height_request(28)
            .halign(gtk::Align::Center)
            .build();
        desc_placeholder.add_css_class("skeleton-block");
        desc_placeholder.add_css_class("skeleton-shimmer");

        card.append(&icon_placeholder);
        card.append(&title_placeholder);
        card.append(&chips_box);
        card.append(&desc_placeholder);

        Self { widget: card }
    }

    pub fn new_compact() -> Self {
        let card = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(6)
            .width_request(160)
            .height_request(200)
            .margin_top(28)
            .margin_bottom(8)
            .margin_start(10)
            .margin_end(10)
            .build();
        card.add_css_class("pkg-card");
        card.add_css_class("compact-card");
        card.add_css_class("skeleton-card");

        let icon_placeholder = gtk::Box::builder()
            .width_request(64)
            .height_request(64)
            .halign(gtk::Align::Center)
            .build();
        icon_placeholder.add_css_class("skeleton-block");
        icon_placeholder.add_css_class("skeleton-shimmer");
        icon_placeholder.add_css_class("card-icon-frame");

        let title_placeholder = gtk::Box::builder()
            .width_request(100)
            .height_request(14)
            .halign(gtk::Align::Center)
            .build();
        title_placeholder.add_css_class("skeleton-block");
        title_placeholder.add_css_class("skeleton-shimmer");

        let chip_placeholder = gtk::Box::builder()
            .width_request(45)
            .height_request(16)
            .halign(gtk::Align::Center)
            .build();
        chip_placeholder.add_css_class("skeleton-block");
        chip_placeholder.add_css_class("skeleton-shimmer");

        card.append(&icon_placeholder);
        card.append(&title_placeholder);
        card.append(&chip_placeholder);

        Self { widget: card }
    }
}

impl Default for SkeletonCard {
    fn default() -> Self {
        Self::new()
    }
}

pub struct SkeletonGrid {
    pub widget: gtk::FlowBox,
}

impl SkeletonGrid {
    pub fn new(count: usize) -> Self {
        let grid = gtk::FlowBox::builder()
            .selection_mode(gtk::SelectionMode::None)
            .row_spacing(16)
            .column_spacing(16)
            .halign(gtk::Align::Fill)
            .homogeneous(true)
            .min_children_per_line(2)
            .max_children_per_line(8)
            .margin_top(8)
            .margin_bottom(8)
            .margin_start(8)
            .margin_end(8)
            .build();
        grid.add_css_class("skeleton-grid");

        for _ in 0..count {
            let card = SkeletonCard::new();
            grid.insert(&card.widget, -1);
        }

        Self { widget: grid }
    }

    pub fn new_compact(count: usize) -> Self {
        let grid = gtk::FlowBox::builder()
            .selection_mode(gtk::SelectionMode::None)
            .row_spacing(12)
            .column_spacing(12)
            .halign(gtk::Align::Fill)
            .homogeneous(true)
            .min_children_per_line(3)
            .max_children_per_line(10)
            .margin_top(8)
            .margin_bottom(8)
            .margin_start(8)
            .margin_end(8)
            .build();
        grid.add_css_class("skeleton-grid");

        for _ in 0..count {
            let card = SkeletonCard::new_compact();
            grid.insert(&card.widget, -1);
        }

        Self { widget: grid }
    }
}
