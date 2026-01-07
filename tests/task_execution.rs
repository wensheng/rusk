//! Integration tests for task execution

mod common;

use rusk::config::{parse_config, validate_config};
use rusk::runner::{Context, Task};
use std::collections::HashMap;

#[test]
fn test_execute_simple_task() {
    let yaml = r#"
tasks:
  hello:
    run: echo "Hello, World!"
"#;

    let config = parse_config(yaml, None).unwrap();
    validate_config(&config).unwrap();

    let task_config = config.tasks.get("hello").unwrap();
    let task = Task::from_config("hello".to_string(), task_config.clone()).unwrap();

    let mut ctx = Context::new();
    let result = task.execute(&mut ctx);

    assert!(result.is_ok());
}

#[test]
fn test_execute_task_with_variables() {
    let yaml = r#"
tasks:
  greet:
    run: echo "Hello, ${name}!"
"#;

    let config = parse_config(yaml, None).unwrap();
    let task_config = config.tasks.get("greet").unwrap();
    let mut task = Task::from_config("greet".to_string(), task_config.clone()).unwrap();

    // Set variables
    task.vars.insert("name".to_string(), "Rust".to_string());

    let mut ctx = Context::new();
    let result = task.execute(&mut ctx);

    assert!(result.is_ok());
}

#[test]
fn test_execute_task_with_failing_command() {
    let yaml = r#"
tasks:
  fail:
    run: "false"
"#;

    let config = parse_config(yaml, None).unwrap();
    let task_config = config.tasks.get("fail").unwrap();
    let task = Task::from_config("fail".to_string(), task_config.clone()).unwrap();

    let mut ctx = Context::new();
    let result = task.execute(&mut ctx);

    assert!(result.is_err());
}

#[test]
fn test_execute_task_with_finally() {
    let yaml = r#"
tasks:
  with_finally:
    run: echo "Running main"
    finally:
      - echo "Running finally"
      - echo "Always runs"
"#;

    let config = parse_config(yaml, None).unwrap();
    let task_config = config.tasks.get("with_finally").unwrap();
    let task = Task::from_config("with_finally".to_string(), task_config.clone()).unwrap();

    let mut ctx = Context::new();
    let result = task.execute(&mut ctx);

    assert!(result.is_ok());
}

#[test]
fn test_execute_task_with_conditional() {
    let yaml = r#"
tasks:
  conditional:
    run:
      - when:
          - equal:
              left: "${env}"
              right: "prod"
        command: echo "Production"
      - echo "Always runs"
"#;

    let config = parse_config(yaml, None).unwrap();
    let task_config = config.tasks.get("conditional").unwrap();
    let mut task = Task::from_config("conditional".to_string(), task_config.clone()).unwrap();

    // Test with env=prod (should run both commands)
    task.vars.insert("env".to_string(), "prod".to_string());
    let mut ctx = Context::new();
    let result = task.execute(&mut ctx);
    assert!(result.is_ok());

    // Test with env=dev (should skip first, run second)
    task.vars.insert("env".to_string(), "dev".to_string());
    let mut ctx2 = Context::new();
    let result2 = task.execute(&mut ctx2);
    assert!(result2.is_ok());
}

#[test]
fn test_execute_task_with_set_environment() {
    let yaml = r#"
tasks:
  set_env:
    run:
      - set-environment:
          MY_VAR: "test_value"
      - command: echo "MY_VAR is set"
"#;

    let config = parse_config(yaml, None).unwrap();
    let task_config = config.tasks.get("set_env").unwrap();
    let task = Task::from_config("set_env".to_string(), task_config.clone()).unwrap();

    let mut ctx = Context::new();
    let result = task.execute(&mut ctx);

    assert!(result.is_ok());
    assert_eq!(ctx.get_var("MY_VAR"), Some(&"test_value".to_string()));
}

#[test]
fn test_execute_multiple_commands() {
    let yaml = r#"
tasks:
  multi:
    run:
      - echo "First"
      - echo "Second"
      - echo "Third"
"#;

    let config = parse_config(yaml, None).unwrap();
    let task_config = config.tasks.get("multi").unwrap();
    let task = Task::from_config("multi".to_string(), task_config.clone()).unwrap();

    let mut ctx = Context::new();
    let result = task.execute(&mut ctx);

    assert!(result.is_ok());
}

#[test]
fn test_task_stack_prevents_recursion() {
    let config_text = r#"
tasks:
  recursive:
    run: echo "This task"
"#;

    let config = parse_config(config_text, None).unwrap();
    let task_config = config.tasks.get("recursive").unwrap();
    let task = Task::from_config("recursive".to_string(), task_config.clone()).unwrap();

    let mut ctx = Context::new();

    // Simulate the task already being in the stack
    ctx.push_task("recursive".to_string());

    let result = task.execute(&mut ctx);

    // Should fail due to recursion detection
    assert!(result.is_err());
}

#[test]
fn test_finally_runs_even_on_failure() {
    use std::fs;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let flag_file = temp_dir.path().join("finally_ran.txt");

    let yaml = format!(
        r#"
tasks:
  fail_with_finally:
    run: "false"
    finally:
      - echo "Finally block" > {}
"#,
        flag_file.display()
    );

    let config = parse_config(&yaml, None).unwrap();
    let task_config = config.tasks.get("fail_with_finally").unwrap();
    let task = Task::from_config("fail_with_finally".to_string(), task_config.clone()).unwrap();

    let mut ctx = Context::new().with_working_dir(temp_dir.path().to_path_buf());
    let result = task.execute(&mut ctx);

    // Task should fail
    assert!(result.is_err());

    // But finally block should have run
    assert!(flag_file.exists());
}

#[test]
fn test_when_condition_command_check() {
    let yaml = r#"
tasks:
  check_cmd:
    run:
      - when:
          - command: which echo
        command: echo "echo command exists"
      - when:
          - command: which nonexistent_command_xyz
        command: echo "This should not run"
      - echo "Done"
"#;

    let config = parse_config(yaml, None).unwrap();
    let task_config = config.tasks.get("check_cmd").unwrap();
    let task = Task::from_config("check_cmd".to_string(), task_config.clone()).unwrap();

    let mut ctx = Context::new();
    let result = task.execute(&mut ctx);

    assert!(result.is_ok());
}

#[test]
fn test_when_condition_file_exists() {
    use std::fs;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("exists.txt");
    fs::write(&test_file, "test").unwrap();

    let yaml = r#"
tasks:
  check_file:
    run:
      - when:
          - exists: exists.txt
        command: echo "File exists"
      - when:
          - exists: notexists.txt
        command: echo "This should not run"
"#;

    let config = parse_config(yaml, None).unwrap();
    let task_config = config.tasks.get("check_file").unwrap();
    let task = Task::from_config("check_file".to_string(), task_config.clone()).unwrap();

    let mut ctx = Context::new().with_working_dir(temp_dir.path().to_path_buf());
    let result = task.execute(&mut ctx);

    assert!(result.is_ok());
}
