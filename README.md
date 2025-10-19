# DeepResearch
DeepResearch is a Rust-based multi-agent system designed to autonomously gather, analyze, and synthesize information for complex business questions — with full traceability and explainability. It showcases advanced AI-native research orchestration and serves as a flagship demonstration of my consulting and engineering expertise.

This repository contains a minimal, graph-first implementation of the DeepResearch agent pipeline, with all multi-agent orchestration powered directly by [`graph_flow`](https://docs.rs/graph-flow/latest/)

---

## Workspace Layout

```
deepresearch-rs/
├── Cargo.toml
├── crates/
│   ├── deepresearch-core   # GraphFlow tasks + workflow runner
│   └── deepresearch-cli    # Demo binary that runs the workflow
├── docs/                   # Testing guide and supporting docs
├── AGENTS.md               # Developer reference
├── PLAN.md                 # Roadmap / milestone tracking
└── PRD.md                  # Product requirements
```

---

## Quick Start

```bash
# Format & lint
cargo fmt
cargo clippy --workspace --all-targets -- -D warnings

# Run tests (offline if dependencies are cached)
cargo test --workspace --offline

# Start local stack (requires Docker Desktop)
docker-compose up -d

# Execute the demo workflow (in-memory sessions)
cargo run --offline -p deepresearch-cli run

# Resume an existing session
cargo run --offline -p deepresearch-cli resume --session <uuid>

# Use Postgres-backed sessions
DATABASE_URL=postgres://deepresearch:deepresearch@localhost:5432/deepresearch \\
  cargo run --offline -F postgres-session -p deepresearch-cli run
```

This produces a critic verdict summarising the analyst’s findings and enumerating supporting sources.
For a deeper walkthrough (stack setup, hybrid retrieval, troubleshooting), see [`docs/USAGE.md`](docs/USAGE.md).

---

## Local Stack (Qdrant + Postgres)

The repository ships with a simple `docker-compose.yml` that launches Qdrant and Postgres:

```bash
docker-compose up -d

# Optional: inspect services
docker ps

# Tear down when finished
docker-compose down
```

Postgres sessions require `DATABASE_URL` to point at the running container (see the quick-start snippet above). In-memory storage remains the default for quick experiments.

---

## Milestone Status

| Milestone | Status | Summary |
|-----------|--------|---------|
| M0 — Graph Foundation | ✅ | Core Researcher → Analyst → Critic tasks wired via `graph_flow` |
| M1 — Observability & Testing | ✅ | Structured tracing, integration test, documented context keys |
| M2 — Branching & Extensibility | ✅ | Conditional manual-review branch, graph customiser, session options |
| M3 — Persistence & Replay | ✅ | Postgres session storage, resume APIs, docker-compose stack |
| M4 — Memory & Retrieval | ✅ | FastEmbed + Qdrant hybrid retriever, CLI ingestion workflow |
| M5 — Fact-Checking & Evaluation | ✅ | Fact-check task, confidence logging, evaluation harness |
| M6+ | 🚧 | See `PLAN.md` for upcoming work (explainability, interfaces, etc.) |

Refer to `PLAN.md` for the full roadmap.

---

## Testing

See `docs/TESTING_GUIDE.md` for the complete matrix. Key commands:

```bash
cargo fmt
cargo clippy --workspace --all-targets -- -D warnings
cargo check --offline
cargo test --offline -p deepresearch-core critic_verdict_is_non_empty
cargo test --offline -p deepresearch-core manual_review_branch_triggers
cargo test --offline -p deepresearch-core resume_session_returns_summary
```

---

## CLI Commands

```bash
# Run new session
cargo run --offline -p deepresearch-cli run --query "Compare EV supply chains"

# Resume existing session (requires persistent storage)
cargo run --offline -p deepresearch-cli resume --session <uuid>

# Use Postgres-backed sessions (feature flag + DATABASE_URL)
DATABASE_URL=postgres://deepresearch:deepresearch@localhost:5432/deepresearch \
  cargo run --offline -F postgres-session -p deepresearch-cli run --session $(uuidgen)

# Ingest local documents into Qdrant (gRPC endpoint required)
cargo run -F qdrant-retriever -p deepresearch-cli ingest --session <uuid> --path ./docs --qdrant-url http://localhost:6334
```

> Requires `curl` available on PATH (used for REST calls to Qdrant).

---

## Hybrid Retrieval (Qdrant + FastEmbed)

1. **Start Qdrant with gRPC enabled.** The bundled `docker-compose.yml` maps both ports:
   ```bash
   docker-compose up -d        # exposes 6333 (REST) and 6334 (gRPC)
   ```
   If you start Qdrant manually, add `-p 6334:6334 -e QDRANT__SERVICE__GRPC_PORT=6334`.
2. **Ingest supporting documents** into the session namespace:
   ```bash
   cargo run -F qdrant-retriever -p deepresearch-cli ingest \
     --session demo \
     --path ./docs \
     --qdrant-url http://localhost:6334
   ```
   The first run downloads the FastEmbed ONNX model (~127 MiB). Reruns reuse the cache under `.fastembed_cache/`.
3. **Run the workflow against Qdrant-backed memory:**
   ```bash
   cargo run -F qdrant-retriever -p deepresearch-cli run \
     --session demo \
     --qdrant-url http://localhost:6334
   ```
4. **Troubleshooting:** A `Unknown error h2 protocol error` means the client reached the REST port (6333). Point `--qdrant-url` at the gRPC port (6334) and ensure the container exposes it.
5. **Tune fact-check behaviour:** customise thresholds via `SessionOptions::with_fact_check_settings(...)` (see `docs/USAGE.md`).

---

## Evaluation Harness

Use the bundled `EvaluationHarness` to aggregate fact-check outcomes from log files:

```rust
use deepresearch_core::{eval::EvaluationHarness, FactCheckSettings};

let metrics = EvaluationHarness::analyze_log("logs/factcheck.jsonl")?;
println!("{}", metrics.summary());
```

(See [`docs/USAGE.md`](docs/USAGE.md) for CLI-friendly workflows.)

---

## Context Keys

| Key | Notes |
|-----|-------|
| `query` | Original user prompt. |
| `research.findings` | Vector of bullet insights from the researcher. |
| `research.sources` | Source URIs attached to findings. |
| `analysis.output` | Structured summary (`AnalystOutput`). |
| `factcheck.confidence` | Confidence score computed by the fact-check task. |
| `factcheck.verified_sources` | Sources sampled during fact-check verification. |
| `factcheck.passed` | Indicates whether the fact-check met the configured threshold. |
| `factcheck.notes` | Human-readable notes about the verification pass. |
| `critique.confident` | Boolean confidence flag from critic. |
| `critique.verdict` | Human-readable verdict string. |
| `final.summary` | Final message from `FinalizeTask`/`ManualReviewTask`. |
| `final.requires_manual` | Marks sessions requiring manual verification. |

(See `AGENTS.md` for more details.)

---

## Graph Customisation

Inject extra tasks or edges with `SessionOptions::with_customizer`:

```rust
use async_trait::async_trait;
use deepresearch_core::{run_research_session_with_options, BaseGraphTasks, SessionOptions};
use graph_flow::{Context, GraphBuilder, NextAction, Task, TaskResult};
use std::sync::Arc;

struct PreCritic;

#[async_trait]
impl Task for PreCritic {
    fn id(&self) -> &str { "pre_critic" }

    async fn run(&self, ctx: Context) -> graph_flow::Result<TaskResult> {
        // Example: tweak context before the critic executes
        ctx.set("analysis.notes", "custom hook executed").await;
        Ok(TaskResult::new(None, NextAction::ContinueAndExecute))
    }
}

let task = Arc::new(PreCritic);
let options = SessionOptions::new("Query").with_customizer(Box::new(move |builder: GraphBuilder, base: &BaseGraphTasks| {
    builder
        .add_task(task.clone())
        .add_edge(base.analyst.id(), task.id())
        .add_edge(task.id(), base.critic.id())
}));

let summary = run_research_session_with_options(options).await?;
```

Customisers run before the default edges are added, so you can intercept or extend the workflow.

---

## GitHub Actions

Basic CI is defined in `.github/workflows/ci.yml` (fmt, clippy, tests).

---

## Licensing

Released under the MIT License. See `LICENSE` for details.
