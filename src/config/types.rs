//! Core configuration types
//!
//! This module defines the data structures that represent a tusk.yml configuration file.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Top-level configuration structure
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    /// Application name (optional)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Application usage description (optional)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage: Option<String>,

    /// Tasks defined in the configuration
    #[serde(default)]
    pub tasks: HashMap<String, Task>,

    /// Global interpreter to use for commands (e.g., ["sh", "-c"])
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interpreter: Option<Vec<String>>,
}

/// A task definition
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Task {
    /// Usage description for help text
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage: Option<String>,

    /// Longer description for help text
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Whether this task is private (hidden from help)
    #[serde(default)]
    pub private: bool,

    /// Whether this task should run quietly
    #[serde(default)]
    pub quiet: bool,

    /// Positional arguments for the task
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub args: HashMap<String, Arg>,

    /// Named options (flags) for the task
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub options: HashMap<String, TaskOption>,

    /// Run items to execute
    #[serde(default, deserialize_with = "deserialize_run_items")]
    pub run: Vec<Run>,

    /// Finally block - always executes, even on error
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub finally: Vec<Run>,

    /// Source files for caching
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source: Vec<String>,

    /// Target files for caching
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub target: Vec<String>,

    /// Include another file as task definition
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include: Option<String>,
}

/// A run item - can be a command, subtask, or environment setter
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Run {
    /// Simple string command
    SimpleCommand(String),

    /// Complex run item with conditionals and multiple actions
    Complex(RunItem),
}

/// A complex run item with conditions and actions
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RunItem {
    /// Conditions that must be met for this run item to execute
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub when: Vec<When>,

    /// Commands to execute
    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        deserialize_with = "deserialize_commands"
    )]
    pub command: Vec<Command>,

    /// Subtasks to execute
    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        deserialize_with = "deserialize_subtasks"
    )]
    pub task: Vec<SubTask>,

    /// Environment variables to set
    #[serde(
        rename = "set-environment",
        default,
        skip_serializing_if = "HashMap::is_empty"
    )]
    pub set_environment: HashMap<String, OptionString>,
}

/// A command to execute
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Command {
    /// Simple string command
    Simple(String),

    /// Complex command with additional options
    Complex(CommandDetail),
}

/// Detailed command specification
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CommandDetail {
    /// The command to execute
    pub exec: String,

    /// What to print when running (defaults to exec)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub print: Option<String>,

    /// Whether to suppress output
    #[serde(default)]
    pub quiet: bool,

    /// Working directory for the command
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dir: Option<String>,
}

/// A reference to a subtask to execute
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum SubTask {
    /// Simple task name
    Simple(String),

    /// Complex subtask with options
    Complex(SubTaskDetail),
}

/// Detailed subtask specification
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SubTaskDetail {
    /// Name of the task to run
    pub name: String,

    /// Options to pass to the subtask
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub options: HashMap<String, String>,
}

/// A conditional expression
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct When {
    /// Check if values are equal
    #[serde(skip_serializing_if = "Option::is_none")]
    pub equal: Option<WhenComparison>,

    /// Check if values are not equal
    #[serde(rename = "not-equal", skip_serializing_if = "Option::is_none")]
    pub not_equal: Option<WhenComparison>,

    /// Check if a command succeeds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,

    /// Check if a path exists
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exists: Option<String>,

    /// Check if environment variable is set
    #[serde(rename = "env-set", skip_serializing_if = "Option::is_none")]
    pub env_set: Option<String>,

    /// Check if environment variable is not set
    #[serde(rename = "env-not-set", skip_serializing_if = "Option::is_none")]
    pub env_not_set: Option<String>,

    /// Check if option is set
    #[serde(rename = "option-set", skip_serializing_if = "Option::is_none")]
    pub option_set: Option<String>,

    /// Check if option is not set
    #[serde(rename = "option-not-set", skip_serializing_if = "Option::is_none")]
    pub option_not_set: Option<String>,
}

/// A comparison for when conditions
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WhenComparison {
    /// Left-hand side of comparison
    pub left: String,

    /// Right-hand side of comparison
    pub right: String,
}

/// An option (flag) definition
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TaskOption {
    /// Usage description for help text
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<String>,

    /// Short flag (single character)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub short: Option<String>,

    /// Option type (string, bool, integer, etc.)
    #[serde(rename = "type", default = "default_option_type")]
    pub option_type: String,

    /// Default value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,

    /// Required option
    #[serde(default)]
    pub required: bool,

    /// Values to pass instead of the raw option value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rewrite: Option<String>,

    /// Environment variable to read from
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment: Option<String>,

    /// Private option (hidden from help)
    #[serde(default)]
    pub private: bool,
}

fn default_option_type() -> String {
    "string".to_string()
}

/// An argument (positional parameter) definition
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Arg {
    /// Usage description for help text
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<String>,

    /// Default value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,

    /// Required argument
    #[serde(default)]
    pub required: bool,

    /// Private argument (hidden from help)
    #[serde(default)]
    pub private: bool,
}

/// An optional string value (used for environment variables)
pub type OptionString = Option<String>;

/// Custom deserializer for run items that handles both single values and arrays
fn deserialize_run_items<'de, D>(deserializer: D) -> Result<Vec<Run>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;
    use serde_yaml::Value;

    let value = Value::deserialize(deserializer)?;

    match value {
        // Single string command
        Value::String(s) => Ok(vec![Run::SimpleCommand(s)]),
        // Array of run items
        Value::Sequence(seq) => {
            let mut runs = Vec::new();
            for item in seq {
                let run = Run::deserialize(item).map_err(D::Error::custom)?;
                runs.push(run);
            }
            Ok(runs)
        }
        // Null or not present
        Value::Null => Ok(Vec::new()),
        _ => Err(D::Error::custom("run must be a string or array")),
    }
}

/// Custom deserializer for commands that handles both single values and arrays
fn deserialize_commands<'de, D>(deserializer: D) -> Result<Vec<Command>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;
    use serde_yaml::Value;

    let value = Value::deserialize(deserializer)?;

    match value {
        // Single string or complex command
        Value::String(s) => Ok(vec![Command::Simple(s)]),
        Value::Mapping(_) => {
            let cmd = Command::deserialize(value).map_err(D::Error::custom)?;
            Ok(vec![cmd])
        }
        // Array of commands
        Value::Sequence(seq) => {
            let mut cmds = Vec::new();
            for item in seq {
                let cmd = Command::deserialize(item).map_err(D::Error::custom)?;
                cmds.push(cmd);
            }
            Ok(cmds)
        }
        // Null or not present
        Value::Null => Ok(Vec::new()),
        _ => Err(D::Error::custom("command must be a string, object, or array")),
    }
}

/// Custom deserializer for subtasks that handles both single values and arrays
fn deserialize_subtasks<'de, D>(deserializer: D) -> Result<Vec<SubTask>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;
    use serde_yaml::Value;

    let value = Value::deserialize(deserializer)?;

    match value {
        // Single string or complex subtask
        Value::String(s) => Ok(vec![SubTask::Simple(s)]),
        Value::Mapping(_) => {
            let task = SubTask::deserialize(value).map_err(D::Error::custom)?;
            Ok(vec![task])
        }
        // Array of subtasks
        Value::Sequence(seq) => {
            let mut tasks = Vec::new();
            for item in seq {
                let task = SubTask::deserialize(item).map_err(D::Error::custom)?;
                tasks.push(task);
            }
            Ok(tasks)
        }
        // Null or not present
        Value::Null => Ok(Vec::new()),
        _ => Err(D::Error::custom("task must be a string, object, or array")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_simple_config() {
        let yaml = r#"
tasks:
  hello:
    usage: Say hello
    run: echo "hello"
"#;
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.tasks.len(), 1);
        assert!(config.tasks.contains_key("hello"));
    }

    #[test]
    fn test_deserialize_complex_task() {
        let yaml = r#"
tasks:
  greet:
    usage: Say hello to someone
    options:
      name:
        usage: Person to greet
        default: World
    run:
      - command: echo "Hello, ${name}!"
"#;
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        let task = config.tasks.get("greet").unwrap();
        assert_eq!(task.usage, Some("Say hello to someone".to_string()));
        assert!(task.options.contains_key("name"));
    }

    #[test]
    fn test_deserialize_when_conditions() {
        let yaml = r#"
tasks:
  conditional:
    run:
      - when:
          - equal:
              left: "${env}"
              right: "production"
        command: echo "Production!"
"#;
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        let task = config.tasks.get("conditional").unwrap();
        assert_eq!(task.run.len(), 1);
    }
}
