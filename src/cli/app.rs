//! Main CLI application

use crate::config::{parse_config_auto, parse_config_file, validate_config, Config};
use crate::error::{ConfigError, RuskError};
use crate::runner::{Context, Task, Verbosity};
use clap::{Arg, ArgAction, ArgMatches, Command};
use std::collections::HashMap;
use std::path::PathBuf;

/// CLI application
pub struct App {
    /// The clap command
    command: Command,
    /// Parsed configuration
    config: Config,
    /// Config file path
    config_path: PathBuf,
}

impl App {
    /// Create a new app from configuration file
    pub fn new() -> Result<Self, RuskError> {
        let (config, config_path) = parse_config_auto()?;
        validate_config(&config)?;

        let command = build_command(&config);

        Ok(App {
            command,
            config,
            config_path,
        })
    }

    /// Create app with a specific config file
    pub fn with_config_file(path: PathBuf) -> Result<Self, RuskError> {
        let config = parse_config_file(&path)?;
        validate_config(&config)?;

        let command = build_command(&config);

        Ok(App {
            command,
            config,
            config_path: path,
        })
    }

    /// Run the application with command line arguments
    pub fn run(mut self) -> Result<(), RuskError> {
        let matches = self.command.clone().get_matches();

        // Handle global flags first
        let verbosity = get_verbosity(&matches);

        // Check if a task was specified
        let (task_name, task_matches) = match matches.subcommand() {
            Some((name, sub_matches)) => (name.to_string(), sub_matches),
            None => {
                // No task specified, show help
                self.command.print_help().unwrap();
                println!();
                return Ok(());
            }
        };

        // Get the task from config
        let task_config = self
            .config
            .tasks
            .get(&task_name)
            .ok_or_else(|| ConfigError::TaskNotFound(task_name.clone()))?;

        // Build task with variables from CLI
        let mut task = Task::from_config(task_name.clone(), task_config.clone())?;

        // Parse options and args from CLI
        let vars = parse_task_vars(&task_config, task_matches)?;
        task.vars = vars;

        // Create execution context
        let mut ctx = Context::new()
            .with_config_path(self.config_path.clone())
            .with_verbosity(verbosity);

        // Set interpreter if specified in config
        if let Some(interpreter) = &self.config.interpreter {
            ctx = ctx.with_interpreter(interpreter.clone());
        }

        // Execute the task
        task.execute(&mut ctx)?;

        Ok(())
    }
}

/// Build the clap command from configuration
fn build_command(config: &Config) -> Command {
    let mut cmd = Command::new(config.name.clone().unwrap_or_else(|| "rusk".to_string()))
        .version(env!("CARGO_PKG_VERSION"))
        .about(config.usage.clone().unwrap_or_else(|| {
            "A modern YAML-based task runner".to_string()
        }))
        .arg(
            Arg::new("file")
                .short('f')
                .long("file")
                .value_name("FILE")
                .help("Path to rusk.yml config file")
                .global(true),
        )
        .arg(
            Arg::new("quiet")
                .short('q')
                .long("quiet")
                .help("Only print command output and errors")
                .action(ArgAction::SetTrue)
                .global(true),
        )
        .arg(
            Arg::new("silent")
                .short('s')
                .long("silent")
                .help("Print no output")
                .action(ArgAction::SetTrue)
                .global(true),
        )
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .help("Print verbose output")
                .action(ArgAction::SetTrue)
                .global(true),
        );

    // Add subcommands for each task
    for (task_name, task) in &config.tasks {
        // Skip private tasks
        if task.private {
            continue;
        }

        let mut task_cmd = Command::new(task_name)
            .about(task.usage.clone().unwrap_or_default());

        // Add long description if available
        if let Some(desc) = &task.description {
            task_cmd = task_cmd.long_about(desc.clone());
        }

        // Add arguments
        for (arg_name, arg) in &task.args {
            if arg.private {
                continue;
            }

            let mut arg_def = Arg::new(arg_name)
                .value_name(arg_name.to_uppercase())
                .help(arg.usage.clone().unwrap_or_default());

            if arg.required {
                arg_def = arg_def.required(true);
            }

            if let Some(default) = &arg.default {
                arg_def = arg_def.default_value(default);
            }

            task_cmd = task_cmd.arg(arg_def);
        }

        // Add options
        for (opt_name, opt) in &task.options {
            if opt.private {
                continue;
            }

            let mut opt_def = Arg::new(opt_name).long(opt_name).help(
                opt.usage
                    .clone()
                    .unwrap_or_else(|| format!("Option: {}", opt_name)),
            );

            // Add short flag if specified
            if let Some(short) = &opt.short {
                if let Some(c) = short.chars().next() {
                    opt_def = opt_def.short(c);
                }
            }

            // Handle different option types
            match opt.option_type.as_str() {
                "bool" | "boolean" => {
                    opt_def = opt_def.action(ArgAction::SetTrue);
                }
                _ => {
                    opt_def = opt_def.value_name(&opt_name.to_uppercase());

                    if let Some(default) = &opt.default {
                        opt_def = opt_def.default_value(default);
                    }

                    if opt.required {
                        opt_def = opt_def.required(true);
                    }
                }
            }

            task_cmd = task_cmd.arg(opt_def);
        }

        cmd = cmd.subcommand(task_cmd);
    }

    cmd
}

/// Get verbosity level from matches
fn get_verbosity(matches: &ArgMatches) -> Verbosity {
    if matches.get_flag("silent") {
        Verbosity::Silent
    } else if matches.get_flag("quiet") {
        Verbosity::Quiet
    } else if matches.get_flag("verbose") {
        Verbosity::Verbose
    } else {
        Verbosity::Normal
    }
}

/// Parse task variables from CLI arguments
fn parse_task_vars(
    task: &crate::config::Task,
    matches: &ArgMatches,
) -> Result<HashMap<String, String>, RuskError> {
    let mut vars = HashMap::new();

    // Parse arguments
    for (arg_name, arg) in &task.args {
        if let Some(value) = matches.get_one::<String>(arg_name) {
            vars.insert(arg_name.clone(), value.clone());
        } else if let Some(default) = &arg.default {
            vars.insert(arg_name.clone(), default.clone());
        }
    }

    // Parse options
    for (opt_name, opt) in &task.options {
        let value = match opt.option_type.as_str() {
            "bool" | "boolean" => {
                if matches.get_flag(opt_name) {
                    "true".to_string()
                } else {
                    "false".to_string()
                }
            }
            _ => {
                if let Some(v) = matches.get_one::<String>(opt_name) {
                    v.clone()
                } else if let Some(default) = &opt.default {
                    default.clone()
                } else if let Some(env_var) = &opt.environment {
                    std::env::var(env_var).unwrap_or_default()
                } else {
                    String::new()
                }
            }
        };

        // Apply rewrite if specified
        let final_value = if let Some(rewrite) = &opt.rewrite {
            rewrite.clone()
        } else {
            value
        };

        if !final_value.is_empty() {
            vars.insert(opt_name.clone(), final_value);
        }
    }

    Ok(vars)
}

/// Run the CLI application with provided arguments
pub fn run() -> Result<(), RuskError> {
    // Check if --file flag is provided first
    let args: Vec<String> = std::env::args().collect();
    let file_path = extract_file_arg(&args);

    let app = if let Some(path) = file_path {
        App::with_config_file(path)?
    } else {
        App::new()?
    };

    app.run()
}

/// Extract --file argument before clap parsing
fn extract_file_arg(args: &[String]) -> Option<PathBuf> {
    for i in 0..args.len() {
        if (args[i] == "--file" || args[i] == "-f") && i + 1 < args.len() {
            return Some(PathBuf::from(&args[i + 1]));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_verbosity_normal() {
        let cmd = Command::new("test")
            .arg(Arg::new("quiet").long("quiet").action(ArgAction::SetTrue))
            .arg(Arg::new("silent").long("silent").action(ArgAction::SetTrue))
            .arg(Arg::new("verbose").long("verbose").action(ArgAction::SetTrue));
        let matches = cmd.get_matches_from(vec!["test"]);
        assert_eq!(get_verbosity(&matches), Verbosity::Normal);
    }

    #[test]
    fn test_extract_file_arg() {
        let args = vec![
            "rusk".to_string(),
            "--file".to_string(),
            "test.yml".to_string(),
        ];
        let path = extract_file_arg(&args);
        assert_eq!(path, Some(PathBuf::from("test.yml")));
    }

    #[test]
    fn test_extract_file_arg_short() {
        let args = vec![
            "rusk".to_string(),
            "-f".to_string(),
            "test.yml".to_string(),
        ];
        let path = extract_file_arg(&args);
        assert_eq!(path, Some(PathBuf::from("test.yml")));
    }
}
