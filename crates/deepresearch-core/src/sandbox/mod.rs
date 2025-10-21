use anyhow::{Context, Result, anyhow};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::process::Command;
use tokio::time;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SandboxOutputKind {
    Binary,
    Text,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SandboxOutputSpec {
    pub path: String,
    pub kind: SandboxOutputKind,
}

impl SandboxOutputSpec {
    pub fn new(path: impl Into<String>, kind: SandboxOutputKind) -> Self {
        Self {
            path: path.into(),
            kind,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxFile {
    pub path: String,
    pub contents: Vec<u8>,
}

impl SandboxFile {
    pub fn new(path: impl Into<String>, contents: impl AsRef<[u8]>) -> Self {
        Self {
            path: path.into(),
            contents: contents.as_ref().to_vec(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SandboxRequest {
    pub script_name: String,
    pub script_contents: String,
    pub args: Vec<String>,
    pub files: Vec<SandboxFile>,
    pub expected_outputs: Vec<SandboxOutputSpec>,
    pub timeout: Duration,
}

impl SandboxRequest {
    pub fn new(script_name: impl Into<String>, script_contents: impl Into<String>) -> Self {
        Self {
            script_name: script_name.into(),
            script_contents: script_contents.into(),
            args: Vec::new(),
            files: Vec::new(),
            expected_outputs: Vec::new(),
            timeout: Duration::from_secs(60),
        }
    }

    pub fn validate(&self) -> Result<()> {
        ensure_relpath(&self.script_name)
            .with_context(|| format!("script name '{}' must be relative", self.script_name))?;
        ensure_not_empty(&self.script_contents, "script_contents")?;
        for file in &self.files {
            ensure_relpath(&file.path)
                .with_context(|| format!("file path '{}' must be relative", file.path))?;
        }
        for spec in &self.expected_outputs {
            ensure_relpath(&spec.path)
                .with_context(|| format!("output path '{}' must be relative", spec.path))?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct SandboxOutput {
    pub spec: SandboxOutputSpec,
    pub bytes: Vec<u8>,
}

impl SandboxOutput {
    pub fn as_text(&self) -> Option<String> {
        if self.spec.kind == SandboxOutputKind::Text {
            Some(String::from_utf8_lossy(&self.bytes).into_owned())
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
pub struct SandboxResult {
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub outputs: Vec<SandboxOutput>,
    pub timed_out: bool,
    pub duration: Duration,
}

#[async_trait]
pub trait SandboxExecutor: Send + Sync {
    async fn execute(&self, request: SandboxRequest) -> Result<SandboxResult>;
}

#[derive(Debug, Clone)]
pub enum DockerRuntimeUser {
    CurrentUser,
    Explicit(String),
}

#[derive(Debug, Clone)]
pub struct DockerSandboxConfig {
    pub image: String,
    pub docker_binary: String,
    pub workspace_root: PathBuf,
    pub memory_limit: Option<String>,
    pub cpus: Option<String>,
    pub tmpfs_size: String,
    pub cap_add: Vec<String>,
    pub env: Vec<(String, String)>,
    pub additional_args: Vec<String>,
    pub read_only_root: bool,
    pub disable_network: bool,
    pub python_binary: String,
    pub user: DockerRuntimeUser,
}

impl Default for DockerSandboxConfig {
    fn default() -> Self {
        Self {
            image: "deepresearch-python-sandbox:latest".to_string(),
            docker_binary: "docker".to_string(),
            workspace_root: std::env::temp_dir().join("deepresearch_sandbox"),
            memory_limit: Some("2g".to_string()),
            cpus: Some("2".to_string()),
            tmpfs_size: "1G".to_string(),
            cap_add: vec![
                "CHOWN".to_string(),
                "SETUID".to_string(),
                "SETGID".to_string(),
                "FOWNER".to_string(),
            ],
            env: vec![("MPLBACKEND".to_string(), "Agg".to_string())],
            additional_args: vec!["--pids-limit".to_string(), "256".to_string()],
            read_only_root: true,
            disable_network: true,
            python_binary: "python".to_string(),
            user: DockerRuntimeUser::CurrentUser,
        }
    }
}

#[derive(Debug)]
pub struct DockerSandboxRunner {
    config: DockerSandboxConfig,
    uid_gid: Option<String>,
}

static SANDBOX_FAILURE_STREAK: AtomicUsize = AtomicUsize::new(0);

impl DockerSandboxRunner {
    pub fn new(config: DockerSandboxConfig) -> Result<Self> {
        std::fs::create_dir_all(&config.workspace_root).with_context(|| {
            format!(
                "failed to create workspace root {}",
                config.workspace_root.display()
            )
        })?;

        let uid_gid = match &config.user {
            DockerRuntimeUser::CurrentUser => current_uid_gid(),
            DockerRuntimeUser::Explicit(user) => Some(user.clone()),
        };

        Ok(Self { config, uid_gid })
    }

    #[tracing::instrument(skip(self, request), fields(script = %request.script_name))]
    async fn execute_internal(&self, request: SandboxRequest) -> Result<SandboxResult> {
        request.validate()?;

        let run_id = Uuid::new_v4().to_string();
        let workspace_dir = self.config.workspace_root.join(&run_id);
        std::fs::create_dir_all(&workspace_dir).with_context(|| {
            format!(
                "failed to create sandbox workspace {}",
                workspace_dir.display()
            )
        })?;

        let guard = WorkspaceGuard::new(workspace_dir.clone());

        write_file(
            &workspace_dir,
            &request.script_name,
            request.script_contents.as_bytes(),
        )?;
        for file in &request.files {
            write_file(&workspace_dir, &file.path, &file.contents)?;
        }

        let docker_args = build_docker_args(
            &self.config,
            &workspace_dir,
            &request,
            self.uid_gid.as_deref(),
        );
        debug!(args = ?docker_args, "prepared docker invocation");

        let mut cmd = Command::new(&self.config.docker_binary);
        cmd.stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        for arg in &docker_args {
            cmd.arg(arg);
        }

        let start = Instant::now();
        info!(
            image = %self.config.image,
            workspace = %workspace_dir.display(),
            "starting sandbox execution"
        );

        let mut child = cmd.spawn().context("failed to spawn docker process")?;
        let stdout_reader = child.stdout.take();
        let stderr_reader = child.stderr.take();

        let stdout_task = tokio::spawn(async move { read_pipe(stdout_reader).await });
        let stderr_task = tokio::spawn(async move { read_pipe(stderr_reader).await });

        let wait_result = time::timeout(request.timeout, child.wait()).await;

        let (timed_out, status) = match wait_result {
            Ok(wait_outcome) => {
                let status = wait_outcome.context("failed to wait for docker process")?;
                (false, status)
            }
            Err(_elapsed) => {
                warn!("sandbox execution timed out; attempting to terminate container");
                if let Err(err) = child.kill().await {
                    warn!(error = %err, "failed to kill docker process after timeout");
                }
                let status = child
                    .wait()
                    .await
                    .context("failed to obtain exit status after timeout")?;
                (true, status)
            }
        };

        let stdout_bytes = stdout_task
            .await
            .context("failed to join stdout collection task")??;
        let stderr_bytes = stderr_task
            .await
            .context("failed to join stderr collection task")??;

        let stdout = String::from_utf8_lossy(&stdout_bytes).into_owned();
        let stderr = String::from_utf8_lossy(&stderr_bytes).into_owned();
        let exit_code = status.code();
        let duration = start.elapsed();

        let mut collected_outputs = Vec::with_capacity(request.expected_outputs.len());
        for spec in &request.expected_outputs {
            let output_path = workspace_dir.join(&spec.path);
            match std::fs::read(&output_path) {
                Ok(bytes) => {
                    collected_outputs.push(SandboxOutput {
                        spec: spec.clone(),
                        bytes,
                    });
                }
                Err(err) => {
                    warn!(
                        path = %output_path.display(),
                        error = %err,
                        "expected output missing from sandbox workspace"
                    );
                }
            }
        }

        drop(guard);

        let success = !timed_out && exit_code.unwrap_or(-1) == 0;
        let failure_streak = if success {
            SANDBOX_FAILURE_STREAK.swap(0, Ordering::Relaxed);
            0
        } else {
            let streak = SANDBOX_FAILURE_STREAK.fetch_add(1, Ordering::Relaxed) + 1;
            if streak >= 3 {
                error!(
                    streak,
                    "sandbox consecutive failure streak exceeded threshold"
                );
            }
            streak
        };

        let status_label = if timed_out {
            "timeout"
        } else if success {
            "success"
        } else {
            "failure"
        };

        info!(
            target: "telemetry.sandbox",
            status = status_label,
            exit_code,
            timed_out,
            duration_ms = duration.as_millis() as u64,
            outputs = collected_outputs.len(),
            failure_streak,
            "sandbox execution finished"
        );

        if !success {
            warn!(
                target: "telemetry.sandbox",
                status = status_label,
                overdue_failures = failure_streak,
                duration_ms = duration.as_millis() as u64,
                "sandbox execution degraded; consider retrying or alerting operations"
            );
        }

        Ok(SandboxResult {
            exit_code,
            stdout,
            stderr,
            outputs: collected_outputs,
            timed_out,
            duration,
        })
    }

    pub async fn execute(&self, request: SandboxRequest) -> Result<SandboxResult> {
        self.execute_internal(request).await
    }
}

#[async_trait]
impl SandboxExecutor for DockerSandboxRunner {
    async fn execute(&self, request: SandboxRequest) -> Result<SandboxResult> {
        self.execute_internal(request).await
    }
}

fn build_docker_args(
    config: &DockerSandboxConfig,
    workspace_dir: &Path,
    request: &SandboxRequest,
    uid_gid: Option<&str>,
) -> Vec<String> {
    let mut args = Vec::new();
    args.push("run".to_string());
    args.push("--rm".to_string());

    if config.disable_network {
        args.push("--network".to_string());
        args.push("none".to_string());
    }

    if let Some(memory) = &config.memory_limit {
        args.push("--memory".to_string());
        args.push(memory.clone());
    }

    if let Some(cpus) = &config.cpus {
        args.push("--cpus".to_string());
        args.push(cpus.clone());
    }

    args.push("--security-opt".to_string());
    args.push("no-new-privileges".to_string());
    args.push("--cap-drop".to_string());
    args.push("ALL".to_string());
    for cap in &config.cap_add {
        args.push("--cap-add".to_string());
        args.push(cap.clone());
    }

    if config.read_only_root {
        args.push("--read-only".to_string());
    }

    args.push("--tmpfs".to_string());
    args.push(format!("/tmp:exec,mode=1777,size={}", config.tmpfs_size));
    args.push("--tmpfs".to_string());
    args.push(format!(
        "/var/tmp:exec,mode=1777,size={}",
        config.tmpfs_size
    ));
    args.push("--tmpfs".to_string());
    args.push("/run:exec,mode=1777,size=64m".to_string());

    args.push("-v".to_string());
    args.push(format!("{}:/workspace:rw", workspace_dir.display()));
    args.push("-w".to_string());
    args.push("/workspace".to_string());

    for (key, value) in &config.env {
        args.push("--env".to_string());
        args.push(format!("{}={}", key, value));
    }

    if let Some(user) = uid_gid {
        args.push("--user".to_string());
        args.push(user.to_string());
    }

    args.extend(config.additional_args.iter().cloned());

    args.push(config.image.clone());
    args.push(config.python_binary.clone());
    args.push(format!("/workspace/{}", request.script_name));
    args.extend(request.args.iter().cloned());

    args
}

fn ensure_not_empty(value: &str, field: &str) -> Result<()> {
    if value.trim().is_empty() {
        Err(anyhow!("{field} must not be empty"))
    } else {
        Ok(())
    }
}

fn ensure_relpath(path: &str) -> Result<PathBuf> {
    let pb = PathBuf::from(path);
    if pb.is_absolute() {
        return Err(anyhow!("path may not be absolute"));
    }
    if pb.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err(anyhow!("path may not contain parent components (..)"));
    }
    Ok(pb)
}

fn write_file(base: &Path, rel: &str, contents: &[u8]) -> Result<()> {
    let rel_path = ensure_relpath(rel)?;
    let full = base.join(&rel_path);
    if let Some(parent) = full.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create parent directory {}", parent.display()))?;
    }
    std::fs::write(&full, contents)
        .with_context(|| format!("failed to write file {}", full.display()))?;
    Ok(())
}

async fn read_pipe<R>(pipe: Option<R>) -> Result<Vec<u8>>
where
    R: AsyncRead + Unpin + Send + 'static,
{
    let mut buffer = Vec::new();
    if let Some(mut reader) = pipe {
        reader
            .read_to_end(&mut buffer)
            .await
            .context("failed to drain sandbox pipe")?;
    }
    Ok(buffer)
}

fn current_uid_gid() -> Option<String> {
    #[cfg(unix)]
    unsafe {
        Some(format!("{}:{}", libc::geteuid(), libc::getegid()))
    }
    #[cfg(not(unix))]
    {
        None
    }
}

struct WorkspaceGuard {
    path: PathBuf,
}

impl WorkspaceGuard {
    fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl Drop for WorkspaceGuard {
    fn drop(&mut self) {
        if self.path.exists()
            && let Err(err) = std::fs::remove_dir_all(&self.path)
        {
            warn!(
                path = %self.path.display(),
                error = %err,
                "failed to clean sandbox workspace"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn build_args_includes_security_flags() {
        let config = DockerSandboxConfig {
            image: "test-image:latest".to_string(),
            docker_binary: "docker".to_string(),
            workspace_root: PathBuf::from("/tmp"),
            memory_limit: Some("2g".to_string()),
            cpus: Some("1.5".to_string()),
            tmpfs_size: "256m".to_string(),
            cap_add: vec!["CHOWN".to_string()],
            env: vec![("MPLBACKEND".to_string(), "Agg".to_string())],
            additional_args: vec!["--pids-limit".to_string(), "128".to_string()],
            read_only_root: true,
            disable_network: true,
            python_binary: "python".to_string(),
            user: DockerRuntimeUser::Explicit("1000:1000".to_string()),
        };

        let request = SandboxRequest {
            script_name: "script.py".to_string(),
            script_contents: "print('hello')".to_string(),
            args: vec!["--foo".to_string()],
            files: Vec::new(),
            expected_outputs: Vec::new(),
            timeout: Duration::from_secs(5),
        };
        let workspace = PathBuf::from("/tmp/workspace");
        let args = build_docker_args(&config, &workspace, &request, Some("1000:1000"));

        assert!(args.contains(&"--read-only".to_string()));
        assert!(args.contains(&"--network".to_string()));
        assert!(args.contains(&"--cap-drop".to_string()));
        assert!(args.contains(&"--cap-add".to_string()));
        assert!(args.contains(&"--security-opt".to_string()));
        assert!(args.contains(&"--tmpfs".to_string()));
        assert!(args.contains(&"--env".to_string()));
        assert!(args.contains(&"--user".to_string()));
        assert!(args.iter().any(|a| a.contains("/workspace/script.py")));
        assert!(args.ends_with(&["--foo".to_string()]));
    }
}
