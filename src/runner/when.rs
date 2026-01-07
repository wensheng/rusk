//! When condition evaluation
//!
//! This module handles evaluating conditional expressions for run items.

use crate::error::{ExecutionError, ExecutionResult};
use crate::runner::{check_command, interpolate, Context, When, WhenCondition};
use std::env;

/// Evaluate a list of when conditions (all must be true - AND logic)
pub fn evaluate_when_list(when_list: &[When], ctx: &Context) -> ExecutionResult<bool> {
    for when in when_list {
        if !evaluate_when(when, ctx)? {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Evaluate a single when condition
pub fn evaluate_when(when: &When, ctx: &Context) -> ExecutionResult<bool> {
    match &when.condition {
        WhenCondition::Always => Ok(true),

        WhenCondition::Equal { left, right } => {
            let left_val = interpolate(left, &ctx.vars).unwrap_or_else(|_| left.clone());
            let right_val = interpolate(right, &ctx.vars).unwrap_or_else(|_| right.clone());
            Ok(left_val == right_val)
        }

        WhenCondition::NotEqual { left, right } => {
            let left_val = interpolate(left, &ctx.vars).unwrap_or_else(|_| left.clone());
            let right_val = interpolate(right, &ctx.vars).unwrap_or_else(|_| right.clone());
            Ok(left_val != right_val)
        }

        WhenCondition::Command(cmd) => {
            // Execute command and check if it succeeds
            check_command(cmd, ctx)
        }

        WhenCondition::Exists(path) => {
            let path_str = interpolate(path, &ctx.vars).unwrap_or_else(|_| path.clone());
            let full_path = ctx.working_dir.join(&path_str);
            Ok(full_path.exists())
        }

        WhenCondition::EnvSet(var_name) => {
            let var = interpolate(var_name, &ctx.vars).unwrap_or_else(|_| var_name.clone());
            Ok(env::var(&var).is_ok())
        }

        WhenCondition::EnvNotSet(var_name) => {
            let var = interpolate(var_name, &ctx.vars).unwrap_or_else(|_| var_name.clone());
            Ok(env::var(&var).is_err())
        }

        WhenCondition::OptionSet(opt_name) => {
            // Check if the option/variable is set in context
            Ok(ctx.vars.contains_key(opt_name))
        }

        WhenCondition::OptionNotSet(opt_name) => {
            // Check if the option/variable is not set in context
            Ok(!ctx.vars.contains_key(opt_name))
        }
    }
}

/// Helper to create a failed condition error
pub fn failed_condition_error(reason: &str) -> ExecutionError {
    ExecutionError::FailedCondition(reason.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_evaluate_always() {
        let ctx = Context::new();
        let when = When {
            condition: WhenCondition::Always,
        };

        assert_eq!(evaluate_when(&when, &ctx).unwrap(), true);
    }

    #[test]
    fn test_evaluate_equal_true() {
        let mut vars = HashMap::new();
        vars.insert("env".to_string(), "production".to_string());

        let ctx = Context::new().with_vars(vars);
        let when = When {
            condition: WhenCondition::Equal {
                left: "${env}".to_string(),
                right: "production".to_string(),
            },
        };

        assert_eq!(evaluate_when(&when, &ctx).unwrap(), true);
    }

    #[test]
    fn test_evaluate_equal_false() {
        let mut vars = HashMap::new();
        vars.insert("env".to_string(), "development".to_string());

        let ctx = Context::new().with_vars(vars);
        let when = When {
            condition: WhenCondition::Equal {
                left: "${env}".to_string(),
                right: "production".to_string(),
            },
        };

        assert_eq!(evaluate_when(&when, &ctx).unwrap(), false);
    }

    #[test]
    fn test_evaluate_not_equal() {
        let mut vars = HashMap::new();
        vars.insert("env".to_string(), "development".to_string());

        let ctx = Context::new().with_vars(vars);
        let when = When {
            condition: WhenCondition::NotEqual {
                left: "${env}".to_string(),
                right: "production".to_string(),
            },
        };

        assert_eq!(evaluate_when(&when, &ctx).unwrap(), true);
    }

    #[test]
    fn test_evaluate_command_success() {
        let ctx = Context::new();
        let when = When {
            condition: WhenCondition::Command("true".to_string()),
        };

        assert_eq!(evaluate_when(&when, &ctx).unwrap(), true);
    }

    #[test]
    fn test_evaluate_command_failure() {
        let ctx = Context::new();
        let when = When {
            condition: WhenCondition::Command("false".to_string()),
        };

        assert_eq!(evaluate_when(&when, &ctx).unwrap(), false);
    }

    #[test]
    fn test_evaluate_exists() {
        use tempfile::TempDir;
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        std::fs::write(&test_file, "test").unwrap();

        let ctx = Context::new().with_working_dir(temp_dir.path().to_path_buf());
        let when = When {
            condition: WhenCondition::Exists("test.txt".to_string()),
        };

        assert_eq!(evaluate_when(&when, &ctx).unwrap(), true);

        let when_not_exists = When {
            condition: WhenCondition::Exists("nonexistent.txt".to_string()),
        };

        assert_eq!(evaluate_when(&when_not_exists, &ctx).unwrap(), false);
    }

    #[test]
    fn test_evaluate_env_set() {
        env::set_var("TEST_RUSK_VAR", "value");

        let ctx = Context::new();
        let when = When {
            condition: WhenCondition::EnvSet("TEST_RUSK_VAR".to_string()),
        };

        assert_eq!(evaluate_when(&when, &ctx).unwrap(), true);

        env::remove_var("TEST_RUSK_VAR");
    }

    #[test]
    fn test_evaluate_env_not_set() {
        env::remove_var("NONEXISTENT_VAR_RUSK");

        let ctx = Context::new();
        let when = When {
            condition: WhenCondition::EnvNotSet("NONEXISTENT_VAR_RUSK".to_string()),
        };

        assert_eq!(evaluate_when(&when, &ctx).unwrap(), true);
    }

    #[test]
    fn test_evaluate_option_set() {
        let mut vars = HashMap::new();
        vars.insert("myoption".to_string(), "value".to_string());

        let ctx = Context::new().with_vars(vars);
        let when = When {
            condition: WhenCondition::OptionSet("myoption".to_string()),
        };

        assert_eq!(evaluate_when(&when, &ctx).unwrap(), true);
    }

    #[test]
    fn test_evaluate_option_not_set() {
        let ctx = Context::new();
        let when = When {
            condition: WhenCondition::OptionNotSet("myoption".to_string()),
        };

        assert_eq!(evaluate_when(&when, &ctx).unwrap(), true);
    }

    #[test]
    fn test_evaluate_when_list_all_true() {
        let mut vars = HashMap::new();
        vars.insert("env".to_string(), "production".to_string());

        let ctx = Context::new().with_vars(vars);
        let when_list = vec![
            When {
                condition: WhenCondition::Equal {
                    left: "${env}".to_string(),
                    right: "production".to_string(),
                },
            },
            When {
                condition: WhenCondition::Command("true".to_string()),
            },
        ];

        assert_eq!(evaluate_when_list(&when_list, &ctx).unwrap(), true);
    }

    #[test]
    fn test_evaluate_when_list_one_false() {
        let mut vars = HashMap::new();
        vars.insert("env".to_string(), "development".to_string());

        let ctx = Context::new().with_vars(vars);
        let when_list = vec![
            When {
                condition: WhenCondition::Equal {
                    left: "${env}".to_string(),
                    right: "production".to_string(),
                },
            },
            When {
                condition: WhenCondition::Command("true".to_string()),
            },
        ];

        // First condition is false, so overall result is false
        assert_eq!(evaluate_when_list(&when_list, &ctx).unwrap(), false);
    }
}
