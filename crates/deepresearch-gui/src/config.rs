use anyhow::{Context, Result};
use std::env;
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub listen_addr: String,
    pub max_concurrency: usize,
    pub default_enable_trace: bool,
    pub assets_dir: PathBuf,
    pub gui_enabled: bool,
    pub auth_token: Option<String>,
    pub storage: StorageBackend,
    pub session_namespace: Option<String>,
    pub otel_endpoint: Option<String>,
}

#[derive(Clone, Debug)]
pub enum StorageBackend {
    InMemory,
    #[cfg(feature = "postgres-session")]
    Postgres {
        url: String,
    },
}

impl AppConfig {
    const DEFAULT_LISTEN_ADDR: &'static str = "0.0.0.0:8080";
    const DEFAULT_ASSETS_DIR: &'static str = "crates/deepresearch-gui/web/dist";

    pub fn from_env() -> Result<Self> {
        let listen_addr =
            env::var("GUI_LISTEN_ADDR").unwrap_or_else(|_| Self::DEFAULT_LISTEN_ADDR.to_string());

        let max_concurrency = env::var("GUI_MAX_CONCURRENCY")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .filter(|value| *value > 0)
            .unwrap_or_else(|| {
                std::thread::available_parallelism()
                    .map(|nz| nz.get())
                    .unwrap_or(4)
            });

        let default_enable_trace = env::var("GUI_DEFAULT_TRACE")
            .ok()
            .map(|value| {
                bool::from_str(&value).with_context(|| "GUI_DEFAULT_TRACE must be true or false")
            })
            .transpose()? // Option<Result> -> Result<Option>
            .unwrap_or(true);

        let assets_dir = env::var("GUI_ASSETS_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(Self::DEFAULT_ASSETS_DIR));
        let assets_dir = if assets_dir.is_relative() {
            env::current_dir()
                .map(|cwd| cwd.join(assets_dir))
                .unwrap_or_else(|_| PathBuf::from(Self::DEFAULT_ASSETS_DIR))
        } else {
            assets_dir
        };

        let gui_enabled = env::var("GUI_ENABLE_GUI")
            .ok()
            .and_then(|value| parse_bool(&value))
            .unwrap_or(false);

        let auth_token = env::var("GUI_AUTH_TOKEN")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());

        let storage = resolve_storage_backend()?;

        let session_namespace = env::var("GUI_SESSION_NAMESPACE")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());

        let otel_endpoint = env::var("GUI_OTEL_ENDPOINT")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());

        let gui_enabled = gui_enabled || auth_token.is_some();

        Ok(Self {
            listen_addr,
            max_concurrency,
            default_enable_trace,
            assets_dir,
            gui_enabled,
            auth_token,
            storage,
            session_namespace,
            otel_endpoint,
        })
    }
}

fn parse_bool(input: &str) -> Option<bool> {
    match input.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => None,
    }
}

fn resolve_storage_backend() -> Result<StorageBackend> {
    match env::var("GUI_STORAGE").ok().as_deref() {
        #[cfg(feature = "postgres-session")]
        Some("postgres") => {
            let url = env::var("GUI_POSTGRES_URL")
                .or_else(|_| env::var("DATABASE_URL"))
                .context(
                    "GUI_POSTGRES_URL or DATABASE_URL must be set when GUI_STORAGE=postgres",
                )?;
            Ok(StorageBackend::Postgres { url })
        }
        #[cfg(not(feature = "postgres-session"))]
        Some("postgres") => Err(anyhow::anyhow!(
            "GUI built without postgres-session support; rebuild with --features postgres-session"
        )),
        _ => Ok(StorageBackend::InMemory),
    }
}
