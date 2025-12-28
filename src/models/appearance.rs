//! Appearance configuration system for LinGet.
//!
//! This module provides a fully configurable appearance system, allowing users
//! to customize borders, effects, layout, and typography through preferences.

use serde::{Deserialize, Serialize};

use super::config::{AccentColor, ColorScheme, LayoutMode};

/// Complete appearance configuration for the application.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppearanceConfig {
    // Theme (references existing config enums)
    pub color_scheme: ColorScheme,
    pub accent_color: AccentColor,
    pub custom_accent_hex: Option<String>,

    // Borders
    pub border_style: BorderStyle,
    pub border_opacity: u8, // 0-100, default 50
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

#[allow(dead_code)]
impl AppearanceConfig {
    /// Default preset - OLED Dark with balanced settings
    pub fn preset_default() -> Self {
        Self::default()
    }

    /// Minimal preset - System theme, subtle styling, compact layout
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

    /// Vibrant preset - OLED Dark with bold styling and effects
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

    /// High Contrast preset - Dark theme with maximum border visibility
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

// =============================================================================
// BORDER STYLE
// =============================================================================

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum BorderStyle {
    None,
    Subtle, // 1px, lower opacity
    #[default]
    Normal, // 2px, medium opacity
    Bold,   // 3px, higher opacity
}

#[allow(dead_code)]
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

// =============================================================================
// BORDER RADIUS
// =============================================================================

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum BorderRadius {
    Sharp, // 4px
    #[default]
    Rounded, // 12px
    Rounder, // 20px
    Pill,  // 999px
}

#[allow(dead_code)]
impl BorderRadius {
    pub fn to_px(self) -> &'static str {
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

// =============================================================================
// GLOW INTENSITY
// =============================================================================

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum GlowIntensity {
    Off,
    Subtle, // 0.15 opacity
    #[default]
    Normal, // 0.3 opacity
    Intense, // 0.5 opacity
}

#[allow(dead_code)]
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

// =============================================================================
// TRANSITION SPEED
// =============================================================================

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum TransitionSpeed {
    Instant, // 0ms
    Fast,    // 100ms
    #[default]
    Normal, // 200ms
    Slow,    // 350ms
}

#[allow(dead_code)]
impl TransitionSpeed {
    pub fn to_ms(self) -> u16 {
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

// =============================================================================
// GRID COLUMNS
// =============================================================================

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum GridColumns {
    Two,
    Three,
    #[default]
    Four,
    Five,
    Six,
}

#[allow(dead_code)]
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

// =============================================================================
// CARD SIZE
// =============================================================================

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum CardSize {
    Compact, // 180x220, icon 48px
    #[default]
    Normal, // 260x300, icon 72px
    Large,   // 320x360, icon 96px
}

#[allow(dead_code)]
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

// =============================================================================
// LIST DENSITY
// =============================================================================

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum ListDensity {
    Compact, // 52px row height, smaller icons
    #[default]
    Normal, // 72px row height
    Comfortable, // 88px row height, larger icons
}

#[allow(dead_code)]
impl ListDensity {
    pub fn row_height(&self) -> i32 {
        match self {
            Self::Compact => 52,
            Self::Normal => 72,
            Self::Comfortable => 88,
        }
    }

    pub fn icon_size(&self) -> i32 {
        match self {
            Self::Compact => 32,
            Self::Normal => 48,
            Self::Comfortable => 64,
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

// =============================================================================
// SPACING LEVEL
// =============================================================================

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum SpacingLevel {
    Tight, // 8px
    #[default]
    Normal, // 16px
    Relaxed, // 24px
}

#[allow(dead_code)]
impl SpacingLevel {
    pub fn to_px(self) -> i32 {
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

// =============================================================================
// SIDEBAR WIDTH
// =============================================================================

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum SidebarWidth {
    Narrow, // 200px
    #[default]
    Normal, // 260px
    Wide,   // 320px
}

#[allow(dead_code)]
impl SidebarWidth {
    pub fn to_px(self) -> i32 {
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

// =============================================================================
// FONT SCALE
// =============================================================================

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum FontScale {
    Smaller, // 90%
    #[default]
    Normal, // 100%
    Larger,  // 110%
    Largest, // 120%
}

#[allow(dead_code)]
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
