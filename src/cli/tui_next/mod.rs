//! The reimagined LinGet TUI ("Radar").
//!
//! A blank-sheet redesign of `cli::tui`: one column of focus, urgency-tiered
//! sections, inline expansion, a queue drawer, and a palette-first key model.
//! This is the default TUI; the classic one is available via
//! `linget tui --classic`.

mod app;
pub(crate) mod cache;
mod palette;
mod ui;

pub use app::App;

use crate::backend::history_tracker::HistoryTracker;
use crate::backend::PackageManager;
use anyhow::{Context, Result};
use crossterm::{
    event::{Event, EventStream, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures::StreamExt;
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, RwLock};

pub async fn run() -> Result<()> {
    enable_raw_mode().context("failed to enable raw mode")?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen).context("failed to enter alternate screen")?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).context("failed to create terminal")?;

    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        default_hook(info);
    }));

    let result = run_loop(&mut terminal).await;

    let _ = std::panic::take_hook();
    let _ = disable_raw_mode();
    let _ = execute!(terminal.backend_mut(), LeaveAlternateScreen);
    let _ = terminal.show_cursor();

    result
}

async fn run_loop(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    let pm = Arc::new(RwLock::new(PackageManager::new_fast()));

    let tracker = HistoryTracker::load().await.ok();
    let history = Arc::new(Mutex::new(tracker));

    let (load_tx, load_rx) = tokio::sync::mpsc::channel(200);
    let (queue_tx, queue_rx) = tokio::sync::mpsc::channel(400);
    let (executor_done_tx, executor_done_rx) = tokio::sync::mpsc::channel(1);

    let mut app = App::new(
        pm.clone(),
        history,
        load_rx,
        queue_tx,
        queue_rx,
        executor_done_tx,
        executor_done_rx,
    );
    app.sources_total = pm.read().await.available_sources().len();
    app.favorites = crate::models::Config::load()
        .favorite_packages
        .into_iter()
        .collect();
    app.sync_queue_from_history().await;
    // Serve the last catalog immediately (if any), then revalidate in the
    // background. First paint should never wait on package backends.
    app.load_cached_catalog();
    App::spawn_loader(pm, load_tx);

    let mut events = EventStream::new();
    let tick_rate = Duration::from_millis(120);
    let mut tick = tokio::time::interval(tick_rate);
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        app.poll_backend().await;
        terminal
            .draw(|frame| ui::draw(frame, &mut app))
            .context("failed to draw frame")?;

        if app.should_quit {
            break;
        }

        tokio::select! {
            maybe_event = events.next() => {
                match maybe_event {
                    Some(Ok(Event::Key(key))) if key.kind != KeyEventKind::Release => {
                        app.handle_key(key).await?;
                    }
                    Some(Ok(Event::Resize(_, _))) => {}
                    _ => {}
                }
            }
            _ = tick.tick() => {
                app.tick();
            }
        }
    }

    Ok(())
}
