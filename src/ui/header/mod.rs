use gtk4::prelude::*;
use gtk4::{self as gtk, gio};
use libadwaita as adw;
use libadwaita::prelude::*;

#[allow(dead_code)]
pub struct Header {
    pub widget: adw::HeaderBar,
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
}

impl Header {
    pub fn new() -> Self {
        let widget = adw::HeaderBar::builder()
            .show_end_title_buttons(true)
            .show_start_title_buttons(true)
            .decoration_layout(":minimize,maximize,close")
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

        let command_center_btn = gtk::ToggleButton::builder()
            .icon_name("format-justify-fill-symbolic")
            .tooltip_text("Command Center")
            .build();
        command_center_btn.add_css_class("flat");

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
        widget.pack_end(&refresh_button);

        let select_button = gtk::ToggleButton::builder()
            .icon_name("selection-mode-symbolic")
            .tooltip_text("Selection Mode (Ctrl+S)")
            .build();
        select_button.add_css_class("flat");
        widget.pack_end(&select_button);

        widget.pack_end(&cmd_box);

        let maximize_button = gtk::Button::builder()
            .icon_name("window-maximize-symbolic")
            .tooltip_text("Maximize")
            .build();
        maximize_button.add_css_class("flat");
        widget.pack_end(&maximize_button);

        let menu_button = gtk::MenuButton::builder()
            .icon_name("open-menu-symbolic")
            .menu_model(&menu)
            .tooltip_text("Main Menu (F10)")
            .build();
        widget.pack_end(&menu_button);

        let version_label = gtk::Label::builder()
            .label(concat!("v", env!("CARGO_PKG_VERSION")))
            .build();
        version_label.add_css_class("dim-label");
        version_label.add_css_class("caption");
        widget.pack_start(&version_label);

        let list_view_btn = gtk::ToggleButton::builder()
            .icon_name("view-list-symbolic")
            .tooltip_text("List View")
            .build();
        list_view_btn.add_css_class("flat");

        let grid_view_btn = gtk::ToggleButton::builder()
            .icon_name("view-grid-symbolic")
            .tooltip_text("Grid View")
            .group(&list_view_btn)
            .build();
        grid_view_btn.add_css_class("flat");

        let view_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .css_classes(vec!["linked"])
            .build();
        view_box.append(&list_view_btn);
        view_box.append(&grid_view_btn);
        widget.pack_start(&view_box);

        let search_entry = gtk::SearchEntry::builder()
            .placeholder_text("Search packages... (/ or Ctrl+F)")
            .hexpand(true)
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
            .build();

        widget.set_title_widget(Some(&search_clamp));

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
