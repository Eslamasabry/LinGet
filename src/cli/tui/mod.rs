#[cfg(feature = "tui")]
mod app;
#[cfg(feature = "tui")]
mod ui;

#[cfg(feature = "tui")]
pub use app::run;

#[cfg(not(feature = "tui"))]
pub async fn run() -> anyhow::Result<()> {
    anyhow::bail!(
        "TUI feature not enabled. Rebuild with: cargo build --features tui"
    );
}
