use gtk4::prelude::*;
use gtk4::{self as gtk, gio};
use libadwaita as adw;

pub struct Header {
    pub widget: adw::HeaderBar,
    pub search_entry: gtk::SearchEntry,
    pub refresh_button: gtk::Button,
    pub undo_button: gtk::Button,
    pub select_button: gtk::ToggleButton,
    pub command_center_btn: gtk::ToggleButton,
    pub command_center_badge: gtk::Label,
}

impl Header {
    pub fn new() -> Self {
        let widget = adw::HeaderBar::builder()
            .show_end_title_buttons(true)
            .show_start_title_buttons(true)
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

        let undo_button = gtk::Button::builder()
            .icon_name("edit-undo-symbolic")
            .tooltip_text("Undo Last Operation (Ctrl+Z)")
            .sensitive(false)
            .visible(false)
            .build();
        undo_button.add_css_class("flat");
        widget.pack_end(&undo_button);

        let select_button = gtk::ToggleButton::builder()
            .icon_name("selection-mode-symbolic")
            .tooltip_text("Selection Mode (Ctrl+S)")
            .build();
        select_button.add_css_class("flat");
        widget.pack_end(&select_button);

        widget.pack_end(&cmd_box);

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

        let search_entry = gtk::SearchEntry::builder()
            .placeholder_text("Search packages... (/ or Ctrl+F)")
            .hexpand(true)
            .build();
        search_entry.add_css_class("search-entry-large");

        let search_clamp = adw::Clamp::builder()
            .maximum_size(500)
            .child(&search_entry)
            .build();

        widget.set_title_widget(Some(&search_clamp));

        Self {
            widget,
            search_entry,
            refresh_button,
            undo_button,
            select_button,
            command_center_btn,
            command_center_badge,
        }
    }
}

impl Default for Header {
    fn default() -> Self {
        Self::new()
    }
}
