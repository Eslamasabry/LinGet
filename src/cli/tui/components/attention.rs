//! Compact Today strip for the package workspace.

use crate::cli::tui::app::App;
use crate::cli::tui::format::truncate_to_width;
use crate::cli::tui::theme::{
    accent, border_unfocused, dim, error, muted, palette, success, text, ROUNDED,
};
use crate::models::PackageStatus;
use ratatui::{
    layout::{Margin, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn draw(frame: &mut Frame, app: &App, area: Rect) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_set(ROUNDED)
        .border_style(border_unfocused());
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let security = app.filter_counts[4];
    let safe_updates = app.filter_counts[2].saturating_sub(security);
    let (_, _, _, failed, _) = app.queue_counts();
    let recommended = recommended_text(app, inner.width as usize);

    let line = Line::from(vec![
        Span::styled(" Today: ", accent()),
        Span::styled("🔒", error()),
        Span::styled(" Security ", muted()),
        Span::styled(security.to_string(), error()),
        Span::styled("   |   ", dim()),
        Span::styled("↻", Style::default().fg(palette::ORANGE())),
        Span::styled(" Safe updates ", muted()),
        Span::styled(
            safe_updates.to_string(),
            Style::default().fg(palette::ORANGE()),
        ),
        Span::styled("   |   ", dim()),
        Span::styled("✖", error()),
        Span::styled(" Failed tasks ", muted()),
        Span::styled(failed.to_string(), error()),
        Span::styled("   |   ", dim()),
        Span::styled("Recommended: ", success().add_modifier(Modifier::BOLD)),
        Span::styled(recommended, text()),
    ]);

    frame.render_widget(
        Paragraph::new(line),
        inner.inner(Margin {
            vertical: 0,
            horizontal: 1,
        }),
    );
}

fn recommended_text(app: &App, width: usize) -> String {
    let label = if let Some(package) = app.current_package() {
        if package.status == PackageStatus::UpdateAvailable {
            format!("Review {} update", package.name)
        } else {
            app.recommended_action_label()
        }
    } else {
        app.recommended_action_label()
    };
    truncate_to_width(&label, width.saturating_sub(86))
}
