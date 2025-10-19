# DeepResearch

[![CI](https://github.com/HendrikReh/deepresearch-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/your-org/deepresearch-rs/actions/workflows/ci.yml)
[![Version](https://img.shields.io/badge/version-0.1.9-informational.svg)](https://github.com/your-org/deepresearch-rs)
[![Rust Edition](https://img.shields.io/badge/Rust-2024-blue.svg)](https://www.rust-lang.org/)
[![OpenAI](https://img.shields.io/badge/OpenAI-Integration-brightgreen.svg)](https://openai.com)
[![Collaboration](https://img.shields.io/badge/Collaboration-Welcome-orange.svg)](https://github.com/your-org/deepresearch-rs/contribute)
[![License](https://img.shields.io/badge/License-GPL--3.0--or--later-purple.svg)](LICENSE)

DeepResearch is a Rust-based multi-agent system that answers complex business questions with explainable reasoning, confidence scoring, and production-grade observability. The entire workflow is modelled as a [`graph_flow`](https://docs.rs/graph-flow/latest/graph_flow/) DAG—no bespoke planners or orchestration frameworks required.

Current capabilities include:
- Researcher → Analyst → Fact-Checker → Critic → Finalise loop with structured context keys
- CLI surface (`query`, `resume`, `explain`, `ingest`, `eval`, `purge`, `bench`) and Axum API (`/query`, `/session/:id`, `/ingest`, `/health`)
- Snapshot-tested summaries, redacted session logging with retention, and automated latency gates in CI

---

## Workspace Layout

```
deepresearch-rs/
├── Cargo.toml
├── crates/
│   ├── deepresearch-core   # GraphFlow tasks + workflow runner
│   └── deepresearch-cli    # CLI utilities and canned workflows
├── docs/
│   ├── CI_GUIDE.md         # CI command matrix & local reproduction
│   ├── RELEASE_CHECKLIST.md# Pre-release verification steps
│   ├── TESTING_GUIDE.md    # Comprehensive test matrix
│   └── USAGE.md            # CLI/API walkthrough & troubleshooting
├── AGENTS.md               # Developer reference & context keys
├── PLAN.md                 # Roadmap / milestone tracking
└── PRD.md                  # Product requirements document
```

---

## Quick Start

```bash
# 1. Format & lint
cargo fmt
cargo clippy --workspace --all-targets -- -D warnings

# 2. Test (mirrors CI)
cargo test --workspace --all-targets -- --nocapture
cargo test --offline -p deepresearch-core finalize_summary_snapshot -- --nocapture

# 3. Run the CLI (in-memory sessions)
cargo run --offline -p deepresearch-cli query "What is fueling sodium-ion adoption?" --format text
cargo run --offline -p deepresearch-cli explain <SESSION_ID> --include-summary
cargo run --offline -p deepresearch-cli bench "Latency smoke" --sessions 8 --concurrency 4 --format json

# 4. Launch the API (optional)
cargo run --offline -p deepresearch-api &
curl -s http://localhost:8080/health | jq
kill $!

# 5. Start the Qdrant/Postgres stack (optional)
docker-compose up -d
```

Each CLI run emits the critic verdict, fact-check confidence, and enumerated sources; `--format json` produces structured payloads. Refer to [`docs/USAGE.md`](docs/USAGE.md) for detailed walkthroughs, hybrid retrieval setup, and troubleshooting tips.

---

## Local Stack (Qdrant + Postgres)

The bundled `docker-compose.yml` starts Qdrant (REST 6333 / gRPC 6334) and Postgres:

```bash
docker-compose up -d
# Optional: docker compose ps
# Teardown: docker-compose down
```

Set `DATABASE_URL=postgres://deepresearch:deepresearch@localhost:5432/deepresearch` to switch the workflow to persistent sessions. Qdrant is required for the hybrid retriever (`--features qdrant-retriever`).

---

## Milestone Status

| Milestone | Status | Summary |
|-----------|--------|---------|
| M0 — Graph Foundation | ✅ | Researcher → Analyst → Critic tasks wired via GraphFlow |
| M1 — Observability & Testing | ✅ | Structured tracing, integration tests, documented context keys |
| M2 — Branching & Extensibility | ✅ | Manual-review branch, graph customiser hook, session options |
| M3 — Persistence & Replay | ✅ | Postgres storage, resume APIs, docker-compose stack |
| M4 — Memory & Retrieval | ✅ | FastEmbed + Qdrant retriever, CLI ingestion |
| M5 — Fact-Checking & Evaluation | ✅ | Fact-check task & evaluation harness |
| M6 — Explainability & Trace | ✅ | Trace collector, Mermaid/GraphViz renderers, CLI/API explainers |
| M7 — Interfaces (CLI & API) | ✅ | Full CLI surface, Axum API with 429 throttling & `/health` |
| M8 — Security, Privacy & Logging | ✅ | Redacted JSONL session logging, retention pruning, purge cleanup |
| M9 — Performance & Release Gates | ✅ | Bench latency guard (CI thresholds avg ≤ 350 ms / p95 ≤ 400 ms), release checklist |

See [`PLAN.md`](PLAN.md) for the detailed roadmap and dated notes.

---

## Testing

`docs/TESTING_GUIDE.md` enumerates the full matrix. Common commands:

```bash
cargo fmt
cargo clippy --workspace --all-targets -- -D warnings
cargo check --offline
cargo test --workspace --all-targets -- --nocapture
cargo test --offline -p deepresearch-core finalize_summary_snapshot -- --nocapture
RUST_LOG=warn cargo run --offline -p deepresearch-cli bench "CI bench" --sessions 8 --concurrency 4 --format json
```

Snapshot updates: run `INSTA_UPDATE=always cargo test --offline -p deepresearch-core finalize_summary_snapshot -- --nocapture` only when intentionally changing the baseline summary.

---

## CLI Reference

```bash
# Run a new session (text output by default)
cargo run --offline -p deepresearch-cli query "Compare EV supply chains" --format text

# Explain an existing session
cargo run --offline -p deepresearch-cli explain <SESSION_ID> --include-summary --explain-format mermaid

# Resume using shared storage
cargo run --offline -p deepresearch-cli resume --session <SESSION_ID>

# Ingest local documents into Qdrant
cargo run -F qdrant-retriever -p deepresearch-cli ingest --session demo --path ./docs --qdrant-url http://localhost:6334

# Evaluate fact-check logs
cargo run --offline -p deepresearch-cli eval data/logs/demo.jsonl --format json

# Purge session state, trace, and logs
cargo run --offline -p deepresearch-cli purge <SESSION_ID>

# Benchmark latency (CI thresholds avg ≤350 ms / p95 ≤400 ms)
RUST_LOG=warn cargo run --offline -p deepresearch-cli bench "Release bench" --sessions 24 --concurrency 6 --format json
```

---

## API Endpoints

Start the Axum server with `cargo run --offline -p deepresearch-api` and call:

```bash
curl -s http://localhost:8080/health | jq
curl -s http://localhost:8080/query \
  -H 'content-type: application/json' \
  -d '{"query":"Assess regional battery incentives","explain":true}' | jq
curl -s "http://localhost:8080/session/<SESSION_ID>?explain=true&include_summary=true" | jq
```

`/health` reports `max_sessions`, `available_sessions`, and `active_sessions`; exceeding configured capacity yields HTTP 429.

---

## Logging & Release

- Session completions append redacted JSON lines under `data/logs/<year>/<month>/`. Secrets (`api_key=…`, `bearer …`, `sk-…`) are replaced with `[REDACTED]` and mirrored into `audit.jsonl`.
- Configure retention via `DEEPRESEARCH_LOG_RETENTION_DAYS` (default 90). Set `DEEPRESEARCH_LOG_DIR` to redirect log storage.
- `deepresearch-cli purge <SESSION>` removes session state, traces, and log/audit entries.
- See [`docs/RELEASE_CHECKLIST.md`](docs/RELEASE_CHECKLIST.md) for pre-release verification (bench thresholds, API smoke, logging audit).

---

## Graph Customisation

Custom tasks can be injected with `SessionOptions::with_customizer`:

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

---

## CI & Release

- CI workflow: `.github/workflows/ci.yml` (fmt, clippy, tests, snapshot guard, bench latency, API smoke). Details in [`docs/CI_GUIDE.md`](docs/CI_GUIDE.md).
- Release procedure: [`docs/RELEASE_CHECKLIST.md`](docs/RELEASE_CHECKLIST.md).

---

## License

Licensed under the GNU General Public License v3.0 (or, at your option, any later version). See `LICENSE` for the full text.
