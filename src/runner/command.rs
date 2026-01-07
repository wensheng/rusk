//! Command execution
//!
//! This module handles executing shell commands.

use crate::error::{ExecutionError, ExecutionResult};
use crate::runner::{interpolate, Command, Context};
use std::process::{Command as StdCommand, Stdio};

/// Execute a command in the given context
pub fn execute_command(cmd: &Command, ctx: &Context) -> ExecutionResult<()> {
    // Get the command string and interpolate variables
    let exec_str = interpolate(cmd.exec(), &ctx.vars).map_err(|e| {
        ExecutionError::InvalidOption {
            name: "command".to_string(),
            error: e.to_string(),
        }
    })?;

    // Print the command if not quiet
    if !cmd.is_quiet() && ctx.verbosity >= crate::runner::context::Verbosity::Normal {
        let print_str = interpolate(cmd.print(), &ctx.vars).unwrap_or_else(|_| cmd.print().to_string());
        eprintln!("[RUN] {}", print_str);
    }

    // Determine working directory
    let working_dir = if let Some(dir) = cmd.dir() {
        let interpolated_dir = interpolate(dir, &ctx.vars).map_err(|e| {
            ExecutionError::InvalidOption {
                name: "dir".to_string(),
                error: e.to_string(),
            }
        })?;
        ctx.working_dir.join(interpolated_dir)
    } else {
        ctx.working_dir.clone()
    };

    // Build the command
    let mut command = StdCommand::new(&ctx.interpreter[0]);

    // Add interpreter args (e.g., "-c" for sh/bash)
    if ctx.interpreter.len() > 1 {
        command.args(&ctx.interpreter[1..]);
    }

    // Add the actual command to execute
    command.arg(&exec_str);

    // Set working directory
    command.current_dir(&working_dir);

    // Set up stdio
    command.stdin(Stdio::inherit());
    command.stdout(Stdio::inherit());
    command.stderr(Stdio::inherit());

    // Set environment variables from context
    for (key, value) in &ctx.vars {
        command.env(key, value);
    }

    // Execute the command
    let status = command.status().map_err(|_e| {
        ExecutionError::CommandFailed(None)
    })?;

    // Check exit status
    if !status.success() {
        return Err(ExecutionError::CommandFailed(status.code()));
    }

    Ok(())
}

/// Check if a command succeeds (for when conditions)
pub fn check_command(cmd_str: &str, ctx: &Context) -> ExecutionResult<bool> {
    // Interpolate the command
    let exec_str = interpolate(cmd_str, &ctx.vars).map_err(|e| {
        ExecutionError::InvalidOption {
            name: "command".to_string(),
            error: e.to_string(),
        }
    })?;

    // Build the command
    let mut command = StdCommand::new(&ctx.interpreter[0]);

    if ctx.interpreter.len() > 1 {
        command.args(&ctx.interpreter[1..]);
    }

    command.arg(&exec_str);
    command.current_dir(&ctx.working_dir);

    // Suppress output
    command.stdout(Stdio::null());
    command.stderr(Stdio::null());

    // Execute and check status
    let status = command.status().map_err(|_| {
        ExecutionError::CommandFailed(None)
    })?;

    Ok(status.success())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_execute_simple_command() {
        let ctx = Context::new();
        let cmd = Command::Simple("echo test".to_string());

        let result = execute_command(&cmd, &ctx);
        assert!(result.is_ok());
    }

    #[test]
    fn test_execute_command_with_variables() {
        let mut vars = HashMap::new();
        vars.insert("name".to_string(), "world".to_string());

        let ctx = Context::new().with_vars(vars);
        let cmd = Command::Simple("echo ${name}".to_string());

        let result = execute_command(&cmd, &ctx);
        assert!(result.is_ok());
    }

    #[test]
    fn test_execute_failing_command() {
        let ctx = Context::new();
        let cmd = Command::Simple("false".to_string());

        let result = execute_command(&cmd, &ctx);
        assert!(result.is_err());
        assert!(matches!(result, Err(ExecutionError::CommandFailed(_))));
    }

    #[test]
    fn test_check_command_success() {
        let ctx = Context::new();
        let result = check_command("true", &ctx);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), true);
    }

    #[test]
    fn test_check_command_failure() {
        let ctx = Context::new();
        let result = check_command("false", &ctx);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), false);
    }

    #[test]
    fn test_check_command_with_variable() {
        let mut vars = HashMap::new();
        vars.insert("cmd".to_string(), "true".to_string());

        let ctx = Context::new().with_vars(vars);
        let result = check_command("${cmd}", &ctx);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), true);
    }
}
