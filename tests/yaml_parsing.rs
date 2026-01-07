//! Integration tests for YAML parsing

mod common;

use rusk::config::{parse_config, parse_config_file, validate_config};

#[test]
fn test_parse_complete_config() {
    let yaml = r#"
name: my-app
usage: My test application

tasks:
  build:
    usage: Build the project
    options:
      release:
        usage: Build in release mode
        type: bool
        short: r
    run:
      - command: cargo build ${release}

  test:
    usage: Run tests
    run: cargo test

  deploy:
    usage: Deploy the application
    options:
      env:
        usage: Environment to deploy to
        default: staging
    run:
      - when:
          - equal:
              left: "${env}"
              right: "production"
        command: echo "Deploying to production"
      - command: echo "Deployment complete"
"#;

    let config = parse_config(yaml, None).unwrap();

    // Validate config structure
    validate_config(&config).unwrap();

    // Check basic properties
    assert_eq!(config.name, Some("my-app".to_string()));
    assert_eq!(config.usage, Some("My test application".to_string()));
    assert_eq!(config.tasks.len(), 3);

    // Check build task
    let build = config.tasks.get("build").unwrap();
    assert_eq!(build.usage, Some("Build the project".to_string()));
    assert!(build.options.contains_key("release"));

    // Check test task
    let test = config.tasks.get("test").unwrap();
    assert_eq!(test.usage, Some("Run tests".to_string()));

    // Check deploy task
    let deploy = config.tasks.get("deploy").unwrap();
    assert_eq!(deploy.usage, Some("Deploy the application".to_string()));
    assert!(deploy.options.contains_key("env"));
}

#[test]
fn test_parse_with_args_and_options() {
    let yaml = r#"
tasks:
  greet:
    usage: Greet someone
    args:
      person:
        usage: Person to greet
        required: true
    options:
      greeting:
        usage: Greeting to use
        default: Hello
    run: echo "${greeting}, ${person}!"
"#;

    let config = parse_config(yaml, None).unwrap();
    validate_config(&config).unwrap();

    let task = config.tasks.get("greet").unwrap();
    assert!(task.args.contains_key("person"));
    assert!(task.options.contains_key("greeting"));
}

#[test]
fn test_parse_with_finally_block() {
    let yaml = r#"
tasks:
  cleanup:
    usage: Task with cleanup
    run: echo "Running main task"
    finally:
      - echo "Cleaning up"
      - echo "Done"
"#;

    let config = parse_config(yaml, None).unwrap();
    validate_config(&config).unwrap();

    let task = config.tasks.get("cleanup").unwrap();
    assert_eq!(task.run.len(), 1);
    assert_eq!(task.finally.len(), 2);
}

#[test]
fn test_parse_with_source_target() {
    let yaml = r#"
tasks:
  compile:
    usage: Compile source files
    source:
      - "src/**/*.rs"
      - "Cargo.toml"
    target:
      - "target/debug/rusk"
    run: cargo build
"#;

    let config = parse_config(yaml, None).unwrap();
    validate_config(&config).unwrap();

    let task = config.tasks.get("compile").unwrap();
    assert_eq!(task.source.len(), 2);
    assert_eq!(task.target.len(), 1);
}

#[test]
fn test_parse_complex_when_conditions() {
    let yaml = r#"
tasks:
  conditional:
    usage: Task with multiple conditions
    run:
      - when:
          - equal:
              left: "${env}"
              right: "prod"
          - command: which docker
        command: echo "Running in production with Docker"
      - when:
          - exists: "/tmp/skip"
        command: echo "Skip file exists"
      - command: echo "Always runs"
"#;

    let config = parse_config(yaml, None).unwrap();
    validate_config(&config).unwrap();

    let task = config.tasks.get("conditional").unwrap();
    assert_eq!(task.run.len(), 3);
}

#[test]
fn test_parse_subtasks() {
    let yaml = r#"
tasks:
  all:
    usage: Run all tasks
    run:
      - task: build
      - task: test
      - task:
          name: deploy
          options:
            env: production

  build:
    run: echo "Building"

  test:
    run: echo "Testing"

  deploy:
    options:
      env:
        default: staging
    run: echo "Deploying to ${env}"
"#;

    let config = parse_config(yaml, None).unwrap();
    validate_config(&config).unwrap();

    let task = config.tasks.get("all").unwrap();
    assert_eq!(task.run.len(), 3);
}

#[test]
fn test_parse_quiet_and_private() {
    let yaml = r#"
tasks:
  public:
    usage: Public task
    run: echo "public"

  private:
    usage: Private task
    private: true
    run: echo "private"

  quiet:
    usage: Quiet task
    quiet: true
    run: echo "quiet"
"#;

    let config = parse_config(yaml, None).unwrap();
    validate_config(&config).unwrap();

    assert!(!config.tasks.get("public").unwrap().private);
    assert!(config.tasks.get("private").unwrap().private);
    assert!(config.tasks.get("quiet").unwrap().quiet);
}

#[test]
fn test_parse_from_file() {
    let yaml = r#"
tasks:
  hello:
    run: echo "Hello from file"
"#;

    let (_temp_dir, config_path) = common::create_test_config(yaml);
    let config = parse_config_file(&config_path).unwrap();

    validate_config(&config).unwrap();
    assert!(config.tasks.contains_key("hello"));
}

#[test]
fn test_invalid_config_missing_target() {
    let yaml = r#"
tasks:
  bad:
    source:
      - "file.txt"
    run: echo "bad"
"#;

    let config = parse_config(yaml, None).unwrap();
    let result = validate_config(&config);

    assert!(result.is_err());
}

#[test]
fn test_invalid_config_duplicate_names() {
    let yaml = r#"
tasks:
  bad:
    args:
      name:
        required: true
    options:
      name:
        type: string
    run: echo "bad"
"#;

    let config = parse_config(yaml, None).unwrap();
    let result = validate_config(&config);

    assert!(result.is_err());
}
