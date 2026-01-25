use crate::models::{AppearanceConfig, ColorScheme};

use gtk4 as gtk;

#[allow(dead_code)]
pub fn generate_appearance_css(config: &AppearanceConfig) -> String {
    let border_px = config.border_style.thickness_px();
    let border_opacity = config.border_opacity as f32 / 100.0;
    let glow_opacity = config.glow_intensity.opacity();
    let radius = config.border_radius.to_px();
    let transition_ms = config.transition_speed.to_ms();

    let oled_css = if config.color_scheme == ColorScheme::OledDark {
        generate_oled_css(
            border_px,
            border_opacity,
            glow_opacity,
            radius,
            transition_ms,
        )
    } else {
        String::new()
    };

    let base_css = generate_base_css(radius, transition_ms);

    format!("{}\n{}", base_css, oled_css)
}

fn generate_base_css(radius: &str, transition_ms: u16) -> String {
    format!(
        r#"
        .pkg-card {{
            border-radius: {radius};
            transition: all {transition_ms}ms ease;
        }}

        .card-icon-frame {{
            border-radius: {radius};
        }}

        .boxed-list {{
            border-radius: {radius};
        }}
    "#
    )
}

fn generate_oled_css(
    border_px: u8,
    border_opacity: f32,
    glow_opacity: f32,
    radius: &str,
    transition_ms: u16,
) -> String {
    let hover_opacity = (border_opacity * 1.6).min(1.0);
    let hover_glow = (glow_opacity * 1.5).min(0.8);
    let selected_border_px = border_px + 2;

    format!(
        r#"
        .oled-dark .pkg-card {{
            border: {border_px}px solid alpha(@accent_color, {border_opacity});
            border-radius: {radius};
            transition: all {transition_ms}ms ease;
        }}

        .oled-dark .pkg-card:hover {{
            border-color: alpha(@accent_color, {hover_opacity});
            box-shadow: 0 8px 24px alpha(@accent_color, {glow_opacity});
        }}

        .oled-dark .pkg-card.selected {{
            border: {selected_border_px}px solid @accent_color;
            box-shadow: 0 0 0 4px alpha(@accent_color, {glow_opacity});
        }}

        .oled-dark .sidebar {{
            border-right: {border_px}px solid alpha(@accent_color, {border_opacity});
        }}

        .oled-dark headerbar {{
            border-bottom: {border_px}px solid alpha(@accent_color, {border_opacity});
        }}

        .oled-dark .boxed-list {{
            border: {border_px}px solid alpha(@accent_color, {border_opacity});
            border-radius: {radius};
        }}

        .oled-dark .boxed-list > row {{
            border-bottom: 1px solid alpha(@accent_color, {border_opacity});
            transition: all {transition_ms}ms ease;
        }}

        .oled-dark .boxed-list > row:last-child {{
            border-bottom: none;
        }}

        .oled-dark .boxed-list > row:hover {{
            background-color: #141414;
            box-shadow: 0 4px 16px alpha(@accent_color, {glow_opacity}),
                        inset 0 1px 0 alpha(@accent_color, 0.15);
        }}

        .oled-dark .boxed-row {{
            border: {border_px}px solid alpha(@accent_color, {border_opacity});
            transition: all {transition_ms}ms ease;
        }}

        .oled-dark .boxed-row:hover {{
            border-color: alpha(@accent_color, {hover_opacity});
            box-shadow: 0 4px 16px alpha(@accent_color, {hover_glow});
        }}

        .oled-dark .details-panel {{
            border-left: {border_px}px solid alpha(@accent_color, {border_opacity});
        }}

        .oled-dark .details-sheet {{
            border: {border_px}px solid alpha(@accent_color, 0.7);
            box-shadow: 0 24px 80px alpha(black, 0.5),
                        0 0 0 1px alpha(@accent_color, 0.4);
        }}

        .oled-dark .card-icon-frame {{
            border: {border_px}px solid alpha(@accent_color, {border_opacity});
            border-radius: {radius};
        }}

        .oled-dark .pkg-card:hover .card-icon-frame {{
            border-color: alpha(@accent_color, 0.7);
        }}

        .oled-dark .icon-frame {{
            border: {border_px}px solid alpha(@accent_color, {border_opacity});
        }}

        .oled-dark .nav-row {{
            transition: all {transition_ms}ms ease;
        }}

        .oled-dark .nav-row:hover:not(:selected) {{
            background-color: alpha(@accent_color, 0.15);
            box-shadow: inset 0 0 0 {border_px}px alpha(@accent_color, {border_opacity});
        }}

        .oled-dark .nav-row:selected {{
            background-color: alpha(@accent_color, 0.25);
            border-left: {selected_border_px}px solid @accent_color;
            box-shadow: inset 0 0 12px alpha(@accent_color, {glow_opacity}),
                        4px 0 12px alpha(@accent_color, 0.15);
        }}

        .oled-dark .provider-row {{
            transition: all {transition_ms}ms ease;
        }}

        .oled-dark .provider-row:hover {{
            background-color: alpha(@accent_color, 0.15);
            box-shadow: inset 0 0 0 {border_px}px alpha(@accent_color, 0.25);
        }}

        .oled-dark .search-entry-large {{
            border: {border_px}px solid alpha(@accent_color, {border_opacity});
            transition: all {transition_ms}ms ease;
        }}

        .oled-dark .search-entry-large:focus-within {{
            border-color: alpha(@accent_color, {hover_opacity});
            box-shadow: 0 0 0 3px alpha(@accent_color, {glow_opacity});
        }}

        .oled-dark button.suggested-action {{
            box-shadow: 0 4px 12px alpha(@accent_color, {glow_opacity});
        }}

        .oled-dark button.suggested-action:hover {{
            box-shadow: 0 6px 20px alpha(@accent_color, {hover_glow});
        }}

        .oled-dark .task-hub-fab {{
            box-shadow: 0 4px 16px alpha(@accent_color, {hover_glow}),
                        0 8px 32px alpha(black, 0.4);
        }}

        .oled-dark .task-hub-fab:hover {{
            box-shadow: 0 8px 28px alpha(@accent_color, 0.8),
                        0 16px 48px alpha(black, 0.5);
        }}

        .oled-dark popover contents {{
            border: {border_px}px solid alpha(@accent_color, {border_opacity});
            box-shadow: 0 8px 32px alpha(black, 0.6),
                        0 0 0 2px alpha(@accent_color, 0.4);
        }}

        .oled-dark .task-hub-popover-container {{
            border: {border_px}px solid alpha(@accent_color, 0.7);
        }}

        .oled-dark .command-palette-window {{
            border: {border_px}px solid alpha(@accent_color, 0.7);
        }}

        .oled-dark toast {{
            border: {border_px}px solid alpha(@accent_color, {border_opacity});
        }}

        .oled-dark .filter-indicator {{
            border: {border_px}px solid alpha(@accent_color, 0.7);
        }}

        .oled-dark .selection-bar {{
            border-top: {border_px}px solid alpha(@accent_color, {border_opacity});
        }}

        .oled-dark .command-center {{
            border-left: {border_px}px solid alpha(@accent_color, {border_opacity});
        }}

        .oled-dark .sandbox-section {{
            border: {border_px}px solid alpha(@accent_color, {border_opacity});
        }}

        .oled-dark .progress-card {{
            border: {border_px}px solid alpha(@accent_color, {border_opacity});
        }}

        .oled-dark .source-filter-btn:checked {{
            background-color: alpha(@accent_color, 0.2);
            box-shadow: inset 0 0 0 {border_px}px alpha(@accent_color, 0.5);
        }}

        .oled-dark .source-filter-btn:hover:not(:checked) {{
            background-color: alpha(@accent_color, 0.12);
        }}

        .oled-dark .chip-btn:hover {{
            background-color: alpha(@accent_color, 0.2);
            box-shadow: inset 0 0 0 {border_px}px alpha(@accent_color, 0.4);
        }}

        .oled-dark .update-dot {{
            box-shadow: 0 0 10px alpha(@accent_color, 0.8),
                        0 0 20px alpha(@accent_color, {glow_opacity});
        }}

        .oled-dark .badge-accent {{
            box-shadow: 0 2px 8px alpha(@accent_color, {glow_opacity});
        }}

        .oled-dark .hero-banner {{
            box-shadow: 0 12px 48px alpha(@accent_color, {glow_opacity}),
                        0 4px 16px alpha(black, 0.4);
        }}

        .oled-dark .pkg-row.row-active-operation {{
            background-color: alpha(@accent_color, 0.2);
            box-shadow: inset 0 0 0 {border_px}px alpha(@accent_color, 0.5),
                        inset 0 0 20px alpha(@accent_color, 0.15);
        }}

        .oled-dark .recent-searches-list > row:hover {{
            background-color: alpha(@accent_color, 0.2);
        }}

        .oled-dark .command-palette-list row:selected {{
            background-color: alpha(@accent_color, 0.3);
            box-shadow: inset 0 0 0 {border_px}px alpha(@accent_color, 0.5);
        }}

        .oled-dark .command-palette-list row:hover:not(:selected) {{
            background-color: alpha(@accent_color, 0.15);
        }}

        .oled-dark switch:checked {{
            box-shadow: 0 0 12px alpha(@accent_color, 0.7);
        }}

        .oled-dark .warning-banner {{
            border: {border_px}px solid alpha(@warning_color, 0.7);
            box-shadow: 0 4px 16px alpha(@warning_color, 0.25);
        }}

        .oled-dark toast.success-toast {{
            box-shadow: 0 4px 16px alpha(@success_color, 0.4);
        }}

        .oled-dark toast.error-toast {{
            box-shadow: 0 4px 16px alpha(@error_color, 0.4);
        }}
    "#
    )
}

#[allow(dead_code)]
pub fn apply_appearance(config: &AppearanceConfig) {
    let css = generate_appearance_css(config);

    if let Some(display) = gtk::gdk::Display::default() {
        let provider = gtk::CssProvider::new();
        provider.load_from_data(&css);

        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_USER,
        );

        tracing::debug!("Applied appearance CSS ({} bytes)", css.len());
    } else {
        tracing::warn!("No default display found, cannot apply appearance CSS");
    }
}
