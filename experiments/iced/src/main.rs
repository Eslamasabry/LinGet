use iced::widget::{
    button, column, container, horizontal_space, row, scrollable, text, text_input, toggler,
    vertical_rule, Row,
};
use iced::{Alignment, Element, Length, Task, Theme};

fn main() -> iced::Result {
    iced::application("LinGet", LinGet::update, LinGet::view)
        .theme(LinGet::theme)
        .window_size(iced::Size::new(1000.0, 700.0))
        .run_with(LinGet::new)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum View {
    Discover,
    Library,
    Updates,
    Favorites,
}

impl std::fmt::Display for View {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            View::Discover => write!(f, "Discover"),
            View::Library => write!(f, "Library"),
            View::Updates => write!(f, "Updates"),
            View::Favorites => write!(f, "Favorites"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PackageSource {
    Apt,
    Flatpak,
    Snap,
    Npm,
    Pip,
    Cargo,
}

impl std::fmt::Display for PackageSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PackageSource::Apt => write!(f, "APT"),
            PackageSource::Flatpak => write!(f, "Flatpak"),
            PackageSource::Snap => write!(f, "Snap"),
            PackageSource::Npm => write!(f, "npm"),
            PackageSource::Pip => write!(f, "pip"),
            PackageSource::Cargo => write!(f, "Cargo"),
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
            Package {
                name: "typescript".into(),
                version: "5.3.2".into(),
                description: "TypeScript language".into(),
                source: PackageSource::Npm,
                has_update: false,
                installed: true,
            },
            Package {
                name: "ripgrep".into(),
                version: "14.0.3".into(),
                description: "Fast grep replacement".into(),
                source: PackageSource::Cargo,
                has_update: false,
                installed: true,
            },
        ]
    }
}

#[derive(Debug, Clone)]
enum Message {
    SearchChanged(String),
    ViewChanged(View),
    PackageSelected(usize),
    CloseDetails,
    RefreshPackages,
    SelectionModeToggled(bool),
    UpdatePackage(usize),
    RemovePackage(usize),
}

struct LinGet {
    search_query: String,
    current_view: View,
    source_filter: Option<PackageSource>,
    packages: Vec<Package>,
    selected_package: Option<usize>,
    selection_mode: bool,
}

impl LinGet {
    fn new() -> (Self, Task<Message>) {
        (
            Self {
                search_query: String::new(),
                current_view: View::Library,
                source_filter: None,
                packages: Package::mock_packages(),
                selected_package: None,
                selection_mode: false,
            },
            Task::none(),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SearchChanged(query) => {
                self.search_query = query;
            }
            Message::ViewChanged(view) => {
                self.current_view = view;
                self.selected_package = None;
            }
            Message::PackageSelected(idx) => {
                self.selected_package = Some(idx);
            }
            Message::CloseDetails => {
                self.selected_package = None;
            }
            Message::RefreshPackages => {}
            Message::SelectionModeToggled(enabled) => {
                self.selection_mode = enabled;
            }
            Message::UpdatePackage(idx) => {
                if let Some(pkg) = self.packages.get_mut(idx) {
                    pkg.has_update = false;
                }
            }
            Message::RemovePackage(idx) => {
                if let Some(pkg) = self.packages.get_mut(idx) {
                    pkg.installed = false;
                }
            }
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let sidebar = self.view_sidebar();
        let content = self.view_content();
        let details = self.view_details();

        let mut main_row = Row::new()
            .push(sidebar)
            .push(vertical_rule(1))
            .push(content);

        if self.selected_package.is_some() {
            main_row = main_row.push(vertical_rule(1)).push(details);
        }

        container(main_row)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn theme(&self) -> Theme {
        Theme::Dark
    }

    fn view_sidebar(&self) -> Element<'_, Message> {
        let nav_items = [
            (View::Discover, "Discover"),
            (View::Library, "Library"),
            (View::Updates, "Updates"),
            (View::Favorites, "Favorites"),
        ];

        let nav_buttons: Vec<Element<Message>> = nav_items
            .into_iter()
            .map(|(view, label)| {
                let is_active = self.current_view == view;
                let btn = button(text(label))
                    .width(Length::Fill)
                    .padding(12)
                    .on_press(Message::ViewChanged(view));

                if is_active {
                    container(btn).style(container::rounded_box).into()
                } else {
                    btn.into()
                }
            })
            .collect();

        let update_count = self.packages.iter().filter(|p| p.has_update).count();
        let total_count = self.packages.len();

        let stats = column![
            text(format!("{} packages", total_count)).size(12),
            text(format!("{} updates", update_count)).size(12),
        ]
        .spacing(4);

        let sources_header = text("Sources").size(14);

        let source_toggles: Vec<Element<Message>> = [
            PackageSource::Apt,
            PackageSource::Flatpak,
            PackageSource::Snap,
            PackageSource::Npm,
            PackageSource::Pip,
            PackageSource::Cargo,
        ]
        .into_iter()
        .map(|source| {
            let count = self.packages.iter().filter(|p| p.source == source).count();
            row![
                text(format!("{}", source)).width(Length::Fill),
                text(format!("{}", count)).size(12),
            ]
            .spacing(8)
            .padding(4)
            .into()
        })
        .collect();

        container(
            column![
                column(nav_buttons).spacing(4),
                container(stats).padding(12),
                sources_header,
                column(source_toggles).spacing(2),
            ]
            .spacing(16)
            .padding(12),
        )
        .width(220)
        .height(Length::Fill)
        .into()
    }

    fn view_content(&self) -> Element<'_, Message> {
        let header = self.view_header();

        let filtered_packages: Vec<(usize, &Package)> = self
            .packages
            .iter()
            .enumerate()
            .filter(|(_, p)| {
                let matches_search = self.search_query.is_empty()
                    || p.name
                        .to_lowercase()
                        .contains(&self.search_query.to_lowercase())
                    || p.description
                        .to_lowercase()
                        .contains(&self.search_query.to_lowercase());
                let matches_source = self.source_filter.is_none_or(|filter| p.source == filter);
                let matches_view = match self.current_view {
                    View::Updates => p.has_update,
                    View::Library => p.installed,
                    _ => true,
                };
                matches_search && matches_source && matches_view
            })
            .collect();

        let package_list: Vec<Element<Message>> = filtered_packages
            .iter()
            .map(|(idx, pkg)| self.view_package_row(*idx, pkg))
            .collect();

        let content: Element<Message> = if package_list.is_empty() {
            container(
                column![
                    text("No packages found").size(24),
                    text("Try adjusting your filters").size(14),
                ]
                .spacing(8)
                .align_x(Alignment::Center),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .center(Length::Fill)
            .into()
        } else {
            scrollable(column(package_list).spacing(2).padding(12))
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        };

        container(column![header, content].spacing(0))
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn view_header(&self) -> Element<'_, Message> {
        let search = text_input("Search packages...", &self.search_query)
            .on_input(Message::SearchChanged)
            .padding(8)
            .width(300);

        let filter_label = self
            .source_filter
            .map_or("All Sources".to_string(), |s| format!("{}", s));

        let refresh_btn = button(text("↻"))
            .padding(8)
            .on_press(Message::RefreshPackages);

        let selection_toggle = toggler(self.selection_mode)
            .label("Select")
            .on_toggle(Message::SelectionModeToggled);

        container(
            row![
                search,
                horizontal_space(),
                text(filter_label),
                refresh_btn,
                selection_toggle,
            ]
            .spacing(12)
            .align_y(Alignment::Center)
            .padding(12),
        )
        .width(Length::Fill)
        .into()
    }

    fn view_package_row(&self, idx: usize, pkg: &Package) -> Element<'_, Message> {
        let source_badge = container(text(format!("{}", pkg.source)).size(11)).padding([2, 6]);

        let update_text = if pkg.has_update {
            text("Update available").size(11)
        } else {
            text("").size(11)
        };

        let action_btn = if pkg.has_update {
            button(text("Update").size(12))
                .padding([4, 8])
                .on_press(Message::UpdatePackage(idx))
        } else {
            button(text("Remove").size(12))
                .padding([4, 8])
                .on_press(Message::RemovePackage(idx))
        };

        let row_content = row![
            column![
                text(pkg.name.clone()).size(14),
                text(pkg.description.clone()).size(12),
            ]
            .spacing(2)
            .width(Length::Fill),
            source_badge,
            update_text,
            action_btn,
        ]
        .spacing(12)
        .align_y(Alignment::Center);

        let row_btn = button(row_content.padding(8))
            .width(Length::Fill)
            .on_press(Message::PackageSelected(idx));

        container(row_btn).width(Length::Fill).into()
    }

    fn view_details(&self) -> Element<'_, Message> {
        let Some(idx) = self.selected_package else {
            return container(text("")).width(0).into();
        };

        let Some(pkg) = self.packages.get(idx) else {
            return container(text("")).width(0).into();
        };

        let close_btn = button(text("×")).padding(8).on_press(Message::CloseDetails);

        let header = row![
            text(pkg.name.clone()).size(20),
            horizontal_space(),
            close_btn
        ]
        .align_y(Alignment::Center);

        let info_rows = column![
            row![text("Version:").width(100), text(pkg.version.clone())],
            row![text("Source:").width(100), text(format!("{}", pkg.source))],
            row![
                text("Status:").width(100),
                text(if pkg.installed {
                    "Installed"
                } else {
                    "Not installed"
                })
            ],
        ]
        .spacing(8);

        let description = text(pkg.description.clone());

        let action_btn = if pkg.has_update {
            button(text("Update"))
                .padding(12)
                .width(Length::Fill)
                .on_press(Message::UpdatePackage(idx))
        } else if pkg.installed {
            button(text("Remove"))
                .padding(12)
                .width(Length::Fill)
                .on_press(Message::RemovePackage(idx))
        } else {
            button(text("Install")).padding(12).width(Length::Fill)
        };

        container(
            column![header, description, info_rows, action_btn]
                .spacing(16)
                .padding(16),
        )
        .width(300)
        .height(Length::Fill)
        .into()
    }
}
