use ratatui::{layout::Rect, Frame};
use crossterm::event::Event;
use crate::cli::tui::app::App;

/// A trait for generic TUI components.
pub trait Component {
    /// Initialize the component
    fn init(&mut self, _area: Rect) -> anyhow::Result<()> {
        Ok(())
    }
    
    /// Draw the component on the given area
    fn draw(&mut self, f: &mut Frame, area: Rect, app: &mut App);

    /// Handle user events (keyboard/mouse)
    fn handle_event(&mut self, _event: &Event) -> anyhow::Result<Option<ComponentAction>> {
        Ok(None)
    }
}

/// Actions that components can return to signal state changes to the App
pub enum ComponentAction {
    /// Request to change focus
    ChangeFocus(crate::cli::tui::state::filters::Focus),
    /// Request to execute a command
    ExecuteCommand(crate::cli::tui::app::CommandId),
    /// Trigger a refresh of the UI or data
    Refresh,
    /// Quit the application
    Quit,
}
