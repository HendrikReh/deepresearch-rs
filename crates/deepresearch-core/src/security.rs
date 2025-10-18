use std::env;

use crate::DeepResearchError;

/// Wrapper around sensitive values to reduce accidental logging.
#[derive(Clone)]
pub struct SecretValue(String);

impl SecretValue {
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for SecretValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "***redacted***")
    }
}

/// Require that a given environment variable is set and non-empty.
pub fn require_env(var: &str) -> Result<SecretValue, DeepResearchError> {
    match env::var(var) {
        Ok(value) if !value.trim().is_empty() => Ok(SecretValue(value)),
        _ => Err(DeepResearchError::MissingSecret(var.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn require_env_success() {
        std::env::set_var("TEST_SECRET", "value");
        let secret = require_env("TEST_SECRET").expect("secret should load");
        assert_eq!(secret.expose(), "value");
    }

    #[test]
    fn require_env_missing() {
        std::env::remove_var("TEST_SECRET_MISSING");
        let err = require_env("TEST_SECRET_MISSING").unwrap_err();
        assert!(matches!(err, DeepResearchError::MissingSecret(_)));
    }
}
