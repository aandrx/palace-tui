// TUI module - handles terminal UI rendering and input
mod app;
mod runner;

pub use app::{App, AppMode};
pub use runner::run;
