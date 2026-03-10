import re

with open("src/cli/tui/ui.rs", "r") as f:
    lines = f.readlines()

def get_fn_bounds(lines, fn_name):
    start = -1
    end = -1
    braces = 0
    in_fn = False
    for i, line in enumerate(lines):
        if line.startswith(f"fn {fn_name}(") or line.startswith(f"pub fn {fn_name}("):
            start = i
            in_fn = True
        if in_fn:
            braces += line.count('{')
            braces -= line.count('}')
            if braces == 0:
                end = i + 1
                break
    return start, end

funcs_to_extract = [
    "draw_filter_bar",
    "draw_status_legend",
    "render_filter_tab",
    "header_filter_hit_test",
    "render_search_input"
]

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

to_delete = []

for fn in funcs_to_extract:
    s, e = get_fn_bounds(lines, fn)
    if s != -1 and e != -1:
        extracted = "".join(lines[s:e])
        if fn != "render_filter_tab" and fn != "render_search_input":
            extracted = extracted.replace(f"fn {fn}", f"pub fn {fn}")
        elif fn == "render_filter_tab" or fn == "render_search_input":
            # Just keep them private in header.rs if they are only used there, but actually render_search_input is only used in header.rs? Wait, no, it's used in draw_filter_bar
            pass
        header_code += extracted + "\n"
        to_delete.append((s, e))

with open("src/cli/tui/components/header.rs", "w") as f:
    f.write(header_code)

to_delete.sort(reverse=True)
for s, e in to_delete:
    del lines[s:e]

with open("src/cli/tui/ui.rs", "w") as f:
    f.writelines(lines)

