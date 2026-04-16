//! Themed styling for the TUI.
//!
//! All styling flows through a runtime-switchable [`Theme`]. Built-in themes
//! include Nord, Dracula, Tokyo Night, Gruvbox Dark, High Contrast, and a
//! Monochrome palette that is auto-selected when `NO_COLOR` is set.
//!
//! External callers should use the semantic helpers (`accent()`, `dim()`,
//! `badge_installed()`, …) or the `palette::cyan()` / `palette::header_bg()`
//! accessors rather than hard-coding raw colours.

use crate::models::PackageSource;
use parking_lot::RwLock;
use ratatui::style::{Color, Modifier, Style};
use ratatui::symbols::border;

/// Full palette for a theme. All named slots are semantic — individual TUI
/// widgets map their needs onto these slots, never onto raw RGB values.
#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub name: &'static str,
    pub monochrome: bool,

    // Foreground accents
    pub cyan: Color,
    pub yellow: Color,
    pub green: Color,
    pub red: Color,
    pub white: Color,
    pub light_gray: Color,
    pub dark_gray: Color,
    pub inactive_border: Color,

    // Extended accents
    pub magenta: Color,
    pub blue: Color,
    pub orange: Color,
    pub teal: Color,
    pub peach: Color,
    pub lavender: Color,
    pub pink: Color,
    pub sky: Color,

    // Backgrounds
    pub header_bg: Color,
    pub footer_bg: Color,
    pub tab_active_bg: Color,
    pub badge_installed_bg: Color,
    pub badge_update_bg: Color,
    pub badge_not_installed_bg: Color,
    pub badge_progress_bg: Color,
    pub scrollbar_track_bg: Color,
}

macro_rules! rgb {
    ($r:expr, $g:expr, $b:expr) => {
        Color::Rgb($r, $g, $b)
    };
}

// ---------- Built-in themes ----------

/// Nord — calm, frosty blues. The historical LinGet default.
pub const NORD: Theme = Theme {
    name: "nord",
    monochrome: false,
    cyan: rgb!(136, 192, 208),
    yellow: rgb!(235, 203, 139),
    green: rgb!(163, 190, 140),
    red: rgb!(191, 97, 106),
    white: rgb!(236, 239, 244),
    light_gray: rgb!(196, 201, 209),
    dark_gray: rgb!(142, 151, 165),
    inactive_border: rgb!(76, 86, 106),
    magenta: rgb!(180, 142, 173),
    blue: rgb!(129, 161, 193),
    orange: rgb!(208, 135, 112),
    teal: rgb!(143, 188, 187),
    peach: rgb!(216, 183, 148),
    lavender: rgb!(180, 190, 254),
    pink: rgb!(191, 120, 150),
    sky: rgb!(129, 185, 223),
    header_bg: rgb!(59, 66, 82),
    footer_bg: rgb!(59, 66, 82),
    tab_active_bg: rgb!(53, 74, 92),
    badge_installed_bg: rgb!(46, 64, 46),
    badge_update_bg: rgb!(74, 67, 42),
    badge_not_installed_bg: rgb!(56, 61, 74),
    badge_progress_bg: rgb!(44, 63, 74),
    scrollbar_track_bg: rgb!(40, 46, 58),
};

/// Dracula — vivid purples, bright accents.
pub const DRACULA: Theme = Theme {
    name: "dracula",
    monochrome: false,
    cyan: rgb!(139, 233, 253),
    yellow: rgb!(241, 250, 140),
    green: rgb!(80, 250, 123),
    red: rgb!(255, 85, 85),
    white: rgb!(248, 248, 242),
    light_gray: rgb!(189, 187, 214),
    dark_gray: rgb!(139, 136, 167),
    inactive_border: rgb!(68, 71, 90),
    magenta: rgb!(255, 121, 198),
    blue: rgb!(98, 114, 164),
    orange: rgb!(255, 184, 108),
    teal: rgb!(139, 233, 253),
    peach: rgb!(255, 184, 108),
    lavender: rgb!(189, 147, 249),
    pink: rgb!(255, 121, 198),
    sky: rgb!(139, 233, 253),
    header_bg: rgb!(40, 42, 54),
    footer_bg: rgb!(40, 42, 54),
    tab_active_bg: rgb!(68, 71, 90),
    badge_installed_bg: rgb!(36, 64, 44),
    badge_update_bg: rgb!(72, 66, 30),
    badge_not_installed_bg: rgb!(60, 58, 74),
    badge_progress_bg: rgb!(42, 60, 80),
    scrollbar_track_bg: rgb!(30, 32, 42),
};

/// Tokyo Night — deep blue-violet, neon accents.
pub const TOKYO_NIGHT: Theme = Theme {
    name: "tokyo-night",
    monochrome: false,
    cyan: rgb!(125, 207, 255),
    yellow: rgb!(224, 175, 104),
    green: rgb!(158, 206, 106),
    red: rgb!(247, 118, 142),
    white: rgb!(192, 202, 245),
    light_gray: rgb!(169, 177, 214),
    dark_gray: rgb!(86, 95, 137),
    inactive_border: rgb!(52, 59, 88),
    magenta: rgb!(187, 154, 247),
    blue: rgb!(122, 162, 247),
    orange: rgb!(255, 158, 100),
    teal: rgb!(115, 218, 202),
    peach: rgb!(255, 158, 100),
    lavender: rgb!(187, 154, 247),
    pink: rgb!(247, 118, 142),
    sky: rgb!(125, 207, 255),
    header_bg: rgb!(26, 27, 38),
    footer_bg: rgb!(26, 27, 38),
    tab_active_bg: rgb!(41, 46, 66),
    badge_installed_bg: rgb!(32, 56, 40),
    badge_update_bg: rgb!(74, 56, 28),
    badge_not_installed_bg: rgb!(48, 54, 76),
    badge_progress_bg: rgb!(30, 52, 76),
    scrollbar_track_bg: rgb!(22, 23, 34),
};

/// Gruvbox Dark — warm earth tones, retro feel.
pub const GRUVBOX: Theme = Theme {
    name: "gruvbox",
    monochrome: false,
    cyan: rgb!(131, 165, 152),
    yellow: rgb!(215, 153, 33),
    green: rgb!(152, 151, 26),
    red: rgb!(204, 36, 29),
    white: rgb!(235, 219, 178),
    light_gray: rgb!(213, 196, 161),
    dark_gray: rgb!(146, 131, 116),
    inactive_border: rgb!(80, 73, 69),
    magenta: rgb!(177, 98, 134),
    blue: rgb!(69, 133, 136),
    orange: rgb!(214, 93, 14),
    teal: rgb!(104, 157, 106),
    peach: rgb!(250, 189, 47),
    lavender: rgb!(211, 134, 155),
    pink: rgb!(251, 73, 52),
    sky: rgb!(131, 165, 152),
    header_bg: rgb!(50, 48, 47),
    footer_bg: rgb!(50, 48, 47),
    tab_active_bg: rgb!(80, 73, 69),
    badge_installed_bg: rgb!(50, 56, 34),
    badge_update_bg: rgb!(72, 58, 22),
    badge_not_installed_bg: rgb!(60, 56, 54),
    badge_progress_bg: rgb!(40, 60, 60),
    scrollbar_track_bg: rgb!(40, 40, 40),
};

/// High Contrast — maximum legibility on any background.
pub const HIGH_CONTRAST: Theme = Theme {
    name: "high-contrast",
    monochrome: false,
    cyan: rgb!(0, 255, 255),
    yellow: rgb!(255, 255, 0),
    green: rgb!(0, 255, 0),
    red: rgb!(255, 80, 80),
    white: rgb!(255, 255, 255),
    light_gray: rgb!(220, 220, 220),
    dark_gray: rgb!(170, 170, 170),
    inactive_border: rgb!(160, 160, 160),
    magenta: rgb!(255, 0, 255),
    blue: rgb!(100, 160, 255),
    orange: rgb!(255, 170, 0),
    teal: rgb!(0, 230, 210),
    peach: rgb!(255, 200, 130),
    lavender: rgb!(210, 180, 255),
    pink: rgb!(255, 150, 200),
    sky: rgb!(130, 200, 255),
    header_bg: rgb!(0, 0, 0),
    footer_bg: rgb!(0, 0, 0),
    tab_active_bg: rgb!(50, 50, 50),
    badge_installed_bg: rgb!(0, 80, 0),
    badge_update_bg: rgb!(90, 70, 0),
    badge_not_installed_bg: rgb!(40, 40, 40),
    badge_progress_bg: rgb!(0, 60, 90),
    scrollbar_track_bg: rgb!(20, 20, 20),
};

/// Monochrome — terminal-native colours, no styling. Used when `NO_COLOR` is
/// set or the user opts in explicitly.
pub const MONOCHROME: Theme = Theme {
    name: "monochrome",
    monochrome: true,
    cyan: Color::Reset,
    yellow: Color::Reset,
    green: Color::Reset,
    red: Color::Reset,
    white: Color::Reset,
    light_gray: Color::Reset,
    dark_gray: Color::Reset,
    inactive_border: Color::Reset,
    magenta: Color::Reset,
    blue: Color::Reset,
    orange: Color::Reset,
    teal: Color::Reset,
    peach: Color::Reset,
    lavender: Color::Reset,
    pink: Color::Reset,
    sky: Color::Reset,
    header_bg: Color::Reset,
    footer_bg: Color::Reset,
    tab_active_bg: Color::Reset,
    badge_installed_bg: Color::Reset,
    badge_update_bg: Color::Reset,
    badge_not_installed_bg: Color::Reset,
    badge_progress_bg: Color::Reset,
    scrollbar_track_bg: Color::Reset,
};

pub const BUILTIN_THEMES: &[&Theme] = &[
    &NORD,
    &DRACULA,
    &TOKYO_NIGHT,
    &GRUVBOX,
    &HIGH_CONTRAST,
    &MONOCHROME,
];

static ACTIVE_THEME: RwLock<&'static Theme> = RwLock::new(&NORD);

/// Return a copy of the currently active theme.
pub fn active() -> Theme {
    **ACTIVE_THEME.read()
}

/// List of available theme names, in display order.
#[allow(dead_code)]
pub fn available_themes() -> Vec<&'static str> {
    BUILTIN_THEMES.iter().map(|t| t.name).collect()
}

/// Set the active theme by name. Returns `true` on success.
pub fn set_theme(name: &str) -> bool {
    let normalized = name.trim().to_ascii_lowercase();
    if let Some(theme) = BUILTIN_THEMES
        .iter()
        .find(|t| t.name.eq_ignore_ascii_case(&normalized))
    {
        *ACTIVE_THEME.write() = *theme;
        return true;
    }
    false
}

/// Advance to the next built-in theme and return its name. Intended for a
/// runtime "cycle theme" palette action.
pub fn cycle_theme() -> &'static str {
    let current = current_theme_name();
    let mut idx = BUILTIN_THEMES
        .iter()
        .position(|t| t.name == current)
        .unwrap_or(0);
    idx = (idx + 1) % BUILTIN_THEMES.len();
    let next = BUILTIN_THEMES[idx];
    *ACTIVE_THEME.write() = next;
    next.name
}

/// Current theme name (e.g. `"nord"`).
pub fn current_theme_name() -> &'static str {
    ACTIVE_THEME.read().name
}

/// Initialise the active theme from the environment. Honours:
///   * `NO_COLOR` (any value)  → forces monochrome
///   * `LINGET_THEME=<name>`   → explicit selection (overrides NO_COLOR only if valid)
pub fn init_from_env() {
    if let Ok(name) = std::env::var("LINGET_THEME") {
        if set_theme(&name) {
            return;
        }
    }
    if std::env::var_os("NO_COLOR").is_some() {
        set_theme("monochrome");
    }
}

// ---------- Borders ----------

pub const ROUNDED: border::Set = border::ROUNDED;

// ---------- Back-compat palette accessors (UPPERCASE kept for call-site stability) ----------

#[allow(non_snake_case, dead_code)]
pub mod palette {
    use super::{active, Color};

    pub fn CYAN() -> Color {
        active().cyan
    }
    pub fn YELLOW() -> Color {
        active().yellow
    }
    pub fn GREEN() -> Color {
        active().green
    }
    pub fn RED() -> Color {
        active().red
    }
    pub fn WHITE() -> Color {
        active().white
    }
    pub fn LIGHT_GRAY() -> Color {
        active().light_gray
    }
    pub fn DARK_GRAY() -> Color {
        active().dark_gray
    }
    pub fn INACTIVE_BORDER() -> Color {
        active().inactive_border
    }
    pub fn MAGENTA() -> Color {
        active().magenta
    }
    pub fn BLUE() -> Color {
        active().blue
    }
    pub fn ORANGE() -> Color {
        active().orange
    }
    pub fn TEAL() -> Color {
        active().teal
    }
    pub fn PEACH() -> Color {
        active().peach
    }
    pub fn LAVENDER() -> Color {
        active().lavender
    }
    pub fn PINK() -> Color {
        active().pink
    }
    pub fn SKY() -> Color {
        active().sky
    }
    pub fn HEADER_BG() -> Color {
        active().header_bg
    }
    pub fn FOOTER_BG() -> Color {
        active().footer_bg
    }
    pub fn TAB_ACTIVE_BG() -> Color {
        active().tab_active_bg
    }
    pub fn BADGE_INSTALLED_BG() -> Color {
        active().badge_installed_bg
    }
    pub fn BADGE_UPDATE_BG() -> Color {
        active().badge_update_bg
    }
    pub fn BADGE_NOT_INSTALLED_BG() -> Color {
        active().badge_not_installed_bg
    }
    pub fn BADGE_PROGRESS_BG() -> Color {
        active().badge_progress_bg
    }
    pub fn SCROLLBAR_TRACK_BG() -> Color {
        active().scrollbar_track_bg
    }
}

// ---------- Semantic style helpers ----------

fn plain(color: Color) -> Style {
    Style::default().fg(color)
}

pub fn text() -> Style {
    plain(active().white)
}
pub fn dim() -> Style {
    plain(active().dark_gray)
}
pub fn muted() -> Style {
    plain(active().light_gray)
}

pub fn accent() -> Style {
    Style::default()
        .fg(active().cyan)
        .add_modifier(Modifier::BOLD)
}

pub fn key_hint() -> Style {
    Style::default()
        .fg(active().yellow)
        .add_modifier(Modifier::BOLD)
}

pub fn border_focused() -> Style {
    plain(active().cyan)
}
pub fn border_unfocused() -> Style {
    plain(active().inactive_border)
}

pub fn row_cursor() -> Style {
    let t = active();
    if t.monochrome {
        Style::default().add_modifier(Modifier::REVERSED | Modifier::BOLD)
    } else {
        Style::default()
            .bg(t.tab_active_bg)
            .fg(t.white)
            .add_modifier(Modifier::BOLD)
    }
}

pub fn row_selected() -> Style {
    let t = active();
    if t.monochrome {
        Style::default().add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
    } else {
        plain(t.yellow)
    }
}

pub fn success() -> Style {
    plain(active().green)
}
pub fn warning() -> Style {
    plain(active().yellow)
}

pub fn error() -> Style {
    Style::default()
        .fg(active().red)
        .add_modifier(Modifier::BOLD)
}

pub fn loading() -> Style {
    plain(active().cyan)
}

pub fn table_header_band() -> Style {
    let t = active();
    if t.monochrome {
        Style::default().add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
    } else {
        Style::default()
            .fg(t.cyan)
            .bg(t.header_bg)
            .add_modifier(Modifier::BOLD)
    }
}

pub fn italic_status() -> Style {
    Style::default()
        .fg(active().light_gray)
        .add_modifier(Modifier::ITALIC)
}

pub fn footer_label() -> Style {
    plain(active().light_gray)
}

pub fn footer_bg() -> Style {
    let t = active();
    if t.monochrome {
        Style::default()
    } else {
        Style::default().bg(t.footer_bg)
    }
}

pub fn section_header() -> Style {
    Style::default()
        .fg(active().magenta)
        .add_modifier(Modifier::BOLD)
}

pub fn header_bar() -> Style {
    let t = active();
    if t.monochrome {
        Style::default().add_modifier(Modifier::REVERSED)
    } else {
        Style::default().bg(t.header_bg)
    }
}

pub fn tab_active() -> Style {
    let t = active();
    if t.monochrome {
        Style::default().add_modifier(Modifier::REVERSED | Modifier::BOLD)
    } else {
        Style::default()
            .fg(t.white)
            .bg(t.tab_active_bg)
            .add_modifier(Modifier::BOLD)
    }
}

pub fn badge_installed() -> Style {
    let t = active();
    if t.monochrome {
        Style::default().add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(t.green)
            .bg(t.badge_installed_bg)
            .add_modifier(Modifier::BOLD)
    }
}

pub fn badge_update() -> Style {
    let t = active();
    if t.monochrome {
        Style::default().add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
    } else {
        Style::default()
            .fg(t.yellow)
            .bg(t.badge_update_bg)
            .add_modifier(Modifier::BOLD)
    }
}

pub fn badge_not_installed() -> Style {
    let t = active();
    if t.monochrome {
        Style::default().add_modifier(Modifier::DIM)
    } else {
        Style::default()
            .fg(t.dark_gray)
            .bg(t.badge_not_installed_bg)
    }
}

pub fn badge_progress() -> Style {
    let t = active();
    if t.monochrome {
        Style::default().add_modifier(Modifier::BOLD | Modifier::ITALIC)
    } else {
        Style::default()
            .fg(t.cyan)
            .bg(t.badge_progress_bg)
            .add_modifier(Modifier::BOLD)
    }
}

pub fn scrollbar_style() -> Style {
    let t = active();
    Style::default().fg(t.dark_gray).bg(t.scrollbar_track_bg)
}

pub fn scrollbar_thumb() -> Style {
    plain(active().cyan)
}

/// Colour associated with a package source, for badges and list columns.
pub fn source_color(source: PackageSource) -> Style {
    let t = active();
    let color = match source {
        PackageSource::Apt => t.blue,
        PackageSource::Dnf => t.orange,
        PackageSource::Pacman => t.teal,
        PackageSource::Zypper => t.green,
        PackageSource::Flatpak => t.magenta,
        PackageSource::Snap => t.peach,
        PackageSource::Npm => t.red,
        PackageSource::Pip => t.yellow,
        PackageSource::Pipx => t.lavender,
        PackageSource::Cargo => t.orange,
        PackageSource::Brew => t.sky,
        PackageSource::Aur => t.cyan,
        PackageSource::Conda => t.green,
        PackageSource::Mamba => t.teal,
        PackageSource::Dart => t.blue,
        PackageSource::Deb => t.pink,
        PackageSource::AppImage => t.peach,
        PackageSource::Winget => t.lavender,
        PackageSource::Chocolatey => t.yellow,
        PackageSource::Scoop => t.sky,
    };
    plain(color)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // All tests in this module touch the global ACTIVE_THEME. A module-level
    // mutex forces them to run sequentially even when cargo parallelises,
    // preventing false failures.
    static TEST_LOCK: Mutex<()> = Mutex::new(());

    fn with_theme_lock<F: FnOnce()>(f: F) {
        let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let original = current_theme_name();
        f();
        assert!(set_theme(original));
    }

    /// Switching themes mutates the global palette so downstream style
    /// helpers pick up the new colours on their next call.
    #[test]
    fn set_theme_updates_active_palette() {
        with_theme_lock(|| {
            assert!(set_theme("dracula"));
            assert_eq!(current_theme_name(), "dracula");
            assert_eq!(accent().fg, Some(DRACULA.cyan));
        });
    }

    #[test]
    fn unknown_theme_name_is_rejected() {
        with_theme_lock(|| {
            let before = current_theme_name();
            assert!(!set_theme("not-a-real-theme"));
            assert_eq!(current_theme_name(), before);
        });
    }

    #[test]
    fn theme_lookup_is_case_insensitive() {
        with_theme_lock(|| {
            assert!(set_theme("NoRd"));
            assert_eq!(current_theme_name(), "nord");
        });
    }

    #[test]
    fn cycle_theme_visits_every_builtin_once() {
        with_theme_lock(|| {
            // Pin a known starting point so the cycle order is deterministic.
            assert!(set_theme("nord"));
            let start = current_theme_name();
            let mut visited = vec![start];
            for _ in 0..BUILTIN_THEMES.len() - 1 {
                visited.push(cycle_theme());
            }
            assert_eq!(cycle_theme(), start);
            let mut sorted = visited.clone();
            sorted.sort();
            sorted.dedup();
            assert_eq!(sorted.len(), BUILTIN_THEMES.len());
        });
    }

    #[test]
    fn monochrome_suppresses_backgrounds() {
        with_theme_lock(|| {
            assert!(set_theme("monochrome"));
            assert_eq!(footer_bg().bg, None);
            let badge = badge_installed();
            assert!(badge.add_modifier.contains(Modifier::BOLD));
            assert_eq!(badge.bg, None);
        });
    }

    #[test]
    fn available_themes_lists_all_builtins() {
        let names = available_themes();
        assert_eq!(names.len(), BUILTIN_THEMES.len());
        assert!(names.contains(&"nord"));
        assert!(names.contains(&"monochrome"));
    }
}
