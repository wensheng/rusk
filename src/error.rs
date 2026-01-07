//! Error types for Rusk

use std::io;
use std::path::PathBuf;
use thiserror::Error;

/// Result type alias for Rusk operations
pub type Result<T> = std::result::Result<T, RuskError>;

/// Main error type for Rusk
#[derive(Error, Debug)]
pub enum RuskError {
    /// Configuration-related errors
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),

    /// Task execution errors
    #[error("Execution error: {0}")]
    Execution(#[from] ExecutionError),

    /// Variable interpolation errors
    #[error("Interpolation error: {0}")]
    Interpolation(#[from] InterpolationError),

    /// I/O errors
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    /// YAML parsing errors
    #[error("YAML parsing error: {0}")]
    Yaml(#[from] serde_yaml::Error),
}

/// Configuration parsing and validation errors
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Failed to find config file (searched: {0})")]
    NotFound(String),

    #[error("Invalid configuration: {0}")]
    Invalid(String),

    #[error("Task source cannot be defined without target")]
    SourceWithoutTarget,

    #[error("Task target cannot be defined without source")]
    TargetWithoutSource,

    #[error("Argument and option '{0}' must have unique names within a task")]
    DuplicateNames(String),

    #[error("Task '{0}' is not defined")]
    TaskNotFound(String),

    #[error("Circular dependency detected: {0}")]
    CircularDependency(String),

    #[error("Failed to include file '{path}': {error}")]
    IncludeFile { path: PathBuf, error: String },
}

/// Task execution errors
#[derive(Error, Debug)]
pub enum ExecutionError {
    #[error("Command failed with exit code {0:?}")]
    CommandFailed(Option<i32>),

    #[error("Failed condition: {0}")]
    FailedCondition(String),

    #[error("Option '{0}' is required but not provided")]
    MissingOption(String),

    #[error("Invalid option value for '{name}': {error}")]
    InvalidOption { name: String, error: String },

    #[error("Cache error: {0}")]
    Cache(String),

    #[error("Environment error: {0}")]
    Environment(String),
}

/// Variable interpolation errors
#[derive(Error, Debug)]
pub enum InterpolationError {
    #[error("Variable '{0}' is not defined")]
    UndefinedVariable(String),

    #[error("Invalid interpolation syntax: {0}")]
    InvalidSyntax(String),

    #[error("Recursive interpolation detected")]
    RecursiveInterpolation,
}

/// Specialized result type for configuration operations
pub type ConfigResult<T> = std::result::Result<T, ConfigError>;

/// Specialized result type for execution operations
pub type ExecutionResult<T> = std::result::Result<T, ExecutionError>;

/// Specialized result type for interpolation operations
pub type InterpolationResult<T> = std::result::Result<T, InterpolationError>;

/// Helper function to determine if an error represents a failed condition
/// (which should be treated as a skip, not a hard error)
pub fn is_failed_condition(err: &ExecutionError) -> bool {
    matches!(err, ExecutionError::FailedCondition(_))
}
