use crate::backend::PackageManager;
use crate::models::{Config, PackageSource};
use gtk4::prelude::*;
use gtk4::{self as gtk, glib};
use libadwaita as adw;
use libadwaita::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct OnboardingWindow {
    pub window: adw::Window,
}

impl OnboardingWindow {
    pub fn new(
        app: &adw::Application,
        config: Rc<RefCell<Config>>,
        pm: Arc<Mutex<PackageManager>>,
        on_complete: impl Fn() + 'static,
    ) -> Self {
        let window = adw::Window::builder()
            .application(app)
            .title("Welcome to LinGet")
            .default_width(600)
            .default_height(500)
            .modal(true)
            .build();

        let carousel = adw::Carousel::builder()
            .allow_scroll_wheel(true)
            .allow_mouse_drag(true)
            .build();

        let indicator = adw::CarouselIndicatorDots::builder()
            .carousel(&carousel)
            .build();

        let welcome_page = Self::build_welcome_page();
        carousel.append(&welcome_page);

        let providers_page = Self::build_providers_page(config.clone(), pm.clone());
        carousel.append(&providers_page);

        let ready_page = Self::build_ready_page();
        carousel.append(&ready_page);

        let nav_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .halign(gtk::Align::Center)
            .spacing(12)
            .margin_bottom(24)
            .build();

        let back_btn = gtk::Button::builder()
            .label("Back")
            .sensitive(false)
            .build();
        back_btn.add_css_class("pill");

        let next_btn = gtk::Button::builder().label("Next").build();
        next_btn.add_css_class("pill");
        next_btn.add_css_class("suggested-action");

        nav_box.append(&back_btn);
        nav_box.append(&next_btn);

        let content = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();

        let header = adw::HeaderBar::builder()
            .show_end_title_buttons(true)
            .show_start_title_buttons(true)
            .build();
        header.set_title_widget(Some(&gtk::Label::new(None)));

        content.append(&header);
        content.append(&carousel);
        content.append(&indicator);
        content.append(&nav_box);

        window.set_content(Some(&content));

        let carousel_back = carousel.clone();
        let next_btn_back = next_btn.clone();
        back_btn.connect_clicked(move |btn| {
            let pos = carousel_back.position();
            if pos > 0.0 {
                carousel_back.scroll_to(&carousel_back.nth_page((pos - 1.0) as u32), true);
            }
            if pos <= 1.0 {
                btn.set_sensitive(false);
            }
            next_btn_back.set_label("Next");
            next_btn_back.remove_css_class("suggested-action");
            next_btn_back.add_css_class("suggested-action");
        });

        let carousel_next = carousel.clone();
        let back_btn_next = back_btn.clone();
        let config_next = config.clone();
        let window_next = window.clone();
        let on_complete = Rc::new(on_complete);
        let on_complete_next = on_complete.clone();
        next_btn.connect_clicked(move |btn| {
            let pos = carousel_next.position();
            let n_pages = carousel_next.n_pages();

            if (pos as u32) >= n_pages - 1 {
                config_next.borrow_mut().onboarding_completed = true;
                let _ = config_next.borrow().save();
                window_next.close();
                on_complete_next();
            } else {
                carousel_next.scroll_to(&carousel_next.nth_page((pos + 1.0) as u32), true);
                back_btn_next.set_sensitive(true);

                if (pos + 1.0) as u32 >= n_pages - 1 {
                    btn.set_label("Get Started");
                }
            }
        });

        let back_btn_carousel = back_btn.clone();
        let next_btn_carousel = next_btn.clone();
        carousel.connect_position_notify(move |carousel| {
            let pos = carousel.position();
            let n_pages = carousel.n_pages();
            back_btn_carousel.set_sensitive(pos > 0.5);
            if pos as u32 >= n_pages - 1 {
                next_btn_carousel.set_label("Get Started");
            } else {
                next_btn_carousel.set_label("Next");
            }
        });

        Self { window }
    }

    fn build_welcome_page() -> gtk::Box {
        let page = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(16)
            .valign(gtk::Align::Center)
            .margin_start(48)
            .margin_end(48)
            .vexpand(true)
            .build();

        let icon = gtk::Image::builder()
            .icon_name("io.github.linget")
            .pixel_size(128)
            .margin_bottom(16)
            .build();

        let title = gtk::Label::builder().label("Welcome to LinGet").build();
        title.add_css_class("title-1");

        let subtitle = gtk::Label::builder()
            .label("Your unified package manager for Linux")
            .build();
        subtitle.add_css_class("dim-label");

        let description = gtk::Label::builder()
            .label(
                "Manage packages from APT, Flatpak, Snap, npm, pip, and more — all in one place.",
            )
            .wrap(true)
            .justify(gtk::Justification::Center)
            .margin_top(24)
            .build();

        page.append(&icon);
        page.append(&title);
        page.append(&subtitle);
        page.append(&description);

        page
    }

    fn build_providers_page(
        config: Rc<RefCell<Config>>,
        pm: Arc<Mutex<PackageManager>>,
    ) -> gtk::Box {
        let page = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(16)
            .margin_start(32)
            .margin_end(32)
            .margin_top(16)
            .vexpand(true)
            .build();

        let title = gtk::Label::builder()
            .label("Available Providers")
            .halign(gtk::Align::Start)
            .build();
        title.add_css_class("title-2");

        let subtitle = gtk::Label::builder()
            .label("LinGet detected these package managers on your system")
            .halign(gtk::Align::Start)
            .wrap(true)
            .build();
        subtitle.add_css_class("dim-label");

        page.append(&title);
        page.append(&subtitle);

        let scrolled = gtk::ScrolledWindow::builder()
            .vexpand(true)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .build();

        let list = gtk::ListBox::builder()
            .selection_mode(gtk::SelectionMode::None)
            .build();
        list.add_css_class("boxed-list");

        let pm_check = pm.clone();
        let config_check = config.clone();
        let list_ref = list.clone();

        glib::spawn_future_local(async move {
            let available_set = {
                let manager = pm_check.lock().await;
                manager.available_sources()
            };

            for source in PackageSource::ALL {
                let is_available = available_set.contains(&source);
                let row = adw::ActionRow::builder()
                    .title(source.to_string())
                    .subtitle(if is_available {
                        "Available"
                    } else {
                        "Not installed"
                    })
                    .build();

                let icon = gtk::Image::builder()
                    .icon_name(source.icon_name())
                    .pixel_size(24)
                    .build();
                row.add_prefix(&icon);

                if is_available {
                    let switch = gtk::Switch::builder()
                        .valign(gtk::Align::Center)
                        .active(true)
                        .build();

                    let config_switch = config_check.clone();
                    let source_switch = source;
                    switch.connect_state_set(move |_, state| {
                        let mut cfg = config_switch.borrow_mut();
                        match source_switch {
                            PackageSource::Apt => cfg.enabled_sources.apt = state,
                            PackageSource::Dnf => cfg.enabled_sources.dnf = state,
                            PackageSource::Pacman => cfg.enabled_sources.pacman = state,
                            PackageSource::Zypper => cfg.enabled_sources.zypper = state,
                            PackageSource::Flatpak => cfg.enabled_sources.flatpak = state,
                            PackageSource::Snap => cfg.enabled_sources.snap = state,
                            PackageSource::Npm => cfg.enabled_sources.npm = state,
                            PackageSource::Pip => cfg.enabled_sources.pip = state,
                            PackageSource::Pipx => cfg.enabled_sources.pipx = state,
                            PackageSource::Cargo => cfg.enabled_sources.cargo = state,
                            PackageSource::Brew => cfg.enabled_sources.brew = state,
                            PackageSource::Aur => cfg.enabled_sources.aur = state,
                            PackageSource::Conda => cfg.enabled_sources.conda = state,
                            PackageSource::Mamba => cfg.enabled_sources.mamba = state,
                            PackageSource::Dart => cfg.enabled_sources.dart = state,
                            PackageSource::Deb => cfg.enabled_sources.deb = state,
                            PackageSource::AppImage => cfg.enabled_sources.appimage = state,
                        }
                        glib::Propagation::Proceed
                    });

                    row.add_suffix(&switch);
                    row.set_activatable_widget(Some(&switch));
                } else {
                    let unavailable_label = gtk::Label::builder().label("Unavailable").build();
                    unavailable_label.add_css_class("dim-label");
                    row.add_suffix(&unavailable_label);
                    row.set_sensitive(false);
                }

                list_ref.append(&row);
            }
        });

        scrolled.set_child(Some(&list));
        page.append(&scrolled);

        page
    }

    fn build_ready_page() -> gtk::Box {
        let page = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(16)
            .valign(gtk::Align::Center)
            .margin_start(48)
            .margin_end(48)
            .vexpand(true)
            .build();

        let icon = gtk::Image::builder()
            .icon_name("emblem-ok-symbolic")
            .pixel_size(64)
            .margin_bottom(16)
            .build();
        icon.add_css_class("success-status");

        let title = gtk::Label::builder().label("You're All Set!").build();
        title.add_css_class("title-1");

        let description = gtk::Label::builder()
            .label("LinGet is ready to help you manage your packages.\n\nUse the sidebar to navigate between views, and the search bar to find packages.")
            .wrap(true)
            .justify(gtk::Justification::Center)
            .margin_top(16)
            .build();

        let tips_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(8)
            .margin_top(24)
            .halign(gtk::Align::Center)
            .build();

        let tips = [
            ("keyboard-symbolic", "Press / to quickly search"),
            ("starred-symbolic", "Star packages to add them to favorites"),
            (
                "emblem-synchronizing-symbolic",
                "Check Updates view for available updates",
            ),
        ];

        for (icon_name, tip_text) in tips {
            let tip_row = gtk::Box::builder()
                .orientation(gtk::Orientation::Horizontal)
                .spacing(8)
                .build();

            let tip_icon = gtk::Image::builder()
                .icon_name(icon_name)
                .pixel_size(16)
                .build();
            tip_icon.add_css_class("dim-label");

            let tip_label = gtk::Label::new(Some(tip_text));
            tip_label.add_css_class("dim-label");

            tip_row.append(&tip_icon);
            tip_row.append(&tip_label);
            tips_box.append(&tip_row);
        }

        page.append(&icon);
        page.append(&title);
        page.append(&description);
        page.append(&tips_box);

        page
    }

    pub fn present(&self) {
        self.window.present();
    }
}
