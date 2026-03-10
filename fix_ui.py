import re

with open("src/cli/tui/ui.rs", "r") as f:
    content = f.read()

# Make functions pub
content = content.replace("fn compose_left_right<'a>", "pub fn compose_left_right<'a>")
content = content.replace("fn spans_width(spans:", "pub fn spans_width(spans:")

def remove_between(content, start_str, end_str):
    start_idx = content.find(start_str)
    if start_idx == -1: return content
    
    end_idx = content.find("fn ", content.find(end_str))
    if end_idx == -1: end_idx = len(content)
    
    return content[:start_idx] + content[end_idx:]

content = remove_between(content, "fn draw_filter_bar", "fn queue_hint_hit_test")
content = remove_between(content, "pub fn header_filter_hit_test", "pub fn preflight_modal_hit_test")
content = remove_between(content, "fn render_search_input", "fn draw_main_content")

with open("src/cli/tui/ui.rs", "w") as f:
    f.write(content)

