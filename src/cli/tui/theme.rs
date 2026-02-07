use ratatui::style::{Color, Modifier, Style};

pub mod palette {
    use super::*;

    pub const CYAN: Color = Color::Rgb(0, 255, 255);
    pub const YELLOW: Color = Color::Rgb(255, 255, 0);
    pub const GREEN: Color = Color::Rgb(0, 255, 0);
    pub const RED: Color = Color::Rgb(255, 0, 0);
    pub const WHITE: Color = Color::Rgb(255, 255, 255);
    pub const LIGHT_GRAY: Color = Color::Rgb(185, 185, 185);
    pub const DARK_GRAY: Color = Color::Rgb(140, 140, 140);
    pub const INACTIVE_BORDER: Color = Color::Rgb(90, 90, 90);
}

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
        .bg(palette::CYAN)
        .fg(Color::Black)
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
        .add_modifier(Modifier::BOLD)
}

pub fn italic_status() -> Style {
    Style::default()
        .fg(palette::LIGHT_GRAY)
        .add_modifier(Modifier::ITALIC)
}

pub fn footer_label() -> Style {
    Style::default().fg(palette::LIGHT_GRAY)
}
