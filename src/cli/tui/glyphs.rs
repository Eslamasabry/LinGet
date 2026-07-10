use ratatui::symbols::border;

#[derive(Debug, Clone, Copy)]
pub struct GlyphSet {
    pub logo: &'static str,
    pub total: &'static str,
    pub installed: &'static str,
    pub update: &'static str,
    pub favorite: &'static str,
    pub warning: &'static str,
    pub sparkle: &'static str,
    pub failed: &'static str,
    pub refresh: &'static str,
    pub horizontal: &'static str,
    pub bar_filled: &'static str,
    pub bar_empty: &'static str,
}

const UNICODE: GlyphSet = GlyphSet {
    logo: "❖",
    total: "∑",
    installed: "✓",
    update: "↑",
    favorite: "★",
    warning: "⚠",
    sparkle: "✦",
    failed: "✗",
    refresh: "↻",
    horizontal: "─",
    bar_filled: "▰",
    bar_empty: "▱",
};

const ASCII: GlyphSet = GlyphSet {
    logo: "*",
    total: "#",
    installed: "+",
    update: "^",
    favorite: "*",
    warning: "!",
    sparkle: "*",
    failed: "x",
    refresh: "r",
    horizontal: "-",
    bar_filled: "#",
    bar_empty: ".",
};

pub fn ascii_mode() -> bool {
    std::env::var("LINGET_TUI_ASCII").is_ok_and(|value| value == "1")
}

pub fn active() -> &'static GlyphSet {
    if ascii_mode() {
        &ASCII
    } else {
        &UNICODE
    }
}

pub fn border_set() -> border::Set {
    if ascii_mode() {
        border::Set {
            top_left: "+",
            top_right: "+",
            bottom_left: "+",
            bottom_right: "+",
            vertical_left: "|",
            vertical_right: "|",
            horizontal_top: "-",
            horizontal_bottom: "-",
        }
    } else {
        border::ROUNDED
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use once_cell::sync::Lazy;
    use std::sync::Mutex;

    static ENV_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

    #[test]
    fn both_glyph_sets_are_single_cell() {
        for set in [UNICODE, ASCII] {
            for glyph in [
                set.logo,
                set.total,
                set.installed,
                set.update,
                set.favorite,
                set.warning,
                set.sparkle,
                set.failed,
                set.refresh,
                set.horizontal,
                set.bar_filled,
                set.bar_empty,
            ] {
                assert_eq!(unicode_width::UnicodeWidthStr::width(glyph), 1);
            }
        }
    }

    #[test]
    fn ascii_environment_selects_ascii_glyphs_and_borders() {
        let _guard = ENV_LOCK.lock().unwrap();
        let previous = std::env::var_os("LINGET_TUI_ASCII");
        std::env::set_var("LINGET_TUI_ASCII", "1");
        assert!(ascii_mode());
        assert_eq!(active().warning, "!");
        assert_eq!(border_set().top_left, "+");
        if let Some(value) = previous {
            std::env::set_var("LINGET_TUI_ASCII", value);
        } else {
            std::env::remove_var("LINGET_TUI_ASCII");
        }
    }
}
