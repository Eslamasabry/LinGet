with open("temp_ui.rs", "r") as f:
    lines = f.readlines()

header_code = '''use crate::cli::tui::app::App;
use crate::cli::tui::state::filters::Filter;
use crate::cli::tui::theme::{accent, badge_installed, badge_not_installed, badge_progress, badge_update, footer_label, header_bar, loading, muted, palette, tab_active, italic_status};
use crate::cli::tui::ui::{compose_left_right, spans_width};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
    Frame,
};
use unicode_width::UnicodeWidthStr;

'''

# draw_filter_bar: lines 110-211
# draw_status_legend: lines 212-226
# render_filter_tab: lines 227-284
# header_filter_hit_test: lines 285-372
# render_search_input: lines 552-575

header_code += "".join(lines[109:211]) + "\n"
header_code += "".join(lines[211:226]) + "\n"
header_code += "".join(lines[226:284]) + "\n"
header_code += "".join(lines[284:372]) + "\n"
header_code += "".join(lines[551:575]) + "\n"

# Add pub to the required functions
header_code = header_code.replace("fn draw_filter_bar", "pub fn draw_filter_bar")
header_code = header_code.replace("fn draw_status_legend", "pub fn draw_status_legend")

with open("src/cli/tui/components/header.rs", "w") as f:
    f.write(header_code)

