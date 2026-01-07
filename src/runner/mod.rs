//! Task execution engine
//!
//! This module handles the execution of tasks, including command running,
//! conditional logic, and dependency resolution.

pub mod command;
pub mod context;
pub mod interpolate;
pub mod task;
pub mod when;

// Module declarations (to be implemented in later phases)
// pub mod run;
// pub mod option;
// pub mod cache;
// pub mod dependencies;

// Re-export main types
pub use command::*;
pub use context::*;
pub use interpolate::*;
pub use task::*;
pub use when::*;
