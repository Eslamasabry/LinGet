use crate::cli::tui::app::App;
use crate::cli::tui::theme::{
    accent, active, badge_installed, badge_update, border_unfocused, dim, error, key_hint, muted,
    palette, source_color, success, warning,
};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

// Visual glyphs — kept as a single vocabulary so the dashboard has a consistent
// "voice". Prefer a single-width glyph + trailing space for alignment.
const GLYPH_LOGO: &str = "❖";
const GLYPH_TOTAL: &str = "∑";
const GLYPH_INSTALLED: &str = "✓";
const GLYPH_UPDATE: &str = "↑";
const GLYPH_FAVORITE: &str = "★";
const GLYPH_SECURITY: &str = "⚠";
const GLYPH_SPARKLE: &str = "✦";

pub fn draw_dashboard(frame: &mut Frame, app: &App, area: Rect) {
    let total = app.filter_counts[0];
    let health_glyph = system_health_glyph(app);
    let title = format!(
        " {} LinGet  ·  {} catalog  {} ",
        GLYPH_LOGO, total, health_glyph,
    );

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_unfocused())
        .border_set(crate::cli::tui::theme::ROUNDED)
        .title(title)
        .title_alignment(Alignment::Left)
        .style(dim());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height < 4 {
        let hint = Paragraph::new(Line::from(vec![
            Span::styled(GLYPH_SPARKLE, accent()),
            Span::raw(" Dashboard collapsed — expand the pane to see stats"),
        ]))
        .style(dim())
        .alignment(Alignment::Center);
        frame.render_widget(hint, inner);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Stat row (icons + counts)
            Constraint::Length(1), // Divider accent
            Constraint::Min(1),    // Source update bars
            Constraint::Length(1), // Call-to-action banner
        ])
        .split(inner);

    // Stat row
    frame.render_widget(
        Paragraph::new(build_stat_line(app)).alignment(Alignment::Center),
        chunks[0],
    );

    // Divider accent — a thin gradient bar that hints at system health colour.
    frame.render_widget(
        Paragraph::new(build_divider(chunks[1].width as usize, health_accent(app)))
            .alignment(Alignment::Center),
        chunks[1],
    );

    // Source update bars
    let bar_lines = build_source_bars(app, chunks[2].width as usize);
    frame.render_widget(Paragraph::new(bar_lines), chunks[2]);

    // Call-to-action banner
    frame.render_widget(
        Paragraph::new(build_cta_line(app)).alignment(Alignment::Left),
        chunks[3],
    );
}

/// Compose the centred stat row.
fn build_stat_line(app: &App) -> Line<'static> {
    let total = app.filter_counts[0];
    let installed = app.filter_counts[1];
    let updates = app.filter_counts[2];
    let favorites = app.filter_counts[3];
    let security = app.filter_counts[4];

    let mut spans: Vec<Span<'static>> = Vec::new();
    spans.extend(stat_chip(GLYPH_TOTAL, total, "total", accent_style()));
    spans.push(Span::raw("   "));
    spans.extend(stat_chip(
        GLYPH_INSTALLED,
        installed,
        "installed",
        badge_installed(),
    ));
    spans.push(Span::raw("   "));
    let update_style = if updates > 0 { badge_update() } else { dim() };
    spans.extend(stat_chip(GLYPH_UPDATE, updates, "updates", update_style));
    spans.push(Span::raw("   "));
    let fav_style = if favorites > 0 { warning() } else { dim() };
    spans.extend(stat_chip(GLYPH_FAVORITE, favorites, "favorites", fav_style));
    spans.push(Span::raw("   "));
    let sec_style = if security > 0 {
        pulse_danger_style(app)
    } else {
        dim()
    };
    spans.extend(stat_chip(GLYPH_SECURITY, security, "security", sec_style));

    Line::from(spans)
}

fn stat_chip(glyph: &str, count: usize, label: &str, style: Style) -> Vec<Span<'static>> {
    vec![
        Span::styled(format!("{} ", glyph), style.add_modifier(Modifier::BOLD)),
        Span::styled(count.to_string(), style.add_modifier(Modifier::BOLD)),
        Span::raw(" "),
        Span::styled(label.to_string(), muted()),
    ]
}

fn accent_style() -> Style {
    accent()
}

/// Danger pulse — used only when unaddressed security updates exist. The
/// subtle alpha-breathing effect nudges the eye without burning cycles.
fn pulse_danger_style(app: &App) -> Style {
    let active_theme = active();
    if active_theme.monochrome {
        return error();
    }
    // Breathe the theme's red toward white and back, so the pulse stays
    // on-palette for every theme instead of a hardcoded RGB.
    let phase = (app.tick % 16) as f32 / 16.0 * std::f32::consts::TAU;
    let t = (phase.sin() * 0.5 + 0.5) * 0.3;
    Style::default()
        .fg(blend(palette::RED(), palette::WHITE(), t))
        .add_modifier(Modifier::BOLD)
}

/// A gradient divider spanning `width` cells, keyed to the system health
/// colour (green when healthy, amber when updates pending, red when security
/// updates need attention).
fn build_divider(width: usize, accent: Color) -> Line<'static> {
    let width = width.saturating_sub(2).max(1);
    let mut spans = Vec::with_capacity(width + 2);
    spans.push(Span::styled(" ", Style::default()));
    for i in 0..width {
        // Fade toward the palette's dark-gray at the edges for a glow feel.
        let t = ((i as f32 / width.max(1) as f32) * 2.0 - 1.0).abs();
        let a = 1.0 - t;
        let col = blend(palette::INACTIVE_BORDER(), accent, a);
        spans.push(Span::styled("─", Style::default().fg(col)));
    }
    Line::from(spans)
}

fn blend(a: Color, b: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    match (a, b) {
        (Color::Rgb(ar, ag, ab), Color::Rgb(br, bg, bb)) => {
            Color::Rgb(lerp(ar, br, t), lerp(ag, bg, t), lerp(ab, bb, t))
        }
        _ => b,
    }
}

fn lerp(a: u8, b: u8, t: f32) -> u8 {
    ((a as f32) * (1.0 - t) + (b as f32) * t)
        .round()
        .clamp(0.0, 255.0) as u8
}

/// Source update bars: each active backend gets a labelled, gradient-filled
/// horizontal bar showing its share of pending updates relative to the busiest
/// source. Empty sources collapse to a thin dim track.
fn build_source_bars(app: &App, width: usize) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let sources = app.visible_sources();
    if sources.is_empty() {
        lines.push(Line::from(Span::styled(
            format!(
                "  {} No sources enabled — press 'o' to configure",
                GLYPH_SPARKLE
            ),
            dim(),
        )));
        return lines;
    }

    let label_width = 10usize;
    let count_width = 4usize;
    let gutter = 4usize; // leading + trailing padding
                         // "  (NNNN installed)" suffix must fit inside the row too, or the bar
                         // pushes it past the border and it renders clipped.
    let installed_suffix_width = 18usize;
    let bar_width = width
        .saturating_sub(label_width + count_width + gutter + installed_suffix_width)
        .max(4);

    let max_updates = sources
        .iter()
        .map(|s| app.source_counts.get(s).copied().unwrap_or([0; 6])[2])
        .max()
        .unwrap_or(0)
        .max(1);

    for source in &sources {
        let counts = app.source_counts.get(source).copied().unwrap_or([0; 6]);
        let updates = counts[2];
        let installed = counts[1];

        let ratio = updates as f64 / max_updates as f64;
        let filled = ((ratio * bar_width as f64).round() as usize).min(bar_width);

        let source_style = source_color(*source);
        let src_fg = extract_fg(source_style).unwrap_or(palette::CYAN());
        let bar = gradient_bar(bar_width, filled, src_fg);

        let mut spans = vec![
            Span::raw("  "),
            Span::styled(
                format!(
                    "{:<width$}",
                    crate::cli::tui::format::truncate_to_width(
                        &source.to_string(),
                        label_width - 2
                    ),
                    width = label_width - 2
                ),
                source_style.add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
        ];
        spans.extend(bar);
        spans.push(Span::raw(" "));
        spans.push(Span::styled(
            format!("{:>3}", updates),
            if updates > 0 { accent_style() } else { dim() },
        ));
        spans.push(Span::styled(
            format!("  ({} installed)", installed),
            muted().add_modifier(Modifier::DIM),
        ));
        lines.push(Line::from(spans));
    }
    lines
}

fn extract_fg(style: Style) -> Option<Color> {
    style.fg
}

/// Gradient bar: filled cells shade from the source colour to a bright accent,
/// giving an at-a-glance sense of "warm = busy" while staying on-palette.
fn gradient_bar(width: usize, filled: usize, color: Color) -> Vec<Span<'static>> {
    let mut spans = Vec::with_capacity(width);
    let track = palette::INACTIVE_BORDER();
    for i in 0..width {
        if i < filled {
            let t = (i as f32 / width.max(1) as f32).clamp(0.0, 1.0);
            let bright = blend(color, palette::WHITE(), 0.35 * t);
            spans.push(Span::styled("▰", Style::default().fg(bright)));
        } else {
            spans.push(Span::styled("▱", Style::default().fg(track)));
        }
    }
    spans
}

/// Context-sensitive call to action at the bottom of the dashboard.
fn build_cta_line(app: &App) -> Line<'static> {
    if app.filter_counts[4] > 0 {
        Line::from(vec![
            Span::raw("  "),
            Span::styled(format!("{} ", GLYPH_SECURITY), error()),
            Span::styled(
                format!(
                    "{} security update{} ",
                    app.filter_counts[4],
                    plural(app.filter_counts[4])
                ),
                Style::default()
                    .fg(palette::WHITE())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("— press ", muted()),
            Span::styled("5", key_hint()),
            Span::styled(" to review, ", muted()),
            Span::styled("w", key_hint()),
            Span::styled(" to queue all", muted()),
        ])
    } else if app.filter_counts[2] > 0 {
        Line::from(vec![
            Span::raw("  "),
            Span::styled(format!("{} ", GLYPH_UPDATE), warning()),
            Span::styled(
                format!(
                    "{} update{} available ",
                    app.filter_counts[2],
                    plural(app.filter_counts[2])
                ),
                Style::default()
                    .fg(palette::WHITE())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("— press ", muted()),
            Span::styled("w", key_hint()),
            Span::styled(" to queue, ", muted()),
            Span::styled("3", key_hint()),
            Span::styled(" to filter", muted()),
        ])
    } else if !app.packages.is_empty() {
        Line::from(vec![
            Span::raw("  "),
            Span::styled(format!("{} ", GLYPH_SPARKLE), success()),
            Span::styled("Everything is up to date  ", success()),
            Span::styled("· press ", muted()),
            Span::styled("/", key_hint()),
            Span::styled(" to discover new packages, ", muted()),
            Span::styled("T", key_hint()),
            Span::styled(" to change theme", muted()),
        ])
    } else {
        Line::from(vec![
            Span::raw("  "),
            Span::styled(format!("{} ", app.spinner_frame()), accent()),
            Span::styled("Loading catalogue…", dim()),
        ])
    }
}

fn plural(n: usize) -> &'static str {
    if n == 1 {
        ""
    } else {
        "s"
    }
}

fn system_health_glyph(app: &App) -> Span<'static> {
    if app.filter_counts[4] > 0 {
        Span::styled(format!("{} needs attention", GLYPH_SECURITY), error())
    } else if app.filter_counts[2] > 0 {
        Span::styled(format!("{} updates pending", GLYPH_UPDATE), warning())
    } else if !app.packages.is_empty() {
        Span::styled(format!("{} healthy", GLYPH_INSTALLED), success())
    } else {
        Span::styled("…".to_string(), dim())
    }
}

fn health_accent(app: &App) -> Color {
    if app.filter_counts[4] > 0 {
        palette::RED()
    } else if app.filter_counts[2] > 0 {
        palette::YELLOW()
    } else if !app.packages.is_empty() {
        palette::GREEN()
    } else {
        palette::CYAN()
    }
}
