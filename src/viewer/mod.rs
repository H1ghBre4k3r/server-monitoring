//! TUI Dashboard Module
//!
//! Provides a terminal user interface for monitoring servers and services.

#[cfg(feature = "dashboard")]
mod app;
#[cfg(feature = "dashboard")]
mod config;
#[cfg(feature = "dashboard")]
mod state;
#[cfg(feature = "dashboard")]
mod ui;
#[cfg(feature = "dashboard")]
mod websocket;

#[cfg(feature = "dashboard")]
pub use app::App;
#[cfg(feature = "dashboard")]
pub use config::Config;
