use gtk4::prelude::*;
use gtk4::{self as gtk};

pub fn show_about_dialog(parent: &impl IsA<gtk::Window>) {
    let about = gtk::AboutDialog::builder()
        .program_name("LinGet")
        .logo_icon_name("package-x-generic")
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
