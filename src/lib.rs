#[cfg(feature = "gui")]
mod app;
pub mod backend;
pub mod cli;
pub mod models;
pub mod product;
mod scheduler_runtime;

#[cfg(feature = "gui")]
mod ui;

#[cfg(feature = "gui")]
pub fn run_gui_app() {
    ui::run_relm4_app();
}
