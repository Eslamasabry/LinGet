use crate::models::PackageSource;
use ratatui::style::{Color, Modifier, Style};
use ratatui::symbols::border;

pub mod palette {
    use super::*;

    pub const CYAN: Color = Color::Rgb(136, 192, 208);
    pub const YELLOW: Color = Color::Rgb(235, 203, 139);
    pub const GREEN: Color = Color::Rgb(163, 190, 140);
    pub const RED: Color = Color::Rgb(191, 97, 106);
    pub const WHITE: Color = Color::Rgb(236, 239, 244);
    pub const LIGHT_GRAY: Color = Color::Rgb(196, 201, 209);
    pub const DARK_GRAY: Color = Color::Rgb(142, 151, 165);
    pub const INACTIVE_BORDER: Color = Color::Rgb(76, 86, 106);

    pub const MAGENTA: Color = Color::Rgb(180, 142, 173);
    pub const BLUE: Color = Color::Rgb(129, 161, 193);
    pub const ORANGE: Color = Color::Rgb(208, 135, 112);
    pub const TEAL: Color = Color::Rgb(143, 188, 187);
    pub const PEACH: Color = Color::Rgb(216, 183, 148);
    pub const LAVENDER: Color = Color::Rgb(180, 190, 254);
    pub const PINK: Color = Color::Rgb(191, 120, 150);
    pub const SKY: Color = Color::Rgb(129, 185, 223);

    pub const HEADER_BG: Color = Color::Rgb(31, 35, 44);
    pub const BADGE_INSTALLED_BG: Color = Color::Rgb(46, 64, 46);
    pub const BADGE_UPDATE_BG: Color = Color::Rgb(74, 67, 42);
    pub const BADGE_NOT_INSTALLED_BG: Color = Color::Rgb(56, 61, 74);
    pub const BADGE_PROGRESS_BG: Color = Color::Rgb(44, 63, 74);
    pub const TAB_ACTIVE_BG: Color = Color::Rgb(53, 74, 92);
}

pub const ROUNDED: border::Set = border::ROUNDED;

pub fn text() -> Style {
    Style::default().fg(palette::WHITE)
}

pub fn dim() -> Style {
    Style::default().fg(palette::DARK_GRAY)
}

pub fn muted() -> Style {
    Style::default().fg(palette::LIGHT_GRAY)
}

pub fn accent() -> Style {
    Style::default()
        .fg(palette::CYAN)
        .add_modifier(Modifier::BOLD)
}

pub fn primary_action_button() -> Style {
    Style::default()
        .bg(palette::TAB_ACTIVE_BG)
        .fg(palette::WHITE)
        .add_modifier(Modifier::BOLD)
}

pub fn key_hint() -> Style {
    Style::default()
        .fg(palette::YELLOW)
        .add_modifier(Modifier::BOLD)
}

pub fn border_focused() -> Style {
    Style::default().fg(palette::CYAN)
}

pub fn border_unfocused() -> Style {
    Style::default().fg(palette::INACTIVE_BORDER)
}

pub fn row_cursor() -> Style {
    Style::default()
        .bg(palette::TAB_ACTIVE_BG)
        .fg(palette::WHITE)
        .add_modifier(Modifier::BOLD)
}

pub fn row_selected() -> Style {
    Style::default().fg(palette::YELLOW)
}

pub fn success() -> Style {
    Style::default().fg(palette::GREEN)
}

pub fn warning() -> Style {
    Style::default().fg(palette::YELLOW)
}

pub fn error() -> Style {
    Style::default()
        .fg(palette::RED)
        .add_modifier(Modifier::BOLD)
}

pub fn loading() -> Style {
    Style::default().fg(palette::CYAN)
}

pub fn table_header() -> Style {
    Style::default()
        .fg(palette::CYAN)
        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
}

pub fn italic_status() -> Style {
    Style::default()
        .fg(palette::LIGHT_GRAY)
        .add_modifier(Modifier::ITALIC)
}

pub fn footer_label() -> Style {
    Style::default().fg(palette::LIGHT_GRAY)
}

pub fn section_header() -> Style {
    Style::default()
        .fg(palette::MAGENTA)
        .add_modifier(Modifier::BOLD)
}

pub fn header_bar() -> Style {
    Style::default().bg(palette::HEADER_BG)
}

pub fn tab_active() -> Style {
    Style::default()
        .fg(palette::WHITE)
        .bg(palette::TAB_ACTIVE_BG)
        .add_modifier(Modifier::BOLD)
}

pub fn badge_installed() -> Style {
    Style::default()
        .fg(palette::GREEN)
        .bg(palette::BADGE_INSTALLED_BG)
        .add_modifier(Modifier::BOLD)
}

pub fn badge_update() -> Style {
    Style::default()
        .fg(palette::YELLOW)
        .bg(palette::BADGE_UPDATE_BG)
        .add_modifier(Modifier::BOLD)
}

pub fn badge_not_installed() -> Style {
    Style::default()
        .fg(palette::DARK_GRAY)
        .bg(palette::BADGE_NOT_INSTALLED_BG)
}

pub fn badge_progress() -> Style {
    Style::default()
        .fg(palette::CYAN)
        .bg(palette::BADGE_PROGRESS_BG)
        .add_modifier(Modifier::BOLD)
}

pub fn scrollbar_style() -> Style {
    Style::default().fg(palette::DARK_GRAY)
}

pub fn scrollbar_thumb() -> Style {
    Style::default().fg(palette::CYAN)
}

pub fn source_color(source: PackageSource) -> Style {
    let color = match source {
        PackageSource::Apt => palette::BLUE,
        PackageSource::Dnf => palette::ORANGE,
        PackageSource::Pacman => palette::TEAL,
        PackageSource::Zypper => palette::GREEN,
        PackageSource::Flatpak => palette::MAGENTA,
        PackageSource::Snap => palette::PEACH,
        PackageSource::Npm => palette::RED,
        PackageSource::Pip => palette::YELLOW,
        PackageSource::Pipx => palette::LAVENDER,
        PackageSource::Cargo => palette::ORANGE,
        PackageSource::Brew => palette::SKY,
        PackageSource::Aur => palette::CYAN,
        PackageSource::Conda => palette::GREEN,
        PackageSource::Mamba => palette::TEAL,
        PackageSource::Dart => palette::BLUE,
        PackageSource::Deb => palette::PINK,
        PackageSource::AppImage => palette::PEACH,
        PackageSource::Winget => palette::LAVENDER,
        PackageSource::Chocolatey => palette::YELLOW,
        PackageSource::Scoop => palette::SKY,
    };
    Style::default().fg(color)
}
