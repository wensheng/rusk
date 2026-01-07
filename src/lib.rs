//! Rusk - A modern YAML-based task runner
//!
//! Rusk is a rewrite of Tusk in Rust, providing a fast and reliable way to define
//! and execute project tasks using simple YAML configuration files.

// Public modules
pub mod cli;
pub mod config;
pub mod error;
pub mod runner;
pub mod ui;
pub mod utils;

// Re-export commonly used types
pub use error::{Result, RuskError};

/// Current version of Rusk
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
