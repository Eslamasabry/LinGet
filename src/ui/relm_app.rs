use crate::backend::{HistoryTracker, PackageManager};
use crate::models::{
    alias::AliasViewData, get_global_recommendations, Config, EnabledSources, LayoutMode, Package,
    PackageSource, PackageStatus, Recommendation,
};
use crate::ui::alias_view::AliasViewAction;
use crate::ui::appearance::apply_appearance;
use crate::ui::header::Header;
use crate::ui::health_dashboard::{
    build_health_dashboard, HealthAction, HealthData, HealthIssueData, Severity,
};
use crate::ui::history_view::{
    build_history_view, filter_entries, HistoryViewAction, HistoryViewData,
};
use crate::ui::notifications;
use crate::ui::package_details::{
    DetailsPanelInit, DetailsPanelInput, DetailsPanelModel, DetailsPanelOutput,
};
use crate::ui::sidebar::{NavItem, SidebarInit, SidebarInput, SidebarModel, SidebarOutput};
use crate::ui::storage_view::{CleanupAction, CleanupStats};
use crate::ui::task_hub::{
    PackageOp, RetrySpec, TaskHubInit, TaskHubInput, TaskHubModel, TaskHubOutput, TaskStatus,
};
use crate::ui::task_queue_view::{build_task_queue_view, TaskQueueAction, TaskQueueViewData};
use crate::ui::widgets::{
    ActionPreview, ActionType, CollectionDialogInit, CollectionDialogInput, CollectionDialogModel,
    CollectionDialogOutput, PackageCardModel, PackageRowInit, PackageRowModel, PackageRowOutput,
    ProgressOverlayInit, ProgressOverlayInput, ProgressOverlayModel, SelectionBarInit,
    SelectionBarInput, SelectionBarModel, SelectionBarOutput,
};
use crate::ui::{EmptyState, SkeletonGrid, SkeletonList};

use gtk4::prelude::*;
use gtk4::{self as gtk, gdk, gio, glib};
use libadwaita as adw;
use libadwaita::prelude::*;
use relm4::factory::FactoryVecDeque;
use relm4::prelude::*;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{watch, Mutex};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum View {
    #[default]
    Home,
    Library,
    Updates,
    Favorites,
    Storage,
    Health,
    History,
    Tasks,
    Aliases,
    Collection(String),
}

impl View {
    pub fn title(&self) -> String {
        match self {
            View::Home => "Home".to_string(),
            View::Library => "Library".to_string(),
            View::Updates => "Updates".to_string(),
            View::Favorites => "Favorites".to_string(),
            View::Storage => "Storage".to_string(),
            View::Health => "Health".to_string(),
            View::History => "History".to_string(),
            View::Tasks => "Scheduled Tasks".to_string(),
            View::Aliases => "Aliases".to_string(),
            View::Collection(name) => name.clone(),
        }
    }
}

impl From<NavItem> for View {
    fn from(item: NavItem) -> Self {
        match item {
            NavItem::Home => View::Home,
            NavItem::Library => View::Library,
            NavItem::Updates => View::Updates,
            NavItem::Favorites => View::Favorites,
            NavItem::Storage => View::Storage,
            NavItem::Health => View::Health,
            NavItem::History => View::History,
            NavItem::Tasks => View::Tasks,
            NavItem::Aliases => View::Aliases,
            NavItem::Collection(name) => View::Collection(name),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[allow(dead_code)]
pub enum SortOrder {
    #[default]
    NameAsc,
    NameDesc,
    SourceAsc,
    SizeDesc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ToastType {
    Info,
    Success,
    Error,
    Warning,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusedAction {
    Install,
    Remove,
    Update,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum AppMsg {
    ViewChanged(View),
    SearchChanged(String),
    DebouncedSearchTrigger(String),
    SourceFilterChanged(Option<PackageSource>),
    UpdateCategoryFilterChanged(Option<crate::models::UpdateCategory>),
    SortOrderChanged(SortOrder),
    LoadPackages,
    PackagesLoaded(Vec<Package>),
    CheckUpdates,
    UpdatesChecked(Vec<Package>),
    LoadingFailed(String),
    ToggleSelectionMode,
    SelectPackage(String, bool),
    SelectAll,
    DeselectAll,
    PackageClicked(Package),
    PackageAction(Package),
    SourceFilterClicked(PackageSource),
    ToggleFavorite(String),
    UpdateAllPackages,
    OperationStarted {
        package_name: String,
        op: String,
    },
    OperationCompleted {
        package_name: String,
        op: String,
    },
    OperationFailed {
        package_name: String,
        error: String,
    },
    ClosePackageDetails,
    ToggleSource(PackageSource),
    EnableDetectedSources,
    RefreshSources,
    ShowToast(String, ToastType),
    UnreadCountChanged(u32),
    RetryOperation(RetrySpec),
    UpdateSelectedPackages,
    RemoveSelectedPackages,
    FocusSearch,
    EscapePressed,
    LoadMore,
    DiscoverSearch(String),
    DiscoverResultsLoaded(Vec<Package>),
    DiscoverSearchFailed(String),
    SetLayoutMode(LayoutMode),
    NewCollection,
    CreateCollection(String),
    TogglePackageCollection {
        pkg_id: String,
        collection: String,
    },
    ShowImage(gtk::gdk::Texture),
    NetworkChanged(bool),
    LoadCleanupStats,
    CleanupStatsLoaded(CleanupStats),
    ConfirmCleanup(CleanupAction),
    ExecuteCleanup(CleanupAction),
    CleanupCompleted {
        source: Option<PackageSource>,
        freed: u64,
    },
    CleanupFailed(String),
    DuplicateAction(crate::ui::storage_view::DuplicateAction),
    PrepareRemovePreview {
        package: Package,
        reverse_deps: Vec<String>,
    },
    LoadHealthData,
    HealthDataLoaded(HealthData),
    HealthAction(HealthAction),
    ExecutePackageAction(Package),
    LoadHistory,
    HistoryLoaded(Vec<crate::models::HistoryEntry>),
    HistoryAction(crate::ui::history_view::HistoryViewAction),
    RecordHistoryEntry(crate::models::HistoryEntry),
    InitializeHistoryTracker,
    HistoryTrackerReady {
        external_changes: Vec<crate::models::HistoryEntry>,
    },
    TakeSnapshot,
    OpenCommandPalette,
    ExecutePaletteCommand(crate::ui::command_palette::PaletteCommand),
    LoadHomeRecommendations,
    HomeRecommendationsLoaded(Vec<Recommendation>),
    InstallHomeRecommendation(String),
    DismissHomeRecommendation(String),
    NavigateList(i32),
    OpenFocusedPackageDetails,
    ActionOnFocusedPackage(FocusedAction),
    ToggleFocusedSelection,
    SelectAllVisible,
    LoadAliases,
    AliasesLoaded(crate::models::alias::AliasManager),
    LoadPackageCommands,
    PackageCommandsLoaded(Vec<crate::models::alias::PackageCommands>),
    PopulateLazyPackages,
    ExpandPackage {
        name: String,
        source: PackageSource,
    },
    PackageCommandsLoadedFor {
        name: String,
        source: PackageSource,
        commands: Vec<crate::models::alias::CommandInfo>,
    },
    CopyToClipboard(String),
    CreateAlias {
        name: String,
        command: String,
        shells: std::collections::HashSet<crate::models::alias::Shell>,
        description: Option<String>,
    },
    DeleteAlias(String),
    AliasOperationCompleted(String),
    AliasOperationFailed(String),
    AliasSearchChanged(String),
    DebouncedAliasSearchTrigger(String),
    ToggleShowExistingAliases,
    FilterAliasesByShell(Option<crate::models::alias::Shell>),
    ScheduleTask(crate::models::ScheduledTask),
    ScheduleBulkTasks(Vec<crate::models::ScheduledTask>),
    CheckScheduledTasks,
    ExecuteScheduledTask(String),
    ScheduledTaskCompleted {
        task_id: String,
        package_name: String,
    },
    ScheduledTaskFailed {
        task_id: String,
        package_name: String,
        error: String,
    },
    CancelScheduledTask(String),
    TaskQueueAction(TaskQueueAction),
    ScheduleAllUpdates,
    ClearCompletedTasks,
    CheckPendingNavigation,
    DowngradePackage {
        package: Package,
        target_version: String,
    },
    Shutdown,
}

pub struct AppModel {
    pub package_manager: Arc<Mutex<PackageManager>>,
    pub config: Rc<RefCell<Config>>,
    pub packages: Vec<Package>,
    pub package_rows: FactoryVecDeque<PackageRowModel>,
    pub package_cards: FactoryVecDeque<PackageCardModel>,
    pub available_sources: HashSet<PackageSource>,
    pub enabled_sources: HashSet<PackageSource>,
    pub current_view: View,
    pub search_query: String,
    pub search_debounce_source: RefCell<Option<glib::SourceId>>,
    pub source_filter: Option<PackageSource>,
    pub update_category_filter: Option<crate::models::UpdateCategory>,
    pub sort_order: SortOrder,
    pub is_loading: bool,
    pub load_error: Option<String>,
    pub selection_mode: bool,
    pub selected_packages: HashSet<String>,
    pub details_visible: bool,
    pub selected_package: Option<Package>,
    pub installed_count: usize,
    pub updates_count: usize,
    pub favorites_count: usize,
    pub show_icons: bool,
    pub pending_task_events: RefCell<Vec<TaskHubInput>>,
    pub pending_toasts: RefCell<Vec<(String, ToastType)>>,
    pub unread_count: u32,
    pub pending_focus_search: RefCell<bool>,
    pub bulk_op_total: usize,
    pub bulk_op_completed: usize,
    pub bulk_op_label: String,
    pub bulk_op_current_item: Option<String>,
    pub operating_package_name: Option<String>,
    pub visible_limit: usize,
    pub total_filtered_count: usize,
    pub layout_mode: LayoutMode,
    pub discover_results: Vec<Package>,
    pub discover_loading: bool,
    pub pending_show_collection_dialog: RefCell<bool>,
    pub pending_sidebar_collections: RefCell<Option<HashMap<String, usize>>>,
    pub pending_show_onboarding: RefCell<bool>,
    pub last_shown_package_id: RefCell<Option<String>>,
    pub provider_counts: HashMap<PackageSource, usize>,
    pub is_offline: bool,
    pub cleanup_stats: CleanupStats,
    pub pending_cleanup_confirm: RefCell<Option<CleanupAction>>,
    pub health_data: HealthData,
    pub pending_action_preview: RefCell<Option<ActionPreview>>,
    pub history_data: HistoryViewData,
    pub history_tracker: Arc<Mutex<Option<crate::backend::HistoryTracker>>>,
    pub alias_data: AliasViewData,
    pub pending_alias_rebuild: RefCell<bool>,
    pub alias_search_debounce_source: RefCell<Option<glib::SourceId>>,
    pub shutdown_signal: watch::Sender<bool>,
    pub pending_command_palette: RefCell<bool>,
    pub home_recommendations: Vec<Recommendation>,
    pub home_recommendations_loading: bool,
    pub pending_home_recommendations_rebuild: RefCell<bool>,
    pub focused_index: usize,
    pub tasks_data: TaskQueueViewData,
    pub pending_tasks_rebuild: RefCell<bool>,
}

const DEFAULT_VISIBLE_LIMIT: usize = 100;
const LOAD_MORE_INCREMENT: usize = 100;

impl AppModel {
    fn filtered_packages(&self) -> (Vec<Package>, usize) {
        let query = &self.search_query;
        let config = self.config.borrow();

        let base_packages = if self.current_view == View::Home {
            &self.discover_results
        } else {
            &self.packages
        };

        let filtered_iter = base_packages.iter().filter(|p| {
            if !self.enabled_sources.contains(&p.source) {
                return false;
            }

            if let Some(filter) = self.source_filter {
                if p.source != filter {
                    return false;
                }
            }

            if self.current_view != View::Home && !query.is_empty() {
                let name_lower = p.name.to_lowercase();
                let desc_lower = p.description.to_lowercase();
                if !name_lower.contains(query) && !desc_lower.contains(query) {
                    return false;
                }
            }

            match self.current_view {
                View::Updates => {
                    if !p.has_update() {
                        return false;
                    }
                    if let Some(cat_filter) = self.update_category_filter {
                        p.update_category == Some(cat_filter)
                    } else {
                        true
                    }
                }
                View::Favorites => config.favorite_packages.contains(&p.id()),
                View::Collection(ref name) => config
                    .collections
                    .get(name)
                    .map(|ids| ids.contains(&p.id()))
                    .unwrap_or(false),
                View::Library
                | View::Home
                | View::Storage
                | View::Health
                | View::History
                | View::Tasks
                | View::Aliases => true,
            }
        });

        let mut packages: Vec<Package> = filtered_iter.cloned().collect();
        let total_count = packages.len();

        match self.sort_order {
            SortOrder::NameAsc => packages.sort_by_cached_key(|p| p.name.to_lowercase()),
            SortOrder::NameDesc => {
                packages.sort_by_cached_key(|p| std::cmp::Reverse(p.name.to_lowercase()))
            }
            SortOrder::SourceAsc => packages.sort_by_key(|p| p.source),
            SortOrder::SizeDesc => packages.sort_by_key(|p| std::cmp::Reverse(p.size)),
        }

        packages.truncate(self.visible_limit);
        (packages, total_count)
    }

    fn update_counts(&mut self) {
        let config = self.config.borrow();
        let enabled_packages: Vec<_> = self
            .packages
            .iter()
            .filter(|p| self.enabled_sources.contains(&p.source))
            .collect();

        self.installed_count = enabled_packages.len();
        self.updates_count = enabled_packages.iter().filter(|p| p.has_update()).count();
        self.favorites_count = enabled_packages
            .iter()
            .filter(|p| config.favorite_packages.contains(&p.id()))
            .count();

        self.provider_counts.clear();
        for pkg in &self.packages {
            *self.provider_counts.entry(pkg.source).or_insert(0) += 1;
        }
    }

    fn refresh_package_list(&mut self) {
        let (filtered, total_count) = self.filtered_packages();
        self.total_filtered_count = total_count;
        let config = self.config.borrow();
        let favorite_ids: HashSet<_> = config.favorite_packages.iter().cloned().collect();

        // Only populate the ACTIVE factory to avoid double work
        let compact = config.ui_compact;
        let scheduler = &config.scheduler;
        match self.layout_mode {
            LayoutMode::List => {
                let mut list_guard = self.package_rows.guard();
                list_guard.clear();
                for pkg in filtered {
                    let is_favorite = favorite_ids.contains(&pkg.id());
                    let is_scheduled = scheduler.has_pending_schedule(&pkg.id());
                    list_guard.push_back(PackageRowInit {
                        package: pkg,
                        is_favorite,
                        selection_mode: self.selection_mode,
                        show_icons: self.show_icons,
                        compact,
                        is_scheduled,
                    });
                }
            }
            LayoutMode::Grid => {
                let mut card_guard = self.package_cards.guard();
                card_guard.clear();
                for pkg in filtered {
                    let is_favorite = favorite_ids.contains(&pkg.id());
                    let is_scheduled = scheduler.has_pending_schedule(&pkg.id());
                    card_guard.push_back(PackageRowInit {
                        package: pkg,
                        is_favorite,
                        selection_mode: self.selection_mode,
                        show_icons: self.show_icons,
                        compact,
                        is_scheduled,
                    });
                }
            }
        }
    }

    fn reset_visible_limit(&mut self) {
        self.visible_limit = DEFAULT_VISIBLE_LIMIT;
    }

    fn get_collection_counts(&self) -> HashMap<String, usize> {
        let config = self.config.borrow();
        let mut counts = HashMap::new();
        for (name, ids) in &config.collections {
            let count = ids
                .iter()
                .filter(|id| self.packages.iter().any(|p| &p.id() == *id))
                .count();
            counts.insert(name.clone(), count);
        }
        counts
    }

    fn get_visible_package(&self, index: usize) -> Option<Package> {
        match self.layout_mode {
            LayoutMode::List => self.package_rows.get(index).map(|r| r.package.clone()),
            LayoutMode::Grid => self.package_cards.get(index).map(|c| c.package.clone()),
        }
    }
}

#[allow(dead_code)]
pub struct AppWidgets {
    header: Header,
    sidebar: Controller<SidebarModel>,
    details_panel: Controller<DetailsPanelModel>,
    task_hub: Controller<TaskHubModel>,
    selection_bar: Controller<SelectionBarModel>,
    progress_overlay: Controller<ProgressOverlayModel>,
    collection_dialog: Controller<CollectionDialogModel>,
    task_hub_popover: gtk::Popover,
    task_hub_btn: gtk::Button,
    task_hub_spinner: gtk::Spinner,
    task_hub_badge: gtk::Label,
    content_stack: gtk::Stack,
    list_grid_stack: gtk::Stack,
    hero_banner: gtk::Box,
    view_title: adw::WindowTitle,
    update_all_btn: gtk::Button,
    category_filter_btn: gtk::MenuButton,
    toast_overlay: adw::ToastOverlay,
    split_view: adw::OverlaySplitView,
    load_more_btn: gtk::Button,
    load_more_label: gtk::Label,
    offline_banner: gtk::Box,
    storage_clamp: adw::Clamp,
    health_clamp: adw::Clamp,
    history_clamp: adw::Clamp,
    tasks_clamp: adw::Clamp,
    alias_clamp: adw::Clamp,
    pub alias_view: crate::ui::alias_view::AliasWidgets,
    home_recommendations_group: adw::PreferencesGroup,
    home_recommendations_box: gtk::Box,
    package_list_scrolled: gtk::ScrolledWindow,
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
            .icon_name("io.github.linget")
            .default_width(1100)
            .default_height(700)
            .resizable(true)
            .build()
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let config = Rc::new(RefCell::new(Config::load()));

        let start_minimized;
        {
            let cfg = config.borrow();
            root.set_resizable(true);
            root.set_decorated(true);
            if cfg.window_width > 0 && cfg.window_height > 0 {
                root.set_default_size(cfg.window_width, cfg.window_height);
            }
            if cfg.window_maximized {
                root.maximize();
            }
            start_minimized = cfg.start_minimized;
        }

        if start_minimized {
            root.set_visible(false);
        }

        {
            let cfg = config.borrow();
            crate::ui::apply_theme_settings(&root, cfg.color_scheme, cfg.accent_color);
            apply_appearance(&cfg.appearance);
        }

        let manager = PackageManager::new();
        let available_sources = manager.available_sources();

        let enabled_from_config = config.borrow().enabled_sources.to_sources();
        let enabled_sources: HashSet<PackageSource> = enabled_from_config
            .into_iter()
            .filter(|s| available_sources.contains(s))
            .collect();

        let package_rows: FactoryVecDeque<PackageRowModel> = FactoryVecDeque::builder()
            .launch(
                gtk::ListBox::builder()
                    .selection_mode(gtk::SelectionMode::None)
                    .css_classes(vec!["boxed-list", "animate-stagger"])
                    .build(),
            )
            .forward(sender.input_sender(), |output| match output {
                PackageRowOutput::Clicked(pkg) => AppMsg::PackageClicked(pkg),
                PackageRowOutput::ActionClicked(pkg) => AppMsg::PackageAction(pkg),
                PackageRowOutput::SourceFilterClicked(pkg) => {
                    AppMsg::SourceFilterClicked(pkg.source)
                }
                PackageRowOutput::FavoriteToggled(pkg) => AppMsg::ToggleFavorite(pkg.id()),
                PackageRowOutput::SelectionChanged(pkg, selected) => {
                    AppMsg::SelectPackage(pkg.id(), selected)
                }
            });

        let package_cards: FactoryVecDeque<PackageCardModel> = FactoryVecDeque::builder()
            .launch(
                gtk::FlowBox::builder()
                    .selection_mode(gtk::SelectionMode::None)
                    .row_spacing(20)
                    .column_spacing(20)
                    .halign(gtk::Align::Fill)
                    .hexpand(true)
                    .valign(gtk::Align::Start)
                    .homogeneous(true)
                    .min_children_per_line(2)
                    .max_children_per_line(4)
                    .margin_top(16)
                    .margin_bottom(16)
                    .margin_start(16)
                    .margin_end(16)
                    .css_classes(vec!["package-grid", "animate-stagger"])
                    .build(),
            )
            .forward(sender.input_sender(), |output| match output {
                PackageRowOutput::Clicked(pkg) => AppMsg::PackageClicked(pkg),
                PackageRowOutput::ActionClicked(pkg) => AppMsg::PackageAction(pkg),
                PackageRowOutput::SourceFilterClicked(pkg) => {
                    AppMsg::SourceFilterClicked(pkg.source)
                }
                PackageRowOutput::FavoriteToggled(pkg) => AppMsg::ToggleFavorite(pkg.id()),
                PackageRowOutput::SelectionChanged(pkg, selected) => {
                    AppMsg::SelectPackage(pkg.id(), selected)
                }
            });

        let layout_mode = config.borrow().layout_mode;
        let (shutdown_signal, _shutdown_rx) = watch::channel(false);

        let model = AppModel {
            package_manager: Arc::new(Mutex::new(manager)),
            config: config.clone(),
            packages: Vec::new(),
            package_rows,
            package_cards,
            available_sources: available_sources.clone(),
            enabled_sources: enabled_sources.clone(),
            current_view: View::Library,
            search_query: String::new(),
            search_debounce_source: RefCell::new(None),
            source_filter: None,
            update_category_filter: None,
            sort_order: SortOrder::default(),
            is_loading: false,
            load_error: None,
            selection_mode: false,
            selected_packages: HashSet::new(),
            details_visible: false,
            selected_package: None,
            installed_count: 0,
            updates_count: 0,
            favorites_count: 0,
            show_icons: true,
            pending_task_events: RefCell::new(Vec::new()),
            pending_toasts: RefCell::new(Vec::new()),
            unread_count: 0,
            pending_focus_search: RefCell::new(false),
            bulk_op_total: 0,
            bulk_op_completed: 0,
            bulk_op_label: String::new(),
            bulk_op_current_item: None,
            operating_package_name: None,
            visible_limit: DEFAULT_VISIBLE_LIMIT,
            total_filtered_count: 0,
            layout_mode,
            discover_results: Vec::new(),
            discover_loading: false,
            pending_show_collection_dialog: RefCell::new(false),
            pending_sidebar_collections: RefCell::new(None),
            pending_show_onboarding: RefCell::new(!config.borrow().onboarding_completed),
            last_shown_package_id: RefCell::new(None),
            provider_counts: HashMap::new(),
            is_offline: !gio::NetworkMonitor::default().is_network_available(),
            cleanup_stats: CleanupStats::default(),
            pending_cleanup_confirm: RefCell::new(None),
            health_data: HealthData::default(),
            pending_action_preview: RefCell::new(None),
            history_data: HistoryViewData::default(),
            history_tracker: Arc::new(Mutex::new(None)),
            alias_data: AliasViewData::default(),
            pending_alias_rebuild: RefCell::new(true),
            alias_search_debounce_source: RefCell::new(None),
            shutdown_signal,
            pending_command_palette: RefCell::new(false),
            home_recommendations: Vec::new(),
            home_recommendations_loading: false,
            pending_home_recommendations_rebuild: RefCell::new(false),
            focused_index: 0,
            tasks_data: TaskQueueViewData::default(),
            pending_tasks_rebuild: RefCell::new(true),
        };

        let header = Header::new();

        header.maximize_button.connect_clicked({
            let root = root.clone();
            move |_| {
                if root.is_maximized() {
                    root.unmaximize();
                } else {
                    root.maximize();
                }
            }
        });

        root.connect_maximized_notify({
            let btn = header.maximize_button.clone();
            move |win| {
                let is_maximized = win.is_maximized();
                btn.set_icon_name(if is_maximized {
                    "view-restore-symbolic"
                } else {
                    "window-maximize-symbolic"
                });
                btn.set_tooltip_text(Some(if is_maximized { "Restore" } else { "Maximize" }));
            }
        });

        let sidebar_init = SidebarInit {
            available_sources,
            enabled_sources,
            library_count: 0,
            updates_count: 0,
            favorites_count: 0,
            collections: std::collections::HashMap::new(),
        };

        let sidebar =
            SidebarModel::builder()
                .launch(sidebar_init)
                .forward(sender.input_sender(), |output| match output {
                    SidebarOutput::ViewChanged(item) => AppMsg::ViewChanged(View::from(item)),
                    SidebarOutput::SourceToggled(source) => AppMsg::ToggleSource(source),
                    SidebarOutput::EnableDetectedSources => AppMsg::EnableDetectedSources,
                    SidebarOutput::NewCollection => AppMsg::NewCollection,
                    SidebarOutput::FilterBySource(source) => AppMsg::SourceFilterChanged(source),
                });

        let view_title = adw::WindowTitle::builder().title("Library").build();

        let content_header = adw::HeaderBar::builder()
            .show_start_title_buttons(false)
            .show_end_title_buttons(false)
            .build();
        content_header.add_css_class("view-toolbar");
        content_header.set_title_widget(Some(&view_title));

        let sort_popover_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(2)
            .margin_top(8)
            .margin_bottom(8)
            .margin_start(8)
            .margin_end(8)
            .build();

        let sort_options = [
            ("Name (A-Z)", SortOrder::NameAsc),
            ("Name (Z-A)", SortOrder::NameDesc),
            ("Source", SortOrder::SourceAsc),
            ("Size (Largest)", SortOrder::SizeDesc),
        ];

        for (label, order) in sort_options {
            let btn = gtk::Button::builder().label(label).build();
            btn.add_css_class("flat");
            btn.add_css_class("sort-option-btn");

            let sender_clone = sender.clone();
            btn.connect_clicked(move |_| {
                sender_clone.input(AppMsg::SortOrderChanged(order));
            });

            sort_popover_box.append(&btn);
        }

        let sort_popover = gtk::Popover::builder()
            .child(&sort_popover_box)
            .has_arrow(true)
            .position(gtk::PositionType::Bottom)
            .build();
        sort_popover.add_css_class("sort-popover");

        let sort_btn = gtk::MenuButton::builder()
            .icon_name("view-sort-descending-symbolic")
            .tooltip_text("Sort packages")
            .popover(&sort_popover)
            .build();
        sort_btn.add_css_class("flat");
        content_header.pack_start(&sort_btn);

        let category_filter_popover_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(2)
            .margin_top(8)
            .margin_bottom(8)
            .margin_start(8)
            .margin_end(8)
            .build();

        let category_options: [(&str, Option<crate::models::UpdateCategory>); 5] = [
            ("All Updates", None),
            ("Security", Some(crate::models::UpdateCategory::Security)),
            ("Bugfix", Some(crate::models::UpdateCategory::Bugfix)),
            ("Feature", Some(crate::models::UpdateCategory::Feature)),
            ("Minor", Some(crate::models::UpdateCategory::Minor)),
        ];

        for (label, category) in category_options {
            let btn = gtk::Button::builder().label(label).build();
            btn.add_css_class("flat");
            btn.add_css_class("sort-option-btn");

            let sender_clone = sender.clone();
            btn.connect_clicked(move |_| {
                sender_clone.input(AppMsg::UpdateCategoryFilterChanged(category));
            });

            category_filter_popover_box.append(&btn);
        }

        let category_filter_popover = gtk::Popover::builder()
            .child(&category_filter_popover_box)
            .has_arrow(true)
            .position(gtk::PositionType::Bottom)
            .build();
        category_filter_popover.add_css_class("sort-popover");

        let category_filter_btn = gtk::MenuButton::builder()
            .icon_name("funnel-symbolic")
            .tooltip_text("Filter by update category")
            .popover(&category_filter_popover)
            .visible(false)
            .build();
        category_filter_btn.add_css_class("flat");
        content_header.pack_start(&category_filter_btn);

        let update_all_btn = gtk::Button::builder()
            .label("Update All")
            .visible(false)
            .build();
        update_all_btn.add_css_class("suggested-action");
        update_all_btn.add_css_class("pill");
        content_header.pack_end(&update_all_btn);

        let spinner = gtk::Spinner::builder().visible(false).build();
        content_header.pack_end(&spinner);

        let transition_ms = config.borrow().appearance.transition_speed.to_ms() as u32;
        let list_grid_stack = gtk::Stack::builder()
            .transition_type(gtk::StackTransitionType::Crossfade)
            .transition_duration(transition_ms)
            .build();
        list_grid_stack.add_named(model.package_rows.widget(), Some("list"));
        list_grid_stack.add_named(model.package_cards.widget(), Some("grid"));

        let hero_banner = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(12)
            .css_classes(vec!["hero-banner"])
            .visible(false)
            .build();

        hero_banner.append(
            &gtk::Label::builder()
                .label("Discover")
                .halign(gtk::Align::Start)
                .css_classes(vec!["hero-badge"])
                .build(),
        );

        hero_banner.append(
            &gtk::Label::builder()
                .label("Discover Modern Linux Apps")
                .halign(gtk::Align::Start)
                .css_classes(vec!["hero-title"])
                .build(),
        );

        hero_banner.append(
            &gtk::Label::builder()
                .label("Browse thousands of Flatpaks, Snaps, and Native packages in one place.")
                .halign(gtk::Align::Start)
                .css_classes(vec!["hero-subtitle"])
                .build(),
        );

        let home_recommendations_group = adw::PreferencesGroup::builder()
            .title("Recommended for You")
            .visible(false)
            .margin_top(16)
            .build();

        let home_recommendations_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(8)
            .build();

        home_recommendations_group.add(&home_recommendations_box);

        let list_box = &list_grid_stack;

        let load_more_label = gtk::Label::builder()
            .label("Showing 200 of 1000 packages")
            .build();
        load_more_label.add_css_class("dim-label");

        let load_more_btn = gtk::Button::builder()
            .label("Load More")
            .halign(gtk::Align::Center)
            .margin_top(16)
            .margin_bottom(16)
            .visible(false)
            .build();
        load_more_btn.add_css_class("pill");

        let load_more_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(8)
            .halign(gtk::Align::Center)
            .margin_top(12)
            .margin_bottom(12)
            .build();
        load_more_box.append(&load_more_label);
        load_more_box.append(&load_more_btn);

        let list_container = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();
        list_container.append(&hero_banner);
        list_container.append(&home_recommendations_group);
        list_container.append(list_box);
        list_container.append(&load_more_box);

        let scrolled = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .propagate_natural_height(false)
            .propagate_natural_width(false)
            .vexpand(true)
            .child(&list_container)
            .build();

        let list_clamp = adw::Clamp::builder()
            .maximum_size(1600)
            .tightening_threshold(1200)
            .child(&scrolled)
            .margin_top(8)
            .margin_bottom(24)
            .margin_start(24)
            .margin_end(24)
            .build();

        let empty_library = EmptyState::empty_library().widget;
        let empty_updates = EmptyState::all_up_to_date().widget;
        empty_updates.add_css_class("success-status");
        let empty_favorites = EmptyState::no_favorites().widget;
        let empty_discover = EmptyState::search_packages().widget;
        let empty_no_results = EmptyState::no_results("").widget;
        let empty_error = EmptyState::error_with_retry("Failed to load packages", {
            let sender = sender.clone();
            move || sender.input(AppMsg::LoadPackages)
        })
        .widget;

        let loading_banner = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(12)
            .halign(gtk::Align::Center)
            .margin_top(24)
            .margin_bottom(16)
            .build();

        let loading_spinner = gtk::Spinner::builder()
            .spinning(true)
            .width_request(24)
            .height_request(24)
            .build();

        let loading_label = gtk::Label::builder()
            .label("Loading packages from enabled sources...")
            .build();
        loading_label.add_css_class("dim-label");

        loading_banner.append(&loading_spinner);
        loading_banner.append(&loading_label);

        let skeleton = SkeletonList::new(8).widget;
        let skeleton_container = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();
        skeleton_container.append(&loading_banner.clone());
        skeleton_container.append(&skeleton);

        let skeleton_clamp = adw::Clamp::builder()
            .maximum_size(1600)
            .tightening_threshold(1200)
            .child(&skeleton_container)
            .margin_top(8)
            .margin_start(24)
            .margin_end(24)
            .build();

        let skeleton_grid = SkeletonGrid::new(12).widget;
        let skeleton_grid_container = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();

        let loading_banner_grid = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(12)
            .halign(gtk::Align::Center)
            .margin_top(24)
            .margin_bottom(16)
            .build();

        let loading_spinner_grid = gtk::Spinner::builder()
            .spinning(true)
            .width_request(24)
            .height_request(24)
            .build();

        let loading_label_grid = gtk::Label::builder()
            .label("Loading packages from enabled sources...")
            .build();
        loading_label_grid.add_css_class("dim-label");

        loading_banner_grid.append(&loading_spinner_grid);
        loading_banner_grid.append(&loading_label_grid);

        skeleton_grid_container.append(&loading_banner_grid);
        skeleton_grid_container.append(&skeleton_grid);

        let skeleton_grid_clamp = adw::Clamp::builder()
            .maximum_size(1600)
            .tightening_threshold(1200)
            .child(&skeleton_grid_container)
            .margin_top(8)
            .margin_start(24)
            .margin_end(24)
            .build();

        let content_stack = gtk::Stack::builder()
            .transition_type(gtk::StackTransitionType::Crossfade)
            .transition_duration(transition_ms)
            .build();
        content_stack.add_named(&list_clamp, Some("list"));
        content_stack.add_named(&empty_library, Some("empty-library"));
        content_stack.add_named(&empty_updates, Some("empty-updates"));
        content_stack.add_named(&empty_favorites, Some("empty-favorites"));
        content_stack.add_named(&empty_discover, Some("empty-discover"));
        content_stack.add_named(&skeleton_clamp, Some("skeleton"));
        content_stack.add_named(&skeleton_grid_clamp, Some("skeleton-grid"));
        content_stack.add_named(&empty_no_results, Some("empty-no-results"));
        content_stack.add_named(&empty_error, Some("error"));

        let storage_scroll = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vexpand(true)
            .build();
        let storage_clamp = adw::Clamp::builder()
            .maximum_size(800)
            .tightening_threshold(600)
            .margin_top(8)
            .margin_start(24)
            .margin_end(24)
            .build();
        storage_scroll.set_child(Some(&storage_clamp));
        content_stack.add_named(&storage_scroll, Some("storage"));

        let health_scroll = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vexpand(true)
            .build();
        let health_clamp = adw::Clamp::builder()
            .maximum_size(600)
            .tightening_threshold(400)
            .margin_top(8)
            .margin_start(24)
            .margin_end(24)
            .build();
        health_scroll.set_child(Some(&health_clamp));
        content_stack.add_named(&health_scroll, Some("health"));

        let history_scroll = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vexpand(true)
            .build();
        let history_clamp = adw::Clamp::builder()
            .maximum_size(800)
            .tightening_threshold(600)
            .margin_top(8)
            .margin_start(24)
            .margin_end(24)
            .build();
        history_scroll.set_child(Some(&history_clamp));
        content_stack.add_named(&history_scroll, Some("history"));

        let tasks_scroll = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vexpand(true)
            .build();
        let tasks_clamp = adw::Clamp::builder()
            .maximum_size(800)
            .tightening_threshold(600)
            .margin_top(8)
            .margin_start(24)
            .margin_end(24)
            .build();
        tasks_scroll.set_child(Some(&tasks_clamp));
        content_stack.add_named(&tasks_scroll, Some("tasks"));

        let alias_scroll = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vexpand(true)
            .build();
        let alias_clamp = adw::Clamp::builder()
            .maximum_size(800)
            .tightening_threshold(600)
            .margin_top(8)
            .margin_start(24)
            .margin_end(24)
            .build();
        alias_scroll.set_child(Some(&alias_clamp));
        content_stack.add_named(&alias_scroll, Some("aliases"));

        let sender_alias = sender.clone();
        let alias_view = crate::ui::alias_view::init_alias_view(move |action| match action {
            AliasViewAction::Refresh => sender_alias.input(AppMsg::LoadAliases),
            AliasViewAction::Create {
                name,
                command,
                shells,
                description,
            } => {
                sender_alias.input(AppMsg::CreateAlias {
                    name,
                    command,
                    shells,
                    description,
                });
            }
            AliasViewAction::Delete(name) => sender_alias.input(AppMsg::DeleteAlias(name)),
            AliasViewAction::SearchChanged(query) => {
                sender_alias.input(AppMsg::AliasSearchChanged(query))
            }
            AliasViewAction::ToggleShowExisting => {
                sender_alias.input(AppMsg::ToggleShowExistingAliases)
            }
            AliasViewAction::FilterByShell(shell) => {
                sender_alias.input(AppMsg::FilterAliasesByShell(shell))
            }
            AliasViewAction::ExpandPackage { name, source } => {
                sender_alias.input(AppMsg::ExpandPackage { name, source })
            }
            AliasViewAction::CopyCommand(path) => sender_alias.input(AppMsg::CopyToClipboard(path)),
        });
        alias_clamp.set_child(Some(&alias_view.container));

        content_stack.set_visible_child_name("skeleton");

        let content_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .hexpand(true)
            .build();
        content_box.append(&content_header);
        content_box.append(&content_stack);

        let details_panel = DetailsPanelModel::builder()
            .launch(DetailsPanelInit {
                pm: model.package_manager.clone(),
                config: model.config.clone(),
            })
            .forward(sender.input_sender(), |output| match output {
                DetailsPanelOutput::Close => AppMsg::ClosePackageDetails,
                DetailsPanelOutput::Reload => AppMsg::LoadPackages,
                DetailsPanelOutput::ShowToast(msg) => AppMsg::ShowToast(msg, ToastType::Info),
                DetailsPanelOutput::ShowImage(texture) => AppMsg::ShowImage(texture),
                DetailsPanelOutput::ToggleCollection { pkg_id, collection } => {
                    AppMsg::TogglePackageCollection { pkg_id, collection }
                }
                DetailsPanelOutput::ScheduleTask(task) => AppMsg::ScheduleTask(task),
            });

        let split_view = adw::OverlaySplitView::builder()
            .content(&content_box)
            .sidebar(details_panel.widget())
            .collapsed(true)
            .show_sidebar(false)
            .sidebar_position(gtk::PackType::End)
            .min_sidebar_width(400.0)
            .max_sidebar_width(500.0)
            .build();

        let task_hub =
            TaskHubModel::builder()
                .launch(TaskHubInit)
                .forward(sender.input_sender(), |output| match output {
                    TaskHubOutput::UnreadCountChanged(count) => AppMsg::UnreadCountChanged(count),
                    TaskHubOutput::RetryOperation(spec) => AppMsg::RetryOperation(spec),
                });

        let task_hub_popover = gtk::Popover::builder()
            .child(task_hub.widget())
            .position(gtk::PositionType::Top)
            .autohide(true)
            .build();
        task_hub_popover.add_css_class("task-hub-popover-container");

        let task_hub_spinner = gtk::Spinner::builder()
            .visible(false)
            .spinning(true)
            .build();
        task_hub_spinner.set_can_target(false);

        let task_hub_badge = gtk::Label::builder()
            .label("0")
            .visible(false)
            .css_classes(vec!["badge-accent", "task-hub-badge"])
            .build();
        task_hub_badge.set_can_target(false);

        let task_hub_btn = gtk::Button::builder()
            .icon_name("format-justify-fill-symbolic")
            .css_classes(vec!["circular", "task-hub-fab"])
            .halign(gtk::Align::End)
            .valign(gtk::Align::End)
            .margin_bottom(24)
            .margin_end(24)
            .build();

        let task_hub_overlay = gtk::Overlay::new();
        task_hub_overlay.set_child(Some(&task_hub_btn));
        task_hub_overlay.add_overlay(&task_hub_spinner);
        task_hub_overlay.add_overlay(&task_hub_badge);
        task_hub_overlay.set_halign(gtk::Align::End);
        task_hub_overlay.set_valign(gtk::Align::End);

        task_hub_popover.set_parent(&task_hub_btn);

        let task_hub_sender = task_hub.sender().clone();
        task_hub_popover.connect_closed(move |_| {
            task_hub_sender.send(TaskHubInput::MarkRead).ok();
        });

        task_hub_btn.connect_clicked({
            let popover = task_hub_popover.clone();
            move |_| {
                popover.popup();
            }
        });

        let selection_bar = SelectionBarModel::builder()
            .launch(SelectionBarInit)
            .forward(sender.input_sender(), |output| match output {
                SelectionBarOutput::SelectAll => AppMsg::SelectAll,
                SelectionBarOutput::DeselectAll => AppMsg::DeselectAll,
                SelectionBarOutput::UpdateSelected => AppMsg::UpdateSelectedPackages,
                SelectionBarOutput::RemoveSelected => AppMsg::RemoveSelectedPackages,
                SelectionBarOutput::ScheduleSelectedUpdates(tasks) => {
                    AppMsg::ScheduleBulkTasks(tasks)
                }
            });

        let progress_overlay = ProgressOverlayModel::builder()
            .launch(ProgressOverlayInit)
            .detach();

        let collection_dialog = CollectionDialogModel::builder()
            .launch(CollectionDialogInit {
                parent: root.clone().upcast(),
            })
            .forward(sender.input_sender(), |output| match output {
                CollectionDialogOutput::Created(name) => AppMsg::CreateCollection(name),
            });

        let offline_banner = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(8)
            .halign(gtk::Align::Center)
            .visible(false)
            .build();
        offline_banner.add_css_class("offline-banner");

        let offline_icon = gtk::Image::builder()
            .icon_name("network-offline-symbolic")
            .pixel_size(16)
            .build();

        let offline_label = gtk::Label::builder()
            .label("You're offline — some features may be unavailable")
            .build();

        let retry_btn = gtk::Button::builder().label("Retry").build();
        retry_btn.add_css_class("flat");
        retry_btn.add_css_class("pill");
        retry_btn.connect_clicked({
            let sender = sender.clone();
            move |_| {
                sender.input(AppMsg::LoadPackages);
            }
        });

        offline_banner.append(&offline_icon);
        offline_banner.append(&offline_label);
        offline_banner.append(&retry_btn);

        let main_paned = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .vexpand(true)
            .build();

        main_paned.append(sidebar.widget());
        main_paned.append(&gtk::Separator::new(gtk::Orientation::Vertical));
        main_paned.append(&split_view);

        let main_container = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();
        main_container.append(&offline_banner);
        main_container.append(&main_paned);
        main_container.append(selection_bar.widget());

        let content_overlay = gtk::Overlay::new();
        content_overlay.set_child(Some(&main_container));
        content_overlay.add_overlay(progress_overlay.widget());
        content_overlay.add_overlay(&task_hub_overlay);

        let toast_overlay = adw::ToastOverlay::new();
        toast_overlay.set_child(Some(&content_overlay));

        let toolbar_view = adw::ToolbarView::new();
        toolbar_view.add_top_bar(&header.widget);
        toolbar_view.set_content(Some(&toast_overlay));

        root.set_content(Some(&toolbar_view));

        let key_controller = gtk::EventControllerKey::new();
        let waiting_for_g = Rc::new(RefCell::new(false));
        let root_for_keys = root.clone();
        key_controller.connect_key_pressed({
            let sender = sender.clone();
            let config = config.clone();
            let waiting_for_g = waiting_for_g.clone();
            move |_, keyval, _keycode, state| {
                let ctrl = state.contains(gtk::gdk::ModifierType::CONTROL_MASK);
                let vim_mode = config.borrow().vim_mode;

                let user_is_typing_in_entry = gtk::prelude::GtkWindowExt::focus(&root_for_keys)
                    .map(|w: gtk::Widget| {
                        w.is::<gtk::Text>()
                            || w.is::<gtk::Entry>()
                            || w.is::<gtk::SearchEntry>()
                            || w.is::<adw::EntryRow>()
                    })
                    .unwrap_or(false);

                // Handle Ctrl+key shortcuts (always active)
                if ctrl {
                    match keyval {
                        gtk::gdk::Key::s | gtk::gdk::Key::S => {
                            sender.input(AppMsg::ToggleSelectionMode);
                            return glib::Propagation::Stop;
                        }
                        gtk::gdk::Key::f | gtk::gdk::Key::F => {
                            sender.input(AppMsg::FocusSearch);
                            return glib::Propagation::Stop;
                        }
                        gtk::gdk::Key::r | gtk::gdk::Key::R => {
                            sender.input(AppMsg::LoadPackages);
                            return glib::Propagation::Stop;
                        }
                        gtk::gdk::Key::k | gtk::gdk::Key::K => {
                            sender.input(AppMsg::OpenCommandPalette);
                            return glib::Propagation::Stop;
                        }
                        gtk::gdk::Key::a | gtk::gdk::Key::A => {
                            sender.input(AppMsg::SelectAllVisible);
                            return glib::Propagation::Stop;
                        }
                        _ => {}
                    }
                }

                // Escape always works
                if keyval == gtk::gdk::Key::Escape {
                    *waiting_for_g.borrow_mut() = false;
                    sender.input(AppMsg::EscapePressed);
                    return glib::Propagation::Stop;
                }

                if !user_is_typing_in_entry {
                    if keyval == gtk::gdk::Key::slash {
                        sender.input(AppMsg::FocusSearch);
                        return glib::Propagation::Stop;
                    }

                    if keyval == gtk::gdk::Key::Return || keyval == gtk::gdk::Key::KP_Enter {
                        sender.input(AppMsg::OpenFocusedPackageDetails);
                        return glib::Propagation::Stop;
                    }

                    if keyval == gtk::gdk::Key::space {
                        sender.input(AppMsg::ToggleFocusedSelection);
                        return glib::Propagation::Stop;
                    }
                }

                if vim_mode && !user_is_typing_in_entry {
                    // Handle g+key sequences
                    if *waiting_for_g.borrow() {
                        *waiting_for_g.borrow_mut() = false;
                        match keyval {
                            gtk::gdk::Key::h => {
                                sender.input(AppMsg::ViewChanged(View::Home));
                                return glib::Propagation::Stop;
                            }
                            gtk::gdk::Key::l => {
                                sender.input(AppMsg::ViewChanged(View::Library));
                                return glib::Propagation::Stop;
                            }
                            gtk::gdk::Key::u => {
                                sender.input(AppMsg::ViewChanged(View::Updates));
                                return glib::Propagation::Stop;
                            }
                            gtk::gdk::Key::s => {
                                sender.input(AppMsg::ViewChanged(View::Storage));
                                return glib::Propagation::Stop;
                            }
                            gtk::gdk::Key::f => {
                                sender.input(AppMsg::ViewChanged(View::Favorites));
                                return glib::Propagation::Stop;
                            }
                            _ => {}
                        }
                    }

                    match keyval {
                        gtk::gdk::Key::j => {
                            sender.input(AppMsg::NavigateList(1));
                            return glib::Propagation::Stop;
                        }
                        gtk::gdk::Key::k => {
                            sender.input(AppMsg::NavigateList(-1));
                            return glib::Propagation::Stop;
                        }
                        gtk::gdk::Key::g => {
                            *waiting_for_g.borrow_mut() = true;
                            return glib::Propagation::Stop;
                        }
                        gtk::gdk::Key::i => {
                            sender.input(AppMsg::ActionOnFocusedPackage(FocusedAction::Install));
                            return glib::Propagation::Stop;
                        }
                        gtk::gdk::Key::r => {
                            sender.input(AppMsg::ActionOnFocusedPackage(FocusedAction::Remove));
                            return glib::Propagation::Stop;
                        }
                        gtk::gdk::Key::u => {
                            sender.input(AppMsg::ActionOnFocusedPackage(FocusedAction::Update));
                            return glib::Propagation::Stop;
                        }
                        _ => {}
                    }
                }

                glib::Propagation::Proceed
            }
        });
        root.add_controller(key_controller);

        root.connect_close_request({
            let config = config.clone();
            let sender = sender.clone();
            move |window| {
                let mut cfg = config.borrow_mut();
                cfg.window_maximized = window.is_maximized();
                if !cfg.window_maximized {
                    let (width, height) = window.default_size();
                    cfg.window_width = width;
                    cfg.window_height = height;
                }
                let _ = cfg.save();

                if cfg.run_in_background {
                    window.set_visible(false);
                    glib::Propagation::Stop
                } else {
                    sender.input(AppMsg::Shutdown);
                    glib::Propagation::Proceed
                }
            }
        });

        {
            let recent_searches = config.borrow().recent_searches.clone();
            header.update_recent_searches(&recent_searches);
        }

        let focus_controller = gtk::EventControllerFocus::new();
        focus_controller.connect_enter({
            let popover = header.search_popover.clone();
            let entry = header.search_entry.clone();
            move |_| {
                if entry.text().is_empty() {
                    popover.popup();
                }
            }
        });
        header.search_entry.add_controller(focus_controller);

        header.recent_searches_box.connect_row_activated({
            let sender = sender.clone();
            let entry = header.search_entry.clone();
            let popover = header.search_popover.clone();
            move |_, row| {
                if let Some(action_row) = row.downcast_ref::<adw::ActionRow>() {
                    let query = action_row.title().to_string();
                    if !query.is_empty() && query != "No recent searches" {
                        entry.set_text(&query);
                        popover.popdown();
                        sender.input(AppMsg::SearchChanged(query));
                    }
                }
            }
        });

        header.search_entry.connect_search_changed({
            let sender = sender.clone();
            let popover = header.search_popover.clone();
            move |entry| {
                if !entry.text().is_empty() {
                    popover.popdown();
                }
                sender.input(AppMsg::SearchChanged(entry.text().to_string()));
            }
        });

        header.refresh_button.connect_clicked({
            let sender = sender.clone();
            move |_| {
                sender.input(AppMsg::LoadPackages);
            }
        });

        header.select_button.connect_toggled({
            let sender = sender.clone();
            move |_| {
                sender.input(AppMsg::ToggleSelectionMode);
            }
        });

        header.command_center_btn.connect_clicked({
            let popover = task_hub_popover.clone();
            move |_| {
                popover.popup();
            }
        });

        header.list_view_btn.connect_toggled({
            let sender = sender.clone();
            move |btn| {
                if btn.is_active() {
                    sender.input(AppMsg::SetLayoutMode(LayoutMode::List));
                }
            }
        });

        header.grid_view_btn.connect_toggled({
            let sender = sender.clone();
            move |btn| {
                if btn.is_active() {
                    sender.input(AppMsg::SetLayoutMode(LayoutMode::Grid));
                }
            }
        });

        match layout_mode {
            LayoutMode::List => header.list_view_btn.set_active(true),
            LayoutMode::Grid => header.grid_view_btn.set_active(true),
        }

        update_all_btn.connect_clicked({
            let sender = sender.clone();
            move |_| {
                sender.input(AppMsg::UpdateAllPackages);
            }
        });

        load_more_btn.connect_clicked({
            let sender = sender.clone();
            move |_| {
                sender.input(AppMsg::LoadMore);
            }
        });

        sender.input(AppMsg::LoadPackages);

        let interval_hours = config.borrow().update_check_interval;
        if interval_hours > 0 {
            let sender_timer = sender.clone();
            let interval_ms = (interval_hours as u64) * 60 * 60 * 1000;
            glib::timeout_add_local(std::time::Duration::from_millis(interval_ms), move || {
                sender_timer.input(AppMsg::CheckUpdates);
                glib::ControlFlow::Continue
            });
        }

        {
            let sender_scheduler = sender.clone();
            glib::timeout_add_local(std::time::Duration::from_secs(60), move || {
                sender_scheduler.input(AppMsg::CheckScheduledTasks);
                glib::ControlFlow::Continue
            });
            sender.input(AppMsg::CheckScheduledTasks);
        }

        {
            let sender_nav = sender.clone();
            glib::timeout_add_local(std::time::Duration::from_millis(500), move || {
                sender_nav.input(AppMsg::CheckPendingNavigation);
                glib::ControlFlow::Continue
            });
        }

        let network_monitor = gio::NetworkMonitor::default();
        network_monitor.connect_network_changed({
            let sender = sender.clone();
            move |monitor, _available| {
                let is_available = monitor.is_network_available();
                sender.input(AppMsg::NetworkChanged(!is_available));
            }
        });

        let widgets = AppWidgets {
            header,
            sidebar,
            details_panel,
            task_hub,
            selection_bar,
            progress_overlay,
            collection_dialog,
            task_hub_popover,
            task_hub_btn,
            task_hub_spinner,
            task_hub_badge,
            content_stack,
            list_grid_stack,
            hero_banner,
            view_title,
            update_all_btn,
            category_filter_btn,
            toast_overlay,
            split_view,
            load_more_btn,
            load_more_label,
            offline_banner,
            storage_clamp,
            health_clamp,
            history_clamp,
            tasks_clamp,
            alias_clamp,
            alias_view,
            home_recommendations_group,
            home_recommendations_box,
            package_list_scrolled: scrolled,
        };

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            AppMsg::ViewChanged(view) => {
                self.current_view = view.clone();
                self.selected_package = None;
                self.details_visible = false;
                self.focused_index = 0;
                self.reset_visible_limit();
                if view != View::Updates {
                    self.update_category_filter = None;
                }
                if view == View::Home {
                    self.discover_results.clear();
                    if !self.search_query.is_empty() {
                        sender.input(AppMsg::DiscoverSearch(self.search_query.clone()));
                    }
                    sender.input(AppMsg::LoadHomeRecommendations);
                }
                if view == View::Storage {
                    sender.input(AppMsg::LoadCleanupStats);
                }
                if view == View::Health {
                    sender.input(AppMsg::LoadHealthData);
                }
                if view == View::History {
                    sender.input(AppMsg::LoadHistory);
                }
                if view == View::Tasks {
                    self.tasks_data.scheduler = self.config.borrow().scheduler.clone();
                    *self.pending_tasks_rebuild.borrow_mut() = true;
                }
                if view == View::Aliases {
                    *self.pending_alias_rebuild.borrow_mut() = true;
                    sender.input(AppMsg::LoadAliases);
                    sender.input(AppMsg::LoadPackageCommands);
                }
                self.refresh_package_list();
            }

            AppMsg::SearchChanged(query) => {
                if let Some(source_id) = self.search_debounce_source.borrow_mut().take() {
                    source_id.remove();
                }

                let query_clone = query.clone();
                let sender_clone = sender.clone();

                let source_id =
                    glib::timeout_add_local_once(Duration::from_millis(300), move || {
                        sender_clone.input(AppMsg::DebouncedSearchTrigger(query_clone));
                    });
                *self.search_debounce_source.borrow_mut() = Some(source_id);
            }

            AppMsg::DebouncedSearchTrigger(query) => {
                self.search_query = query.to_lowercase();
                self.reset_visible_limit();

                if !query.trim().is_empty() && query.len() >= 2 {
                    let mut config = self.config.borrow_mut();
                    let trimmed = query.trim().to_string();
                    config.recent_searches.retain(|s| s != &trimmed);
                    config.recent_searches.insert(0, trimmed);
                    config.recent_searches.truncate(5);
                    let _ = config.save();
                }

                if self.current_view == View::Home && !query.is_empty() {
                    sender.input(AppMsg::DiscoverSearch(query));
                } else {
                    self.refresh_package_list();
                }
            }

            AppMsg::SourceFilterChanged(source) => {
                self.source_filter = source;
                self.reset_visible_limit();
                self.refresh_package_list();
            }

            AppMsg::UpdateCategoryFilterChanged(category) => {
                self.update_category_filter = category;
                self.reset_visible_limit();
                self.refresh_package_list();
            }

            AppMsg::SortOrderChanged(order) => {
                self.sort_order = order;
                self.refresh_package_list();
            }

            AppMsg::LoadPackages => {
                self.is_loading = true;
                self.load_error = None;

                let pm = self.package_manager.clone();
                let sender = sender.clone();
                let shutdown = self.shutdown_signal.clone();

                relm4::spawn(async move {
                    let mut shutdown_rx = shutdown.subscribe();
                    let fut = async {
                        let manager = pm.lock().await;
                        manager.list_all_installed().await
                    };

                    tokio::select! {
                        _ = shutdown_rx.changed() => {}
                        result = fut => {
                            match result {
                                Ok(pkgs) => sender.input(AppMsg::PackagesLoaded(pkgs)),
                                Err(e) => sender.input(AppMsg::LoadingFailed(e.to_string())),
                            }
                        }
                    }
                });
            }

            AppMsg::PackagesLoaded(mut packages) => {
                let is_first_load = self.packages.is_empty();
                for pkg in &mut packages {
                    if pkg.available_version.is_some() && pkg.update_category.is_none() {
                        pkg.update_category = Some(pkg.detect_update_category());
                    }
                }
                self.packages = packages;
                self.is_loading = false;
                self.update_counts();
                self.refresh_package_list();

                if self.config.borrow().check_updates_on_startup {
                    sender.input(AppMsg::CheckUpdates);
                }

                if is_first_load {
                    sender.input(AppMsg::InitializeHistoryTracker);
                } else {
                    sender.input(AppMsg::TakeSnapshot);
                }

                if self.current_view == View::Home {
                    sender.input(AppMsg::LoadHomeRecommendations);
                }

                if self.current_view == View::Aliases {
                    sender.input(AppMsg::PopulateLazyPackages);
                    *self.pending_alias_rebuild.borrow_mut() = true;
                }
            }

            AppMsg::CheckUpdates => {
                let pm = self.package_manager.clone();
                let sender = sender.clone();
                let shutdown = self.shutdown_signal.clone();

                relm4::spawn(async move {
                    let mut shutdown_rx = shutdown.subscribe();
                    let fut = async {
                        let manager = pm.lock().await;
                        manager.check_all_updates().await
                    };

                    tokio::select! {
                        _ = shutdown_rx.changed() => {}
                        result = fut => {
                            match result {
                                Ok(updates) => sender.input(AppMsg::UpdatesChecked(updates)),
                                Err(e) => {
                                    tracing::warn!("Failed to check updates: {}", e);
                                }
                            }
                        }
                    }
                });
            }

            AppMsg::UpdatesChecked(updates) => {
                for update in updates {
                    if let Some(pkg) = self
                        .packages
                        .iter_mut()
                        .find(|p| p.name == update.name && p.source == update.source)
                    {
                        pkg.available_version = update.available_version.clone();
                        pkg.status = PackageStatus::UpdateAvailable;
                        pkg.update_category = Some(pkg.detect_update_category());
                    }
                }
                self.update_counts();
                self.refresh_package_list();

                let count = self.updates_count;
                if count > 0 && self.config.borrow().show_notifications {
                    notifications::send_updates_available_notification(count);
                    sender.input(AppMsg::ShowToast(
                        format!(
                            "{} update{} available",
                            count,
                            if count == 1 { "" } else { "s" }
                        ),
                        ToastType::Info,
                    ));
                }
            }

            AppMsg::LoadingFailed(error) => {
                self.is_loading = false;
                self.load_error = Some(error);
            }

            AppMsg::ToggleSelectionMode => {
                self.selection_mode = !self.selection_mode;
                if !self.selection_mode {
                    self.selected_packages.clear();
                }
                self.refresh_package_list();
            }

            AppMsg::SelectPackage(id, selected) => {
                if selected {
                    self.selected_packages.insert(id);
                } else {
                    self.selected_packages.remove(&id);
                }
            }

            AppMsg::SelectAll => {
                let (filtered, _) = self.filtered_packages();
                for pkg in filtered {
                    self.selected_packages.insert(pkg.id());
                }
                self.refresh_package_list();
            }

            AppMsg::DeselectAll => {
                self.selected_packages.clear();
                self.refresh_package_list();
            }

            AppMsg::PackageClicked(pkg) => {
                let pkg_id = pkg.id();
                crate::ui::set_ui_marker(format!("PackageClicked {} [{}]", pkg.name, pkg_id));
                tracing::info!(
                    pkg_id = %pkg_id,
                    pkg_name = %pkg.name,
                    source = ?pkg.source,
                    "Package clicked"
                );

                self.selected_package = Some(pkg);
                self.details_visible = true;
            }

            AppMsg::PackageAction(pkg) => {
                let action_type = match pkg.status {
                    PackageStatus::Installed => ActionType::Remove,
                    PackageStatus::UpdateAvailable => ActionType::Update,
                    PackageStatus::NotInstalled => ActionType::Install,
                    _ => return,
                };

                if action_type == ActionType::Remove {
                    let pm = self.package_manager.clone();
                    let pkg_clone = pkg.clone();
                    let sender = sender.clone();

                    relm4::spawn(async move {
                        let reverse_deps = {
                            let manager = pm.lock().await;
                            if let Some(backend) = manager.get_backend(pkg_clone.source) {
                                backend
                                    .get_reverse_dependencies(&pkg_clone.name)
                                    .await
                                    .unwrap_or_default()
                            } else {
                                Vec::new()
                            }
                        };

                        sender.input(AppMsg::PrepareRemovePreview {
                            package: pkg_clone,
                            reverse_deps,
                        });
                    });
                } else {
                    let mut preview = ActionPreview::new(action_type);
                    preview.add_package(pkg.clone());
                    *self.pending_action_preview.borrow_mut() = Some(preview);
                }
            }

            AppMsg::PrepareRemovePreview {
                package,
                reverse_deps,
            } => {
                let preview =
                    ActionPreview::new(ActionType::Remove).with_reverse_dependencies(reverse_deps);
                let mut preview = preview;
                preview.add_package(package);
                *self.pending_action_preview.borrow_mut() = Some(preview);
            }

            AppMsg::ExecutePackageAction(pkg) => match pkg.status {
                PackageStatus::Installed => {
                    let pm = self.package_manager.clone();
                    let tracker = self.history_tracker.clone();
                    let name = pkg.name.clone();
                    let sender = sender.clone();
                    let pkg_for_history = pkg.clone();

                    sender.input(AppMsg::OperationStarted {
                        package_name: name.clone(),
                        op: "Removing".to_string(),
                    });

                    relm4::spawn(async move {
                        let result = {
                            let manager = pm.lock().await;
                            manager.remove(&pkg).await
                        };
                        match result {
                            Ok(_) => {
                                {
                                    let mut guard = tracker.lock().await;
                                    if let Some(ref mut t) = *guard {
                                        t.record_remove(&pkg_for_history).await;
                                    }
                                }
                                sender.input(AppMsg::OperationCompleted {
                                    package_name: name,
                                    op: "Removed".to_string(),
                                });
                                sender.input(AppMsg::LoadPackages);
                            }
                            Err(e) => {
                                sender.input(AppMsg::OperationFailed {
                                    package_name: name,
                                    error: e.to_string(),
                                });
                            }
                        }
                    });
                }
                PackageStatus::UpdateAvailable => {
                    let pm = self.package_manager.clone();
                    let tracker = self.history_tracker.clone();
                    let name = pkg.name.clone();
                    let old_version = Some(pkg.version.clone());
                    let sender = sender.clone();
                    let pkg_for_history = pkg.clone();

                    sender.input(AppMsg::OperationStarted {
                        package_name: name.clone(),
                        op: "Updating".to_string(),
                    });

                    relm4::spawn(async move {
                        let result = {
                            let manager = pm.lock().await;
                            manager.update(&pkg).await
                        };
                        match result {
                            Ok(_) => {
                                {
                                    let mut guard = tracker.lock().await;
                                    if let Some(ref mut t) = *guard {
                                        t.record_update(&pkg_for_history, old_version).await;
                                    }
                                }
                                sender.input(AppMsg::OperationCompleted {
                                    package_name: name,
                                    op: "Updated".to_string(),
                                });
                                sender.input(AppMsg::LoadPackages);
                            }
                            Err(e) => {
                                sender.input(AppMsg::OperationFailed {
                                    package_name: name,
                                    error: e.to_string(),
                                });
                            }
                        }
                    });
                }
                PackageStatus::NotInstalled => {
                    let pm = self.package_manager.clone();
                    let tracker = self.history_tracker.clone();
                    let name = pkg.name.clone();
                    let sender = sender.clone();
                    let pkg_for_history = pkg.clone();

                    sender.input(AppMsg::OperationStarted {
                        package_name: name.clone(),
                        op: "Installing".to_string(),
                    });

                    relm4::spawn(async move {
                        let result = {
                            let manager = pm.lock().await;
                            manager.install(&pkg).await
                        };
                        match result {
                            Ok(_) => {
                                {
                                    let mut guard = tracker.lock().await;
                                    if let Some(ref mut t) = *guard {
                                        t.record_install(&pkg_for_history).await;
                                    }
                                }
                                sender.input(AppMsg::OperationCompleted {
                                    package_name: name,
                                    op: "Installed".to_string(),
                                });
                                sender.input(AppMsg::LoadPackages);
                            }
                            Err(e) => {
                                sender.input(AppMsg::OperationFailed {
                                    package_name: name,
                                    error: e.to_string(),
                                });
                            }
                        }
                    });
                }
                _ => {}
            },

            AppMsg::SourceFilterClicked(source) => {
                self.source_filter = Some(source);
                self.refresh_package_list();
            }

            AppMsg::UpdateAllPackages => {
                let packages: Vec<Package> = self
                    .packages
                    .iter()
                    .filter(|p| p.has_update() && self.enabled_sources.contains(&p.source))
                    .cloned()
                    .collect();

                if !packages.is_empty() {
                    self.bulk_op_total = packages.len();
                    self.bulk_op_completed = 0;
                    self.bulk_op_label = format!("Updating {} packages…", packages.len());
                }

                for pkg in packages {
                    sender.input(AppMsg::PackageAction(pkg));
                }
            }

            AppMsg::OperationStarted { package_name, op } => {
                self.operating_package_name = Some(package_name.clone());
                if self.bulk_op_total > 0 {
                    self.bulk_op_current_item = Some(package_name.clone());
                }

                self.pending_task_events
                    .borrow_mut()
                    .push(TaskHubInput::BeginTask {
                        title: format!("{} {}", op, package_name),
                        details: String::new(),
                        retry_spec: None,
                    });
            }

            AppMsg::OperationCompleted { package_name, op } => {
                self.operating_package_name = None;
                self.pending_task_events
                    .borrow_mut()
                    .push(TaskHubInput::AddEvent {
                        status: TaskStatus::Success,
                        title: format!("{} {}", op, package_name),
                        details: String::new(),
                        command: None,
                    });

                if self.bulk_op_total > 0 {
                    self.bulk_op_completed += 1;
                    if self.bulk_op_completed >= self.bulk_op_total {
                        self.bulk_op_total = 0;
                        self.bulk_op_completed = 0;
                        self.bulk_op_current_item = None;
                        sender.input(AppMsg::LoadHealthData);
                    }
                } else {
                    sender.input(AppMsg::LoadHealthData);
                }
            }

            AppMsg::OperationFailed {
                package_name,
                error,
            } => {
                self.operating_package_name = None;
                self.pending_task_events
                    .borrow_mut()
                    .push(TaskHubInput::AddEvent {
                        status: TaskStatus::Error,
                        title: format!("Failed: {}", package_name),
                        details: error,
                        command: None,
                    });

                if self.bulk_op_total > 0 {
                    self.bulk_op_completed += 1;
                    if self.bulk_op_completed >= self.bulk_op_total {
                        self.bulk_op_total = 0;
                        self.bulk_op_completed = 0;
                        self.bulk_op_current_item = None;
                    }
                }
            }

            AppMsg::ClosePackageDetails => {
                self.selected_package = None;
                self.details_visible = false;
            }

            AppMsg::ToggleSource(source) => {
                if self.enabled_sources.contains(&source) {
                    self.enabled_sources.remove(&source);
                } else if self.available_sources.contains(&source) {
                    self.enabled_sources.insert(source);
                }

                {
                    let mut config = self.config.borrow_mut();
                    config.enabled_sources = EnabledSources::from_sources(&self.enabled_sources);
                    let _ = config.save();
                }

                self.update_counts();
                sender.input(AppMsg::LoadPackages);
            }

            AppMsg::EnableDetectedSources => {
                self.enabled_sources = self.available_sources.clone();

                {
                    let mut config = self.config.borrow_mut();
                    config.enabled_sources = EnabledSources::from_sources(&self.enabled_sources);
                    let _ = config.save();
                }

                self.update_counts();
                sender.input(AppMsg::LoadPackages);
            }

            AppMsg::RefreshSources => {
                sender.input(AppMsg::LoadPackages);
            }

            AppMsg::ToggleFavorite(pkg_id) => {
                {
                    let mut config = self.config.borrow_mut();
                    if config.favorite_packages.contains(&pkg_id) {
                        config.favorite_packages.retain(|id| id != &pkg_id);
                    } else {
                        config.favorite_packages.push(pkg_id.clone());
                    }
                    let _ = config.save();
                }
                self.update_counts();
                self.refresh_package_list();
            }

            AppMsg::ShowToast(msg, toast_type) => {
                if !msg.is_empty() {
                    self.pending_toasts.borrow_mut().push((msg, toast_type));
                }
            }

            AppMsg::UnreadCountChanged(count) => {
                self.unread_count = count;
            }

            AppMsg::RetryOperation(spec) => match spec {
                RetrySpec::Package { package, op } => {
                    let mut pkg = *package;
                    match op {
                        PackageOp::Install => {
                            pkg.status = PackageStatus::NotInstalled;
                        }
                        PackageOp::Update => {
                            pkg.status = PackageStatus::UpdateAvailable;
                        }
                        PackageOp::Remove | PackageOp::Downgrade | PackageOp::DowngradeTo(_) => {
                            pkg.status = PackageStatus::Installed;
                        }
                    }
                    // Skip preview for retries - user already confirmed
                    sender.input(AppMsg::ExecutePackageAction(pkg));
                }
                RetrySpec::BulkUpdate { packages } => {
                    for mut pkg in packages {
                        pkg.status = PackageStatus::UpdateAvailable;
                        sender.input(AppMsg::ExecutePackageAction(pkg));
                    }
                }
                RetrySpec::BulkRemove { packages } => {
                    for mut pkg in packages {
                        pkg.status = PackageStatus::Installed;
                        sender.input(AppMsg::ExecutePackageAction(pkg));
                    }
                }
            },

            AppMsg::UpdateSelectedPackages => {
                let packages: Vec<Package> = self
                    .packages
                    .iter()
                    .filter(|p| {
                        self.selected_packages.contains(&p.id())
                            && p.has_update()
                            && self.enabled_sources.contains(&p.source)
                    })
                    .cloned()
                    .collect();

                if !packages.is_empty() {
                    self.bulk_op_total = packages.len();
                    self.bulk_op_completed = 0;
                    self.bulk_op_label = format!("Updating {} packages…", packages.len());
                }

                for pkg in packages {
                    sender.input(AppMsg::PackageAction(pkg));
                }
            }

            AppMsg::RemoveSelectedPackages => {
                let packages: Vec<Package> = self
                    .packages
                    .iter()
                    .filter(|p| {
                        self.selected_packages.contains(&p.id())
                            && self.enabled_sources.contains(&p.source)
                    })
                    .cloned()
                    .collect();

                if !packages.is_empty() {
                    self.bulk_op_total = packages.len();
                    self.bulk_op_completed = 0;
                    self.bulk_op_label = format!("Removing {} packages…", packages.len());
                }

                for pkg in packages {
                    sender.input(AppMsg::PackageAction(pkg));
                }
            }

            AppMsg::FocusSearch => {
                *self.pending_focus_search.borrow_mut() = true;
            }

            AppMsg::EscapePressed => {
                if self.details_visible {
                    self.selected_package = None;
                    self.details_visible = false;
                } else if self.selection_mode {
                    self.selection_mode = false;
                    self.selected_packages.clear();
                    self.refresh_package_list();
                }
            }

            AppMsg::LoadMore => {
                self.visible_limit += LOAD_MORE_INCREMENT;
                self.refresh_package_list();
            }

            AppMsg::DiscoverSearch(query) => {
                if query.len() < 2 {
                    self.discover_results.clear();
                    self.refresh_package_list();
                    return;
                }

                self.discover_loading = true;
                let pm = self.package_manager.clone();
                let sender = sender.clone();

                relm4::spawn(async move {
                    let result = {
                        let manager = pm.lock().await;
                        manager.search(&query).await
                    };
                    match result {
                        Ok(pkgs) => sender.input(AppMsg::DiscoverResultsLoaded(pkgs)),
                        Err(e) => sender.input(AppMsg::DiscoverSearchFailed(e.to_string())),
                    }
                });
            }

            AppMsg::DiscoverResultsLoaded(packages) => {
                self.discover_loading = false;
                self.discover_results = packages;
                self.refresh_package_list();
            }

            AppMsg::DiscoverSearchFailed(error) => {
                self.discover_loading = false;
                self.discover_results.clear();
                sender.input(AppMsg::ShowToast(
                    format!("Search failed: {}", error),
                    ToastType::Error,
                ));
            }

            AppMsg::SetLayoutMode(mode) => {
                self.layout_mode = mode;
                {
                    let mut config = self.config.borrow_mut();
                    config.layout_mode = mode;
                    let _ = config.save();
                }
                self.refresh_package_list();
            }

            AppMsg::NewCollection => {
                *self.pending_show_collection_dialog.borrow_mut() = true;
            }

            AppMsg::CreateCollection(name) => {
                {
                    let mut config = self.config.borrow_mut();
                    if !config.collections.contains_key(&name) {
                        config.collections.insert(name.clone(), Vec::new());
                        let _ = config.save();
                    }
                }
                let collection_counts = self.get_collection_counts();
                self.pending_sidebar_collections
                    .borrow_mut()
                    .replace(collection_counts);
                sender.input(AppMsg::ShowToast(
                    format!("Created collection: {}", name),
                    ToastType::Success,
                ));
            }

            AppMsg::TogglePackageCollection { pkg_id, collection } => {
                let mut added = false;
                {
                    let mut config = self.config.borrow_mut();
                    if let Some(pkg_ids) = config.collections.get_mut(&collection) {
                        if let Some(pos) = pkg_ids.iter().position(|id| id == &pkg_id) {
                            pkg_ids.remove(pos);
                        } else {
                            pkg_ids.push(pkg_id.clone());
                            added = true;
                        }
                        let _ = config.save();
                    }
                }
                let collection_counts = self.get_collection_counts();
                self.pending_sidebar_collections
                    .borrow_mut()
                    .replace(collection_counts);

                if self.current_view == View::Collection(collection.clone()) {
                    self.refresh_package_list();
                }

                let action = if added { "Added to" } else { "Removed from" };
                sender.input(AppMsg::ShowToast(
                    format!("{} {}", action, collection),
                    ToastType::Success,
                ));
            }

            AppMsg::ShowImage(texture) => {
                let window = relm4::main_adw_application().active_window();
                let mut builder = adw::Window::builder()
                    .title("Screenshot Preview")
                    .default_width(1000)
                    .default_height(700)
                    .modal(true);

                if let Some(ref win) = window {
                    builder = builder.transient_for(win);
                }

                let image_window = builder.build();

                let scrolled = gtk::ScrolledWindow::builder()
                    .hscrollbar_policy(gtk::PolicyType::Automatic)
                    .vscrollbar_policy(gtk::PolicyType::Automatic)
                    .build();

                let image = gtk::Picture::builder()
                    .paintable(&texture)
                    .can_shrink(true)
                    .build();

                scrolled.set_child(Some(&image));
                image_window.set_content(Some(&scrolled));
                image_window.present();
            }

            AppMsg::NetworkChanged(is_offline) => {
                self.is_offline = is_offline;
            }

            AppMsg::LoadCleanupStats => {
                self.cleanup_stats.is_loading = true;
                let pm = self.package_manager.clone();
                let enabled_sources = self.enabled_sources.clone();
                let sender = sender.clone();

                relm4::spawn(async move {
                    let manager = pm.lock().await;
                    let mut stats = CleanupStats::default();

                    for source in enabled_sources {
                        if let Some(backend) = manager.get_backend(source) {
                            if let Ok(cache_size) = backend.get_cache_size().await {
                                if cache_size > 0 {
                                    stats.cache_sizes.insert(source, cache_size);
                                    stats.total_recoverable += cache_size;
                                }
                            }
                            if let Ok(orphans) = backend.get_orphaned_packages().await {
                                let count = orphans.len();
                                if count > 0 {
                                    stats.total_orphaned += count;
                                    stats.orphaned_packages.insert(source, orphans);
                                }
                            }
                        }
                    }

                    sender.input(AppMsg::CleanupStatsLoaded(stats));
                });
            }

            AppMsg::CleanupStatsLoaded(stats) => {
                self.cleanup_stats = stats;
                self.cleanup_stats.is_loading = false;
            }

            AppMsg::ConfirmCleanup(action) => match action {
                CleanupAction::Refresh => {
                    sender.input(AppMsg::LoadCleanupStats);
                }
                _ => {
                    *self.pending_cleanup_confirm.borrow_mut() = Some(action);
                }
            },

            AppMsg::ExecuteCleanup(action) => {
                let pm = self.package_manager.clone();
                let sender = sender.clone();

                match action {
                    CleanupAction::Refresh => {
                        sender.input(AppMsg::LoadCleanupStats);
                    }
                    CleanupAction::CleanAll => {
                        let sources: Vec<_> =
                            self.cleanup_stats.cache_sizes.keys().copied().collect();
                        for source in sources {
                            sender
                                .input(AppMsg::ExecuteCleanup(CleanupAction::CleanSource(source)));
                        }
                    }
                    CleanupAction::CleanSource(source) => {
                        sender.input(AppMsg::OperationStarted {
                            package_name: format!("{} cache", source),
                            op: "Cleaning".to_string(),
                        });

                        relm4::spawn(async move {
                            let manager = pm.lock().await;
                            if let Some(backend) = manager.get_backend(source) {
                                match backend.cleanup_cache().await {
                                    Ok(freed) => {
                                        sender.input(AppMsg::CleanupCompleted {
                                            source: Some(source),
                                            freed,
                                        });
                                    }
                                    Err(e) => {
                                        sender.input(AppMsg::CleanupFailed(e.to_string()));
                                    }
                                }
                            }
                        });
                    }
                    CleanupAction::RemoveOrphans(source) => {
                        sender.input(AppMsg::OperationStarted {
                            package_name: format!("{} orphans", source),
                            op: "Removing".to_string(),
                        });

                        relm4::spawn(async move {
                            let manager = pm.lock().await;
                            if let Some(backend) = manager.get_backend(source) {
                                match backend.cleanup_cache().await {
                                    Ok(freed) => {
                                        sender.input(AppMsg::CleanupCompleted {
                                            source: Some(source),
                                            freed,
                                        });
                                    }
                                    Err(e) => {
                                        sender.input(AppMsg::CleanupFailed(e.to_string()));
                                    }
                                }
                            }
                        });
                    }
                }
            }

            AppMsg::CleanupCompleted { source, freed } => {
                let source_name = source
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "All sources".to_string());
                let freed_str = humansize::format_size(freed, humansize::BINARY);

                self.pending_task_events
                    .borrow_mut()
                    .push(TaskHubInput::AddEvent {
                        status: TaskStatus::Success,
                        title: format!("Cleaned {}", source_name),
                        details: format!("Freed {}", freed_str),
                        command: None,
                    });

                let tracker = self.history_tracker.clone();
                relm4::spawn(async move {
                    let mut guard = tracker.lock().await;
                    if let Some(ref mut t) = *guard {
                        t.record_cleanup(source, freed).await;
                    }
                });

                sender.input(AppMsg::LoadCleanupStats);
                sender.input(AppMsg::LoadHealthData);
                sender.input(AppMsg::ShowToast(
                    format!("Freed {}", freed_str),
                    ToastType::Success,
                ));
            }

            AppMsg::CleanupFailed(error) => {
                self.pending_task_events
                    .borrow_mut()
                    .push(TaskHubInput::AddEvent {
                        status: TaskStatus::Error,
                        title: "Cleanup failed".to_string(),
                        details: error.clone(),
                        command: None,
                    });

                sender.input(AppMsg::ShowToast(
                    format!("Cleanup failed: {}", error),
                    ToastType::Error,
                ));
            }

            AppMsg::DuplicateAction(dup_action) => {
                use crate::ui::storage_view::DuplicateAction;
                match dup_action {
                    DuplicateAction::RemovePackage(pkg) => {
                        sender.input(AppMsg::PackageAction(*pkg));
                    }
                    DuplicateAction::RemoveGroup(group) => {
                        for pkg in group.packages.clone() {
                            if Some(pkg.source) != group.suggested_keep {
                                sender.input(AppMsg::PackageAction(pkg));
                            }
                        }
                    }
                }
            }

            AppMsg::LoadHealthData => {
                self.health_data.is_loading = true;
                let pm = self.package_manager.clone();
                let enabled_sources = self.enabled_sources.clone();
                let cleanup_stats = self.cleanup_stats.clone();
                let updates_count = self.packages.iter().filter(|p| p.has_update()).count();
                let sender = sender.clone();

                relm4::spawn(async move {
                    let manager = pm.lock().await;
                    let mut orphaned: std::collections::HashMap<PackageSource, usize> =
                        std::collections::HashMap::new();

                    for source in &enabled_sources {
                        if let Some(backend) = manager.get_backend(*source) {
                            if let Ok(orphans) = backend.get_orphaned_packages().await {
                                if !orphans.is_empty() {
                                    orphaned.insert(*source, orphans.len());
                                }
                            }
                        }
                    }

                    let lock_statuses = manager.check_all_lock_status().await;

                    let health = crate::models::SystemHealth::compute(
                        updates_count,
                        0,
                        orphaned.clone(),
                        cleanup_stats.total_recoverable,
                    );

                    let mut issues: Vec<HealthIssueData> = Vec::new();

                    for issue in &health.issues {
                        let severity = match issue.severity() {
                            crate::models::IssueSeverity::Critical => Severity::Critical,
                            crate::models::IssueSeverity::Warning => Severity::Warning,
                            crate::models::IssueSeverity::Info => Severity::Warning,
                        };

                        let (icon, action_id): (&str, String) = match issue {
                            crate::models::HealthIssue::SecurityUpdates { .. } => (
                                "software-update-urgent-symbolic",
                                "security_updates".to_string(),
                            ),
                            crate::models::HealthIssue::PendingUpdates { .. } => (
                                "software-update-available-symbolic",
                                "pending_updates".to_string(),
                            ),
                            crate::models::HealthIssue::RecoverableSpace { .. } => {
                                ("drive-harddisk-symbolic", "cleanup_space".to_string())
                            }
                            crate::models::HealthIssue::OrphanedPackages { .. } => {
                                ("user-trash-symbolic", "remove_orphans".to_string())
                            }
                            crate::models::HealthIssue::BrokenDependencies { .. } => {
                                ("dialog-error-symbolic", "broken_deps".to_string())
                            }
                            crate::models::HealthIssue::UnreachableRepo { .. } => {
                                ("network-offline-symbolic", "unreachable_repo".to_string())
                            }
                            crate::models::HealthIssue::PackageManagerLocked { holder, .. } => {
                                let action_id = format!(
                                    "lock_process:{}",
                                    holder.as_deref().unwrap_or("unknown")
                                );
                                ("system-lock-screen-symbolic", action_id)
                            }
                        };

                        issues.push(HealthIssueData {
                            icon,
                            title: issue.title(),
                            subtitle: format!("{:?} priority", severity),
                            severity,
                            action_id,
                        });
                    }

                    for (source, lock_status) in &lock_statuses {
                        let action_id = format!(
                            "lock_process:{}",
                            lock_status.lock_holder.as_deref().unwrap_or("unknown")
                        );
                        let title = match &lock_status.lock_holder {
                            Some(holder) => format!("{} is locked by '{}'", source, holder),
                            None => format!("{} is locked by another process", source),
                        };
                        issues.insert(
                            0,
                            HealthIssueData {
                                icon: "system-lock-screen-symbolic",
                                title,
                                subtitle: "Critical priority - operations blocked".to_string(),
                                severity: Severity::Critical,
                                action_id,
                            },
                        );
                    }

                    if issues.is_empty() {
                        issues.push(HealthIssueData {
                            icon: "emblem-ok-symbolic",
                            title: "No issues detected".to_string(),
                            subtitle: "Your system is healthy".to_string(),
                            severity: Severity::Good,
                            action_id: String::new(),
                        });
                    }

                    let data = HealthData {
                        score: health.score,
                        issues,
                        is_loading: false,
                    };

                    sender.input(AppMsg::HealthDataLoaded(data));
                });
            }

            AppMsg::HealthDataLoaded(data) => {
                self.health_data = data;
            }

            AppMsg::HealthAction(action) => match action {
                HealthAction::Refresh => {
                    sender.input(AppMsg::LoadHealthData);
                }
                HealthAction::FixSecurityUpdates | HealthAction::FixPendingUpdates => {
                    sender.input(AppMsg::ViewChanged(View::Updates));
                }
                HealthAction::CleanupSpace => {
                    sender.input(AppMsg::ViewChanged(View::Storage));
                }
                HealthAction::RemoveOrphans => {
                    sender.input(AppMsg::ViewChanged(View::Storage));
                }
                HealthAction::ViewLockProcess(holder) => {
                    let msg = if holder == "unknown" {
                        "A package manager is locked. Wait for the other process to finish."
                            .to_string()
                    } else {
                        format!(
                            "Locked by '{}'. Wait for it to finish or terminate it.",
                            holder
                        )
                    };
                    sender.input(AppMsg::ShowToast(msg, ToastType::Warning));
                }
            },

            AppMsg::LoadHistory => {
                self.history_data.is_loading = true;
                let tracker = self.history_tracker.clone();
                let filter = self.history_data.filter;
                let search = self.history_data.search_query.clone();
                let sender = sender.clone();

                relm4::spawn(async move {
                    let tracker_guard = tracker.lock().await;
                    let entries = if let Some(ref t) = *tracker_guard {
                        filter_entries(t.history(), filter, &search)
                    } else {
                        Vec::new()
                    };
                    sender.input(AppMsg::HistoryLoaded(entries));
                });
            }

            AppMsg::HistoryLoaded(entries) => {
                self.history_data.entries = entries;
                self.history_data.is_loading = false;
            }

            AppMsg::HistoryAction(action) => match action {
                HistoryViewAction::Refresh => {
                    sender.input(AppMsg::LoadHistory);
                }
                HistoryViewAction::FilterChanged(filter) => {
                    self.history_data.filter = filter;
                    sender.input(AppMsg::LoadHistory);
                }
                HistoryViewAction::Search(query) => {
                    self.history_data.search_query = query;
                    sender.input(AppMsg::LoadHistory);
                }
                HistoryViewAction::ToggleSelectionMode => {
                    self.history_data.selection_mode = !self.history_data.selection_mode;
                    if !self.history_data.selection_mode {
                        self.history_data.selected_entries.clear();
                    }
                }
                HistoryViewAction::SelectEntry(entry_id, selected) => {
                    if selected {
                        self.history_data.selected_entries.insert(entry_id);
                    } else {
                        self.history_data.selected_entries.remove(&entry_id);
                    }
                }
                HistoryViewAction::SelectAll => {
                    for entry in &self.history_data.entries {
                        if entry.is_reversible() {
                            self.history_data.selected_entries.insert(entry.id.clone());
                        }
                    }
                }
                HistoryViewAction::DeselectAll => {
                    self.history_data.selected_entries.clear();
                }
                HistoryViewAction::BulkUndo(entry_ids) => {
                    self.history_data.selection_mode = false;
                    self.history_data.selected_entries.clear();

                    for entry_id in entry_ids {
                        sender.input(AppMsg::HistoryAction(HistoryViewAction::Undo(entry_id)));
                    }
                }
                HistoryViewAction::Export => {
                    let tracker = self.history_tracker.clone();
                    let sender = sender.clone();

                    let app = relm4::main_adw_application();
                    let window = app.active_window();

                    let dialog = gtk::FileChooserNative::builder()
                        .title("Export History")
                        .action(gtk::FileChooserAction::Save)
                        .modal(true)
                        .build();

                    if let Some(ref win) = window {
                        dialog.set_transient_for(Some(win));
                    }

                    dialog.set_current_name("linget-history.json");

                    let json_filter = gtk::FileFilter::new();
                    json_filter.set_name(Some("JSON files"));
                    json_filter.add_pattern("*.json");
                    dialog.add_filter(&json_filter);

                    let csv_filter = gtk::FileFilter::new();
                    csv_filter.set_name(Some("CSV files"));
                    csv_filter.add_pattern("*.csv");
                    dialog.add_filter(&csv_filter);

                    dialog.connect_response(move |dialog, response| {
                        if response == gtk::ResponseType::Accept {
                            if let Some(file) = dialog.file() {
                                if let Some(path) = file.path() {
                                    let is_csv = path
                                        .extension()
                                        .map(|ext| ext.to_string_lossy().to_lowercase() == "csv")
                                        .unwrap_or(false);

                                    let tracker = tracker.clone();
                                    let sender = sender.clone();

                                    glib::spawn_future_local(async move {
                                        let export_result = {
                                            let guard = tracker.lock().await;
                                            if let Some(ref t) = *guard {
                                                if is_csv {
                                                    t.export_csv().await
                                                } else {
                                                    t.export_json().await
                                                }
                                            } else {
                                                Err(anyhow::anyhow!(
                                                    "History tracker not initialized"
                                                ))
                                            }
                                        };

                                        match export_result {
                                            Ok(content) => match std::fs::write(&path, content) {
                                                Ok(_) => {
                                                    sender.input(AppMsg::ShowToast(
                                                        format!(
                                                            "Exported to {}",
                                                            path.file_name()
                                                                .map(|n| {
                                                                    n.to_string_lossy().to_string()
                                                                })
                                                                .unwrap_or_else(|| {
                                                                    "file".to_string()
                                                                })
                                                        ),
                                                        ToastType::Success,
                                                    ));
                                                }
                                                Err(e) => {
                                                    sender.input(AppMsg::ShowToast(
                                                        format!("Failed to save: {}", e),
                                                        ToastType::Error,
                                                    ));
                                                }
                                            },
                                            Err(e) => {
                                                sender.input(AppMsg::ShowToast(
                                                    format!("Export failed: {}", e),
                                                    ToastType::Error,
                                                ));
                                            }
                                        }
                                    });
                                }
                            }
                        }
                    });

                    dialog.show();
                }
                HistoryViewAction::Undo(entry_id) => {
                    let tracker = self.history_tracker.clone();
                    let packages = self.packages.clone();
                    let sender = sender.clone();

                    relm4::spawn(async move {
                        let entry_opt = {
                            let guard = tracker.lock().await;
                            guard.as_ref().and_then(|t| {
                                t.history()
                                    .entries
                                    .iter()
                                    .find(|e| e.id == entry_id)
                                    .cloned()
                            })
                        };

                        let Some(entry) = entry_opt else {
                            sender.input(AppMsg::ShowToast(
                                "Could not find history entry".to_string(),
                                ToastType::Error,
                            ));
                            return;
                        };

                        use crate::models::HistoryOperation;

                        match entry.operation {
                            HistoryOperation::Install | HistoryOperation::ExternalInstall => {
                                if let Some(pkg) = packages.iter().find(|p| {
                                    p.name == entry.package_name && p.source == entry.package_source
                                }) {
                                    let mut undo_pkg = pkg.clone();
                                    undo_pkg.status = PackageStatus::Installed;
                                    sender.input(AppMsg::ExecutePackageAction(undo_pkg));

                                    let mut guard = tracker.lock().await;
                                    if let Some(ref mut t) = *guard {
                                        t.mark_undone(&entry_id);
                                        let _ = t.save().await;
                                    }
                                } else {
                                    sender.input(AppMsg::ShowToast(
                                        format!("Package {} not found", entry.package_name),
                                        ToastType::Error,
                                    ));
                                }
                            }
                            HistoryOperation::Remove | HistoryOperation::ExternalRemove => {
                                let undo_pkg = Package {
                                    name: entry.package_name.clone(),
                                    version: entry.version_before.clone().unwrap_or_default(),
                                    source: entry.package_source,
                                    status: PackageStatus::NotInstalled,
                                    description: String::new(),
                                    size: None,
                                    available_version: None,
                                    install_date: None,
                                    homepage: None,
                                    license: None,
                                    maintainer: None,
                                    dependencies: Vec::new(),
                                    update_category: None,
                                    enrichment: None,
                                };
                                sender.input(AppMsg::ExecutePackageAction(undo_pkg));

                                let mut guard = tracker.lock().await;
                                if let Some(ref mut t) = *guard {
                                    t.mark_undone(&entry_id);
                                    let _ = t.save().await;
                                }
                            }
                            HistoryOperation::Update | HistoryOperation::ExternalUpdate => {
                                if let Some(old_version) = &entry.version_before {
                                    if let Some(pkg) = packages.iter().find(|p| {
                                        p.name == entry.package_name
                                            && p.source == entry.package_source
                                    }) {
                                        sender.input(AppMsg::DowngradePackage {
                                            package: pkg.clone(),
                                            target_version: old_version.clone(),
                                        });

                                        let mut guard = tracker.lock().await;
                                        if let Some(ref mut t) = *guard {
                                            t.mark_undone(&entry_id);
                                            let _ = t.save().await;
                                        }
                                    } else {
                                        sender.input(AppMsg::ShowToast(
                                            format!("Package {} not found", entry.package_name),
                                            ToastType::Error,
                                        ));
                                    }
                                } else {
                                    sender.input(AppMsg::ShowToast(
                                        "Previous version unknown - cannot downgrade".to_string(),
                                        ToastType::Warning,
                                    ));
                                }
                            }
                            HistoryOperation::Downgrade => {
                                sender.input(AppMsg::ShowToast(
                                    "Upgrade after downgrade not yet supported".to_string(),
                                    ToastType::Warning,
                                ));
                            }
                            HistoryOperation::Cleanup => {
                                sender.input(AppMsg::ShowToast(
                                    "Cannot undo cleanup operations".to_string(),
                                    ToastType::Warning,
                                ));
                            }
                        }

                        sender.input(AppMsg::LoadHistory);
                    });
                }
            },

            AppMsg::RecordHistoryEntry(entry) => {
                let tracker = self.history_tracker.clone();
                relm4::spawn(async move {
                    let mut tracker_guard = tracker.lock().await;
                    if let Some(ref mut t) = *tracker_guard {
                        t.history_mut().add(entry);
                        let _ = t.save().await;
                    }
                });
            }

            AppMsg::InitializeHistoryTracker => {
                let tracker_arc = self.history_tracker.clone();
                let packages = self.packages.clone();
                let sender = sender.clone();

                relm4::spawn(async move {
                    match HistoryTracker::load().await {
                        Ok(tracker) => {
                            let external_changes = tracker.detect_external_changes(&packages);
                            {
                                let mut guard = tracker_arc.lock().await;
                                *guard = Some(tracker);
                            }
                            sender.input(AppMsg::HistoryTrackerReady { external_changes });
                        }
                        Err(e) => {
                            tracing::warn!(error = %e, "Failed to load history tracker");
                            let tracker = HistoryTracker::load().await.unwrap_or_else(|_| {
                                panic!("Critical: Cannot initialize history tracker")
                            });
                            let mut guard = tracker_arc.lock().await;
                            *guard = Some(tracker);
                        }
                    }
                });
            }

            AppMsg::HistoryTrackerReady { external_changes } => {
                if !external_changes.is_empty() {
                    let count = external_changes.len();
                    let tracker = self.history_tracker.clone();

                    relm4::spawn(async move {
                        let mut guard = tracker.lock().await;
                        if let Some(ref mut t) = *guard {
                            t.apply_external_changes(external_changes).await;
                        }
                    });

                    sender.input(AppMsg::ShowToast(
                        format!(
                            "Detected {} external package change{}",
                            count,
                            if count == 1 { "" } else { "s" }
                        ),
                        ToastType::Info,
                    ));
                }

                sender.input(AppMsg::TakeSnapshot);
            }

            AppMsg::TakeSnapshot => {
                let tracker = self.history_tracker.clone();
                let packages = self.packages.clone();

                relm4::spawn(async move {
                    let mut guard = tracker.lock().await;
                    if let Some(ref mut t) = *guard {
                        t.take_snapshot(&packages).await;
                    }
                });
            }

            AppMsg::OpenCommandPalette => {
                *self.pending_command_palette.borrow_mut() = true;
            }

            AppMsg::ExecutePaletteCommand(cmd) => {
                use crate::ui::command_palette::PaletteCommand;

                match cmd {
                    PaletteCommand::UpdateAll => {
                        sender.input(AppMsg::UpdateAllPackages);
                    }
                    PaletteCommand::CleanCaches => {
                        sender.input(AppMsg::ExecuteCleanup(CleanupAction::CleanAll));
                    }
                    PaletteCommand::GoToHome => {
                        sender.input(AppMsg::ViewChanged(View::Home));
                    }
                    PaletteCommand::GoToLibrary => {
                        sender.input(AppMsg::ViewChanged(View::Library));
                    }
                    PaletteCommand::GoToUpdates => {
                        sender.input(AppMsg::ViewChanged(View::Updates));
                    }
                    PaletteCommand::GoToStorage => {
                        sender.input(AppMsg::ViewChanged(View::Storage));
                    }
                    PaletteCommand::GoToHealth => {
                        sender.input(AppMsg::ViewChanged(View::Health));
                    }
                    PaletteCommand::GoToHistory => {
                        sender.input(AppMsg::ViewChanged(View::History));
                    }
                    PaletteCommand::GoToFavorites => {
                        sender.input(AppMsg::ViewChanged(View::Favorites));
                    }
                    PaletteCommand::GoToTasks => {
                        sender.input(AppMsg::ViewChanged(View::Tasks));
                    }
                    PaletteCommand::ScheduleAllUpdates => {
                        sender.input(AppMsg::ScheduleAllUpdates);
                    }
                    PaletteCommand::RefreshPackages => {
                        sender.input(AppMsg::LoadPackages);
                    }
                    PaletteCommand::ToggleSelectionMode => {
                        sender.input(AppMsg::ToggleSelectionMode);
                    }
                    PaletteCommand::OpenPreferences => {
                        let app = relm4::main_adw_application();
                        app.activate_action("preferences", None);
                    }
                    PaletteCommand::ShowShortcuts => {
                        let app = relm4::main_adw_application();
                        app.activate_action("shortcuts", None);
                    }
                    PaletteCommand::ExportPackages => {
                        let app = relm4::main_adw_application();
                        app.activate_action("export", None);
                    }
                    PaletteCommand::ImportPackages => {
                        let app = relm4::main_adw_application();
                        app.activate_action("import", None);
                    }
                    PaletteCommand::Search(query) => {
                        sender.input(AppMsg::SearchChanged(query));
                    }
                }
            }

            AppMsg::LoadHomeRecommendations => {
                if self.packages.is_empty() {
                    return;
                }
                self.home_recommendations_loading = true;

                let installed_names: Vec<String> =
                    self.packages.iter().map(|p| p.name.clone()).collect();
                let dismissed = self.config.borrow().dismissed_recommendations.clone();
                let sender = sender.clone();

                relm4::spawn(async move {
                    let dismissed_set: std::collections::HashSet<String> =
                        dismissed.into_iter().collect();
                    let recs = get_global_recommendations(&installed_names, &dismissed_set, 6);
                    sender.input(AppMsg::HomeRecommendationsLoaded(recs));
                });
            }

            AppMsg::HomeRecommendationsLoaded(recs) => {
                self.home_recommendations = recs;
                self.home_recommendations_loading = false;
                *self.pending_home_recommendations_rebuild.borrow_mut() = true;
            }

            AppMsg::InstallHomeRecommendation(name) => {
                sender.input(AppMsg::SearchChanged(name.clone()));
                sender.input(AppMsg::ShowToast(
                    format!("Searching for '{}'", name),
                    ToastType::Info,
                ));
            }

            AppMsg::DismissHomeRecommendation(name) => {
                {
                    let mut cfg = self.config.borrow_mut();
                    if !cfg.dismissed_recommendations.contains(&name) {
                        cfg.dismissed_recommendations.push(name.clone());
                        let _ = cfg.save();
                    }
                }
                self.home_recommendations.retain(|r| r.name != name);
                *self.pending_home_recommendations_rebuild.borrow_mut() = true;
            }

            AppMsg::NavigateList(delta) => {
                let count = match self.layout_mode {
                    LayoutMode::List => self.package_rows.len(),
                    LayoutMode::Grid => self.package_cards.len(),
                };
                if count == 0 {
                    return;
                }
                let new_index = if delta > 0 {
                    (self.focused_index + delta as usize).min(count.saturating_sub(1))
                } else {
                    self.focused_index.saturating_sub((-delta) as usize)
                };
                self.focused_index = new_index;
            }

            AppMsg::OpenFocusedPackageDetails => {
                if let Some(pkg) = self.get_visible_package(self.focused_index) {
                    self.selected_package = Some(pkg.clone());
                    self.details_visible = true;
                }
            }

            AppMsg::ActionOnFocusedPackage(action) => {
                if let Some(pkg) = self.get_visible_package(self.focused_index) {
                    let should_act = match action {
                        FocusedAction::Install => pkg.status == PackageStatus::NotInstalled,
                        FocusedAction::Remove => {
                            matches!(
                                pkg.status,
                                PackageStatus::Installed | PackageStatus::UpdateAvailable
                            )
                        }
                        FocusedAction::Update => pkg.has_update(),
                    };
                    if should_act {
                        sender.input(AppMsg::PackageAction(pkg.clone()));
                    }
                }
            }

            AppMsg::ToggleFocusedSelection => {
                if let Some(pkg) = self.get_visible_package(self.focused_index) {
                    let pkg_id = pkg.id();
                    if self.selected_packages.contains(&pkg_id) {
                        self.selected_packages.remove(&pkg_id);
                    } else {
                        self.selected_packages.insert(pkg_id);
                    }
                    if !self.selection_mode && !self.selected_packages.is_empty() {
                        self.selection_mode = true;
                    }
                }
            }

            AppMsg::SelectAllVisible => {
                let visible_ids: Vec<String> = match self.layout_mode {
                    LayoutMode::List => self
                        .package_rows
                        .iter()
                        .map(|row| row.package.id())
                        .collect(),
                    LayoutMode::Grid => self
                        .package_cards
                        .iter()
                        .map(|card| card.package.id())
                        .collect(),
                };
                for id in visible_ids {
                    self.selected_packages.insert(id);
                }
                if !self.selection_mode && !self.selected_packages.is_empty() {
                    self.selection_mode = true;
                }
            }

            AppMsg::LoadAliases => {
                self.alias_data.is_loading = true;
                *self.pending_alias_rebuild.borrow_mut() = true;
                let sender = sender.clone();

                std::thread::spawn(move || {
                    let mut manager = crate::models::alias::AliasManager::new();
                    let _ = manager.load_existing_aliases();
                    manager.scan_available_commands();
                    sender.input(AppMsg::AliasesLoaded(manager));
                });
            }

            AppMsg::AliasesLoaded(manager) => {
                let existing_lazy_packages =
                    std::mem::take(&mut self.alias_data.manager.lazy_packages);
                let existing_package_commands =
                    std::mem::take(&mut self.alias_data.manager.package_commands);

                self.alias_data.manager = manager;
                self.alias_data.manager.lazy_packages = existing_lazy_packages;
                self.alias_data.manager.package_commands = existing_package_commands;

                self.alias_data.is_loading = false;
                *self.pending_alias_rebuild.borrow_mut() = true;
            }

            AppMsg::LoadPackageCommands => {
                sender.input(AppMsg::PopulateLazyPackages);
            }

            AppMsg::PopulateLazyPackages => {
                use crate::models::alias::LazyPackage;

                let source_priority = |s: PackageSource| -> u8 {
                    match s {
                        PackageSource::Cargo => 0,
                        PackageSource::Pipx => 1,
                        PackageSource::Npm => 2,
                        PackageSource::Pip => 3,
                        PackageSource::Dart => 4,
                        PackageSource::Brew => 5,
                        PackageSource::Flatpak => 6,
                        PackageSource::Snap => 7,
                        PackageSource::AppImage => 8,
                        _ => 9,
                    }
                };

                let mut filtered: Vec<_> = self.packages.iter().collect();

                filtered.sort_by_key(|p| source_priority(p.source));

                self.alias_data.manager.lazy_packages = filtered
                    .into_iter()
                    .map(|p| LazyPackage {
                        name: p.name.clone(),
                        source: p.source,
                        loading: false,
                        loaded: false,
                    })
                    .collect();
                *self.pending_alias_rebuild.borrow_mut() = true;
            }

            AppMsg::ExpandPackage { name, source } => {
                tracing::info!(
                    package = %name,
                    source = ?source,
                    "ExpandPackage message received"
                );

                let lazy_pkg_state = self.alias_data.manager.get_lazy_package(&name, source);
                tracing::info!(
                    package = %name,
                    found = lazy_pkg_state.is_some(),
                    loaded = lazy_pkg_state.map(|p| p.loaded),
                    loading = lazy_pkg_state.map(|p| p.loading),
                    "Lazy package lookup result"
                );

                if let Some(lazy_pkg) = lazy_pkg_state {
                    if lazy_pkg.loaded || lazy_pkg.loading {
                        tracing::info!(package = %name, "Skipping - already loaded or loading");
                        return;
                    }
                }

                self.alias_data
                    .manager
                    .set_package_loading(&name, source, true);
                *self.pending_alias_rebuild.borrow_mut() = true;

                let pm = self.package_manager.clone();
                let pkg_name = name.clone();
                let pkg_source = source;
                let sender = sender.clone();
                let shutdown = self.shutdown_signal.clone();

                relm4::spawn(async move {
                    use crate::models::alias::CommandInfo;

                    tracing::info!(
                        package = %pkg_name,
                        source = ?pkg_source,
                        "Discovering package commands"
                    );

                    let mut shutdown_rx = shutdown.subscribe();
                    let fut = async {
                        let manager = pm.lock().await;
                        manager.get_package_commands(&pkg_name, pkg_source).await
                    };

                    let commands = tokio::select! {
                        _ = shutdown_rx.changed() => {
                            return;
                        }
                        result = fut => {
                            match result {
                                Ok(cmds) => cmds,
                                Err(e) => {
                                    tracing::warn!(
                                        package = %pkg_name,
                                        source = ?pkg_source,
                                        error = %e,
                                        "Failed to get package commands"
                                    );
                                    Vec::new()
                                }
                            }
                        }
                    };

                    tracing::info!(
                        package = %pkg_name,
                        source = ?pkg_source,
                        command_count = commands.len(),
                        "Discovered package commands"
                    );

                    let cmds: Vec<CommandInfo> = commands
                        .into_iter()
                        .map(|(name, path)| {
                            let subcommands = crate::models::alias::discover_subcommands(&name);
                            CommandInfo {
                                name,
                                path,
                                description: None,
                                subcommands,
                            }
                        })
                        .collect();

                    sender.input(AppMsg::PackageCommandsLoadedFor {
                        name: pkg_name,
                        source: pkg_source,
                        commands: cmds,
                    });
                });
            }

            AppMsg::PackageCommandsLoadedFor {
                name,
                source,
                commands,
            } => {
                self.alias_data
                    .manager
                    .set_package_commands(&name, source, commands);
                *self.pending_alias_rebuild.borrow_mut() = true;
            }

            AppMsg::PackageCommandsLoaded(package_commands) => {
                self.alias_data.manager.package_commands = package_commands;
            }

            AppMsg::CopyToClipboard(text) => {
                if let Some(display) = gdk::Display::default() {
                    display.clipboard().set_text(&text);
                    sender.input(AppMsg::ShowToast(
                        "Copied to clipboard".to_string(),
                        ToastType::Success,
                    ));
                }
            }

            AppMsg::CreateAlias {
                name,
                command,
                shells,
                description,
            } => {
                let mut alias =
                    crate::models::alias::ShellAlias::new(name.clone(), command, shells);
                alias.description = description;

                let mut manager = self.alias_data.manager.clone();
                let sender = sender.clone();

                std::thread::spawn(move || match manager.add_alias(alias) {
                    Ok(_) => {
                        sender.input(AppMsg::AliasOperationCompleted(format!(
                            "Created alias '{}'. Restart terminal or run 'source ~/.bashrc' to use it",
                            name
                        )));
                        sender.input(AppMsg::LoadAliases);
                    }
                    Err(e) => {
                        sender.input(AppMsg::AliasOperationFailed(format!(
                            "Failed to create alias: {}",
                            e
                        )));
                    }
                });
            }

            AppMsg::DeleteAlias(name) => {
                let mut manager = self.alias_data.manager.clone();
                let sender = sender.clone();
                let name_clone = name.clone();

                std::thread::spawn(move || match manager.delete_alias(&name) {
                    Ok(_) => {
                        sender.input(AppMsg::AliasOperationCompleted(format!(
                            "Deleted alias '{}'",
                            name_clone
                        )));
                        sender.input(AppMsg::LoadAliases);
                    }
                    Err(e) => {
                        sender.input(AppMsg::AliasOperationFailed(format!(
                            "Failed to delete alias: {}",
                            e
                        )));
                    }
                });
            }

            AppMsg::AliasOperationCompleted(msg) => {
                sender.input(AppMsg::ShowToast(msg, ToastType::Success));
            }

            AppMsg::AliasOperationFailed(msg) => {
                sender.input(AppMsg::ShowToast(msg, ToastType::Error));
            }

            AppMsg::AliasSearchChanged(query) => {
                if let Some(source_id) = self.alias_search_debounce_source.borrow_mut().take() {
                    source_id.remove();
                }

                self.alias_data.search_query = query.clone();

                let sender_clone = sender.clone();
                let source_id =
                    glib::timeout_add_local_once(Duration::from_millis(200), move || {
                        sender_clone.input(AppMsg::DebouncedAliasSearchTrigger(query));
                    });
                *self.alias_search_debounce_source.borrow_mut() = Some(source_id);
            }

            AppMsg::DebouncedAliasSearchTrigger(_query) => {
                *self.pending_alias_rebuild.borrow_mut() = true;
            }

            AppMsg::ToggleShowExistingAliases => {
                self.alias_data.show_existing = !self.alias_data.show_existing;
                *self.pending_alias_rebuild.borrow_mut() = true;
            }

            AppMsg::FilterAliasesByShell(shell) => {
                self.alias_data.filter_shell = shell;
                *self.pending_alias_rebuild.borrow_mut() = true;
            }

            AppMsg::ScheduleTask(task) => {
                let package_name = task.package_name.clone();
                let scheduled_time = task.scheduled_at.format("%H:%M").to_string();
                let show_notif = self.config.borrow().show_notifications;

                let mut config = self.config.borrow_mut();
                config.scheduler.add_task(task);
                if let Err(e) = config.save() {
                    tracing::error!("Failed to save scheduled task: {}", e);
                }
                drop(config);

                if show_notif {
                    notifications::send_task_scheduled_notification(&package_name, &scheduled_time);
                }
            }

            AppMsg::ScheduleBulkTasks(tasks) => {
                let count = tasks.len();
                let show_notif = self.config.borrow().show_notifications;
                {
                    let mut config = self.config.borrow_mut();
                    for task in tasks {
                        config.scheduler.add_task(task);
                    }
                    if let Err(e) = config.save() {
                        tracing::error!("Failed to save bulk scheduled tasks: {}", e);
                    }
                }

                if show_notif {
                    let body = if count == 1 {
                        "1 package update scheduled".to_string()
                    } else {
                        format!("{} package updates scheduled", count)
                    };
                    notifications::send_system_notification(
                        "Tasks Scheduled",
                        &body,
                        Some("linget-bulk-scheduled"),
                    );
                }

                sender.input(AppMsg::ShowToast(
                    format!("Scheduled {} package updates", count),
                    ToastType::Success,
                ));
                sender.input(AppMsg::DeselectAll);
            }

            AppMsg::CheckScheduledTasks => {
                let config = self.config.borrow();
                let due_tasks: Vec<_> = config
                    .scheduler
                    .due_tasks()
                    .iter()
                    .map(|t| t.id.clone())
                    .collect();
                drop(config);

                for task_id in due_tasks {
                    sender.input(AppMsg::ExecuteScheduledTask(task_id));
                }
            }

            AppMsg::ExecuteScheduledTask(task_id) => {
                let task_opt = {
                    let config = self.config.borrow();
                    config
                        .scheduler
                        .tasks
                        .iter()
                        .find(|t| t.id == task_id)
                        .cloned()
                };

                if let Some(task) = task_opt {
                    if task.completed {
                        return;
                    }

                    self.tasks_data.running_task_id = Some(task_id.clone());
                    *self.pending_tasks_rebuild.borrow_mut() = true;

                    let pm = self.package_manager.clone();
                    let config = self.config.clone();
                    let sender = sender.clone();
                    let task_id_clone = task_id.clone();
                    let package_name = task.package_name.clone();

                    glib::spawn_future_local(async move {
                        let packages = {
                            let manager = pm.lock().await;
                            manager.list_all_installed().await.unwrap_or_default()
                        };

                        let pkg = packages.iter().find(|p| p.id() == task.package_id);

                        let result = if let Some(pkg) = pkg {
                            let manager = pm.lock().await;
                            match task.operation {
                                crate::models::ScheduledOperation::Update => {
                                    manager.update(pkg).await
                                }
                                crate::models::ScheduledOperation::Install => {
                                    manager.install(pkg).await
                                }
                                crate::models::ScheduledOperation::Remove => {
                                    manager.remove(pkg).await
                                }
                            }
                        } else {
                            Err(anyhow::anyhow!("Package not found"))
                        };

                        match result {
                            Ok(_) => {
                                let mut cfg = config.borrow_mut();
                                if let Some(t) = cfg
                                    .scheduler
                                    .tasks
                                    .iter_mut()
                                    .find(|t| t.id == task_id_clone)
                                {
                                    t.mark_completed();
                                }
                                cfg.scheduler.cleanup_old_tasks();
                                let _ = cfg.save();
                                drop(cfg);

                                sender.input(AppMsg::ScheduledTaskCompleted {
                                    task_id: task_id_clone,
                                    package_name,
                                });
                            }
                            Err(e) => {
                                let mut cfg = config.borrow_mut();
                                if let Some(t) = cfg
                                    .scheduler
                                    .tasks
                                    .iter_mut()
                                    .find(|t| t.id == task_id_clone)
                                {
                                    t.mark_failed(e.to_string());
                                }
                                let _ = cfg.save();
                                drop(cfg);

                                sender.input(AppMsg::ScheduledTaskFailed {
                                    task_id: task_id_clone,
                                    package_name,
                                    error: e.to_string(),
                                });
                            }
                        }
                    });
                }
            }

            AppMsg::ScheduledTaskCompleted { package_name, .. } => {
                self.tasks_data.scheduler = self.config.borrow().scheduler.clone();
                self.tasks_data.running_task_id = None;
                *self.pending_tasks_rebuild.borrow_mut() = true;

                if self.config.borrow().show_notifications {
                    notifications::send_task_completed_notification(&package_name);
                }

                sender.input(AppMsg::ShowToast(
                    format!("Scheduled update for {} completed", package_name),
                    ToastType::Success,
                ));
                sender.input(AppMsg::LoadPackages);
            }

            AppMsg::ScheduledTaskFailed {
                package_name,
                error,
                ..
            } => {
                self.tasks_data.scheduler = self.config.borrow().scheduler.clone();
                self.tasks_data.running_task_id = None;
                *self.pending_tasks_rebuild.borrow_mut() = true;

                if self.config.borrow().show_notifications {
                    notifications::send_task_failed_notification(&package_name, &error);
                }

                sender.input(AppMsg::ShowToast(
                    format!("Scheduled update for {} failed: {}", package_name, error),
                    ToastType::Error,
                ));
            }

            AppMsg::CancelScheduledTask(task_id) => {
                let mut config = self.config.borrow_mut();
                config.scheduler.remove_task(&task_id);
                if let Err(e) = config.save() {
                    tracing::error!("Failed to save after canceling task: {}", e);
                }
                self.tasks_data.scheduler = config.scheduler.clone();
                *self.pending_tasks_rebuild.borrow_mut() = true;
            }

            AppMsg::TaskQueueAction(action) => match action {
                TaskQueueAction::Cancel(task_id) => {
                    sender.input(AppMsg::CancelScheduledTask(task_id));
                }
                TaskQueueAction::Refresh => {
                    self.tasks_data.scheduler = self.config.borrow().scheduler.clone();
                    *self.pending_tasks_rebuild.borrow_mut() = true;
                }
                TaskQueueAction::RunNow(task_id) => {
                    sender.input(AppMsg::ExecuteScheduledTask(task_id));
                }
                TaskQueueAction::ClearCompleted => {
                    sender.input(AppMsg::ClearCompletedTasks);
                }
                TaskQueueAction::Retry(task_id) => {
                    let task_opt = {
                        let config = self.config.borrow();
                        config
                            .scheduler
                            .tasks
                            .iter()
                            .find(|t| t.id == task_id)
                            .cloned()
                    };

                    if let Some(task) = task_opt {
                        {
                            let mut config = self.config.borrow_mut();
                            if let Some(t) =
                                config.scheduler.tasks.iter_mut().find(|t| t.id == task_id)
                            {
                                t.completed = false;
                                t.error = None;
                                t.scheduled_at = chrono::Utc::now();
                            }
                            let _ = config.save();
                        }

                        self.tasks_data.scheduler = self.config.borrow().scheduler.clone();
                        *self.pending_tasks_rebuild.borrow_mut() = true;

                        sender.input(AppMsg::ExecuteScheduledTask(task.id));
                    }
                }
            },

            AppMsg::ScheduleAllUpdates => {
                use crate::models::{SchedulePreset, ScheduledOperation, ScheduledTask};

                let packages: Vec<Package> = self
                    .packages
                    .iter()
                    .filter(|p| p.has_update() && self.enabled_sources.contains(&p.source))
                    .cloned()
                    .collect();

                if packages.is_empty() {
                    sender.input(AppMsg::ShowToast(
                        "No updates available to schedule".to_string(),
                        ToastType::Info,
                    ));
                    return;
                }

                if let Some(scheduled_at) = SchedulePreset::Tonight.to_datetime() {
                    let tasks: Vec<ScheduledTask> = packages
                        .iter()
                        .map(|pkg| {
                            ScheduledTask::new(
                                pkg.id(),
                                pkg.name.clone(),
                                pkg.source,
                                ScheduledOperation::Update,
                                scheduled_at,
                            )
                        })
                        .collect();

                    sender.input(AppMsg::ScheduleBulkTasks(tasks));
                    sender.input(AppMsg::ViewChanged(View::Tasks));
                }
            }

            AppMsg::ClearCompletedTasks => {
                {
                    let mut config = self.config.borrow_mut();
                    config.scheduler.tasks.retain(|t| !t.completed);
                    if let Err(e) = config.save() {
                        tracing::error!("Failed to save after clearing tasks: {}", e);
                    }
                }
                self.tasks_data.scheduler = self.config.borrow().scheduler.clone();
                *self.pending_tasks_rebuild.borrow_mut() = true;
                sender.input(AppMsg::ShowToast(
                    "Cleared completed tasks".to_string(),
                    ToastType::Success,
                ));
            }

            AppMsg::DowngradePackage {
                package,
                target_version,
            } => {
                let pm = self.package_manager.clone();
                let tracker = self.history_tracker.clone();
                let name = package.name.clone();
                let sender = sender.clone();

                sender.input(AppMsg::OperationStarted {
                    package_name: name.clone(),
                    op: format!("Downgrading to {}", target_version),
                });

                let pkg_for_history = package.clone();
                let version_clone = target_version.clone();

                relm4::spawn(async move {
                    let result = {
                        let manager = pm.lock().await;
                        manager.downgrade_to(&package, &target_version).await
                    };

                    match result {
                        Ok(_) => {
                            {
                                let mut guard = tracker.lock().await;
                                if let Some(ref mut t) = *guard {
                                    t.record_downgrade(&pkg_for_history, &version_clone).await;
                                }
                            }

                            sender.input(AppMsg::OperationCompleted {
                                package_name: name.clone(),
                                op: format!("Downgraded to {}", version_clone),
                            });
                            sender.input(AppMsg::LoadPackages);
                        }
                        Err(e) => {
                            sender.input(AppMsg::OperationFailed {
                                package_name: name,
                                error: e.to_string(),
                            });
                        }
                    }
                });
            }

            AppMsg::CheckPendingNavigation => {
                if let Some(nav) = notifications::take_pending_nav() {
                    let view = match nav {
                        notifications::NotificationNavRequest::ViewTasks => View::Tasks,
                        notifications::NotificationNavRequest::ViewUpdates => View::Updates,
                    };
                    sender.input(AppMsg::ViewChanged(view));
                }
            }

            AppMsg::Shutdown => {
                let _ = self.shutdown_signal.send(true);
            }
        }
    }

    fn update_view(&self, widgets: &mut Self::Widgets, _sender: ComponentSender<Self>) {
        let start = Instant::now();

        widgets.offline_banner.set_visible(self.is_offline);

        widgets.view_title.set_title(&self.current_view.title());

        widgets
            .update_all_btn
            .set_visible(self.current_view == View::Updates && self.updates_count > 0);

        widgets
            .category_filter_btn
            .set_visible(self.current_view == View::Updates && self.updates_count > 0);

        let filtered_count = match self.layout_mode {
            LayoutMode::List => self.package_rows.len(),
            LayoutMode::Grid => self.package_cards.len(),
        };

        if self.current_view == View::Health {
            let health_data = self.health_data.clone();
            let sender_clone = _sender.clone();
            let health_content = build_health_dashboard(&health_data, move |action| {
                sender_clone.input(AppMsg::HealthAction(action));
            });
            widgets.health_clamp.set_child(Some(&health_content));
            widgets.content_stack.set_visible_child_name("health");
        } else if self.current_view == View::History {
            let history_data = self.history_data.clone();
            let sender_clone = _sender.clone();
            let history_content = build_history_view(&history_data, move |action| {
                sender_clone.input(AppMsg::HistoryAction(action));
            });
            widgets.history_clamp.set_child(Some(&history_content));
            widgets.content_stack.set_visible_child_name("history");
        } else if self.current_view == View::Tasks {
            if *self.pending_tasks_rebuild.borrow() {
                let tasks_data = self.tasks_data.clone();
                let sender_clone = _sender.clone();
                let tasks_content = build_task_queue_view(&tasks_data, move |action| {
                    sender_clone.input(AppMsg::TaskQueueAction(action));
                });
                widgets.tasks_clamp.set_child(Some(&tasks_content));
                *self.pending_tasks_rebuild.borrow_mut() = false;
            }
            widgets.content_stack.set_visible_child_name("tasks");
        } else if self.current_view == View::Aliases {
            if *self.pending_alias_rebuild.borrow() {
                let alias_data = self.alias_data.clone();
                let sender_clone = _sender.clone();
                crate::ui::alias_view::update_alias_view(
                    &widgets.alias_view,
                    &alias_data,
                    move |action| match action {
                        AliasViewAction::Refresh => sender_clone.input(AppMsg::LoadAliases),
                        AliasViewAction::Create {
                            name,
                            command,
                            shells,
                            description,
                        } => {
                            sender_clone.input(AppMsg::CreateAlias {
                                name,
                                command,
                                shells,
                                description,
                            });
                        }
                        AliasViewAction::Delete(name) => {
                            sender_clone.input(AppMsg::DeleteAlias(name))
                        }
                        AliasViewAction::SearchChanged(query) => {
                            sender_clone.input(AppMsg::AliasSearchChanged(query))
                        }
                        AliasViewAction::ToggleShowExisting => {
                            sender_clone.input(AppMsg::ToggleShowExistingAliases)
                        }
                        AliasViewAction::FilterByShell(shell) => {
                            sender_clone.input(AppMsg::FilterAliasesByShell(shell))
                        }
                        AliasViewAction::ExpandPackage { name, source } => {
                            sender_clone.input(AppMsg::ExpandPackage { name, source })
                        }
                        AliasViewAction::CopyCommand(path) => {
                            sender_clone.input(AppMsg::CopyToClipboard(path))
                        }
                    },
                );
                *self.pending_alias_rebuild.borrow_mut() = false;
            }
            widgets.content_stack.set_visible_child_name("aliases");
        } else if self.current_view == View::Storage {
            let stats = crate::ui::storage_view::StorageStats::compute(&self.packages, 20);
            let cleanup_stats = self.cleanup_stats.clone();
            let duplicates = crate::ui::storage_view::detect_duplicates(&self.packages);
            let sender_cleanup = _sender.clone();
            let sender_dup = _sender.clone();
            let storage_content = crate::ui::storage_view::build_storage_view(
                &stats,
                &cleanup_stats,
                &duplicates,
                move |action| {
                    sender_cleanup.input(AppMsg::ConfirmCleanup(action));
                },
                move |dup_action| {
                    sender_dup.input(AppMsg::DuplicateAction(dup_action));
                },
            );
            widgets.storage_clamp.set_child(Some(&storage_content));
            widgets.content_stack.set_visible_child_name("storage");
        } else if self.is_loading || self.discover_loading {
            widgets.content_stack.set_visible_child_name("skeleton");
        } else if self.load_error.is_some() {
            widgets.content_stack.set_visible_child_name("error");
        } else if filtered_count == 0 {
            let empty_page = match self.current_view {
                View::Home => {
                    if self.search_query.is_empty() {
                        "empty-discover"
                    } else {
                        "empty-library"
                    }
                }
                View::Updates => "empty-updates",
                View::Favorites => "empty-favorites",
                View::Library => "empty-library",
                View::Storage => "storage",
                View::Health => "health",
                View::History => "history",
                View::Aliases => "aliases",
                View::Tasks => "tasks",
                View::Collection(_) => "empty-library",
            };
            widgets.content_stack.set_visible_child_name(empty_page);
        } else {
            widgets.content_stack.set_visible_child_name("list");
        }

        widgets.sidebar.emit(SidebarInput::SetCounts {
            library: self.installed_count,
            updates: self.updates_count,
            favorites: self.favorites_count,
        });

        widgets.sidebar.emit(SidebarInput::UpdateAvailability {
            available: self.available_sources.clone(),
            enabled: self.enabled_sources.clone(),
        });

        widgets.sidebar.emit(SidebarInput::SetAllProviderCounts(
            self.provider_counts.clone(),
        ));

        widgets
            .sidebar
            .emit(SidebarInput::SetActiveFilter(self.source_filter));

        widgets.header.select_button.set_active(self.selection_mode);

        {
            let config = self.config.borrow();
            widgets
                .header
                .update_recent_searches(&config.recent_searches);
        }

        match self.layout_mode {
            LayoutMode::List => {
                widgets.list_grid_stack.set_visible_child_name("list");
                widgets.header.list_view_btn.set_active(true);
            }
            LayoutMode::Grid => {
                widgets.list_grid_stack.set_visible_child_name("grid");
                widgets.header.grid_view_btn.set_active(true);
            }
        }

        if self.selection_mode {
            widgets.selection_bar.emit(SelectionBarInput::Show);
            widgets
                .selection_bar
                .emit(SelectionBarInput::SetCount(self.selected_packages.len()));

            let mut has_updates = false;
            for pkg in &self.packages {
                if self.selected_packages.contains(&pkg.id()) && pkg.has_update() {
                    has_updates = true;
                    break;
                }
            }
            widgets
                .selection_bar
                .emit(SelectionBarInput::SetHasUpdates(has_updates));

            let selected_with_updates: Vec<(String, String, PackageSource)> = self
                .packages
                .iter()
                .filter(|p| self.selected_packages.contains(&p.id()) && p.has_update())
                .map(|p| (p.id(), p.name.clone(), p.source))
                .collect();
            widgets
                .selection_bar
                .emit(SelectionBarInput::SetSelectedPackages(
                    selected_with_updates,
                ));
        } else {
            widgets.selection_bar.emit(SelectionBarInput::Hide);
        }

        if self.bulk_op_total > 0 {
            widgets
                .progress_overlay
                .emit(ProgressOverlayInput::SetLabel(self.bulk_op_label.clone()));
            widgets
                .progress_overlay
                .emit(ProgressOverlayInput::SetStepProgress {
                    current: self.bulk_op_completed,
                    total: self.bulk_op_total,
                    item_name: self.bulk_op_current_item.clone(),
                });
            widgets.progress_overlay.emit(ProgressOverlayInput::Show);
        } else {
            widgets.progress_overlay.emit(ProgressOverlayInput::Hide);
            widgets.progress_overlay.emit(ProgressOverlayInput::Reset);
        }

        crate::ui::set_ui_marker(if self.details_visible {
            "AppSplitViewShowDetails"
        } else {
            "AppSplitViewHideDetails"
        });
        widgets.split_view.set_show_sidebar(self.details_visible);

        widgets
            .hero_banner
            .set_visible(self.current_view == View::Home && self.search_query.is_empty());

        let show_home_recs = self.current_view == View::Home
            && self.search_query.is_empty()
            && !self.home_recommendations.is_empty();
        widgets
            .home_recommendations_group
            .set_visible(show_home_recs);

        if *self.pending_home_recommendations_rebuild.borrow() {
            *self.pending_home_recommendations_rebuild.borrow_mut() = false;

            while let Some(child) = widgets.home_recommendations_box.first_child() {
                widgets.home_recommendations_box.remove(&child);
            }

            if !self.home_recommendations.is_empty() {
                for rec in &self.home_recommendations {
                    let subtitle = if rec.triggered_by.is_empty() {
                        rec.description.clone()
                    } else {
                        format!(
                            "{} • Based on: {}",
                            rec.description,
                            rec.triggered_by.join(", ")
                        )
                    };
                    let row = adw::ActionRow::builder()
                        .title(&rec.name)
                        .subtitle(&subtitle)
                        .build();

                    let cat_icon = gtk::Image::builder().icon_name(&rec.category_icon).build();
                    row.add_prefix(&cat_icon);

                    let button_box = gtk::Box::builder()
                        .orientation(gtk::Orientation::Horizontal)
                        .spacing(4)
                        .valign(gtk::Align::Center)
                        .build();

                    let install_btn = gtk::Button::builder()
                        .icon_name("system-search-symbolic")
                        .tooltip_text("Search & Install")
                        .css_classes(vec!["flat", "circular"])
                        .build();

                    let dismiss_btn = gtk::Button::builder()
                        .icon_name("window-close-symbolic")
                        .tooltip_text("Dismiss")
                        .css_classes(vec!["flat", "circular"])
                        .build();

                    let name_for_install = rec.name.clone();
                    let sender_install = _sender.clone();
                    install_btn.connect_clicked(move |_| {
                        sender_install
                            .input(AppMsg::InstallHomeRecommendation(name_for_install.clone()));
                    });

                    let name_for_dismiss = rec.name.clone();
                    let sender_dismiss = _sender.clone();
                    dismiss_btn.connect_clicked(move |_| {
                        sender_dismiss
                            .input(AppMsg::DismissHomeRecommendation(name_for_dismiss.clone()));
                    });

                    button_box.append(&install_btn);
                    button_box.append(&dismiss_btn);
                    row.add_suffix(&button_box);

                    widgets.home_recommendations_box.append(&row);
                }
            }
        }

        if let Some(ref pkg) = self.selected_package {
            if self.details_visible {
                let pkg_id = pkg.id();
                let last_id = self.last_shown_package_id.borrow().clone();
                if last_id.as_ref() != Some(&pkg_id) {
                    crate::ui::set_ui_marker(format!("DetailsShowPackage [{}]", pkg_id));
                    tracing::debug!(pkg_id = %pkg_id, "Emitting details panel ShowPackage");

                    *self.last_shown_package_id.borrow_mut() = Some(pkg_id);
                    widgets
                        .details_panel
                        .emit(DetailsPanelInput::ShowPackage(Box::new(pkg.clone())));
                }
            }
        } else if self.last_shown_package_id.borrow().is_some() {
            *self.last_shown_package_id.borrow_mut() = None;
            widgets.details_panel.emit(DetailsPanelInput::Clear);
        }

        for event in self.pending_task_events.borrow_mut().drain(..) {
            widgets.task_hub.emit(event);
        }

        for (msg, toast_type) in self.pending_toasts.borrow_mut().drain(..) {
            let toast = adw::Toast::builder().title(&msg).timeout(3).build();

            let icon_name = match toast_type {
                ToastType::Success => Some("emblem-ok-symbolic"),
                ToastType::Error => Some("dialog-error-symbolic"),
                ToastType::Warning => Some("dialog-warning-symbolic"),
                ToastType::Info => None,
            };

            if let Some(icon) = icon_name {
                let hbox = gtk::Box::new(gtk::Orientation::Horizontal, 8);
                let icon_widget = gtk::Image::from_icon_name(icon);
                icon_widget.add_css_class(match toast_type {
                    ToastType::Success => "success",
                    ToastType::Error => "error",
                    ToastType::Warning => "warning",
                    ToastType::Info => "dim-label",
                });
                let label = gtk::Label::new(Some(&msg));
                hbox.append(&icon_widget);
                hbox.append(&label);
                toast.set_custom_title(Some(&hbox));
            }

            widgets.toast_overlay.add_toast(toast);
        }

        let has_activity = self.unread_count > 0 || self.bulk_op_total > 0;
        if has_activity {
            widgets.task_hub_btn.add_css_class("has-activity");
        } else {
            widgets.task_hub_btn.remove_css_class("has-activity");
        }

        if self.unread_count > 0 {
            widgets
                .task_hub_badge
                .set_label(&self.unread_count.to_string());
            widgets.task_hub_badge.set_visible(true);
        } else {
            widgets.task_hub_badge.set_visible(false);
        }

        widgets.task_hub_spinner.set_visible(self.bulk_op_total > 0);

        if *self.pending_focus_search.borrow() {
            *self.pending_focus_search.borrow_mut() = false;
            widgets.header.search_entry.grab_focus();
        }

        let visible_count = match self.layout_mode {
            LayoutMode::List => self.package_rows.len(),
            LayoutMode::Grid => self.package_cards.len(),
        };
        let has_more = self.total_filtered_count > visible_count;
        widgets.load_more_btn.set_visible(has_more);
        widgets
            .load_more_label
            .set_visible(has_more || visible_count > 0);

        if has_more {
            widgets.load_more_label.set_label(&format!(
                "Showing {} of {} packages",
                visible_count, self.total_filtered_count
            ));
        } else if visible_count > 0 {
            widgets
                .load_more_label
                .set_label(&format!("Showing all {} packages", visible_count));
        }

        if *self.pending_show_collection_dialog.borrow() {
            *self.pending_show_collection_dialog.borrow_mut() = false;
            widgets.collection_dialog.emit(CollectionDialogInput::Show);
            let dialog_widgets = widgets.collection_dialog.widgets();
            widgets.collection_dialog.model().present(
                &dialog_widgets,
                widgets.split_view.upcast_ref::<gtk::Widget>(),
            );
        }

        if let Some(action) = self.pending_cleanup_confirm.borrow_mut().take() {
            let (heading, body, items) = match &action {
                CleanupAction::CleanAll => {
                    let total_str = self.cleanup_stats.total_recoverable_display();
                    let mut all_items: Vec<String> = Vec::new();

                    for (source, packages) in &self.cleanup_stats.orphaned_packages {
                        for pkg in packages {
                            let size_str = pkg
                                .size
                                .map(|s| humansize::format_size(s, humansize::BINARY))
                                .unwrap_or_default();
                            if size_str.is_empty() {
                                all_items.push(format!("• {} ({})", pkg.name, source));
                            } else {
                                all_items
                                    .push(format!("• {} ({}) - {}", pkg.name, source, size_str));
                            }
                        }
                    }

                    (
                        "Clean All Caches?".to_string(),
                        format!(
                            "This will free approximately {}.\n\nThe following items will be removed:",
                            total_str
                        ),
                        all_items,
                    )
                }
                CleanupAction::CleanSource(source) => {
                    let size = self
                        .cleanup_stats
                        .cache_sizes
                        .get(source)
                        .copied()
                        .unwrap_or(0);
                    let size_str = humansize::format_size(size, humansize::BINARY);

                    let packages = self
                        .cleanup_stats
                        .orphaned_packages
                        .get(source)
                        .cloned()
                        .unwrap_or_default();

                    let items: Vec<String> = packages
                        .iter()
                        .map(|pkg| {
                            let pkg_size = pkg
                                .size
                                .map(|s| {
                                    format!(" ({})", humansize::format_size(s, humansize::BINARY))
                                })
                                .unwrap_or_default();
                            format!("• {}{}", pkg.description, pkg_size)
                        })
                        .collect();

                    let intro = match *source {
                        PackageSource::Apt => format!(
                            "This will run 'apt clean' to remove downloaded package files.\n\n\
                             This will free approximately {}.",
                            size_str
                        ),
                        PackageSource::Flatpak => format!(
                            "This will free approximately {}.\n\nThe following unused runtimes will be removed:",
                            size_str
                        ),
                        PackageSource::Snap => format!(
                            "This will free approximately {}.\n\nThe following old revisions will be removed:",
                            size_str
                        ),
                        _ => format!(
                            "This will free approximately {}.\n\nThe following items will be removed:",
                            size_str
                        ),
                    };

                    (format!("Clean {} Cache?", source), intro, items)
                }
                CleanupAction::RemoveOrphans(source) => {
                    let packages = self
                        .cleanup_stats
                        .orphaned_packages
                        .get(source)
                        .cloned()
                        .unwrap_or_default();

                    let items: Vec<String> = packages
                        .iter()
                        .map(|pkg| format!("• {}", pkg.name))
                        .collect();

                    (
                        format!("Remove {} Orphaned Packages?", source),
                        "The following packages will be removed:".to_string(),
                        items,
                    )
                }
                CleanupAction::Refresh => (String::new(), String::new(), Vec::new()),
            };

            if !heading.is_empty() {
                let full_body = if items.is_empty() {
                    format!("{}\n\nThis action cannot be undone.", body)
                } else {
                    let items_list = items.into_iter().take(15).collect::<Vec<_>>().join("\n");
                    let truncated_note = if self.cleanup_stats.total_orphaned > 15 {
                        format!("\n... and {} more", self.cleanup_stats.total_orphaned - 15)
                    } else {
                        String::new()
                    };
                    format!(
                        "{}\n\n{}{}\n\nThis action cannot be undone.",
                        body, items_list, truncated_note
                    )
                };

                let dialog = adw::MessageDialog::builder()
                    .heading(&heading)
                    .body(&full_body)
                    .build();

                dialog.add_response("cancel", "Cancel");
                dialog.add_response("clean", "Clean");
                dialog.set_default_response(Some("cancel"));
                dialog.set_close_response("cancel");
                dialog.set_response_appearance("clean", adw::ResponseAppearance::Destructive);

                if let Some(window) = widgets
                    .split_view
                    .root()
                    .and_then(|r| r.downcast::<adw::ApplicationWindow>().ok())
                {
                    dialog.set_transient_for(Some(&window));
                }

                let sender = _sender.clone();
                dialog.connect_response(None, move |_, response| {
                    if response == "clean" {
                        sender.input(AppMsg::ExecuteCleanup(action.clone()));
                    }
                });

                dialog.present();
            }
        }

        if let Some(preview) = self.pending_action_preview.borrow_mut().take() {
            let packages = preview.packages.clone();
            let sender_confirm = _sender.clone();
            let sender_cancel = _sender.clone();

            preview.show_dialog(
                &widgets.split_view,
                move || {
                    for pkg in &packages {
                        sender_confirm.input(AppMsg::ExecutePackageAction(pkg.clone()));
                    }
                },
                move || {
                    sender_cancel.input(AppMsg::ShowToast(
                        "Action cancelled".to_string(),
                        ToastType::Info,
                    ));
                },
            );
        }

        if let Some(collections) = self.pending_sidebar_collections.borrow_mut().take() {
            widgets
                .sidebar
                .emit(SidebarInput::UpdateCollections(collections));
        }

        if *self.pending_show_onboarding.borrow() {
            *self.pending_show_onboarding.borrow_mut() = false;

            let config = self.config.clone();
            let dialog = adw::MessageDialog::builder()
                .heading("Welcome to LinGet!")
                .body(
                    "Manage all your packages from APT, Flatpak, Snap, npm, pip, and more \
                     in one unified interface.\n\n\
                     • Use the sidebar to navigate between views\n\
                     • Enable or disable package sources in the sidebar\n\
                     • Press Ctrl+F to search, Ctrl+R to refresh",
                )
                .build();
            dialog.add_response("continue", "Get Started");
            dialog.set_default_response(Some("continue"));
            dialog.set_response_appearance("continue", adw::ResponseAppearance::Suggested);
            if let Some(window) = widgets
                .split_view
                .root()
                .and_then(|r| r.downcast::<adw::ApplicationWindow>().ok())
            {
                dialog.set_transient_for(Some(&window));
            }

            dialog.connect_response(None, move |_, _| {
                let mut cfg = config.borrow_mut();
                cfg.onboarding_completed = true;
                let _ = cfg.save();
            });

            dialog.present();
        }

        if *self.pending_command_palette.borrow() {
            *self.pending_command_palette.borrow_mut() = false;

            let palette = crate::ui::command_palette::CommandPalette::new(&widgets.split_view);
            let sender = _sender.clone();
            palette.connect_command(move |cmd| {
                sender.input(AppMsg::ExecutePaletteCommand(cmd));
            });
            palette.present();
        }

        let vim_mode = self.config.borrow().vim_mode;
        match self.layout_mode {
            LayoutMode::List => {
                let list_box = self.package_rows.widget();
                let adj = widgets.package_list_scrolled.vadjustment();
                let mut idx = 0;
                let mut child = list_box.first_child();
                while let Some(row) = child {
                    if vim_mode && idx == self.focused_index {
                        row.add_css_class("keyboard-focused");
                        if let Some((_, row_y)) = row.translate_coordinates(list_box, 0.0, 0.0) {
                            let row_height = row.height() as f64;
                            let view_height = widgets.package_list_scrolled.height() as f64;
                            let current_pos = adj.value();
                            if row_y < current_pos {
                                adj.set_value(row_y);
                            } else if row_y + row_height > current_pos + view_height {
                                adj.set_value(row_y + row_height - view_height);
                            }
                        }
                    } else {
                        row.remove_css_class("keyboard-focused");
                    }
                    idx += 1;
                    child = row.next_sibling();
                }
            }
            LayoutMode::Grid => {
                let flow_box = self.package_cards.widget();
                let adj = widgets.package_list_scrolled.vadjustment();
                let mut idx = 0;
                let mut child = flow_box.first_child();
                while let Some(wrapper) = child {
                    if let Some(card) = wrapper.first_child() {
                        if vim_mode && idx == self.focused_index {
                            card.add_css_class("keyboard-focused");
                            if let Some((_, card_y)) =
                                wrapper.translate_coordinates(flow_box, 0.0, 0.0)
                            {
                                let card_height = wrapper.height() as f64;
                                let view_height = widgets.package_list_scrolled.height() as f64;
                                let current_pos = adj.value();
                                if card_y < current_pos {
                                    adj.set_value(card_y);
                                } else if card_y + card_height > current_pos + view_height {
                                    adj.set_value(card_y + card_height - view_height);
                                }
                            }
                        } else {
                            card.remove_css_class("keyboard-focused");
                        }
                    }
                    idx += 1;
                    child = wrapper.next_sibling();
                }
            }
        }

        let elapsed = start.elapsed();
        if elapsed > Duration::from_millis(50) {
            tracing::warn!(
                elapsed_ms = elapsed.as_millis() as u64,
                view = ?self.current_view,
                "App update_view slow"
            );
        }
    }
}

pub fn run_relm4_app() {
    let app = adw::Application::builder()
        .application_id("io.github.linget")
        .build();

    app.connect_startup(|_| {
        crate::app::init_startup();
    });

    app.connect_activate(|app| {
        crate::ui::start_ui_watchdog();
        glib::set_application_name("LinGet");
        app.set_accels_for_action("win.close", &["<Ctrl>w"]);
        app.set_accels_for_action("app.preferences", &["<Ctrl>comma"]);
        app.set_accels_for_action("app.quit", &["<Ctrl>q"]);

        let about_action = gio::SimpleAction::new("about", None);
        about_action.connect_activate({
            let app = app.clone();
            move |_, _| {
                let window = app.active_window();
                let about = adw::AboutWindow::builder()
                    .application_name("LinGet")
                    .application_icon("io.github.linget")
                    .developer_name("Eslam Sabry")
                    .version(env!("CARGO_PKG_VERSION"))
                    .website("https://github.com/Eslamasabry/LinGet")
                    .issue_url("https://github.com/Eslamasabry/LinGet/issues")
                    .license_type(gtk::License::Gpl30)
                    .comments("A modern, unified package manager for Linux")
                    .developers(vec!["Eslam Sabry"])
                    .build();
                about.set_transient_for(window.as_ref());
                about.present();
            }
        });
        app.add_action(&about_action);

        let shortcuts_action = gio::SimpleAction::new("shortcuts", None);
        shortcuts_action.connect_activate({
            let app = app.clone();
            move |_, _| {
                let window = app.active_window();
                let dialog = adw::MessageDialog::builder()
                    .heading("Keyboard Shortcuts")
                    .body(
                        "Ctrl+F  Search packages\n\
                         Ctrl+R  Refresh\n\
                         Ctrl+S  Selection mode\n\
                         Ctrl+,  Open preferences\n\
                         Ctrl+Q  Quit\n\
                         Ctrl+W  Close window\n\
                         Escape  Cancel / close panel",
                    )
                    .build();
                dialog.add_response("close", "Close");
                dialog.set_default_response(Some("close"));
                dialog.set_transient_for(window.as_ref());
                dialog.present();
            }
        });
        app.add_action(&shortcuts_action);

        let prefs_action = gio::SimpleAction::new("preferences", None);
        prefs_action.connect_activate({
            let app = app.clone();
            move |_, _| {
                if let Some(window) = app.active_window() {
                    let config = Rc::new(RefCell::new(Config::load()));
                    let window_clone = window.clone();
                    let config_clone = config.clone();
                    let prefs_window = crate::ui::build_preferences_window(
                        &window,
                        config,
                        move |scheme, accent| {
                            crate::ui::apply_theme_settings(&window_clone, scheme, accent);
                            apply_appearance(&config_clone.borrow().appearance);
                        },
                    );
                    prefs_window.present();
                }
            }
        });
        app.add_action(&prefs_action);

        let diag_action = gio::SimpleAction::new("diagnostics", None);
        diag_action.connect_activate({
            let app = app.clone();
            move |_, _| {
                if let Some(window) = app.active_window() {
                    if let Some(toast_overlay) = window
                        .first_child()
                        .and_then(|c| c.first_child())
                        .and_then(|c| c.downcast::<adw::ToastOverlay>().ok())
                    {
                        toast_overlay.add_toast(adw::Toast::new(
                            "Run 'linget providers' in terminal for diagnostics",
                        ));
                    }
                }
            }
        });
        app.add_action(&diag_action);

        let import_action = gio::SimpleAction::new("import", None);
        import_action.connect_activate({
            let app = app.clone();
            move |_, _| {
                if let Some(window) = app.active_window() {
                    let manager = PackageManager::new();
                    let pm = Arc::new(Mutex::new(manager));
                    crate::ui::show_import_dialog(&window, pm);
                }
            }
        });
        app.add_action(&import_action);

        let export_action = gio::SimpleAction::new("export", None);
        export_action.connect_activate({
            let app = app.clone();
            move |_, _| {
                if let Some(window) = app.active_window() {
                    let manager = PackageManager::new();
                    let pm = Arc::new(Mutex::new(manager));
                    crate::ui::show_export_dialog(&window, pm);
                }
            }
        });
        app.add_action(&export_action);

        let quit_action = gio::SimpleAction::new("quit", None);
        quit_action.connect_activate({
            let app = app.clone();
            move |_, _| {
                app.quit();
            }
        });
        app.add_action(&quit_action);

        let view_tasks_action = gio::SimpleAction::new("view-tasks", None);
        view_tasks_action.connect_activate(|_, _| {
            notifications::set_pending_nav(notifications::NotificationNavRequest::ViewTasks);
        });
        app.add_action(&view_tasks_action);

        let view_updates_action = gio::SimpleAction::new("view-updates", None);
        view_updates_action.connect_activate(|_, _| {
            notifications::set_pending_nav(notifications::NotificationNavRequest::ViewUpdates);
        });
        app.add_action(&view_updates_action);
    });

    let app = RelmApp::from_app(app);
    app.run::<AppModel>(());
}
