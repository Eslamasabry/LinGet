# LinGet Appearance System - Design Document

## Overview

A fully configurable appearance system for LinGet, allowing users to customize every visual aspect through Preferences → Appearance. All settings apply in real-time.

> **Technical Note**: GTK CSS does not support CSS custom properties (`var()`), `calc()`, or many modern CSS features. All dynamic styling must be achieved by generating complete CSS rules and injecting them via `CssProvider`.

---

## 1. Configuration Structure

### 1.1 Rust Data Model

```rust
// src/models/config.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppearanceConfig {
    // Theme
    pub color_scheme: ColorScheme,
    pub accent_color: AccentColor,
    pub custom_accent_hex: Option<String>,

    // Borders
    pub border_style: BorderStyle,
    pub border_opacity: u8,  // 0-100, default 50
    pub use_accent_borders: bool,
    pub border_radius: BorderRadius,

    // Effects
    pub glow_intensity: GlowIntensity,
    pub hover_animations: bool,
    pub transition_speed: TransitionSpeed,

    // Layout
    pub default_view_mode: LayoutMode,
    pub grid_columns: GridColumns,
    pub card_size: CardSize,
    pub list_density: ListDensity,
    pub spacing: SpacingLevel,
    pub sidebar_width: SidebarWidth,

    // Typography
    pub font_scale: FontScale,
    pub show_descriptions: bool,
}

impl Default for AppearanceConfig {
    fn default() -> Self {
        Self {
            color_scheme: ColorScheme::OledDark,
            accent_color: AccentColor::System,
            custom_accent_hex: None,
            border_style: BorderStyle::Normal,
            border_opacity: 50,
            use_accent_borders: true,
            border_radius: BorderRadius::Rounded,
            glow_intensity: GlowIntensity::Normal,
            hover_animations: true,
            transition_speed: TransitionSpeed::Normal,
            default_view_mode: LayoutMode::Grid,
            grid_columns: GridColumns::Four,
            card_size: CardSize::Normal,
            list_density: ListDensity::Normal,
            spacing: SpacingLevel::Normal,
            sidebar_width: SidebarWidth::Normal,
            font_scale: FontScale::Normal,
            show_descriptions: true,
        }
    }
}
```

### 1.2 Enums with Display Names

```rust
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum BorderStyle {
    None,
    Subtle,    // 1px, lower opacity
    #[default]
    Normal,    // 2px, medium opacity
    Bold,      // 3px, higher opacity
}

impl BorderStyle {
    pub fn thickness_px(&self) -> u8 {
        match self {
            Self::None => 0,
            Self::Subtle => 1,
            Self::Normal => 2,
            Self::Bold => 3,
        }
    }
    
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Subtle => "Subtle",
            Self::Normal => "Normal",
            Self::Bold => "Bold",
        }
    }
    
    pub fn all() -> &'static [Self] {
        &[Self::None, Self::Subtle, Self::Normal, Self::Bold]
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum BorderRadius {
    Sharp,     // 4px
    #[default]
    Rounded,   // 12px
    Rounder,   // 20px
    Pill,      // 999px
}

impl BorderRadius {
    pub fn to_px(&self) -> &'static str {
        match self {
            Self::Sharp => "4px",
            Self::Rounded => "12px",
            Self::Rounder => "20px",
            Self::Pill => "999px",
        }
    }
    
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Sharp => "Sharp",
            Self::Rounded => "Rounded",
            Self::Rounder => "Rounder",
            Self::Pill => "Pill",
        }
    }
    
    pub fn all() -> &'static [Self] {
        &[Self::Sharp, Self::Rounded, Self::Rounder, Self::Pill]
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum GlowIntensity {
    Off,
    Subtle,    // 0.15 opacity
    #[default]
    Normal,    // 0.3 opacity
    Intense,   // 0.5 opacity
}

impl GlowIntensity {
    pub fn opacity(&self) -> f32 {
        match self {
            Self::Off => 0.0,
            Self::Subtle => 0.15,
            Self::Normal => 0.3,
            Self::Intense => 0.5,
        }
    }
    
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::Subtle => "Subtle",
            Self::Normal => "Normal",
            Self::Intense => "Intense",
        }
    }
    
    pub fn all() -> &'static [Self] {
        &[Self::Off, Self::Subtle, Self::Normal, Self::Intense]
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum TransitionSpeed {
    Instant,   // 0ms
    Fast,      // 100ms
    #[default]
    Normal,    // 200ms
    Slow,      // 350ms
}

impl TransitionSpeed {
    pub fn to_ms(&self) -> u16 {
        match self {
            Self::Instant => 0,
            Self::Fast => 100,
            Self::Normal => 200,
            Self::Slow => 350,
        }
    }
    
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Instant => "Instant",
            Self::Fast => "Fast",
            Self::Normal => "Normal",
            Self::Slow => "Slow",
        }
    }
    
    pub fn all() -> &'static [Self] {
        &[Self::Instant, Self::Fast, Self::Normal, Self::Slow]
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum GridColumns {
    Two,
    Three,
    #[default]
    Four,
    Five,
    Six,
}

impl GridColumns {
    pub fn count(&self) -> u8 {
        match self {
            Self::Two => 2,
            Self::Three => 3,
            Self::Four => 4,
            Self::Five => 5,
            Self::Six => 6,
        }
    }
    
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Two => "2",
            Self::Three => "3",
            Self::Four => "4",
            Self::Five => "5",
            Self::Six => "6",
        }
    }
    
    pub fn all() -> &'static [Self] {
        &[Self::Two, Self::Three, Self::Four, Self::Five, Self::Six]
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum CardSize {
    Compact,   // 180x220, icon 48px
    #[default]
    Normal,    // 260x300, icon 72px
    Large,     // 320x360, icon 96px
}

impl CardSize {
    pub fn dimensions(&self) -> (i32, i32) {
        match self {
            Self::Compact => (180, 220),
            Self::Normal => (260, 300),
            Self::Large => (320, 360),
        }
    }
    
    pub fn icon_size(&self) -> i32 {
        match self {
            Self::Compact => 48,
            Self::Normal => 72,
            Self::Large => 96,
        }
    }
    
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Compact => "Compact",
            Self::Normal => "Normal",
            Self::Large => "Large",
        }
    }
    
    pub fn all() -> &'static [Self] {
        &[Self::Compact, Self::Normal, Self::Large]
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum ListDensity {
    Compact,     // 52px row height, smaller icons
    #[default]
    Normal,      // 72px row height
    Comfortable, // 88px row height, larger icons
}

impl ListDensity {
    pub fn row_height(&self) -> i32 {
        match self {
            Self::Compact => 52,
            Self::Normal => 72,
            Self::Comfortable => 88,
        }
    }
    
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Compact => "Compact",
            Self::Normal => "Normal",
            Self::Comfortable => "Comfortable",
        }
    }
    
    pub fn all() -> &'static [Self] {
        &[Self::Compact, Self::Normal, Self::Comfortable]
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum SpacingLevel {
    Tight,     // 8px
    #[default]
    Normal,    // 16px
    Relaxed,   // 24px
}

impl SpacingLevel {
    pub fn to_px(&self) -> i32 {
        match self {
            Self::Tight => 8,
            Self::Normal => 16,
            Self::Relaxed => 24,
        }
    }
    
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Tight => "Tight",
            Self::Normal => "Normal",
            Self::Relaxed => "Relaxed",
        }
    }
    
    pub fn all() -> &'static [Self] {
        &[Self::Tight, Self::Normal, Self::Relaxed]
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum SidebarWidth {
    Narrow,    // 200px
    #[default]
    Normal,    // 260px
    Wide,      // 320px
}

impl SidebarWidth {
    pub fn to_px(&self) -> i32 {
        match self {
            Self::Narrow => 200,
            Self::Normal => 260,
            Self::Wide => 320,
        }
    }
    
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Narrow => "Narrow",
            Self::Normal => "Normal",
            Self::Wide => "Wide",
        }
    }
    
    pub fn all() -> &'static [Self] {
        &[Self::Narrow, Self::Normal, Self::Wide]
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum FontScale {
    Smaller,   // 90%
    #[default]
    Normal,    // 100%
    Larger,    // 110%
    Largest,   // 120%
}

impl FontScale {
    pub fn multiplier(&self) -> f32 {
        match self {
            Self::Smaller => 0.9,
            Self::Normal => 1.0,
            Self::Larger => 1.1,
            Self::Largest => 1.2,
        }
    }
    
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Smaller => "90%",
            Self::Normal => "100%",
            Self::Larger => "110%",
            Self::Largest => "120%",
        }
    }
    
    pub fn all() -> &'static [Self] {
        &[Self::Smaller, Self::Normal, Self::Larger, Self::Largest]
    }
}
```

---

## 2. Dynamic CSS Generation

Since GTK CSS doesn't support variables, we generate complete CSS rules:

```rust
// src/ui/appearance.rs

use crate::models::{AppearanceConfig, ColorScheme};

pub fn generate_appearance_css(config: &AppearanceConfig) -> String {
    let border_px = config.border_style.thickness_px();
    let border_opacity = config.border_opacity as f32 / 100.0;
    let glow_opacity = config.glow_intensity.opacity();
    let radius = config.border_radius.to_px();
    let transition_ms = config.transition_speed.to_ms();
    
    // Only generate OLED-specific styles if using OLED theme
    let oled_css = if config.color_scheme == ColorScheme::OledDark {
        generate_oled_css(border_px, border_opacity, glow_opacity, radius, transition_ms)
    } else {
        String::new()
    };
    
    let base_css = generate_base_css(radius, transition_ms);
    
    format!("{}\n{}", base_css, oled_css)
}

fn generate_base_css(radius: &str, transition_ms: u16) -> String {
    format!(r#"
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
    "#)
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
    
    format!(r#"
        .oled-dark .pkg-card {{
            border: {border_px}px solid alpha(@accent_color, {border_opacity});
            border-radius: {radius};
            transition: all {transition_ms}ms ease;
        }}
        
        .oled-dark .pkg-card:hover {{
            border-color: alpha(@accent_color, {hover_opacity});
            box-shadow: 0 8px 24px alpha(@accent_color, {glow_opacity});
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
        
        .oled-dark .card-icon-frame {{
            border: {border_px}px solid alpha(@accent_color, {border_opacity});
            border-radius: {radius};
        }}
        
        .oled-dark .nav-row:hover:not(:selected) {{
            box-shadow: inset 0 0 0 {border_px}px alpha(@accent_color, {border_opacity});
        }}
        
        .oled-dark .nav-row:selected {{
            border-left: {}px solid @accent_color;
            box-shadow: inset 0 0 12px alpha(@accent_color, {glow_opacity});
        }}
        
        .oled-dark .search-entry-large {{
            border: {border_px}px solid alpha(@accent_color, {border_opacity});
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
        
        .oled-dark popover contents {{
            border: {border_px}px solid alpha(@accent_color, {border_opacity});
        }}
        
        .oled-dark toast {{
            border: {border_px}px solid alpha(@accent_color, {border_opacity});
        }}
    "#, border_px + 2)  // nav-row selected gets thicker border
}

/// Apply appearance config to the application
pub fn apply_appearance(window: &impl gtk::prelude::IsA<gtk::Widget>, config: &AppearanceConfig) {
    let css = generate_appearance_css(config);
    
    if let Some(display) = gtk::gdk::Display::default() {
        let provider = gtk::CssProvider::new();
        provider.load_from_data(&css);
        
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_USER,
        );
    }
}
```

---

## 3. Preferences UI Layout

### 3.1 Appearance Tab

```
┌─────────────────────────────────────────────────────────────────┐
│ Appearance                                                       │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│ ┌─ Presets ───────────────────────────────────────────────────┐ │
│ │                                                              │ │
│ │  Quick Start      [Default] [Minimal] [Vibrant] [Contrast]  │ │
│ │                                                              │ │
│ └──────────────────────────────────────────────────────────────┘ │
│                                                                  │
│ ┌─ Theme ─────────────────────────────────────────────────────┐ │
│ │                                                              │ │
│ │  Color Scheme     [System] [Light] [Dark] [OLED Dark]       │ │
│ │                                                              │ │
│ │  Accent Color     [●][●][●][●][●][●][●][●][●]               │ │
│ │                    Sys Bl Te Gr Ye Or Re Pi Pu              │ │
│ │                                                              │ │
│ └──────────────────────────────────────────────────────────────┘ │
│                                                                  │
│ ┌─ Borders ───────────────────────────────────────────────────┐ │
│ │                                                              │ │
│ │  Border Style     [None] [Subtle] [Normal] [Bold]           │ │
│ │                                                              │ │
│ │  Border Opacity   ────────────●────────  50%                │ │
│ │                                                              │ │
│ │  Use Accent Color for Borders                  [═══════●]   │ │
│ │                                                              │ │
│ │  Corner Radius    [Sharp] [Rounded] [Rounder] [Pill]        │ │
│ │                                                              │ │
│ └──────────────────────────────────────────────────────────────┘ │
│                                                                  │
│ ┌─ Effects ───────────────────────────────────────────────────┐ │
│ │                                                              │ │
│ │  Glow Intensity   [Off] [Subtle] [Normal] [Intense]         │ │
│ │                                                              │ │
│ │  Hover Animations                              [═══════●]   │ │
│ │                                                              │ │
│ │  Animation Speed  [Instant] [Fast] [Normal] [Slow]          │ │
│ │                                                              │ │
│ └──────────────────────────────────────────────────────────────┘ │
│                                                                  │
│ ┌─ Layout ────────────────────────────────────────────────────┐ │
│ │                                                              │ │
│ │  Default View     [≡ List]  [⊞ Grid]                        │ │
│ │                                                              │ │
│ │  Grid Columns     [2] [3] [4] [5] [6]                       │ │
│ │                                                              │ │
│ │  Card Size        [Compact] [Normal] [Large]                │ │
│ │                                                              │ │
│ │  List Density     [Compact] [Normal] [Comfortable]          │ │
│ │                                                              │ │
│ │  Spacing          [Tight] [Normal] [Relaxed]                │ │
│ │                                                              │ │
│ └──────────────────────────────────────────────────────────────┘ │
│                                                                  │
│ ┌─ Typography ────────────────────────────────────────────────┐ │
│ │                                                              │ │
│ │  Interface Scale  [90%] [100%] [110%] [120%]                │ │
│ │                                                              │ │
│ │  Show Package Descriptions                     [═══════●]   │ │
│ │                                                              │ │
│ └──────────────────────────────────────────────────────────────┘ │
│                                                                  │
│                              [Reset All to Defaults]             │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### 3.2 Built-in Presets

| Preset          | Description                                         |
| --------------- | --------------------------------------------------- |
| **Default**     | OLED Dark, Normal borders (50%), Normal glow, 4 cols |
| **Minimal**     | System theme, Subtle borders (25%), No glow, Compact |
| **Vibrant**     | OLED Dark, Bold borders (80%), Intense glow, Large   |
| **High Contrast** | Dark theme, Bold borders (100%), No glow, Sharp corners |

---

## 4. Implementation Files

### 4.1 New Files

| File                             | Purpose                                  |
| -------------------------------- | ---------------------------------------- |
| `src/models/appearance.rs`       | AppearanceConfig struct and all enums    |
| `src/ui/appearance.rs`           | CSS generation and application functions |
| `src/ui/preferences/appearance.rs` | Appearance preferences page UI           |

### 4.2 Modified Files

| File                   | Changes                                                |
| ---------------------- | ------------------------------------------------------ |
| `src/models/config.rs` | Add `appearance: AppearanceConfig` field               |
| `src/models/mod.rs`    | Export appearance module                               |
| `src/ui/preferences.rs` | Add Appearance tab, wire up settings                   |
| `src/ui/relm_app.rs`   | Apply appearance on startup, react to changes          |
| `src/ui/widgets/package_card.rs` | Read card_size from config                   |
| `resources/style.css`  | Remove hardcoded OLED values (now generated dynamically) |

---

## 5. Implementation Order

### Phase 1: Data Model ✓
1. Create `src/models/appearance.rs` with all enums
2. Add `AppearanceConfig` struct with defaults
3. Add `appearance` field to main `Config`
4. Implement serialization migration for existing configs

### Phase 2: CSS Generation
1. Create `src/ui/appearance.rs`
2. Implement `generate_appearance_css()`
3. Implement `apply_appearance()`
4. Remove conflicting static CSS from `style.css`

### Phase 3: Preferences UI
1. Add Presets group with 4 preset buttons
2. Add Theme group (existing, move here)
3. Add Borders group with controls
4. Add Effects group with controls
5. Add Layout group with controls
6. Add Typography group with controls
7. Add Reset button

### Phase 4: Runtime Application
1. Call `apply_appearance()` on app startup
2. Re-apply on any preference change
3. Update FlowBox columns dynamically
4. Update card dimensions dynamically

---

## 6. Preset Definitions

```rust
impl AppearanceConfig {
    pub fn preset_default() -> Self {
        Self::default()
    }
    
    pub fn preset_minimal() -> Self {
        Self {
            color_scheme: ColorScheme::System,
            border_style: BorderStyle::Subtle,
            border_opacity: 25,
            glow_intensity: GlowIntensity::Off,
            card_size: CardSize::Compact,
            spacing: SpacingLevel::Tight,
            ..Self::default()
        }
    }
    
    pub fn preset_vibrant() -> Self {
        Self {
            color_scheme: ColorScheme::OledDark,
            border_style: BorderStyle::Bold,
            border_opacity: 80,
            glow_intensity: GlowIntensity::Intense,
            card_size: CardSize::Large,
            spacing: SpacingLevel::Relaxed,
            ..Self::default()
        }
    }
    
    pub fn preset_high_contrast() -> Self {
        Self {
            color_scheme: ColorScheme::Dark,
            border_style: BorderStyle::Bold,
            border_opacity: 100,
            border_radius: BorderRadius::Sharp,
            glow_intensity: GlowIntensity::Off,
            ..Self::default()
        }
    }
}
```

---

## 7. Signal Flow

```
User changes setting in Preferences
         │
         ▼
┌─────────────────────┐
│ Update Config       │
│ (AppearanceConfig)  │
└─────────────────────┘
         │
         ├──────────────────────────┐
         ▼                          ▼
┌─────────────────────┐   ┌─────────────────────┐
│ Save to config.toml │   │ generate_appearance │
└─────────────────────┘   │ _css()              │
                          └─────────────────────┘
                                    │
                                    ▼
                          ┌─────────────────────┐
                          │ CssProvider         │
                          │ .load_from_data()   │
                          └─────────────────────┘
                                    │
                                    ▼
                          ┌─────────────────────┐
                          │ style_context_add   │
                          │ _provider_for       │
                          │ _display()          │
                          └─────────────────────┘
                                    │
                                    ▼
                          ┌─────────────────────┐
                          │ UI updates in       │
                          │ real-time           │
                          └─────────────────────┘
```

---

## 8. Migration Strategy

For users with existing `config.toml`:

1. If `appearance` section missing, use `AppearanceConfig::default()`
2. Existing `color_scheme` and `accent_color` migrate into `appearance.color_scheme` and `appearance.accent_color`
3. Old fields deprecated but still read for one version
4. Clean migration with no data loss
