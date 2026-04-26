use gtk4::prelude::*;
use gtk4::{self as gtk, gio};
use libadwaita as adw;
use libadwaita::prelude::*;

#[allow(dead_code)]
pub struct Header {
    pub widget: gtk::Box,
    pub search_entry: gtk::SearchEntry,
    pub search_popover: gtk::Popover,
    pub recent_searches_box: gtk::ListBox,
    pub refresh_button: gtk::Button,
    pub select_button: gtk::ToggleButton,
    pub command_center_btn: gtk::ToggleButton,
    pub command_center_badge: gtk::Label,
    pub maximize_button: gtk::Button,
    pub list_view_btn: gtk::ToggleButton,
    pub grid_view_btn: gtk::ToggleButton,
    pub browse_tab: gtk::ToggleButton,
    pub updates_tab: gtk::ToggleButton,
    pub installed_tab: gtk::ToggleButton,
    pub sources_tab: gtk::ToggleButton,
    pub queue_tab: gtk::ToggleButton,
    pub health_tab: gtk::ToggleButton,
    pub top_tasks_label: gtk::Label,
    pub top_failed_label: gtk::Label,
    pub top_time_label: gtk::Label,
}

impl Header {
    pub fn new() -> Self {
        let widget = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(18)
            .css_classes(vec!["ops-topbar"])
            .build();

        let menu = gio::Menu::new();

        let backup_section = gio::Menu::new();
        backup_section.append(Some("Import Packages..."), Some("app.import"));
        backup_section.append(Some("Export Packages..."), Some("app.export"));
        menu.append_section(Some("Backup"), &backup_section);

        let app_section = gio::Menu::new();
        app_section.append(Some("Preferences"), Some("app.preferences"));
        app_section.append(Some("Diagnostics"), Some("app.diagnostics"));
        app_section.append(Some("Keyboard Shortcuts"), Some("app.shortcuts"));
        app_section.append(Some("About LinGet"), Some("app.about"));
        menu.append_section(None, &app_section);

        let brand = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(10)
            .valign(gtk::Align::Center)
            .build();

        let brand_name = gtk::Label::builder().label("LinGet").build();
        brand_name.add_css_class("ops-brand");

        let brand_dot = gtk::Label::builder().label("•").build();
        brand_dot.add_css_class("ops-muted");

        let brand_tagline = gtk::Label::builder()
            .label("The Linux Package Manager")
            .build();
        brand_tagline.add_css_class("ops-muted");

        brand.append(&brand_name);
        brand.append(&brand_dot);
        brand.append(&brand_tagline);

        let nav = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(6)
            .hexpand(true)
            .halign(gtk::Align::Center)
            .valign(gtk::Align::Center)
            .css_classes(vec!["ops-tabs"])
            .build();

        let browse_tab = gtk::ToggleButton::builder().label("Browse").build();
        browse_tab.add_css_class("ops-tab");

        let updates_tab = gtk::ToggleButton::builder()
            .label("Updates")
            .group(&browse_tab)
            .build();
        updates_tab.add_css_class("ops-tab");

        let installed_tab = gtk::ToggleButton::builder()
            .label("Installed")
            .group(&browse_tab)
            .build();
        installed_tab.add_css_class("ops-tab");

        let sources_tab = gtk::ToggleButton::builder()
            .label("Sources")
            .group(&browse_tab)
            .build();
        sources_tab.add_css_class("ops-tab");

        let queue_tab = gtk::ToggleButton::builder()
            .label("Queue")
            .group(&browse_tab)
            .build();
        queue_tab.add_css_class("ops-tab");

        let health_tab = gtk::ToggleButton::builder()
            .label("Health")
            .group(&browse_tab)
            .build();
        health_tab.add_css_class("ops-tab");

        nav.append(&browse_tab);
        nav.append(&updates_tab);
        nav.append(&installed_tab);
        nav.append(&sources_tab);
        nav.append(&queue_tab);
        nav.append(&health_tab);

        let status = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(14)
            .valign(gtk::Align::Center)
            .halign(gtk::Align::End)
            .build();

        let top_tasks_label = gtk::Label::builder().label("Tasks: 0").build();
        top_tasks_label.add_css_class("ops-muted");

        let separator_a = gtk::Label::builder().label("|").build();
        separator_a.add_css_class("ops-muted");

        let top_failed_label = gtk::Label::builder().label("Failed: 0").build();
        top_failed_label.add_css_class("ops-danger-text");

        let top_time_label = gtk::Label::builder()
            .label(chrono::Local::now().format("%H:%M").to_string())
            .build();
        top_time_label.add_css_class("ops-muted");

        status.append(&top_tasks_label);
        status.append(&separator_a);
        status.append(&top_failed_label);
        status.append(&top_time_label);

        let command_center_btn = gtk::ToggleButton::builder()
            .icon_name("format-justify-fill-symbolic")
            .tooltip_text("Command Center")
            .build();
        command_center_btn.add_css_class("flat");
        command_center_btn.add_css_class("ops-icon-button");

        let command_center_badge = gtk::Label::builder().label("0").visible(false).build();
        command_center_badge.add_css_class("badge-accent");
        command_center_badge.set_valign(gtk::Align::Center);

        let cmd_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(6)
            .build();
        cmd_box.append(&command_center_btn);
        cmd_box.append(&command_center_badge);

        let refresh_button = gtk::Button::builder()
            .icon_name("view-refresh-symbolic")
            .tooltip_text("Refresh (Ctrl+R)")
            .build();
        refresh_button.add_css_class("flat");
        refresh_button.add_css_class("ops-hidden-tool");

        let select_button = gtk::ToggleButton::builder()
            .icon_name("selection-mode-symbolic")
            .tooltip_text("Selection Mode (Ctrl+S)")
            .build();
        select_button.add_css_class("flat");
        select_button.add_css_class("ops-hidden-tool");

        let maximize_button = gtk::Button::builder()
            .icon_name("window-maximize-symbolic")
            .tooltip_text("Maximize")
            .build();
        maximize_button.add_css_class("flat");
        maximize_button.add_css_class("ops-icon-button");

        let menu_button = gtk::MenuButton::builder()
            .icon_name("open-menu-symbolic")
            .menu_model(&menu)
            .tooltip_text("Main Menu (F10)")
            .build();
        menu_button.add_css_class("flat");
        menu_button.add_css_class("ops-icon-button");

        let version_label = gtk::Label::builder()
            .label(concat!("v", env!("CARGO_PKG_VERSION")))
            .visible(false)
            .build();
        version_label.add_css_class("dim-label");
        version_label.add_css_class("caption");

        let list_view_btn = gtk::ToggleButton::builder()
            .icon_name("view-list-symbolic")
            .tooltip_text("List View")
            .build();
        list_view_btn.add_css_class("flat");
        list_view_btn.add_css_class("ops-hidden-tool");

        let grid_view_btn = gtk::ToggleButton::builder()
            .icon_name("view-grid-symbolic")
            .tooltip_text("Grid View")
            .group(&list_view_btn)
            .build();
        grid_view_btn.add_css_class("flat");
        grid_view_btn.add_css_class("ops-hidden-tool");

        let view_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .css_classes(vec!["linked"])
            .visible(false)
            .build();
        view_box.append(&list_view_btn);
        view_box.append(&grid_view_btn);

        let search_entry = gtk::SearchEntry::builder()
            .placeholder_text("Search packages... (/ or Ctrl+F)")
            .hexpand(true)
            .visible(false)
            .build();
        search_entry.add_css_class("search-entry-large");

        let recent_searches_box = gtk::ListBox::builder()
            .selection_mode(gtk::SelectionMode::None)
            .build();
        recent_searches_box.add_css_class("boxed-list");
        recent_searches_box.add_css_class("recent-searches-list");

        let recent_header = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(8)
            .margin_start(12)
            .margin_end(12)
            .margin_top(8)
            .margin_bottom(4)
            .build();

        let recent_icon = gtk::Image::builder()
            .icon_name("document-open-recent-symbolic")
            .pixel_size(16)
            .build();
        recent_icon.add_css_class("dim-label");

        let recent_label = gtk::Label::builder()
            .label("Recent Searches")
            .halign(gtk::Align::Start)
            .hexpand(true)
            .build();
        recent_label.add_css_class("caption");
        recent_label.add_css_class("dim-label");

        recent_header.append(&recent_icon);
        recent_header.append(&recent_label);

        let popover_content = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(4)
            .margin_top(8)
            .margin_bottom(8)
            .build();
        popover_content.append(&recent_header);
        popover_content.append(&recent_searches_box);

        let search_popover = gtk::Popover::builder()
            .child(&popover_content)
            .has_arrow(true)
            .position(gtk::PositionType::Bottom)
            .autohide(true)
            .build();
        search_popover.add_css_class("recent-searches-popover");
        search_popover.set_parent(&search_entry);

        let search_clamp = adw::Clamp::builder()
            .maximum_size(500)
            .child(&search_entry)
            .visible(false)
            .build();

        let utility_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(6)
            .valign(gtk::Align::Center)
            .build();
        utility_box.append(&refresh_button);
        utility_box.append(&select_button);
        utility_box.append(&cmd_box);
        utility_box.append(&view_box);
        utility_box.append(&version_label);
        utility_box.append(&search_clamp);
        utility_box.append(&maximize_button);
        utility_box.append(&menu_button);

        widget.append(&brand);
        widget.append(&nav);
        widget.append(&status);
        widget.append(&utility_box);

        Self {
            widget,
            search_entry,
            search_popover,
            recent_searches_box,
            refresh_button,
            select_button,
            command_center_btn,
            command_center_badge,
            maximize_button,
            list_view_btn,
            grid_view_btn,
            browse_tab,
            updates_tab,
            installed_tab,
            sources_tab,
            queue_tab,
            health_tab,
            top_tasks_label,
            top_failed_label,
            top_time_label,
        }
    }

    pub fn update_recent_searches(&self, searches: &[String]) {
        while let Some(child) = self.recent_searches_box.first_child() {
            self.recent_searches_box.remove(&child);
        }

        if searches.is_empty() {
            let empty_row = adw::ActionRow::builder()
                .title("No recent searches")
                .subtitle("Your search history will appear here")
                .build();
            empty_row.add_css_class("dim-label");
            self.recent_searches_box.append(&empty_row);
        } else {
            for query in searches {
                let row = adw::ActionRow::builder()
                    .title(query)
                    .activatable(true)
                    .build();

                let icon = gtk::Image::builder()
                    .icon_name("edit-find-symbolic")
                    .build();
                icon.add_css_class("dim-label");
                row.add_prefix(&icon);

                let arrow = gtk::Image::builder().icon_name("go-next-symbolic").build();
                arrow.add_css_class("dim-label");
                row.add_suffix(&arrow);

                self.recent_searches_box.append(&row);
            }
        }
    }
}

impl Default for Header {
    fn default() -> Self {
        Self::new()
    }
}
