use std::env;

use anyhow::{Context, Result};
use deepresearch_core::{
    DockerSandboxConfig, DockerSandboxRunner, SandboxOutputKind, SandboxOutputSpec, SandboxRequest,
};

fn sandbox_tests_enabled() -> bool {
    env::var("DEEPRESEARCH_SANDBOX_TESTS")
        .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

fn docker_available() -> bool {
    std::process::Command::new("docker")
        .arg("version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn load_config() -> DockerSandboxConfig {
    let mut config = DockerSandboxConfig::default();
    if let Ok(image) = env::var("DEEPRESEARCH_SANDBOX_IMAGE") {
        config.image = image;
    }
    if let Ok(binary) = env::var("DEEPRESEARCH_DOCKER_BIN") {
        config.docker_binary = binary;
    }
    config
}

#[tokio::test]
#[ignore]
async fn sandbox_generates_expected_artifacts() -> Result<()> {
    if !sandbox_tests_enabled() {
        eprintln!("DEEPRESEARCH_SANDBOX_TESTS not enabled; skipping sandbox smoke test");
        return Ok(());
    }
    if !docker_available() {
        eprintln!("docker binary not available on PATH; skipping sandbox smoke test");
        return Ok(());
    }

    let config = load_config();
    let runner = DockerSandboxRunner::new(config).context("failed to init sandbox runner")?;

    let script = r#"
import matplotlib.pyplot as plt
import networkx as nx
import subprocess
import pathlib

plt.figure()
plt.plot([0, 1], [0, 1])
plt.title("Sandbox Headless Plot")
plt.savefig("plot.png")
plt.savefig("plot.pdf")

graph = nx.DiGraph()
graph.add_edge("A", "B")
graph.add_edge("B", "C")
nx.nx_pydot.write_dot(graph, "graph.dot")
subprocess.run(["dot", "-Tsvg", "graph.dot", "-o", "graph.svg"], check=True)

pathlib.Path("diagram.mmd").write_text("graph TD; A-->B; B-->C")
subprocess.run(["mmdc", "-i", "diagram.mmd", "-o", "diagram.svg"], check=True)

print("sandbox run completed")
"#;

    let mut request = SandboxRequest::new("math_tool.py", script);
    request.expected_outputs = vec![
        SandboxOutputSpec::new("plot.png", SandboxOutputKind::Binary),
        SandboxOutputSpec::new("plot.pdf", SandboxOutputKind::Binary),
        SandboxOutputSpec::new("graph.svg", SandboxOutputKind::Text),
        SandboxOutputSpec::new("diagram.svg", SandboxOutputKind::Text),
    ];

    let result = runner
        .execute(request)
        .await
        .context("sandbox execution failed")?;

    assert!(
        !result.timed_out,
        "sandbox execution unexpectedly timed out"
    );

    if result.exit_code != Some(0) {
        eprintln!("=== SANDBOX STDOUT ===");
        eprintln!("{}", result.stdout);
        eprintln!("=== SANDBOX STDERR ===");
        eprintln!("{}", result.stderr);
    }

    assert_eq!(
        result.exit_code,
        Some(0),
        "sandbox exit code unexpected: {:?}",
        result.exit_code
    );
    assert!(
        result.outputs.len() >= 4,
        "sandbox outputs missing: {:?}",
        result
            .outputs
            .iter()
            .map(|o| &o.spec.path)
            .collect::<Vec<_>>()
    );

    let mut svg_found = 0usize;
    for output in &result.outputs {
        assert!(
            !output.bytes.is_empty(),
            "output {} unexpectedly empty",
            output.spec.path
        );
        if matches!(output.spec.kind, SandboxOutputKind::Text) {
            let text = String::from_utf8_lossy(&output.bytes);
            assert!(
                text.contains("<svg"),
                "SVG output {} missing <svg tag",
                output.spec.path
            );
            svg_found += 1;
        }
    }
    assert!(
        svg_found >= 2,
        "expected two SVG artefacts, found {}",
        svg_found
    );

    Ok(())
}
