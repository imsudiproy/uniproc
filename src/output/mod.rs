//! Output and presentation layer.
//!
//! Exposes modules responsible for rendering the collected data to the user,
//! either via the interactive `tui` dashboard, or through headless data
//! exporters (`csv`, `json`).

pub mod csv;
pub mod json;
pub mod tui;
