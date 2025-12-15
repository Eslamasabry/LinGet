use gtk4::prelude::*;
use gtk4::{self as gtk};

pub fn show_about_dialog(parent: &impl IsA<gtk::Window>) {
    // Use app icon if available, fall back to generic package icon
    let logo_icon = {
        let custom = crate::app::APP_ID;
        if let Some(display) = gtk::gdk::Display::default() {
            if gtk::IconTheme::for_display(&display).has_icon(custom) {
                custom
            } else {
                "system-software-install"
            }
        } else {
            "system-software-install"
        }
    };

    let about = gtk::AboutDialog::builder()
        .program_name("LinGet")
        .logo_icon_name(logo_icon)
        .version(crate::app::APP_VERSION)
        .authors(vec!["LinGet Contributors".to_string()])
        .license_type(gtk::License::Gpl30)
        .website("https://github.com/linget/linget")
        .website_label("GitHub Repository")
        .comments("A modern package manager for Linux.\n\nManage packages from APT, Flatpak, npm, and pip all in one place.")
        .copyright("© 2025 LinGet Contributors")
        .transient_for(parent)
        .modal(true)
        .build();

    about.present();
}
