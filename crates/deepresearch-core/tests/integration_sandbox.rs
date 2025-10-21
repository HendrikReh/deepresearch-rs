use anyhow::Result;
use deepresearch_core::{
    DockerSandboxConfig, DockerSandboxRunner, SandboxOutputKind, SandboxOutputSpec, SandboxRequest,
};
use tokio::runtime::Runtime;

fn build_runtime() -> Runtime {
    Runtime::new().expect("tokio runtime")
}

#[test]
fn sandbox_produces_expected_artifacts() -> Result<()> {
    if std::env::var("DEEPRESEARCH_SANDBOX_TESTS")
        .map(|val| val != "1" && !val.eq_ignore_ascii_case("true"))
        .unwrap_or(true)
    {
        eprintln!("DEEPRESEARCH_SANDBOX_TESTS not enabled; skipping sandbox integration test");
        return Ok(());
    }

    let runtime = build_runtime();
    runtime.block_on(async {
        let mut config = DockerSandboxConfig::default();
        if let Ok(image) = std::env::var("DEEPRESEARCH_SANDBOX_IMAGE") {
            config.image = image;
        }
        let runner = DockerSandboxRunner::new(config)?;
        let script = r#"
import matplotlib.pyplot as plt
import networkx as nx
import subprocess

plt.plot([0, 1], [0, 1])
plt.savefig("plot.png")

G = nx.DiGraph()
G.add_edge("A", "B")
nx.nx_pydot.write_dot(G, "graph.dot")
subprocess.run(["dot", "-Tsvg", "graph.dot", "-o", "graph.svg"], check=True)

with open("diagram.mmd", "w") as f:
    f.write("graph TD;A-->B")
subprocess.run(["mmdc", "-i", "diagram.mmd", "-o", "diagram.svg"], check=True)
"#;

        let mut request = SandboxRequest::new("integration.py", script);
        request.expected_outputs = vec![
            SandboxOutputSpec::new("plot.png", SandboxOutputKind::Binary),
            SandboxOutputSpec::new("graph.svg", SandboxOutputKind::Text),
            SandboxOutputSpec::new("diagram.svg", SandboxOutputKind::Text),
        ];

        let result = runner.execute(request).await?;
        assert_eq!(result.exit_code, Some(0));
        assert!(!result.timed_out);
        assert!(result.outputs.len() >= 3);
        Ok::<_, anyhow::Error>(())
    })?;

    Ok(())
}
