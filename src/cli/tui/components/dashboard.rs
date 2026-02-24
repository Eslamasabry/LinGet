use crate::cli::tui::app::App;
use crate::cli::tui::theme::{accent, dim, source_color, text};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn draw_dashboard(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Dashboard ")
        .title_alignment(Alignment::Center)
        .style(dim());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height < 4 {
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(1),
        ])
        .split(inner);

    let title_paragraph = Paragraph::new(Line::from(Span::styled("LinGet TUI", accent())))
        .alignment(Alignment::Center);
    frame.render_widget(title_paragraph, chunks[0]);

    let summary = format!(
        "Total Packages: {} | Installed: {} | Updates: {} | Favorites: {}",
        app.filter_counts[0],
        app.filter_counts[1],
        app.filter_counts[2],
        app.filter_counts[3]
    );

    let summary_paragraph = Paragraph::new(Line::from(Span::styled(summary, text())))
        .alignment(Alignment::Center);
    frame.render_widget(summary_paragraph, chunks[1]);

    let mut lines = vec![Line::from(Span::styled("Available Sources:", dim()))];
    for (source, counts) in &app.source_counts {
        lines.push(Line::from(vec![
            Span::styled(format!("  - {}: ", source), source_color(*source)),
            Span::styled(format!("{} updates", counts[2]), accent()),
        ]));
    }
    
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled("Press F2 to browse packages.", dim())));

    frame.render_widget(Paragraph::new(lines), chunks[2]);
}