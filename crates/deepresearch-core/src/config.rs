use std::{
    env, fs,
    path::{Path, PathBuf},
};

use serde::Deserialize;

use crate::{require_env, DeepResearchError, SecretValue};

const DEFAULT_CONFIG_PATH: &str = "config.toml";
const CONFIG_PATH_ENV: &str = "DEEPRESEARCH_CONFIG";

/// Top-level configuration structure (see `PRD.md §4`).
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub llm: LlmConfig,
    pub qdrant: QdrantConfig,
    pub planner: PlannerConfig,
    pub factcheck: FactcheckConfig,
    pub logging: LoggingConfig,
}

impl Config {
    /// Resolve the configured LLM secret value (from environment only).
    pub fn llm_api_key(&self) -> Result<SecretValue, DeepResearchError> {
        require_env(&self.llm.api_key_env)
    }
}

/// Helper to load configuration with best-practice guard rails.
pub struct ConfigLoader;

impl ConfigLoader {
    /// Load configuration from a provided path or discoverable defaults.
    ///
    /// Resolution order:
    /// 1. Explicit `path` argument.
    /// 2. `DEEPRESEARCH_CONFIG` environment variable.
    /// 3. `config.toml` in the current working directory.
    pub fn load(path: Option<PathBuf>) -> Result<Config, DeepResearchError> {
        let candidate = resolve_path(path)?;
        let raw = fs::read_to_string(&candidate)
            .map_err(|err| DeepResearchError::config_io(candidate.clone(), err))?;
        let config: Config = toml::from_str(&raw)
            .map_err(|err| DeepResearchError::InvalidConfiguration(err.to_string()))?;

        Self::validate(&config)?;
        Ok(config)
    }

    fn validate(config: &Config) -> Result<(), DeepResearchError> {
        if config.llm.api_key_env.trim().is_empty() {
            return Err(DeepResearchError::InvalidConfiguration(
                "llm.api_key_env must reference an environment variable".into(),
            ));
        }

        // Ensure environment variable exists at load time to discourage inline secrets.
        require_env(&config.llm.api_key_env)?;
        Ok(())
    }
}

fn resolve_path(path: Option<PathBuf>) -> Result<PathBuf, DeepResearchError> {
    if let Some(path) = path {
        return Ok(path);
    }

    if let Ok(from_env) = env::var(CONFIG_PATH_ENV) {
        if !from_env.trim().is_empty() {
            return Ok(PathBuf::from(from_env));
        }
    }

    Ok(Path::new(DEFAULT_CONFIG_PATH).to_path_buf())
}

#[derive(Debug, Clone, Deserialize)]
pub struct LlmConfig {
    pub provider: String,
    pub model: String,
    #[serde(default)]
    pub api_key_env: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct QdrantConfig {
    pub url: String,
    pub collection: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PlannerConfig {
    pub max_iterations: u16,
    pub confidence_threshold: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FactcheckConfig {
    #[serde(default = "FactcheckConfig::default_min_confidence")]
    pub min_confidence: f32,
    #[serde(default = "FactcheckConfig::default_verification_count")]
    pub verification_count: u8,
    #[serde(default = "FactcheckConfig::default_timeout_ms")]
    pub timeout_ms: u64,
}

impl FactcheckConfig {
    const fn default_min_confidence() -> f32 {
        0.75
    }

    const fn default_verification_count() -> u8 {
        3
    }

    const fn default_timeout_ms() -> u64 {
        20_000
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
}
