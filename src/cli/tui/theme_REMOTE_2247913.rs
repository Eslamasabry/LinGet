use ratatui::style::{Color, Modifier, Style};

pub mod palette {
    use super::*;

    pub const CYAN: Color = Color::Rgb(0, 255, 255);
    pub const YELLOW: Color = Color::Rgb(255, 255, 0);
    pub const GREEN: Color = Color::Rgb(0, 255, 0);
    pub const RED: Color = Color::Rgb(255, 0, 0);
    pub const WHITE: Color = Color::Rgb(255, 255, 255);
    pub const DARK_GRAY: Color = Color::Rgb(100, 100, 100);
    pub const INACTIVE_BORDER: Color = Color::Rgb(60, 60, 60);
}

pub fn title_style() -> Style {
    Style::default()
        .fg(palette::CYAN)
        .add_modifier(Modifier::BOLD)
}

pub fn title_bar() -> Style {
    Style::default().fg(palette::WHITE)
}

pub fn accent_color() -> Color {
    palette::YELLOW
}

pub fn title_color() -> Color {
    palette::CYAN
}

pub fn panel() -> Style {
    Style::default().fg(palette::WHITE)
}

pub fn panel_title() -> Style {
    Style::default()
        .fg(palette::WHITE)
        .add_modifier(Modifier::BOLD)
}

pub fn panel_title_active() -> Style {
    Style::default()
        .fg(palette::YELLOW)
        .add_modifier(Modifier::BOLD)
}

pub fn dim() -> Style {
    Style::default().fg(palette::DARK_GRAY)
}

pub fn accent() -> Style {
    Style::default().fg(palette::YELLOW)
}

pub fn selection() -> Style {
    Style::default()
        .bg(palette::DARK_GRAY)
        .add_modifier(Modifier::BOLD)
}

pub fn selection_focused() -> Style {
    Style::default().add_modifier(Modifier::REVERSED)
}

pub fn border_active() -> Style {
    Style::default().fg(palette::YELLOW)
}

pub fn border_inactive() -> Style {
    Style::default().fg(palette::INACTIVE_BORDER)
}

pub fn status_installed() -> Style {
    Style::default().fg(palette::GREEN)
}

pub fn status_update() -> Style {
    Style::default().fg(palette::YELLOW)
}

pub fn status_not_installed() -> Style {
    Style::default().fg(palette::DARK_GRAY)
}

pub fn status_loading() -> Style {
    Style::default().fg(palette::CYAN)
}

pub fn status_removing() -> Style {
    Style::default().fg(palette::RED)
}

pub fn table_header() -> Style {
    Style::default()
        .fg(palette::CYAN)
        .add_modifier(Modifier::BOLD)
}

pub fn key_hint() -> Style {
    Style::default()
        .fg(palette::YELLOW)
        .add_modifier(Modifier::BOLD)
}

pub fn description() -> Style {
    Style::default().fg(palette::DARK_GRAY)
}

pub fn separator() -> Style {
    Style::default().fg(palette::DARK_GRAY)
}

pub fn mode_normal() -> Style {
    Style::default().fg(palette::GREEN)
}

pub fn mode_search() -> Style {
    Style::default().fg(palette::YELLOW)
}

pub fn mode_confirm() -> Style {
    Style::default().fg(palette::RED)
}

pub fn label() -> Style {
    Style::default()
        .fg(palette::DARK_GRAY)
        .add_modifier(Modifier::BOLD)
}
