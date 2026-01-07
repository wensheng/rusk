//! Configuration validation
//!
//! This module provides validation logic for configuration files.

use crate::config::types::{Config, Task};
use crate::error::{ConfigError, ConfigResult};
use std::collections::HashSet;

/// Validate a complete configuration
pub fn validate_config(config: &Config) -> ConfigResult<()> {
    // Validate each task
    for (name, task) in &config.tasks {
        validate_task(name, task)?;
    }

    // Check for circular dependencies between tasks
    detect_circular_task_dependencies(config)?;

    Ok(())
}

/// Validate a single task
pub fn validate_task(_name: &str, task: &Task) -> ConfigResult<()> {
    // Check source/target consistency
    if !task.source.is_empty() && task.target.is_empty() {
        return Err(ConfigError::SourceWithoutTarget);
    }
    if !task.target.is_empty() && task.source.is_empty() {
        return Err(ConfigError::TargetWithoutSource);
    }

    // Check for duplicate names between args and options
    for arg_name in task.args.keys() {
        if task.options.contains_key(arg_name) {
            return Err(ConfigError::DuplicateNames(arg_name.clone()));
        }
    }

    // Validate option types
    for (_opt_name, option) in &task.options {
        validate_option_type(&option.option_type)?;
    }

    Ok(())
}

/// Validate an option type string
fn validate_option_type(option_type: &str) -> ConfigResult<()> {
    match option_type {
        "string" | "bool" | "boolean" | "int" | "integer" | "float" => Ok(()),
        _ => Err(ConfigError::Invalid(format!(
            "Invalid option type: {}. Must be one of: string, bool, int, float",
            option_type
        ))),
    }
}

/// Detect circular dependencies in task subtask relationships
fn detect_circular_task_dependencies(config: &Config) -> ConfigResult<()> {
    for (task_name, _) in &config.tasks {
        let mut visited = HashSet::new();
        let mut stack = Vec::new();
        check_task_cycle(config, task_name, &mut visited, &mut stack)?;
    }
    Ok(())
}

/// Recursively check for cycles in task dependencies
fn check_task_cycle(
    config: &Config,
    task_name: &str,
    visited: &mut HashSet<String>,
    stack: &mut Vec<String>,
) -> ConfigResult<()> {
    // Check if we've found a cycle
    if stack.contains(&task_name.to_string()) {
        stack.push(task_name.to_string());
        return Err(ConfigError::CircularDependency(stack.join(" -> ")));
    }

    // Skip if already fully processed
    if visited.contains(task_name) {
        return Ok(());
    }

    // Get the task
    let task = config.tasks.get(task_name).ok_or_else(|| {
        ConfigError::TaskNotFound(task_name.to_string())
    })?;

    // Add to stack
    stack.push(task_name.to_string());

    // Check all subtasks
    for run in &task.run {
        let subtasks = match run {
            crate::config::types::Run::SimpleCommand(_) => vec![],
            crate::config::types::Run::Complex(item) => item
                .task
                .iter()
                .map(|st| match st {
                    crate::config::types::SubTask::Simple(name) => name.clone(),
                    crate::config::types::SubTask::Complex(detail) => detail.name.clone(),
                })
                .collect(),
        };

        for subtask_name in subtasks {
            check_task_cycle(config, &subtask_name, visited, stack)?;
        }
    }

    // Also check finally blocks
    for run in &task.finally {
        let subtasks = match run {
            crate::config::types::Run::SimpleCommand(_) => vec![],
            crate::config::types::Run::Complex(item) => item
                .task
                .iter()
                .map(|st| match st {
                    crate::config::types::SubTask::Simple(name) => name.clone(),
                    crate::config::types::SubTask::Complex(detail) => detail.name.clone(),
                })
                .collect(),
        };

        for subtask_name in subtasks {
            check_task_cycle(config, &subtask_name, visited, stack)?;
        }
    }

    // Remove from stack and mark as visited
    stack.pop();
    visited.insert(task_name.to_string());

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::types::{Arg, Run, RunItem, SubTask, SubTaskDetail, TaskOption};
    use std::collections::HashMap;

    #[test]
    fn test_validate_source_without_target() {
        let mut config = Config {
            name: None,
            usage: None,
            tasks: HashMap::new(),
            interpreter: None,
        };

        let task = Task {
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

        config.tasks.insert("test".to_string(), task);

        let result = validate_config(&config);
        assert!(result.is_err());
        assert!(matches!(result, Err(ConfigError::SourceWithoutTarget)));
    }

    #[test]
    fn test_validate_duplicate_names() {
        let mut config = Config {
            name: None,
            usage: None,
            tasks: HashMap::new(),
            interpreter: None,
        };

        let mut args = HashMap::new();
        args.insert(
            "name".to_string(),
            Arg {
                usage: None,
                default: None,
                required: false,
                private: false,
            },
        );

        let mut options = HashMap::new();
        options.insert(
            "name".to_string(),
            TaskOption {
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

        let task = Task {
            usage: None,
            description: None,
            private: false,
            quiet: false,
            args,
            options,
            run: vec![],
            finally: vec![],
            source: vec![],
            target: vec![],
            include: None,
        };

        config.tasks.insert("test".to_string(), task);

        let result = validate_config(&config);
        assert!(result.is_err());
        assert!(matches!(result, Err(ConfigError::DuplicateNames(_))));
    }

    #[test]
    fn test_validate_invalid_option_type() {
        let option = TaskOption {
            usage: None,
            short: None,
            option_type: "invalid_type".to_string(),
            default: None,
            required: false,
            rewrite: None,
            environment: None,
            private: false,
        };

        let result = validate_option_type(&option.option_type);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_valid_option_types() {
        for opt_type in &["string", "bool", "boolean", "int", "integer", "float"] {
            let result = validate_option_type(opt_type);
            assert!(result.is_ok(), "Failed for type: {}", opt_type);
        }
    }

    #[test]
    fn test_detect_circular_dependency() {
        let mut config = Config {
            name: None,
            usage: None,
            tasks: HashMap::new(),
            interpreter: None,
        };

        // Create task A that depends on task B
        let task_a = Task {
            usage: None,
            description: None,
            private: false,
            quiet: false,
            args: HashMap::new(),
            options: HashMap::new(),
            run: vec![Run::Complex(RunItem {
                when: vec![],
                command: vec![],
                task: vec![SubTask::Simple("b".to_string())],
                set_environment: HashMap::new(),
            })],
            finally: vec![],
            source: vec![],
            target: vec![],
            include: None,
        };

        // Create task B that depends on task A (circular!)
        let task_b = Task {
            usage: None,
            description: None,
            private: false,
            quiet: false,
            args: HashMap::new(),
            options: HashMap::new(),
            run: vec![Run::Complex(RunItem {
                when: vec![],
                command: vec![],
                task: vec![SubTask::Simple("a".to_string())],
                set_environment: HashMap::new(),
            })],
            finally: vec![],
            source: vec![],
            target: vec![],
            include: None,
        };

        config.tasks.insert("a".to_string(), task_a);
        config.tasks.insert("b".to_string(), task_b);

        let result = validate_config(&config);
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(ConfigError::CircularDependency(_))
        ));
    }

    #[test]
    fn test_validate_valid_config() {
        let mut config = Config {
            name: Some("test-app".to_string()),
            usage: Some("Test application".to_string()),
            tasks: HashMap::new(),
            interpreter: None,
        };

        let task = Task {
            usage: Some("Test task".to_string()),
            description: None,
            private: false,
            quiet: false,
            args: HashMap::new(),
            options: HashMap::new(),
            run: vec![Run::SimpleCommand("echo test".to_string())],
            finally: vec![],
            source: vec![],
            target: vec![],
            include: None,
        };

        config.tasks.insert("test".to_string(), task);

        let result = validate_config(&config);
        assert!(result.is_ok());
    }
}
