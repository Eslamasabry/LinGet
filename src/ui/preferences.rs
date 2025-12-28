use crate::models::{
    AccentColor, AppearanceConfig, BorderRadius, BorderStyle, CardSize, ColorScheme, Config,
    FontScale, GlowIntensity, GridColumns, LayoutMode, ListDensity, SidebarWidth, SpacingLevel,
    TransitionSpeed,
};
use crate::ui::appearance::apply_appearance;

use gtk4::prelude::*;
use gtk4::{self as gtk};
use libadwaita as adw;
use libadwaita::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

thread_local! {
    static ACCENT_PROVIDER: RefCell<Option<gtk::CssProvider>> = const { RefCell::new(None) };
}

pub fn apply_theme_settings(
    window: &impl IsA<gtk::Widget>,
    scheme: ColorScheme,
    accent: AccentColor,
) {
    let style_manager = adw::StyleManager::default();

    match scheme {
        ColorScheme::System => {
            style_manager.set_color_scheme(adw::ColorScheme::Default);
            window.remove_css_class("oled-dark");
        }
        ColorScheme::Light => {
            style_manager.set_color_scheme(adw::ColorScheme::ForceLight);
            window.remove_css_class("oled-dark");
        }
        ColorScheme::Dark => {
            style_manager.set_color_scheme(adw::ColorScheme::ForceDark);
            window.remove_css_class("oled-dark");
        }
        ColorScheme::OledDark => {
            style_manager.set_color_scheme(adw::ColorScheme::ForceDark);
            window.add_css_class("oled-dark");
        }
    }

    if let Some(display) = gtk::gdk::Display::default() {
        ACCENT_PROVIDER.with(|cell| {
            if let Some(old_provider) = cell.borrow_mut().take() {
                gtk::style_context_remove_provider_for_display(&display, &old_provider);
            }
        });

        if let Some(css_color) = accent.css_color() {
            let provider = gtk::CssProvider::new();
            provider.load_from_data(&format!(
                r#"
                @define-color accent_bg_color {};
                @define-color accent_color {};
                @define-color accent_fg_color white;
                "#,
                css_color, css_color
            ));

            gtk::style_context_add_provider_for_display(
                &display,
                &provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION + 1,
            );

            ACCENT_PROVIDER.with(|cell| {
                *cell.borrow_mut() = Some(provider);
            });
        }
    }
}

pub fn build_preferences_window<F>(
    parent: &impl IsA<gtk::Window>,
    config: Rc<RefCell<Config>>,
    on_theme_changed: F,
) -> adw::PreferencesWindow
where
    F: Fn(ColorScheme, AccentColor) + Clone + 'static,
{
    let window = adw::PreferencesWindow::builder()
        .title("Preferences")
        .transient_for(parent)
        .modal(true)
        .default_width(600)
        .default_height(500)
        .build();

    let general_page = build_general_page(config.clone());
    let appearance_page = build_appearance_page(config.clone(), on_theme_changed);
    let behavior_page = build_behavior_page(config.clone());

    window.add(&general_page);
    window.add(&appearance_page);
    window.add(&behavior_page);

    window
}

fn build_general_page(config: Rc<RefCell<Config>>) -> adw::PreferencesPage {
    let page = adw::PreferencesPage::builder()
        .title("General")
        .icon_name("preferences-system-symbolic")
        .build();

    let updates_group = adw::PreferencesGroup::builder()
        .title("Updates")
        .description("Configure how LinGet checks for package updates")
        .build();

    let check_updates_row = adw::SwitchRow::builder()
        .title("Check for updates on startup")
        .subtitle("Automatically check for package updates when LinGet starts")
        .build();
    check_updates_row.set_active(config.borrow().check_updates_on_startup);
    {
        let config = config.clone();
        check_updates_row.connect_active_notify(move |row| {
            config.borrow_mut().check_updates_on_startup = row.is_active();
            let _ = config.borrow().save();
        });
    }
    updates_group.add(&check_updates_row);

    let interval_row = adw::SpinRow::builder()
        .title("Update check interval")
        .subtitle("Hours between automatic update checks (0 to disable)")
        .adjustment(
            &gtk::Adjustment::builder()
                .lower(0.0)
                .upper(168.0)
                .step_increment(1.0)
                .page_increment(6.0)
                .build(),
        )
        .build();
    interval_row.set_value(config.borrow().update_check_interval as f64);
    {
        let config = config.clone();
        interval_row.connect_value_notify(move |row| {
            config.borrow_mut().update_check_interval = row.value() as u32;
            let _ = config.borrow().save();
        });
    }
    updates_group.add(&interval_row);

    let notifications_group = adw::PreferencesGroup::builder()
        .title("Notifications")
        .build();

    let notifications_row = adw::SwitchRow::builder()
        .title("Show notifications")
        .subtitle("Display desktop notifications for updates and completed operations")
        .build();
    notifications_row.set_active(config.borrow().show_notifications);
    {
        let config = config.clone();
        notifications_row.connect_active_notify(move |row| {
            config.borrow_mut().show_notifications = row.is_active();
            let _ = config.borrow().save();
        });
    }
    notifications_group.add(&notifications_row);

    page.add(&updates_group);
    page.add(&notifications_group);

    page
}

fn build_appearance_page<F>(
    config: Rc<RefCell<Config>>,
    on_theme_changed: F,
) -> adw::PreferencesPage
where
    F: Fn(ColorScheme, AccentColor) + Clone + 'static,
{
    let page = adw::PreferencesPage::builder()
        .title("Appearance")
        .icon_name("applications-graphics-symbolic")
        .build();

    let theme_group = adw::PreferencesGroup::builder()
        .title("Theme")
        .description("Customize the application appearance")
        .build();

    let color_scheme_row = adw::ComboRow::builder()
        .title("Color scheme")
        .subtitle("Choose light, dark, or follow system preference")
        .build();

    let scheme_names: Vec<&str> = ColorScheme::all()
        .iter()
        .map(|s| s.display_name())
        .collect();
    let scheme_model = gtk::StringList::new(&scheme_names);
    color_scheme_row.set_model(Some(&scheme_model));
    let current_scheme = config.borrow().color_scheme;
    color_scheme_row.set_selected(
        ColorScheme::all()
            .iter()
            .position(|s| *s == current_scheme)
            .unwrap_or(0) as u32,
    );
    {
        let config = config.clone();
        let on_theme_changed = on_theme_changed.clone();
        color_scheme_row.connect_selected_notify(move |row| {
            let scheme = ColorScheme::all()
                .get(row.selected() as usize)
                .copied()
                .unwrap_or_default();
            config.borrow_mut().color_scheme = scheme;
            let _ = config.borrow().save();
            on_theme_changed(scheme, config.borrow().accent_color);
        });
    }
    theme_group.add(&color_scheme_row);

    let accent_row = adw::ActionRow::builder()
        .title("Accent color")
        .subtitle("Choose the highlight color for buttons and links")
        .build();

    let accent_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(8)
        .valign(gtk::Align::Center)
        .build();

    let current_accent = config.borrow().accent_color;
    let mut group_leader: Option<gtk::ToggleButton> = None;

    for accent in AccentColor::all() {
        let btn = gtk::ToggleButton::builder()
            .width_request(28)
            .height_request(28)
            .tooltip_text(accent.display_name())
            .build();
        btn.add_css_class("circular");
        btn.add_css_class("accent-swatch");

        if let Some(leader) = &group_leader {
            btn.set_group(Some(leader));
        } else {
            group_leader = Some(btn.clone());
        }

        if *accent == AccentColor::System {
            btn.add_css_class("accent-system");
            btn.set_child(Some(
                &gtk::Image::builder()
                    .icon_name("emblem-system-symbolic")
                    .pixel_size(14)
                    .build(),
            ));
        } else if let Some(color) = accent.css_color() {
            let css_class = format!("accent-{}", accent.display_name().to_lowercase());
            btn.add_css_class(&css_class);
            let provider = gtk::CssProvider::new();
            provider.load_from_data(&format!(
                ".accent-swatch.{} {{ background-color: {}; }}",
                css_class, color
            ));
            btn.style_context().add_provider(&provider, 800);
        }

        btn.set_active(*accent == current_accent);

        let config = config.clone();
        let accent = *accent;
        let on_theme_changed = on_theme_changed.clone();
        btn.connect_toggled(move |btn| {
            if btn.is_active() {
                config.borrow_mut().accent_color = accent;
                let _ = config.borrow().save();
                on_theme_changed(config.borrow().color_scheme, accent);
            }
        });

        accent_box.append(&btn);
    }

    accent_row.add_suffix(&accent_box);
    theme_group.add(&accent_row);

    page.add(&theme_group);

    let layout_group = adw::PreferencesGroup::builder()
        .title("Layout")
        .description("Customize how packages are displayed")
        .build();

    let layout_row = adw::ComboRow::builder()
        .title("Package layout")
        .subtitle("Choose between list view or grid view")
        .build();

    let layout_model = gtk::StringList::new(&["List", "Grid"]);
    layout_row.set_model(Some(&layout_model));
    layout_row.set_selected(match config.borrow().layout_mode {
        LayoutMode::List => 0,
        LayoutMode::Grid => 1,
    });
    {
        let config = config.clone();
        layout_row.connect_selected_notify(move |row| {
            config.borrow_mut().layout_mode = match row.selected() {
                0 => LayoutMode::List,
                _ => LayoutMode::Grid,
            };
            let _ = config.borrow().save();
        });
    }
    layout_group.add(&layout_row);

    let compact_row = adw::SwitchRow::builder()
        .title("Compact mode")
        .subtitle("Use smaller package cards and rows")
        .build();
    compact_row.set_active(config.borrow().ui_compact);
    {
        let config = config.clone();
        compact_row.connect_active_notify(move |row| {
            config.borrow_mut().ui_compact = row.is_active();
            let _ = config.borrow().save();
        });
    }
    layout_group.add(&compact_row);

    let icons_row = adw::SwitchRow::builder()
        .title("Show package icons")
        .subtitle("Display application icons in the package list")
        .build();
    icons_row.set_active(config.borrow().ui_show_icons);
    {
        let config = config.clone();
        icons_row.connect_active_notify(move |row| {
            config.borrow_mut().ui_show_icons = row.is_active();
            let _ = config.borrow().save();
        });
    }
    layout_group.add(&icons_row);

    let grid_cols_row = adw::ComboRow::builder()
        .title("Grid columns")
        .subtitle("Number of columns in grid view")
        .build();
    let grid_cols_names: Vec<&str> = GridColumns::all()
        .iter()
        .map(|g| g.display_name())
        .collect();
    let grid_cols_model = gtk::StringList::new(&grid_cols_names);
    grid_cols_row.set_model(Some(&grid_cols_model));
    let current_cols = config.borrow().appearance.grid_columns;
    grid_cols_row.set_selected(
        GridColumns::all()
            .iter()
            .position(|g| *g == current_cols)
            .unwrap_or(2) as u32,
    );
    {
        let config = config.clone();
        grid_cols_row.connect_selected_notify(move |row| {
            let cols = GridColumns::all()
                .get(row.selected() as usize)
                .copied()
                .unwrap_or_default();
            config.borrow_mut().appearance.grid_columns = cols;
            let _ = config.borrow().save();
        });
    }
    layout_group.add(&grid_cols_row);

    let card_size_row = adw::ComboRow::builder()
        .title("Card size")
        .subtitle("Size of package cards in grid view")
        .build();
    let card_size_names: Vec<&str> = CardSize::all().iter().map(|c| c.display_name()).collect();
    let card_size_model = gtk::StringList::new(&card_size_names);
    card_size_row.set_model(Some(&card_size_model));
    let current_card = config.borrow().appearance.card_size;
    card_size_row.set_selected(
        CardSize::all()
            .iter()
            .position(|c| *c == current_card)
            .unwrap_or(1) as u32,
    );
    {
        let config = config.clone();
        card_size_row.connect_selected_notify(move |row| {
            let size = CardSize::all()
                .get(row.selected() as usize)
                .copied()
                .unwrap_or_default();
            config.borrow_mut().appearance.card_size = size;
            let _ = config.borrow().save();
        });
    }
    layout_group.add(&card_size_row);

    let list_density_row = adw::ComboRow::builder()
        .title("List density")
        .subtitle("Row height and spacing in list view")
        .build();
    let list_density_names: Vec<&str> = ListDensity::all()
        .iter()
        .map(|l| l.display_name())
        .collect();
    let list_density_model = gtk::StringList::new(&list_density_names);
    list_density_row.set_model(Some(&list_density_model));
    let current_density = config.borrow().appearance.list_density;
    list_density_row.set_selected(
        ListDensity::all()
            .iter()
            .position(|l| *l == current_density)
            .unwrap_or(1) as u32,
    );
    {
        let config = config.clone();
        list_density_row.connect_selected_notify(move |row| {
            let density = ListDensity::all()
                .get(row.selected() as usize)
                .copied()
                .unwrap_or_default();
            config.borrow_mut().appearance.list_density = density;
            let _ = config.borrow().save();
        });
    }
    layout_group.add(&list_density_row);

    let spacing_row = adw::ComboRow::builder()
        .title("Content spacing")
        .subtitle("Space between elements")
        .build();
    let spacing_names: Vec<&str> = SpacingLevel::all()
        .iter()
        .map(|s| s.display_name())
        .collect();
    let spacing_model = gtk::StringList::new(&spacing_names);
    spacing_row.set_model(Some(&spacing_model));
    let current_spacing = config.borrow().appearance.spacing;
    spacing_row.set_selected(
        SpacingLevel::all()
            .iter()
            .position(|s| *s == current_spacing)
            .unwrap_or(1) as u32,
    );
    {
        let config = config.clone();
        spacing_row.connect_selected_notify(move |row| {
            let spacing = SpacingLevel::all()
                .get(row.selected() as usize)
                .copied()
                .unwrap_or_default();
            config.borrow_mut().appearance.spacing = spacing;
            let _ = config.borrow().save();
        });
    }
    layout_group.add(&spacing_row);

    let sidebar_row = adw::ComboRow::builder()
        .title("Sidebar width")
        .subtitle("Width of the navigation sidebar")
        .build();
    let sidebar_names: Vec<&str> = SidebarWidth::all()
        .iter()
        .map(|s| s.display_name())
        .collect();
    let sidebar_model = gtk::StringList::new(&sidebar_names);
    sidebar_row.set_model(Some(&sidebar_model));
    let current_sidebar = config.borrow().appearance.sidebar_width;
    sidebar_row.set_selected(
        SidebarWidth::all()
            .iter()
            .position(|s| *s == current_sidebar)
            .unwrap_or(1) as u32,
    );
    {
        let config = config.clone();
        sidebar_row.connect_selected_notify(move |row| {
            let width = SidebarWidth::all()
                .get(row.selected() as usize)
                .copied()
                .unwrap_or_default();
            config.borrow_mut().appearance.sidebar_width = width;
            let _ = config.borrow().save();
        });
    }
    layout_group.add(&sidebar_row);

    page.add(&layout_group);

    // =========================================================================
    // BORDERS GROUP
    // =========================================================================
    let borders_group = adw::PreferencesGroup::builder()
        .title("Borders")
        .description("Customize border appearance (OLED Dark theme)")
        .build();

    let border_style_row = adw::ComboRow::builder()
        .title("Border style")
        .subtitle("Thickness of accent-colored borders")
        .build();
    let border_style_names: Vec<&str> = BorderStyle::all()
        .iter()
        .map(|s| s.display_name())
        .collect();
    let border_style_model = gtk::StringList::new(&border_style_names);
    border_style_row.set_model(Some(&border_style_model));
    let current_border_style = config.borrow().appearance.border_style;
    border_style_row.set_selected(
        BorderStyle::all()
            .iter()
            .position(|s| *s == current_border_style)
            .unwrap_or(2) as u32,
    );
    {
        let config = config.clone();
        let on_theme_changed = on_theme_changed.clone();
        border_style_row.connect_selected_notify(move |row| {
            let style = BorderStyle::all()
                .get(row.selected() as usize)
                .copied()
                .unwrap_or_default();
            config.borrow_mut().appearance.border_style = style;
            let _ = config.borrow().save();
            apply_appearance(&config.borrow().appearance);
            on_theme_changed(config.borrow().color_scheme, config.borrow().accent_color);
        });
    }
    borders_group.add(&border_style_row);

    let opacity_row = adw::SpinRow::builder()
        .title("Border opacity")
        .subtitle("Visibility of accent-colored borders (0-100%)")
        .adjustment(
            &gtk::Adjustment::builder()
                .lower(0.0)
                .upper(100.0)
                .step_increment(5.0)
                .page_increment(10.0)
                .build(),
        )
        .build();
    opacity_row.set_value(config.borrow().appearance.border_opacity as f64);
    {
        let config = config.clone();
        let on_theme_changed = on_theme_changed.clone();
        opacity_row.connect_value_notify(move |row| {
            config.borrow_mut().appearance.border_opacity = row.value() as u8;
            let _ = config.borrow().save();
            apply_appearance(&config.borrow().appearance);
            on_theme_changed(config.borrow().color_scheme, config.borrow().accent_color);
        });
    }
    borders_group.add(&opacity_row);

    let border_radius_row = adw::ComboRow::builder()
        .title("Corner radius")
        .subtitle("Roundness of card and panel corners")
        .build();
    let radius_names: Vec<&str> = BorderRadius::all()
        .iter()
        .map(|r| r.display_name())
        .collect();
    let radius_model = gtk::StringList::new(&radius_names);
    border_radius_row.set_model(Some(&radius_model));
    let current_radius = config.borrow().appearance.border_radius;
    border_radius_row.set_selected(
        BorderRadius::all()
            .iter()
            .position(|r| *r == current_radius)
            .unwrap_or(1) as u32,
    );
    {
        let config = config.clone();
        let on_theme_changed = on_theme_changed.clone();
        border_radius_row.connect_selected_notify(move |row| {
            let radius = BorderRadius::all()
                .get(row.selected() as usize)
                .copied()
                .unwrap_or_default();
            config.borrow_mut().appearance.border_radius = radius;
            let _ = config.borrow().save();
            apply_appearance(&config.borrow().appearance);
            on_theme_changed(config.borrow().color_scheme, config.borrow().accent_color);
        });
    }
    borders_group.add(&border_radius_row);

    page.add(&borders_group);

    // =========================================================================
    // EFFECTS GROUP
    // =========================================================================
    let effects_group = adw::PreferencesGroup::builder()
        .title("Effects")
        .description("Glow and animation effects (OLED Dark theme)")
        .build();

    let glow_row = adw::ComboRow::builder()
        .title("Glow intensity")
        .subtitle("Ambient glow effect on hover and focus")
        .build();
    let glow_names: Vec<&str> = GlowIntensity::all()
        .iter()
        .map(|g| g.display_name())
        .collect();
    let glow_model = gtk::StringList::new(&glow_names);
    glow_row.set_model(Some(&glow_model));
    let current_glow = config.borrow().appearance.glow_intensity;
    glow_row.set_selected(
        GlowIntensity::all()
            .iter()
            .position(|g| *g == current_glow)
            .unwrap_or(2) as u32,
    );
    {
        let config = config.clone();
        let on_theme_changed = on_theme_changed.clone();
        glow_row.connect_selected_notify(move |row| {
            let glow = GlowIntensity::all()
                .get(row.selected() as usize)
                .copied()
                .unwrap_or_default();
            config.borrow_mut().appearance.glow_intensity = glow;
            let _ = config.borrow().save();
            apply_appearance(&config.borrow().appearance);
            on_theme_changed(config.borrow().color_scheme, config.borrow().accent_color);
        });
    }
    effects_group.add(&glow_row);

    let speed_row = adw::ComboRow::builder()
        .title("Animation speed")
        .subtitle("Speed of hover and transition animations")
        .build();
    let speed_names: Vec<&str> = TransitionSpeed::all()
        .iter()
        .map(|s| s.display_name())
        .collect();
    let speed_model = gtk::StringList::new(&speed_names);
    speed_row.set_model(Some(&speed_model));
    let current_speed = config.borrow().appearance.transition_speed;
    speed_row.set_selected(
        TransitionSpeed::all()
            .iter()
            .position(|s| *s == current_speed)
            .unwrap_or(2) as u32,
    );
    {
        let config = config.clone();
        let on_theme_changed = on_theme_changed.clone();
        speed_row.connect_selected_notify(move |row| {
            let speed = TransitionSpeed::all()
                .get(row.selected() as usize)
                .copied()
                .unwrap_or_default();
            config.borrow_mut().appearance.transition_speed = speed;
            let _ = config.borrow().save();
            apply_appearance(&config.borrow().appearance);
            on_theme_changed(config.borrow().color_scheme, config.borrow().accent_color);
        });
    }
    effects_group.add(&speed_row);

    let animations_row = adw::SwitchRow::builder()
        .title("Hover animations")
        .subtitle("Enable animated transitions on hover")
        .build();
    animations_row.set_active(config.borrow().appearance.hover_animations);
    {
        let config = config.clone();
        let on_theme_changed = on_theme_changed.clone();
        animations_row.connect_active_notify(move |row| {
            config.borrow_mut().appearance.hover_animations = row.is_active();
            let _ = config.borrow().save();
            apply_appearance(&config.borrow().appearance);
            on_theme_changed(config.borrow().color_scheme, config.borrow().accent_color);
        });
    }
    effects_group.add(&animations_row);

    page.add(&effects_group);

    // =========================================================================
    // PRESETS GROUP
    // =========================================================================
    let presets_group = adw::PreferencesGroup::builder()
        .title("Quick Presets")
        .description("Apply pre-configured appearance settings")
        .build();

    let presets_row = adw::ActionRow::builder()
        .title("Apply preset")
        .subtitle("Choose a preset to quickly configure appearance")
        .build();

    let presets_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(8)
        .valign(gtk::Align::Center)
        .build();

    let preset_buttons = [
        ("Default", "view-restore-symbolic"),
        ("Minimal", "view-list-symbolic"),
        ("Vibrant", "starred-symbolic"),
        ("Contrast", "display-brightness-symbolic"),
    ];

    for (name, icon) in preset_buttons {
        let btn = gtk::Button::builder()
            .icon_name(icon)
            .tooltip_text(name)
            .build();
        btn.add_css_class("flat");
        btn.add_css_class("circular");

        let config = config.clone();
        let on_theme_changed = on_theme_changed.clone();
        let name = name.to_string();

        let border_style_row = border_style_row.clone();
        let opacity_row = opacity_row.clone();
        let border_radius_row = border_radius_row.clone();
        let glow_row = glow_row.clone();
        let speed_row = speed_row.clone();
        let animations_row = animations_row.clone();
        let color_scheme_row = color_scheme_row.clone();

        btn.connect_clicked(move |_| {
            let preset = match name.as_str() {
                "Minimal" => AppearanceConfig::preset_minimal(),
                "Vibrant" => AppearanceConfig::preset_vibrant(),
                "Contrast" => AppearanceConfig::preset_high_contrast(),
                _ => AppearanceConfig::preset_default(),
            };

            {
                let mut cfg = config.borrow_mut();
                cfg.appearance = preset.clone();
                cfg.color_scheme = preset.color_scheme;
                cfg.accent_color = preset.accent_color;
                let _ = cfg.save();
            }

            border_style_row.set_selected(
                BorderStyle::all()
                    .iter()
                    .position(|s| *s == preset.border_style)
                    .unwrap_or(2) as u32,
            );
            opacity_row.set_value(preset.border_opacity as f64);
            border_radius_row.set_selected(
                BorderRadius::all()
                    .iter()
                    .position(|r| *r == preset.border_radius)
                    .unwrap_or(1) as u32,
            );
            glow_row.set_selected(
                GlowIntensity::all()
                    .iter()
                    .position(|g| *g == preset.glow_intensity)
                    .unwrap_or(2) as u32,
            );
            speed_row.set_selected(
                TransitionSpeed::all()
                    .iter()
                    .position(|s| *s == preset.transition_speed)
                    .unwrap_or(2) as u32,
            );
            animations_row.set_active(preset.hover_animations);
            color_scheme_row.set_selected(
                ColorScheme::all()
                    .iter()
                    .position(|s| *s == preset.color_scheme)
                    .unwrap_or(0) as u32,
            );

            apply_appearance(&config.borrow().appearance);
            on_theme_changed(config.borrow().color_scheme, config.borrow().accent_color);
        });

        presets_box.append(&btn);
    }

    presets_row.add_suffix(&presets_box);
    presets_group.add(&presets_row);

    page.add(&presets_group);

    // =========================================================================
    // TYPOGRAPHY GROUP
    // =========================================================================
    let typography_group = adw::PreferencesGroup::builder()
        .title("Typography")
        .description("Font and text display settings")
        .build();

    let font_scale_row = adw::ComboRow::builder()
        .title("Interface scale")
        .subtitle("Scale factor for text and UI elements")
        .build();
    let font_scale_names: Vec<&str> = FontScale::all().iter().map(|f| f.display_name()).collect();
    let font_scale_model = gtk::StringList::new(&font_scale_names);
    font_scale_row.set_model(Some(&font_scale_model));
    let current_scale = config.borrow().appearance.font_scale;
    font_scale_row.set_selected(
        FontScale::all()
            .iter()
            .position(|f| *f == current_scale)
            .unwrap_or(1) as u32,
    );
    {
        let config = config.clone();
        font_scale_row.connect_selected_notify(move |row| {
            let scale = FontScale::all()
                .get(row.selected() as usize)
                .copied()
                .unwrap_or_default();
            config.borrow_mut().appearance.font_scale = scale;
            let _ = config.borrow().save();
        });
    }
    typography_group.add(&font_scale_row);

    let descriptions_row = adw::SwitchRow::builder()
        .title("Show package descriptions")
        .subtitle("Display package descriptions in list and grid views")
        .build();
    descriptions_row.set_active(config.borrow().appearance.show_descriptions);
    {
        let config = config.clone();
        descriptions_row.connect_active_notify(move |row| {
            config.borrow_mut().appearance.show_descriptions = row.is_active();
            let _ = config.borrow().save();
        });
    }
    typography_group.add(&descriptions_row);

    page.add(&typography_group);

    // =========================================================================
    // RESET GROUP
    // =========================================================================
    let reset_group = adw::PreferencesGroup::new();

    let reset_row = adw::ActionRow::builder()
        .title("Reset appearance to defaults")
        .subtitle("Restore all appearance settings to their default values")
        .activatable(true)
        .build();
    reset_row.add_css_class("destructive-action");
    reset_row.add_suffix(&gtk::Image::from_icon_name("view-refresh-symbolic"));

    {
        let config = config.clone();
        let on_theme_changed = on_theme_changed.clone();
        let border_style_row = border_style_row.clone();
        let opacity_row = opacity_row.clone();
        let border_radius_row = border_radius_row.clone();
        let glow_row = glow_row.clone();
        let speed_row = speed_row.clone();
        let animations_row = animations_row.clone();
        let color_scheme_row = color_scheme_row.clone();
        let grid_cols_row = grid_cols_row.clone();
        let card_size_row = card_size_row.clone();
        let list_density_row = list_density_row.clone();
        let spacing_row = spacing_row.clone();
        let sidebar_row = sidebar_row.clone();
        let font_scale_row = font_scale_row.clone();
        let descriptions_row = descriptions_row.clone();
        let layout_row = layout_row.clone();
        let compact_row = compact_row.clone();
        let icons_row = icons_row.clone();

        reset_row.connect_activated(move |row| {
            let defaults = AppearanceConfig::default();

            {
                let mut cfg = config.borrow_mut();
                cfg.appearance = defaults.clone();
                cfg.color_scheme = defaults.color_scheme;
                cfg.accent_color = defaults.accent_color;
                cfg.layout_mode = defaults.default_view_mode;
                cfg.ui_compact = false;
                cfg.ui_show_icons = true;
                let _ = cfg.save();
            }

            color_scheme_row.set_selected(
                ColorScheme::all()
                    .iter()
                    .position(|s| *s == defaults.color_scheme)
                    .unwrap_or(0) as u32,
            );
            border_style_row.set_selected(
                BorderStyle::all()
                    .iter()
                    .position(|s| *s == defaults.border_style)
                    .unwrap_or(2) as u32,
            );
            opacity_row.set_value(defaults.border_opacity as f64);
            border_radius_row.set_selected(
                BorderRadius::all()
                    .iter()
                    .position(|r| *r == defaults.border_radius)
                    .unwrap_or(1) as u32,
            );
            glow_row.set_selected(
                GlowIntensity::all()
                    .iter()
                    .position(|g| *g == defaults.glow_intensity)
                    .unwrap_or(2) as u32,
            );
            speed_row.set_selected(
                TransitionSpeed::all()
                    .iter()
                    .position(|s| *s == defaults.transition_speed)
                    .unwrap_or(2) as u32,
            );
            animations_row.set_active(defaults.hover_animations);
            layout_row.set_selected(match defaults.default_view_mode {
                LayoutMode::List => 0,
                LayoutMode::Grid => 1,
            });
            compact_row.set_active(false);
            icons_row.set_active(true);
            grid_cols_row.set_selected(
                GridColumns::all()
                    .iter()
                    .position(|g| *g == defaults.grid_columns)
                    .unwrap_or(2) as u32,
            );
            card_size_row.set_selected(
                CardSize::all()
                    .iter()
                    .position(|c| *c == defaults.card_size)
                    .unwrap_or(1) as u32,
            );
            list_density_row.set_selected(
                ListDensity::all()
                    .iter()
                    .position(|l| *l == defaults.list_density)
                    .unwrap_or(1) as u32,
            );
            spacing_row.set_selected(
                SpacingLevel::all()
                    .iter()
                    .position(|s| *s == defaults.spacing)
                    .unwrap_or(1) as u32,
            );
            sidebar_row.set_selected(
                SidebarWidth::all()
                    .iter()
                    .position(|s| *s == defaults.sidebar_width)
                    .unwrap_or(1) as u32,
            );
            font_scale_row.set_selected(
                FontScale::all()
                    .iter()
                    .position(|f| *f == defaults.font_scale)
                    .unwrap_or(1) as u32,
            );
            descriptions_row.set_active(defaults.show_descriptions);

            apply_appearance(&config.borrow().appearance);
            on_theme_changed(config.borrow().color_scheme, config.borrow().accent_color);

            if let Some(window) = row.root().and_downcast::<adw::PreferencesWindow>() {
                let toast = adw::Toast::new("Appearance reset to defaults");
                window.add_toast(toast);
            }
        });
    }

    reset_group.add(&reset_row);
    page.add(&reset_group);

    page
}

fn build_behavior_page(config: Rc<RefCell<Config>>) -> adw::PreferencesPage {
    let page = adw::PreferencesPage::builder()
        .title("Behavior")
        .icon_name("preferences-desktop-apps-symbolic")
        .build();

    let keyboard_group = adw::PreferencesGroup::builder()
        .title("Keyboard")
        .description("Configure keyboard navigation shortcuts")
        .build();

    let vim_mode_row = adw::SwitchRow::builder()
        .title("Vim-style navigation")
        .subtitle("Use j/k to navigate, i/r/u for actions, g+key for views")
        .build();
    vim_mode_row.set_active(config.borrow().vim_mode);
    {
        let config = config.clone();
        vim_mode_row.connect_active_notify(move |row| {
            config.borrow_mut().vim_mode = row.is_active();
            let _ = config.borrow().save();
        });
    }
    keyboard_group.add(&vim_mode_row);

    page.add(&keyboard_group);

    let tray_group = adw::PreferencesGroup::builder()
        .title("System Tray")
        .description("Configure system tray integration")
        .build();

    let background_row = adw::SwitchRow::builder()
        .title("Run in background")
        .subtitle("Keep LinGet running in the system tray when the window is closed")
        .build();
    background_row.set_active(config.borrow().run_in_background);
    {
        let config = config.clone();
        background_row.connect_active_notify(move |row| {
            config.borrow_mut().run_in_background = row.is_active();
            let _ = config.borrow().save();
        });
    }
    tray_group.add(&background_row);

    let minimized_row = adw::SwitchRow::builder()
        .title("Start minimized")
        .subtitle("Start LinGet minimized to the system tray")
        .build();
    minimized_row.set_active(config.borrow().start_minimized);
    {
        let config = config.clone();
        minimized_row.connect_active_notify(move |row| {
            config.borrow_mut().start_minimized = row.is_active();
            let _ = config.borrow().save();
        });
    }
    tray_group.add(&minimized_row);

    let data_group = adw::PreferencesGroup::builder().title("Data").build();

    let clear_searches_row = adw::ActionRow::builder()
        .title("Clear recent searches")
        .subtitle("Remove all saved search history")
        .activatable(true)
        .build();
    clear_searches_row.add_suffix(&gtk::Image::from_icon_name("go-next-symbolic"));
    {
        let config = config.clone();
        clear_searches_row.connect_activated(move |row| {
            config.borrow_mut().recent_searches.clear();
            let _ = config.borrow().save();
            if let Some(window) = row.root().and_downcast::<adw::PreferencesWindow>() {
                let toast = adw::Toast::new("Search history cleared");
                window.add_toast(toast);
            }
        });
    }
    data_group.add(&clear_searches_row);

    let reset_row = adw::ActionRow::builder()
        .title("Reset onboarding")
        .subtitle("Show the welcome screen on next launch")
        .activatable(true)
        .build();
    reset_row.add_suffix(&gtk::Image::from_icon_name("go-next-symbolic"));
    {
        let config = config.clone();
        reset_row.connect_activated(move |row| {
            config.borrow_mut().onboarding_completed = false;
            let _ = config.borrow().save();
            if let Some(window) = row.root().and_downcast::<adw::PreferencesWindow>() {
                let toast = adw::Toast::new("Onboarding will show on next launch");
                window.add_toast(toast);
            }
        });
    }
    data_group.add(&reset_row);

    page.add(&tray_group);
    page.add(&data_group);

    page
}
