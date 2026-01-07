//! Variable interpolation for strings
//!
//! This module handles parsing and replacing variables in strings using the ${var} syntax.

use crate::error::{InterpolationError, InterpolationResult};
use regex::Regex;
use std::collections::HashMap;
use std::env;

/// Interpolate variables in a string
///
/// Supports:
/// - `${var}` - variable from context
/// - Environment variables (when not found in context)
pub fn interpolate(s: &str, vars: &HashMap<String, String>) -> InterpolationResult<String> {
    // Regex to match ${var} patterns
    let re = Regex::new(r"\$\{([^}]+)\}").unwrap();

    let mut result = s.to_string();
    let mut seen = std::collections::HashSet::new();

    // Loop to handle nested interpolation
    loop {
        let mut changed = false;

        result = re
            .replace_all(&result, |caps: &regex::Captures| {
                let var_name = &caps[1];

                // Check for recursive interpolation
                if !seen.insert(var_name.to_string()) {
                    return format!("${{{}}}", var_name); // Leave it unchanged to detect later
                }

                // Try to get from provided variables first
                if let Some(value) = vars.get(var_name) {
                    changed = true;
                    return value.clone();
                }

                // Try environment variables
                if let Ok(value) = env::var(var_name) {
                    changed = true;
                    return value;
                }

                // If variable not found, leave it as-is for now
                // We'll validate later if needed
                format!("${{{}}}", var_name)
            })
            .to_string();

        if !changed {
            break;
        }

        // Detect infinite recursion
        if seen.len() > 100 {
            return Err(InterpolationError::RecursiveInterpolation);
        }
    }

    Ok(result)
}

/// Interpolate with strict mode - errors on undefined variables
pub fn interpolate_strict(
    s: &str,
    vars: &HashMap<String, String>,
) -> InterpolationResult<String> {
    let result = interpolate(s, vars)?;

    // Check if there are any remaining ${} patterns
    let re = Regex::new(r"\$\{([^}]+)\}").unwrap();
    if let Some(caps) = re.captures(&result) {
        let var_name = &caps[1];
        return Err(InterpolationError::UndefinedVariable(
            var_name.to_string(),
        ));
    }

    Ok(result)
}

/// Interpolate all values in a HashMap
pub fn interpolate_map(
    map: &HashMap<String, String>,
    vars: &HashMap<String, String>,
) -> InterpolationResult<HashMap<String, String>> {
    let mut result = HashMap::new();

    for (key, value) in map {
        result.insert(key.clone(), interpolate(value, vars)?);
    }

    Ok(result)
}

/// Interpolate a list of strings
pub fn interpolate_list(
    list: &[String],
    vars: &HashMap<String, String>,
) -> InterpolationResult<Vec<String>> {
    list.iter()
        .map(|s| interpolate(s, vars))
        .collect::<InterpolationResult<Vec<String>>>()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_interpolation() {
        let mut vars = HashMap::new();
        vars.insert("name".to_string(), "world".to_string());

        let result = interpolate("Hello, ${name}!", &vars).unwrap();
        assert_eq!(result, "Hello, world!");
    }

    #[test]
    fn test_multiple_variables() {
        let mut vars = HashMap::new();
        vars.insert("first".to_string(), "John".to_string());
        vars.insert("last".to_string(), "Doe".to_string());

        let result = interpolate("${first} ${last}", &vars).unwrap();
        assert_eq!(result, "John Doe");
    }

    #[test]
    fn test_environment_variable() {
        env::set_var("TEST_VAR_RUSK", "test_value");

        let vars = HashMap::new();
        let result = interpolate("Value: ${TEST_VAR_RUSK}", &vars).unwrap();
        assert_eq!(result, "Value: test_value");

        env::remove_var("TEST_VAR_RUSK");
    }

    #[test]
    fn test_undefined_variable_lenient() {
        let vars = HashMap::new();
        let result = interpolate("Hello, ${undefined}!", &vars).unwrap();
        assert_eq!(result, "Hello, ${undefined}!");
    }

    #[test]
    fn test_undefined_variable_strict() {
        let vars = HashMap::new();
        let result = interpolate_strict("Hello, ${undefined}!", &vars);
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(InterpolationError::UndefinedVariable(_))
        ));
    }

    #[test]
    fn test_nested_interpolation() {
        let mut vars = HashMap::new();
        vars.insert("inner".to_string(), "value".to_string());
        vars.insert("outer".to_string(), "${inner}".to_string());

        let result = interpolate("Result: ${outer}", &vars).unwrap();
        assert_eq!(result, "Result: value");
    }

    #[test]
    fn test_no_interpolation() {
        let vars = HashMap::new();
        let result = interpolate("No variables here", &vars).unwrap();
        assert_eq!(result, "No variables here");
    }

    #[test]
    fn test_interpolate_map() {
        let mut vars = HashMap::new();
        vars.insert("env".to_string(), "production".to_string());

        let mut map = HashMap::new();
        map.insert("key1".to_string(), "value-${env}".to_string());
        map.insert("key2".to_string(), "static".to_string());

        let result = interpolate_map(&map, &vars).unwrap();
        assert_eq!(result.get("key1").unwrap(), "value-production");
        assert_eq!(result.get("key2").unwrap(), "static");
    }

    #[test]
    fn test_interpolate_list() {
        let mut vars = HashMap::new();
        vars.insert("name".to_string(), "test".to_string());

        let list = vec![
            "file-${name}.txt".to_string(),
            "static.txt".to_string(),
        ];

        let result = interpolate_list(&list, &vars).unwrap();
        assert_eq!(result[0], "file-test.txt");
        assert_eq!(result[1], "static.txt");
    }

    #[test]
    fn test_empty_variable_name() {
        let vars = HashMap::new();
        let result = interpolate("Value: ${}", &vars).unwrap();
        // Should leave it unchanged
        assert_eq!(result, "Value: ${}");
    }

    #[test]
    fn test_escaped_braces() {
        let vars = HashMap::new();
        // Note: This implementation doesn't support escaping yet
        // This test documents current behavior
        let result = interpolate("Not a var: ${{name}}", &vars).unwrap();
        assert!(result.contains("${{name}}"));
    }
}
