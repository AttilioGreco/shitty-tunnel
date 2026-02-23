use anyhow::Result;
use regex::Regex;
use std::sync::LazyLock;

/// Regex to match environment variable references like ${VAR_NAME}
static ENV_VAR_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\$\{([A-Z_][A-Z0-9_]*)\}").unwrap());

/// Expands environment variables in the input string.
/// Replaces patterns like ${VAR_NAME} with the value of the environment variable.
///
/// # Examples
/// ```
/// use st_infra::config::env::expand_env_vars;
/// unsafe { std::env::set_var("MY_SECRET", "secret123") };
/// let result = expand_env_vars("key = \"${MY_SECRET}\"").unwrap();
/// assert_eq!(result, "key = \"secret123\"");
/// ```
///
/// # Errors
/// Returns an error if a referenced environment variable is not set.
pub fn expand_env_vars(input: &str) -> Result<String> {
    let mut result = input.to_string();
    let mut missing_vars = Vec::new();

    for cap in ENV_VAR_PATTERN.captures_iter(input) {
        let var_name = &cap[1];
        match std::env::var(var_name) {
            Ok(value) => {
                let pattern = format!("${{{}}}", var_name);
                result = result.replace(&pattern, &value);
            }
            Err(_) => {
                missing_vars.push(var_name.to_string());
            }
        }
    }

    if !missing_vars.is_empty() {
        anyhow::bail!(
            "missing required environment variables: {}",
            missing_vars.join(", ")
        );
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_single_var() {
        unsafe { std::env::set_var("TEST_VAR", "hello") };
        let result = expand_env_vars("value = \"${TEST_VAR}\"").unwrap();
        assert_eq!(result, "value = \"hello\"");
    }

    #[test]
    fn test_expand_multiple_vars() {
        unsafe {
            std::env::set_var("VAR1", "foo");
            std::env::set_var("VAR2", "bar");
        }
        let result = expand_env_vars("a = \"${VAR1}\"\nb = \"${VAR2}\"").unwrap();
        assert_eq!(result, "a = \"foo\"\nb = \"bar\"");
    }

    #[test]
    fn test_missing_var_returns_error() {
        let result = expand_env_vars("key = \"${NONEXISTENT_VAR}\"");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("NONEXISTENT_VAR"));
    }

    #[test]
    fn test_no_vars_unchanged() {
        let input = "key = \"plain_value\"";
        let result = expand_env_vars(input).unwrap();
        assert_eq!(result, input);
    }
}
