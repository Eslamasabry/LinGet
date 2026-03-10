with open("temp_ui.rs", "r") as f:
    lines = f.readlines()

# Ranges to delete:
# render_search_input: lines 552-575
# header_filter_hit_test: lines 285-372
# render_filter_tab: lines 227-284
# draw_status_legend: lines 212-226
# draw_filter_bar: lines 110-211

del lines[551:575]
del lines[284:372]
del lines[226:284]
del lines[211:226]
del lines[109:211]

with open("src/cli/tui/ui.rs", "w") as f:
    f.writelines(lines)
