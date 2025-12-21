use gtk4::prelude::*;
use libadwaita as adw;
use libadwaita::prelude::*;
use relm4::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum View {
    Discover,
    Library,
    Updates,
    Favorites,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PackageSource {
    Apt,
    Flatpak,
    Snap,
}

impl std::fmt::Display for PackageSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PackageSource::Apt => write!(f, "APT"),
            PackageSource::Flatpak => write!(f, "Flatpak"),
            PackageSource::Snap => write!(f, "Snap"),
        }
    }
}

#[derive(Debug, Clone)]
struct Package {
    name: String,
    version: String,
    description: String,
    source: PackageSource,
    has_update: bool,
    installed: bool,
}

impl Package {
    fn mock_packages() -> Vec<Self> {
        vec![
            Package {
                name: "firefox".into(),
                version: "120.0".into(),
                description: "Mozilla Firefox Web Browser".into(),
                source: PackageSource::Apt,
                has_update: true,
                installed: true,
            },
            Package {
                name: "com.spotify.Client".into(),
                version: "1.2.25".into(),
                description: "Music streaming service".into(),
                source: PackageSource::Flatpak,
                has_update: false,
                installed: true,
            },
            Package {
                name: "discord".into(),
                version: "0.0.35".into(),
                description: "Voice and text chat".into(),
                source: PackageSource::Snap,
                has_update: true,
                installed: true,
            },
        ]
    }
}

struct AppModel {
    search_query: String,
    current_view: View,
    packages: Vec<Package>,
    selected_package: Option<usize>,
    selection_mode: bool,
}

#[derive(Debug)]
enum AppMsg {
    SearchChanged(String),
    ViewChanged(View),
    PackageSelected(usize),
    CloseDetails,
    Refresh,
    ToggleSelectionMode,
    ActionClicked,
}

struct AppWidgets {
    details_box: gtk4::Box,
    details_name: gtk4::Label,
    details_desc: gtk4::Label,
    details_version: gtk4::Label,
    details_source: gtk4::Label,
    details_status: gtk4::Label,
    details_action: gtk4::Button,
}

impl SimpleComponent for AppModel {
    type Init = ();
    type Input = AppMsg;
    type Output = ();
    type Root = adw::ApplicationWindow;
    type Widgets = AppWidgets;

    fn init_root() -> Self::Root {
        adw::ApplicationWindow::builder()
            .title("LinGet")
            .default_width(1000)
            .default_height(700)
            .build()
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = AppModel {
            search_query: String::new(),
            current_view: View::Library,
            packages: Package::mock_packages(),
            selected_package: None,
            selection_mode: false,
        };

        let toolbar = adw::ToolbarView::new();

        let header = adw::HeaderBar::new();
        header.set_title_widget(Some(&gtk4::Label::new(Some("LinGet"))));

        let search = gtk4::SearchEntry::new();
        search.set_placeholder_text(Some("Search packages..."));
        search.set_width_request(300);
        {
            let sender = sender.clone();
            search.connect_search_changed(move |entry| {
                sender.input(AppMsg::SearchChanged(entry.text().to_string()));
            });
        }
        header.pack_start(&search);

        let refresh_btn = gtk4::Button::from_icon_name("view-refresh-symbolic");
        refresh_btn.set_tooltip_text(Some("Refresh"));
        {
            let sender = sender.clone();
            refresh_btn.connect_clicked(move |_| sender.input(AppMsg::Refresh));
        }
        header.pack_end(&refresh_btn);

        let select_btn = gtk4::ToggleButton::new();
        select_btn.set_icon_name("selection-mode-symbolic");
        select_btn.set_tooltip_text(Some("Selection mode"));
        {
            let sender = sender.clone();
            select_btn.connect_toggled(move |_| sender.input(AppMsg::ToggleSelectionMode));
        }
        header.pack_end(&select_btn);

        toolbar.add_top_bar(&header);

        let main_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);

        let sidebar = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
        sidebar.set_width_request(220);
        sidebar.set_margin_all(12);

        let nav_list = gtk4::ListBox::new();
        nav_list.set_selection_mode(gtk4::SelectionMode::Single);
        nav_list.add_css_class("navigation-sidebar");

        for (_view, icon, label) in [
            (View::Discover, "system-search-symbolic", "Discover"),
            (View::Library, "folder-symbolic", "Library"),
            (
                View::Updates,
                "software-update-available-symbolic",
                "Updates",
            ),
            (View::Favorites, "starred-symbolic", "Favorites"),
        ] {
            let row = adw::ActionRow::new();
            row.set_title(label);
            let image = gtk4::Image::from_icon_name(icon);
            row.add_prefix(&image);
            row.set_activatable(true);
            nav_list.append(&row);
        }

        {
            let sender = sender.clone();
            nav_list.connect_row_selected(move |_, row| {
                if let Some(row) = row {
                    let view = match row.index() {
                        0 => View::Discover,
                        1 => View::Library,
                        2 => View::Updates,
                        3 => View::Favorites,
                        _ => View::Library,
                    };
                    sender.input(AppMsg::ViewChanged(view));
                }
            });
        }

        sidebar.append(&nav_list);
        sidebar.append(&gtk4::Separator::new(gtk4::Orientation::Horizontal));

        let sources_label = gtk4::Label::new(Some("Sources"));
        sources_label.set_xalign(0.0);
        sources_label.add_css_class("heading");
        sidebar.append(&sources_label);

        let sources_list = gtk4::ListBox::new();
        sources_list.set_selection_mode(gtk4::SelectionMode::None);
        sources_list.add_css_class("boxed-list");

        for source in [
            PackageSource::Apt,
            PackageSource::Flatpak,
            PackageSource::Snap,
        ] {
            let count = model.packages.iter().filter(|p| p.source == source).count();
            let row = adw::ActionRow::new();
            row.set_title(&format!("{}", source));
            let count_label = gtk4::Label::new(Some(&format!("{}", count)));
            row.add_suffix(&count_label);
            sources_list.append(&row);
        }

        sidebar.append(&sources_list);

        main_box.append(&sidebar);
        main_box.append(&gtk4::Separator::new(gtk4::Orientation::Vertical));

        let content_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        content_box.set_hexpand(true);

        let scrolled = gtk4::ScrolledWindow::new();
        scrolled.set_vexpand(true);
        scrolled.set_hexpand(true);

        let clamp = adw::Clamp::new();
        clamp.set_maximum_size(800);
        clamp.set_margin_all(12);

        let package_list = gtk4::ListBox::new();
        package_list.set_selection_mode(gtk4::SelectionMode::None);
        package_list.add_css_class("boxed-list");

        for (idx, pkg) in model.packages.iter().enumerate() {
            let row = adw::ActionRow::new();
            row.set_title(&pkg.name);
            row.set_subtitle(&pkg.description);
            row.set_activatable(true);

            let source_label = gtk4::Label::new(Some(&format!("{}", pkg.source)));
            source_label.add_css_class("dim-label");
            row.add_suffix(&source_label);

            if pkg.has_update {
                let update_label = gtk4::Label::new(Some("Update"));
                update_label.add_css_class("accent");
                row.add_suffix(&update_label);
            }

            let sender = sender.clone();
            row.connect_activated(move |_| {
                sender.input(AppMsg::PackageSelected(idx));
            });

            package_list.append(&row);
        }

        clamp.set_child(Some(&package_list));
        scrolled.set_child(Some(&clamp));
        content_box.append(&scrolled);

        main_box.append(&content_box);

        let details_box = gtk4::Box::new(gtk4::Orientation::Vertical, 12);
        details_box.set_width_request(300);
        details_box.set_margin_all(16);
        details_box.set_visible(false);

        let details_header = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        let details_name = gtk4::Label::new(None);
        details_name.add_css_class("title-2");
        details_name.set_hexpand(true);
        details_name.set_xalign(0.0);

        let close_btn = gtk4::Button::from_icon_name("window-close-symbolic");
        close_btn.add_css_class("flat");
        {
            let sender = sender.clone();
            close_btn.connect_clicked(move |_| sender.input(AppMsg::CloseDetails));
        }

        details_header.append(&details_name);
        details_header.append(&close_btn);
        details_box.append(&details_header);

        let details_desc = gtk4::Label::new(None);
        details_desc.set_wrap(true);
        details_desc.set_xalign(0.0);
        details_desc.add_css_class("dim-label");
        details_box.append(&details_desc);

        let info_list = gtk4::ListBox::new();
        info_list.add_css_class("boxed-list");

        let version_row = adw::ActionRow::new();
        version_row.set_title("Version");
        let details_version = gtk4::Label::new(None);
        version_row.add_suffix(&details_version);
        info_list.append(&version_row);

        let source_row = adw::ActionRow::new();
        source_row.set_title("Source");
        let details_source = gtk4::Label::new(None);
        source_row.add_suffix(&details_source);
        info_list.append(&source_row);

        let status_row = adw::ActionRow::new();
        status_row.set_title("Status");
        let details_status = gtk4::Label::new(None);
        status_row.add_suffix(&details_status);
        info_list.append(&status_row);

        details_box.append(&info_list);

        let details_action = gtk4::Button::with_label("Update");
        details_action.set_hexpand(true);
        details_action.add_css_class("suggested-action");
        {
            let sender = sender.clone();
            details_action.connect_clicked(move |_| sender.input(AppMsg::ActionClicked));
        }
        details_box.append(&details_action);

        main_box.append(&gtk4::Separator::new(gtk4::Orientation::Vertical));
        main_box.append(&details_box);

        toolbar.set_content(Some(&main_box));
        root.set_content(Some(&toolbar));

        let widgets = AppWidgets {
            details_box,
            details_name,
            details_desc,
            details_version,
            details_source,
            details_status,
            details_action,
        };

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            AppMsg::SearchChanged(query) => {
                self.search_query = query;
            }
            AppMsg::ViewChanged(view) => {
                self.current_view = view;
                self.selected_package = None;
            }
            AppMsg::PackageSelected(idx) => {
                self.selected_package = Some(idx);
            }
            AppMsg::CloseDetails => {
                self.selected_package = None;
            }
            AppMsg::Refresh => {}
            AppMsg::ToggleSelectionMode => {
                self.selection_mode = !self.selection_mode;
            }
            AppMsg::ActionClicked => {
                if let Some(idx) = self.selected_package {
                    if let Some(pkg) = self.packages.get_mut(idx) {
                        if pkg.has_update {
                            pkg.has_update = false;
                        } else {
                            pkg.installed = false;
                        }
                    }
                }
            }
        }
    }

    fn update_view(&self, widgets: &mut Self::Widgets, _sender: ComponentSender<Self>) {
        if let Some(idx) = self.selected_package {
            if let Some(pkg) = self.packages.get(idx) {
                widgets.details_box.set_visible(true);
                widgets.details_name.set_label(&pkg.name);
                widgets.details_desc.set_label(&pkg.description);
                widgets.details_version.set_label(&pkg.version);
                widgets.details_source.set_label(&format!("{}", pkg.source));
                widgets.details_status.set_label(if pkg.installed {
                    "Installed"
                } else {
                    "Not installed"
                });

                if pkg.has_update {
                    widgets.details_action.set_label("Update");
                    widgets
                        .details_action
                        .remove_css_class("destructive-action");
                    widgets.details_action.add_css_class("suggested-action");
                } else {
                    widgets.details_action.set_label("Remove");
                    widgets.details_action.remove_css_class("suggested-action");
                    widgets.details_action.add_css_class("destructive-action");
                }
            }
        } else {
            widgets.details_box.set_visible(false);
        }
    }
}

fn main() {
    let app = adw::Application::builder()
        .application_id("io.github.linget.relm4")
        .build();

    let app = RelmApp::from_app(app);
    app.run::<AppModel>(());
}
