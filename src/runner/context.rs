//! Execution context for task running
//!
//! The context tracks all the state needed during task execution.

use std::collections::HashMap;
use std::env;
use std::path::PathBuf;

/// Execution context that tracks state during task execution
pub struct Context {
    /// Current working directory
    pub working_dir: PathBuf,

    /// Configuration file path
    pub config_path: Option<PathBuf>,

    /// Variables (from options, args, set-environment, etc.)
    pub vars: HashMap<String, String>,

    /// Custom interpreter (e.g., ["bash", "-c"])
    pub interpreter: Vec<String>,

    /// Stack of tasks being executed (for detecting recursion)
    pub task_stack: Vec<String>,

    /// Verbosity level
    pub verbosity: Verbosity,
}

/// Verbosity levels for output
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Verbosity {
    Silent = 0,
    Quiet = 1,
    Normal = 2,
    Verbose = 3,
}

impl Context {
    /// Create a new context with default settings
    pub fn new() -> Self {
        Context {
            working_dir: env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            config_path: None,
            vars: HashMap::new(),
            interpreter: vec!["sh".to_string(), "-c".to_string()],
            task_stack: Vec::new(),
            verbosity: Verbosity::Normal,
        }
    }

    /// Create a context with a specific working directory
    pub fn with_working_dir(mut self, dir: PathBuf) -> Self {
        self.working_dir = dir;
        self
    }

    /// Set the configuration file path
    pub fn with_config_path(mut self, path: PathBuf) -> Self {
        self.config_path = Some(path);
        self
    }

    /// Set variables
    pub fn with_vars(mut self, vars: HashMap<String, String>) -> Self {
        self.vars = vars;
        self
    }

    /// Set a single variable
    pub fn set_var(&mut self, key: String, value: String) {
        self.vars.insert(key, value);
    }

    /// Get a variable value
    pub fn get_var(&self, key: &str) -> Option<&String> {
        self.vars.get(key)
    }

    /// Set the interpreter
    pub fn with_interpreter(mut self, interpreter: Vec<String>) -> Self {
        self.interpreter = interpreter;
        self
    }

    /// Set verbosity level
    pub fn with_verbosity(mut self, verbosity: Verbosity) -> Self {
        self.verbosity = verbosity;
        self
    }

    /// Push a task onto the execution stack
    pub fn push_task(&mut self, task_name: String) {
        self.task_stack.push(task_name);
    }

    /// Pop a task from the execution stack
    pub fn pop_task(&mut self) -> Option<String> {
        self.task_stack.pop()
    }

    /// Check if a task is in the execution stack (detect recursion)
    pub fn is_task_in_stack(&self, task_name: &str) -> bool {
        self.task_stack.iter().any(|t| t == task_name)
    }

    /// Get the current task name (top of stack)
    pub fn current_task(&self) -> Option<&String> {
        self.task_stack.last()
    }

    /// Get all task names in the stack
    pub fn task_names(&self) -> Vec<String> {
        self.task_stack.clone()
    }

    /// Get the directory for the config file (or current dir)
    pub fn config_dir(&self) -> PathBuf {
        self.config_path
            .as_ref()
            .and_then(|p| p.parent())
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| self.working_dir.clone())
    }

    /// Print info message
    pub fn print_info(&self, message: &str) {
        if self.verbosity >= Verbosity::Normal {
            eprintln!("[INFO] {}", message);
        }
    }

    /// Print error message
    pub fn print_error(&self, message: &str) {
        if self.verbosity >= Verbosity::Quiet {
            eprintln!("[ERROR] {}", message);
        }
    }

    /// Print debug message (only in verbose mode)
    pub fn print_debug(&self, message: &str) {
        if self.verbosity >= Verbosity::Verbose {
            eprintln!("[DEBUG] {}", message);
        }
    }

    /// Print task start message
    pub fn print_task_start(&self, task_name: &str) {
        self.print_info(&format!("Running task: {}", task_name));
    }

    /// Print task complete message
    pub fn print_task_complete(&self, task_name: &str) {
        self.print_debug(&format!("Task completed: {}", task_name));
    }

    /// Print task skip message
    pub fn print_task_skip(&self, task_name: &str, reason: &str) {
        self.print_debug(&format!("Skipping task '{}': {}", task_name, reason));
    }
}

impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_new() {
        let ctx = Context::new();
        assert_eq!(ctx.verbosity, Verbosity::Normal);
        assert_eq!(ctx.interpreter, vec!["sh", "-c"]);
        assert!(ctx.vars.is_empty());
        assert!(ctx.task_stack.is_empty());
    }

    #[test]
    fn test_context_with_vars() {
        let mut vars = HashMap::new();
        vars.insert("key".to_string(), "value".to_string());

        let ctx = Context::new().with_vars(vars);
        assert_eq!(ctx.get_var("key"), Some(&"value".to_string()));
    }

    #[test]
    fn test_context_set_var() {
        let mut ctx = Context::new();
        ctx.set_var("test".to_string(), "value".to_string());
        assert_eq!(ctx.get_var("test"), Some(&"value".to_string()));
    }

    #[test]
    fn test_task_stack() {
        let mut ctx = Context::new();

        assert!(!ctx.is_task_in_stack("task1"));

        ctx.push_task("task1".to_string());
        assert!(ctx.is_task_in_stack("task1"));
        assert_eq!(ctx.current_task(), Some(&"task1".to_string()));

        ctx.push_task("task2".to_string());
        assert!(ctx.is_task_in_stack("task2"));
        assert_eq!(ctx.current_task(), Some(&"task2".to_string()));

        let popped = ctx.pop_task();
        assert_eq!(popped, Some("task2".to_string()));
        assert!(!ctx.is_task_in_stack("task2"));
        assert_eq!(ctx.current_task(), Some(&"task1".to_string()));
    }

    #[test]
    fn test_verbosity_levels() {
        assert!(Verbosity::Verbose > Verbosity::Normal);
        assert!(Verbosity::Normal > Verbosity::Quiet);
        assert!(Verbosity::Quiet > Verbosity::Silent);
    }

    #[test]
    fn test_with_interpreter() {
        let ctx = Context::new().with_interpreter(vec!["bash".to_string(), "-c".to_string()]);
        assert_eq!(ctx.interpreter, vec!["bash", "-c"]);
    }

    #[test]
    fn test_with_verbosity() {
        let ctx = Context::new().with_verbosity(Verbosity::Verbose);
        assert_eq!(ctx.verbosity, Verbosity::Verbose);
    }
}
