//! Task execution types and logic
//!
//! This module contains the runtime representation of tasks and execution logic.

use crate::config;
use crate::error::{ConfigError, ConfigResult, ExecutionError, ExecutionResult};
use crate::runner::{evaluate_when_list, execute_command, interpolate, Context};
use std::collections::HashMap;

/// Runtime task representation
///
/// This differs from config::Task by including computed fields needed during execution
#[derive(Debug, Clone)]
pub struct Task {
    /// Task name
    pub name: String,

    /// Usage description
    pub usage: Option<String>,

    /// Longer description
    pub description: Option<String>,

    /// Whether this task is private
    pub private: bool,

    /// Whether this task should run quietly
    pub quiet: bool,

    /// Positional arguments
    pub args: HashMap<String, Arg>,

    /// Named options
    pub options: HashMap<String, TaskOption>,

    /// Run items to execute
    pub run: Vec<Run>,

    /// Finally block
    pub finally: Vec<Run>,

    /// Source files for caching
    pub source: Vec<String>,

    /// Target files for caching
    pub target: Vec<String>,

    /// Resolved variable values for this task execution
    pub vars: HashMap<String, String>,
}

impl Task {
    /// Create a new task from configuration
    pub fn from_config(name: String, config: config::Task) -> ConfigResult<Self> {
        // Validate task configuration
        Self::validate_config(&config)?;

        Ok(Task {
            name,
            usage: config.usage,
            description: config.description,
            private: config.private,
            quiet: config.quiet,
            args: config
                .args
                .into_iter()
                .map(|(k, v)| (k.clone(), Arg::from_config(k, v)))
                .collect(),
            options: config
                .options
                .into_iter()
                .map(|(k, v)| (k.clone(), TaskOption::from_config(k, v)))
                .collect(),
            run: config.run.into_iter().map(Run::from_config).collect(),
            finally: config.finally.into_iter().map(Run::from_config).collect(),
            source: config.source,
            target: config.target,
            vars: HashMap::new(),
        })
    }

    /// Validate task configuration
    fn validate_config(config: &config::Task) -> ConfigResult<()> {
        // Check source/target consistency
        if !config.source.is_empty() && config.target.is_empty() {
            return Err(ConfigError::SourceWithoutTarget);
        }
        if !config.target.is_empty() && config.source.is_empty() {
            return Err(ConfigError::TargetWithoutSource);
        }

        // Check for duplicate names between args and options
        for (arg_name, _) in &config.args {
            if config.options.contains_key(arg_name) {
                return Err(ConfigError::DuplicateNames(arg_name.clone()));
            }
        }

        Ok(())
    }

    /// Get all dependencies (options required by when conditions, etc.)
    pub fn dependencies(&self) -> Vec<String> {
        let mut deps = Vec::new();

        // Add option dependencies
        for option in self.options.values() {
            deps.extend(option.dependencies());
        }

        // Add dependencies from when conditions
        for run in self.run.iter().chain(self.finally.iter()) {
            deps.extend(run.dependencies());
        }

        deps
    }

    /// Execute the task in the given context
    pub fn execute(&self, ctx: &mut Context) -> ExecutionResult<()> {
        // Check for recursion
        if ctx.is_task_in_stack(&self.name) {
            return Err(ExecutionError::CommandFailed(Some(1)));
        }

        // Push task onto stack
        ctx.push_task(self.name.clone());

        // Print task start
        ctx.print_task_start(&self.name);

        // Merge task vars into context
        for (key, value) in &self.vars {
            ctx.set_var(key.clone(), value.clone());
        }

        // Execute with finally block handling
        let result = self.execute_run_items(ctx);

        // Always run finally blocks
        if !self.finally.is_empty() {
            ctx.print_debug("Running finally block...");
            if let Err(e) = self.execute_finally_items(ctx) {
                // If run succeeded but finally failed, return finally error
                // If run failed, keep the run error
                if result.is_ok() {
                    ctx.pop_task();
                    return Err(e);
                }
            }
        }

        // Pop task from stack
        ctx.pop_task();

        if result.is_ok() {
            ctx.print_task_complete(&self.name);
        }

        result
    }

    /// Execute the main run items
    fn execute_run_items(&self, ctx: &mut Context) -> ExecutionResult<()> {
        for run in &self.run {
            self.execute_run_item(run, ctx)?;
        }
        Ok(())
    }

    /// Execute finally items
    fn execute_finally_items(&self, ctx: &mut Context) -> ExecutionResult<()> {
        for run in &self.finally {
            self.execute_run_item(run, ctx)?;
        }
        Ok(())
    }

    /// Execute a single run item
    fn execute_run_item(&self, run: &Run, ctx: &mut Context) -> ExecutionResult<()> {
        // Check when conditions
        if !run.when.is_empty() {
            let should_run = evaluate_when_list(&run.when, ctx)?;
            if !should_run {
                // Skip this run item
                return Ok(());
            }
        }

        // Execute commands
        for cmd in &run.commands {
            execute_command(cmd, ctx)?;
        }

        // Execute subtasks
        for subtask in &run.subtasks {
            self.execute_subtask(subtask, ctx)?;
        }

        // Set environment variables
        if !run.set_environment.is_empty() {
            for (key, value) in &run.set_environment {
                match value {
                    Some(val) => {
                        let interpolated = interpolate(val, &ctx.vars)
                            .unwrap_or_else(|_| val.clone());
                        std::env::set_var(key, &interpolated);
                        ctx.set_var(key.clone(), interpolated);
                    }
                    None => {
                        std::env::remove_var(key);
                        ctx.vars.remove(key);
                    }
                }
            }
        }

        Ok(())
    }

    /// Execute a subtask (placeholder - will be implemented with full task registry)
    fn execute_subtask(&self, _subtask: &SubTask, _ctx: &mut Context) -> ExecutionResult<()> {
        // This will be implemented when we have a task registry in the CLI
        // For now, just skip subtasks
        Ok(())
    }
}

/// Runtime representation of a run item
#[derive(Debug, Clone)]
pub struct Run {
    /// Conditions that must be met
    pub when: Vec<When>,

    /// Commands to execute
    pub commands: Vec<Command>,

    /// Subtasks to execute
    pub subtasks: Vec<SubTask>,

    /// Environment variables to set
    pub set_environment: HashMap<String, Option<String>>,
}

impl Run {
    /// Create from config
    pub fn from_config(config: config::Run) -> Self {
        match config {
            config::Run::SimpleCommand(cmd) => Run {
                when: Vec::new(),
                commands: vec![Command::Simple(cmd)],
                subtasks: Vec::new(),
                set_environment: HashMap::new(),
            },
            config::Run::Complex(item) => Run {
                when: item.when.into_iter().map(When::from_config).collect(),
                commands: item
                    .command
                    .into_iter()
                    .map(Command::from_config)
                    .collect(),
                subtasks: item
                    .task
                    .into_iter()
                    .map(SubTask::from_config)
                    .collect(),
                set_environment: item.set_environment,
            },
        }
    }

    /// Get dependencies from this run item
    pub fn dependencies(&self) -> Vec<String> {
        let mut deps = Vec::new();
        for when in &self.when {
            deps.extend(when.dependencies());
        }
        deps
    }
}

/// Runtime representation of a command
#[derive(Debug, Clone)]
pub enum Command {
    /// Simple command string
    Simple(String),

    /// Complex command with options
    Complex {
        exec: String,
        print: String,
        quiet: bool,
        dir: Option<String>,
    },
}

impl Command {
    /// Create from config
    pub fn from_config(config: config::Command) -> Self {
        match config {
            config::Command::Simple(cmd) => Command::Simple(cmd),
            config::Command::Complex(detail) => Command::Complex {
                print: detail.print.clone().unwrap_or_else(|| detail.exec.clone()),
                exec: detail.exec,
                quiet: detail.quiet,
                dir: detail.dir,
            },
        }
    }

    /// Get the command to execute
    pub fn exec(&self) -> &str {
        match self {
            Command::Simple(cmd) => cmd,
            Command::Complex { exec, .. } => exec,
        }
    }

    /// Get what to print
    pub fn print(&self) -> &str {
        match self {
            Command::Simple(cmd) => cmd,
            Command::Complex { print, .. } => print,
        }
    }

    /// Check if this command is quiet
    pub fn is_quiet(&self) -> bool {
        match self {
            Command::Simple(_) => false,
            Command::Complex { quiet, .. } => *quiet,
        }
    }

    /// Get the working directory
    pub fn dir(&self) -> Option<&str> {
        match self {
            Command::Simple(_) => None,
            Command::Complex { dir, .. } => dir.as_deref(),
        }
    }
}

/// Runtime representation of a subtask reference
#[derive(Debug, Clone)]
pub struct SubTask {
    pub name: String,
    pub options: HashMap<String, String>,
}

impl SubTask {
    pub fn from_config(config: config::SubTask) -> Self {
        match config {
            config::SubTask::Simple(name) => SubTask {
                name,
                options: HashMap::new(),
            },
            config::SubTask::Complex(detail) => SubTask {
                name: detail.name,
                options: detail.options,
            },
        }
    }
}

/// Runtime representation of a when condition
#[derive(Debug, Clone)]
pub struct When {
    pub condition: WhenCondition,
}

impl When {
    pub fn from_config(config: config::When) -> Self {
        // Determine which condition type is set
        let condition = if let Some(eq) = config.equal {
            WhenCondition::Equal {
                left: eq.left,
                right: eq.right,
            }
        } else if let Some(ne) = config.not_equal {
            WhenCondition::NotEqual {
                left: ne.left,
                right: ne.right,
            }
        } else if let Some(cmd) = config.command {
            WhenCondition::Command(cmd)
        } else if let Some(path) = config.exists {
            WhenCondition::Exists(path)
        } else if let Some(var) = config.env_set {
            WhenCondition::EnvSet(var)
        } else if let Some(var) = config.env_not_set {
            WhenCondition::EnvNotSet(var)
        } else if let Some(opt) = config.option_set {
            WhenCondition::OptionSet(opt)
        } else if let Some(opt) = config.option_not_set {
            WhenCondition::OptionNotSet(opt)
        } else {
            // Default to always true if no condition specified
            WhenCondition::Always
        };

        When { condition }
    }

    /// Get dependencies from this condition
    pub fn dependencies(&self) -> Vec<String> {
        match &self.condition {
            WhenCondition::OptionSet(name) | WhenCondition::OptionNotSet(name) => {
                vec![name.clone()]
            }
            _ => Vec::new(),
        }
    }
}

/// Types of when conditions
#[derive(Debug, Clone)]
pub enum WhenCondition {
    Equal { left: String, right: String },
    NotEqual { left: String, right: String },
    Command(String),
    Exists(String),
    EnvSet(String),
    EnvNotSet(String),
    OptionSet(String),
    OptionNotSet(String),
    Always,
}

/// Runtime representation of an option
#[derive(Debug, Clone)]
pub struct TaskOption {
    pub name: String,
    pub usage: Option<String>,
    pub short: Option<String>,
    pub option_type: OptionType,
    pub default: Option<String>,
    pub required: bool,
    pub rewrite: Option<String>,
    pub environment: Option<String>,
    pub private: bool,
}

impl TaskOption {
    pub fn from_config(name: String, config: config::TaskOption) -> Self {
        let option_type = match config.option_type.as_str() {
            "bool" | "boolean" => OptionType::Bool,
            "int" | "integer" => OptionType::Integer,
            "float" => OptionType::Float,
            _ => OptionType::String,
        };

        TaskOption {
            name,
            usage: config.usage,
            short: config.short,
            option_type,
            default: config.default,
            required: config.required,
            rewrite: config.rewrite,
            environment: config.environment,
            private: config.private,
        }
    }

    pub fn dependencies(&self) -> Vec<String> {
        // Options don't have dependencies in the basic model
        Vec::new()
    }
}

/// Option value types
#[derive(Debug, Clone, PartialEq)]
pub enum OptionType {
    String,
    Bool,
    Integer,
    Float,
}

/// Runtime representation of an argument
#[derive(Debug, Clone)]
pub struct Arg {
    pub name: String,
    pub usage: Option<String>,
    pub default: Option<String>,
    pub required: bool,
    pub private: bool,
}

impl Arg {
    pub fn from_config(name: String, config: config::Arg) -> Self {
        Arg {
            name,
            usage: config.usage,
            default: config.default,
            required: config.required,
            private: config.private,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_task_validation_source_without_target() {
        let mut config = config::Task {
            usage: None,
            description: None,
            private: false,
            quiet: false,
            args: HashMap::new(),
            options: HashMap::new(),
            run: vec![],
            finally: vec![],
            source: vec!["src.txt".to_string()],
            target: vec![],
            include: None,
        };

        let result = Task::validate_config(&config);
        assert!(result.is_err());
        assert!(matches!(result, Err(ConfigError::SourceWithoutTarget)));
    }

    #[test]
    fn test_task_validation_duplicate_names() {
        let mut config = config::Task {
            usage: None,
            description: None,
            private: false,
            quiet: false,
            args: {
                let mut args = HashMap::new();
                args.insert(
                    "name".to_string(),
                    config::Arg {
                        usage: None,
                        default: None,
                        required: false,
                        private: false,
                    },
                );
                args
            },
            options: {
                let mut opts = HashMap::new();
                opts.insert(
                    "name".to_string(),
                    config::TaskOption {
                        usage: None,
                        short: None,
                        option_type: "string".to_string(),
                        default: None,
                        required: false,
                        rewrite: None,
                        environment: None,
                        private: false,
                    },
                );
                opts
            },
            run: vec![],
            finally: vec![],
            source: vec![],
            target: vec![],
            include: None,
        };

        let result = Task::validate_config(&config);
        assert!(result.is_err());
    }
}
