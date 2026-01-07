//! Configuration file parsing and discovery

use crate::config::types::{Config, Task};
use crate::error::{ConfigError, ConfigResult, RuskError};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

/// Default configuration file names to search for
const CONFIG_FILE_NAMES: &[&str] = &["rusk.yml", "rusk.yaml"];

/// Find the configuration file by searching current and parent directories
pub fn find_config_file() -> ConfigResult<PathBuf> {
    find_config_file_from(env::current_dir().map_err(|e| {
        ConfigError::Invalid(format!("Failed to get current directory: {}", e))
    })?)
}

/// Find the configuration file starting from a specific directory
pub fn find_config_file_from(start_dir: PathBuf) -> ConfigResult<PathBuf> {
    let mut current_dir = start_dir.clone();
    let mut searched_paths = Vec::new();

    loop {
        for file_name in CONFIG_FILE_NAMES {
            let config_path = current_dir.join(file_name);
            searched_paths.push(config_path.display().to_string());

            if config_path.exists() && config_path.is_file() {
                return Ok(config_path);
            }
        }

        // Try parent directory
        match current_dir.parent() {
            Some(parent) => current_dir = parent.to_path_buf(),
            None => {
                // Reached root without finding config
                return Err(ConfigError::NotFound(searched_paths.join(", ")));
            }
        }
    }
}

/// Parse a configuration file from a path
pub fn parse_config_file(path: &Path) -> Result<Config, RuskError> {
    let contents = fs::read_to_string(path)
        .map_err(|e| ConfigError::Invalid(format!("Failed to read file: {}", e)))?;

    parse_config(&contents, Some(path))
}

/// Parse configuration from a string
pub fn parse_config(yaml: &str, config_path: Option<&Path>) -> Result<Config, RuskError> {
    let mut config: Config = serde_yaml::from_str(yaml)?;

    // Process includes if present
    if let Some(base_path) = config_path {
        process_includes(&mut config, base_path)?;
    }

    Ok(config)
}

/// Process include directives in tasks
fn process_includes(config: &mut Config, config_path: &Path) -> Result<(), RuskError> {
    let base_dir = config_path.parent().unwrap_or_else(|| Path::new("."));

    let task_names: Vec<String> = config.tasks.keys().cloned().collect();

    for task_name in task_names {
        if let Some(task) = config.tasks.get(&task_name) {
            if let Some(include_path) = &task.include {
                // Read and parse the included file
                let full_include_path = base_dir.join(include_path);

                let included_task = load_included_task(&full_include_path)?;

                // Replace the task with the included content
                config.tasks.insert(task_name.clone(), included_task);
            }
        }
    }

    Ok(())
}

/// Load a task from an included file
fn load_included_task(path: &Path) -> Result<Task, RuskError> {
    let contents = fs::read_to_string(path).map_err(|e| {
        ConfigError::IncludeFile {
            path: path.to_path_buf(),
            error: e.to_string(),
        }
    })?;

    let task: Task = serde_yaml::from_str(&contents).map_err(|e| {
        ConfigError::IncludeFile {
            path: path.to_path_buf(),
            error: e.to_string(),
        }
    })?;

    Ok(task)
}

/// Parse configuration with automatic file discovery
pub fn parse_config_auto() -> Result<(Config, PathBuf), RuskError> {
    let config_path = find_config_file()?;
    let config = parse_config_file(&config_path)?;
    Ok((config, config_path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_parse_simple_config() {
        let yaml = r#"
tasks:
  hello:
    usage: Say hello
    run: echo "hello"
"#;
        let config = parse_config(yaml, None).unwrap();
        assert_eq!(config.tasks.len(), 1);
        assert!(config.tasks.contains_key("hello"));
    }

    #[test]
    fn test_find_config_in_current_dir() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("rusk.yml");

        fs::write(
            &config_path,
            r#"
tasks:
  test:
    run: echo "test"
"#,
        )
        .unwrap();

        let found = find_config_file_from(temp_dir.path().to_path_buf()).unwrap();
        assert_eq!(found, config_path);
    }

    #[test]
    fn test_find_config_in_parent_dir() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("rusk.yml");
        let sub_dir = temp_dir.path().join("subdir");

        fs::create_dir(&sub_dir).unwrap();
        fs::write(
            &config_path,
            r#"
tasks:
  test:
    run: echo "test"
"#,
        )
        .unwrap();

        let found = find_config_file_from(sub_dir).unwrap();
        assert_eq!(found, config_path);
    }

    #[test]
    fn test_config_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let result = find_config_file_from(temp_dir.path().to_path_buf());
        assert!(result.is_err());
        assert!(matches!(result, Err(ConfigError::NotFound(_))));
    }

    #[test]
    fn test_parse_config_with_name_and_usage() {
        let yaml = r#"
name: my-app
usage: My application
tasks:
  hello:
    run: echo "hello"
"#;
        let config = parse_config(yaml, None).unwrap();
        assert_eq!(config.name, Some("my-app".to_string()));
        assert_eq!(config.usage, Some("My application".to_string()));
    }

    #[test]
    fn test_parse_config_with_interpreter() {
        let yaml = r#"
interpreter:
  - bash
  - -c
tasks:
  hello:
    run: echo "hello"
"#;
        let config = parse_config(yaml, None).unwrap();
        assert_eq!(
            config.interpreter,
            Some(vec!["bash".to_string(), "-c".to_string()])
        );
    }
}
