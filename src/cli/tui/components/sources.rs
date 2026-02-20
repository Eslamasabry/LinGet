use crate::cli::tui::app::App;
use crate::cli::tui::state::filters::Focus;
use crate::cli::tui::theme::{
    accent, row_selected, scrollbar_style, scrollbar_thumb, source_color, text,
};
use crate::cli::tui::ui::{panel_block, source_count_label, window_start};
use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{List, ListItem, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};

pub fn draw_sources_panel(frame: &mut Frame, app: &App, area: Rect) {
    let focused = app.focus == Focus::Sources && !app.queue_expanded;
    let block = panel_block(" Sources ".to_string(), focused, app.compact);
    let inner = block.inner(area);
    let visible = app.visible_sources();
    let total = visible.len() + 1;
    let selected = app.source_index();
    let visible_rows = inner.height as usize;
    let start = window_start(total, visible_rows, selected);
    let end = (start + visible_rows).min(total);

    let mut items = Vec::new();
    for idx in start..end {
        let selected_row = idx == selected;
        let (label, label_style) = if idx == 0 {
            let count_str = source_count_label(
                app.filter,
                [
                    app.filter_counts[0],
                    app.filter_counts[1],
                    app.filter_counts[2],
                    app.filter_counts[3],
                ],
            );
            (
                format!("All{}", count_str),
                if selected_row { accent() } else { text() },
            )
        } else {
            let source = visible[idx - 1];
            let counts = app
                .source_counts
                .get(&source)
                .copied()
                .unwrap_or([0, 0, 0, 0]);
            let count_str = source_count_label(app.filter, counts);
            (
                format!("{}{}", source, count_str),
                if selected_row {
                    accent()
                } else {
                    source_color(source)
                },
            )
        };
        items.push(ListItem::new(Line::from(vec![
            Span::styled(if selected_row { "▸ " } else { "  " }, row_selected()),
            Span::styled(label, label_style),
        ])));
    }

    let list = List::new(items).block(block);
    frame.render_widget(list, area);

    if total > visible_rows {
        let mut scrollbar_state = ScrollbarState::new(total).position(selected);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .style(scrollbar_style())
            .thumb_style(scrollbar_thumb());
        frame.render_stateful_widget(
            scrollbar,
            area.inner(ratatui::layout::Margin {
                vertical: 1,
                horizontal: 0,
            }),
            &mut scrollbar_state,
        );
    }
}
