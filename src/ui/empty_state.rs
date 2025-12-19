#![allow(dead_code)]

use gtk4::prelude::*;
use gtk4::{self as gtk};
use libadwaita as adw;

pub struct EmptyState {
    pub widget: adw::StatusPage,
}

impl EmptyState {
    pub fn new(icon_name: &str, title: &str, description: &str) -> Self {
        let status_page = adw::StatusPage::builder()
            .icon_name(icon_name)
            .title(title)
            .description(description)
            .vexpand(true)
            .hexpand(true)
            .build();

        Self {
            widget: status_page,
        }
    }

    pub fn with_child(
        icon_name: &str,
        title: &str,
        description: &str,
        child: &impl IsA<gtk::Widget>,
    ) -> Self {
        let status_page = adw::StatusPage::builder()
            .icon_name(icon_name)
            .title(title)
            .description(description)
            .child(child)
            .vexpand(true)
            .hexpand(true)
            .build();

        Self {
            widget: status_page,
        }
    }

    pub fn all_up_to_date() -> Self {
        Self::new(
            "emblem-ok-symbolic",
            "All Up to Date",
            "All your packages are at their latest versions.",
        )
    }

    pub fn no_updates() -> Self {
        Self::new(
            "software-update-available-symbolic",
            "No Updates Available",
            "Check back later for new updates.",
        )
    }

    pub fn search_packages() -> Self {
        Self::new(
            "system-search-symbolic",
            "Search for Packages",
            "Type in the search box above to find packages to install.",
        )
    }

    pub fn no_results(query: &str) -> Self {
        Self::new(
            "edit-find-symbolic",
            "No Results Found",
            &format!(
                "No packages match \"{}\". Try a different search term.",
                query
            ),
        )
    }

    pub fn no_favorites() -> Self {
        let hint_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(8)
            .halign(gtk::Align::Center)
            .build();

        let hint_label = gtk::Label::builder()
            .label("Click the ★ icon on any package to add it to favorites.")
            .wrap(true)
            .justify(gtk::Justification::Center)
            .build();
        hint_label.add_css_class("dim-label");
        hint_box.append(&hint_label);

        Self::with_child(
            "starred-symbolic",
            "No Favorites Yet",
            "Keep track of your most-used packages here.",
            &hint_box,
        )
    }

    pub fn empty_library() -> Self {
        Self::new(
            "package-x-generic-symbolic",
            "No Packages Found",
            "No packages are installed from the enabled sources.",
        )
    }

    pub fn provider_unavailable(provider_name: &str, install_hint: Option<&str>) -> Self {
        let description = match install_hint {
            Some(hint) => format!(
                "{} is not installed on your system.\n\n{}",
                provider_name, hint
            ),
            None => format!("{} is not available on your system.", provider_name),
        };

        Self::new(
            "dialog-warning-symbolic",
            &format!("{} Not Available", provider_name),
            &description,
        )
    }

    pub fn loading() -> Self {
        let spinner = gtk::Spinner::builder()
            .spinning(true)
            .width_request(32)
            .height_request(32)
            .build();

        Self::with_child(
            "content-loading-symbolic",
            "Loading...",
            "Please wait while packages are being loaded.",
            &spinner,
        )
    }

    pub fn error(message: &str) -> Self {
        Self::new("dialog-error-symbolic", "Something Went Wrong", message)
    }

    pub fn error_with_retry<F>(message: &str, on_retry: F) -> Self
    where
        F: Fn() + 'static,
    {
        let retry_btn = gtk::Button::builder()
            .label("Try Again")
            .halign(gtk::Align::Center)
            .build();
        retry_btn.add_css_class("pill");
        retry_btn.add_css_class("suggested-action");
        retry_btn.connect_clicked(move |_| on_retry());

        Self::with_child(
            "dialog-error-symbolic",
            "Something Went Wrong",
            message,
            &retry_btn,
        )
    }

    pub fn first_run() -> Self {
        Self::new(
            "starred-symbolic",
            "Welcome to LinGet",
            "Your unified package manager for Linux. Get started by exploring your installed packages or searching for new ones.",
        )
    }

    pub fn set_icon(&self, icon_name: &str) {
        self.widget.set_icon_name(Some(icon_name));
    }

    pub fn set_title(&self, title: &str) {
        self.widget.set_title(title);
    }

    pub fn set_description(&self, description: &str) {
        self.widget.set_description(Some(description));
    }
}
