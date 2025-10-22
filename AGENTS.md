# DeepResearch Agent Pipeline — Developer Guide

This repo hosts a fresh graph-first implementation of DeepResearch. All agent behaviour is composed with the [`graph_flow`](https://docs.rs/graph-flow/latest/graph_flow/) crate; there are no bespoke orchestrators or ad-hoc DAG executors.

# General Prompt Rule
**Always add** 'use context7' to each user prompt

---

## System Snapshot
- **Workflow:** Researcher → Math Tool → Analyst → Critic tasks executed through `graph_flow`.
- **Crates:**  
  - `deepresearch-core` — reusable tasks and workflow helpers.  
  - `deepresearch-cli` — demo binary that runs the default research session.
- **Primary dependencies:** `graph-flow`, `tokio`, `anyhow`, `tracing`, `qdrant-client`, `fastembed`, `dashmap`.

---

## Module Guide

| Path | Purpose | Notes |
|------|---------|-------|
| `crates/deepresearch-core/src/tasks.rs` | Implements `ResearchTask`, `MathToolTask`, `AnalystTask`, `CriticTask` (`graph_flow::Task`) | Stores intermediate state in `Context` keys like `research.*`, `math.*`, `analysis.*`, `critique.*` |
| `crates/deepresearch-core/src/sandbox/mod.rs` | Hardened Docker sandbox runner and request/response types | Executes Python math/stats scripts with read-only rootfs, tmpfs scratch, and output collection |
| `crates/deepresearch-core/src/workflow.rs` | Builds the workflow graph and runs sessions via `FlowRunner` | Uses `InMemorySessionStorage` and loops until `ExecutionStatus::Completed` |
| `crates/deepresearch-cli/src/main.rs` | Initializes tracing and runs a sample session for a hard-coded query | Prints the critic verdict + summary string returned from `run_research_session` |

---

## How the Graph Runs
1. `build_graph()` registers the core tasks on a `GraphBuilder`: Researcher → Math Tool → Analyst → Critic → {Finalize, ManualReview}.  
2. A new session is created with `Session::new_from_task(...)`; the query is stored in the session `Context`.  
3. `FlowRunner::run` executes step-by-step; the critic decides whether to branch to `FinalizeTask` or `ManualReviewTask` via `add_conditional_edge`.  
4. The final report is stored under `final.summary`; clients can extend the graph by providing a `GraphCustomizer` through `SessionOptions`.

Agents may extend their behaviour by reading/writing new context keys; the workflow automatically carries state across tasks thanks to `graph_flow::Context`.

---

## Shared Context Keys

| Key | Producer | Type | Purpose |
|-----|----------|------|---------|
| `query` | `ResearchTask` (seeded via workflow) | `String` | User prompt driving the session. |
| `research.findings` | `ResearchTask` | `Vec<String>` | Bullet insights gathered during retrieval. |
| `research.sources` | `ResearchTask` | `Vec<String>` | Source URIs backing the findings. |
| `analysis.output` | `AnalystTask` | `AnalystOutput` (summary/highlight/sources) | Structured synthesis consumed by the critic. |
| `math.request` | Upstream agent / `SessionOptions` | `MathToolRequest` | Python script + assets to execute inside the sandbox. |
| `math.result` | `MathToolTask` | `MathToolResult` (status, stdout/stderr, outputs) | Captures execution status, metrics, and artefacts. |
| `math.outputs` | `MathToolTask` | `Vec<MathToolOutput>` | Binary/text artefacts emitted by the script (PNG/SVG/PDF/etc.). |
| `math.status` | `MathToolTask` | `String` (`success`, `failure`, `timeout`, `skipped`) | Convenience status used by downstream tasks for branching. |
| `math.degradation_note` | `MathToolTask` | `String` | Operator-facing message when sandbox execution degrades (appended to analyst summary). |
| `math.retry_recommended` | `MathToolTask` | `bool` | Indicates whether retrying the sandbox is advisable. |
| `math.alert_required` | `MathToolTask` | `bool` | Flags hard failures/timeouts; mirror in dashboards for alerting. |
| `analysis.math_retry_recommended` | `AnalystTask` | `bool` | Propagates retry guidance downstream if math degraded. |
| `analysis.math_alert_required` | `AnalystTask` | `bool` | Signals to Critic/clients that math outputs were unavailable. |
| `critique.confident` | `CriticTask` | `bool` | Indicates whether automated checks pass (set synchronously for conditional edge). |
| `critique.verdict` | `CriticTask` | `String` | Human-readable verdict surfaced to the end user. |
| `final.summary` | `FinalizeTask` / `ManualReviewTask` | `String` | Final message returned to the caller. |
| `final.requires_manual` | `ManualReviewTask` / `FinalizeTask` | `bool` | Flags sessions requiring manual oversight. |
| `trace.enabled` | Workflow bootstrap | `bool` | Toggles capture of per-task trace events. |
| `trace.collector` | All tasks via helper | `TraceCollector` | Accumulates structured `TraceEvent`s for persistence and explainability tooling. |

All tasks emit tracing spans (`task.research`, `task.analyst`, `task.critic`) and attach structured fields (query, counts, confidence) for observability.

---

## Extending the Graph

Use `SessionOptions::with_customizer` to inject additional tasks or edges before the default wiring is applied:

```rust
use deepresearch_core::{run_research_session_with_options, SessionOptions, BaseGraphTasks};
use graph_flow::{Context, GraphBuilder, NextAction, Task, TaskResult};
use async_trait::async_trait;
use std::sync::Arc;

struct PostProcess;

#[async_trait]
impl Task for PostProcess {
    fn id(&self) -> &str { "post_process" }

    async fn run(&self, ctx: Context) -> graph_flow::Result<TaskResult> {
        ctx.set("final.summary", format!("{}\n(Post-processed)", ctx.get::<String>("final.summary").await.unwrap_or_default())).await;
        Ok(TaskResult::new(None, NextAction::End))
    }
}

let task = Arc::new(PostProcess);
let options = SessionOptions::new("Custom query").with_customizer(Box::new(move |builder: GraphBuilder, base: &BaseGraphTasks| {
    builder
        .add_task(task.clone())
        .add_edge(base.finalize.id(), task.id())
}));

// Requires building with `--features deepresearch-core/qdrant-retriever`
let summary = run_research_session_with_options(options).await?;
```

Customisers run *before* the default edges are added, allowing you to intercept or extend the workflow.

---

## Development Workflow

```bash
cargo fmt                    # format
cargo check --offline        # build without hitting crates.io
cargo run -p deepresearch-cli
cargo test --offline --workspace --all-targets -- --nocapture   # mirrors CI
cargo test --offline -p deepresearch-core finalize_summary_snapshot -- --nocapture
```

Add new tasks by implementing `graph_flow::Task` and registering them in `build_graph()`. Prefer `NextAction::ContinueAndExecute` for straight-line execution and `NextAction::End` or `WaitForInput` for pauses.
- Use `SessionOptions::with_sandbox_executor(Arc<dyn SandboxExecutor>)` (or the matching `ResumeOptions` helper) to enable the hardened Python sandbox defined in `containers/python-sandbox/Dockerfile`.
- `MathToolTask` writes results to `math.*` keys; downstream agents should inspect `math.status` / `math.result` to decide whether to trust numeric outputs or fall back.
- Sandbox validation: `docker build -t deepresearch-python-sandbox:latest -f containers/python-sandbox/Dockerfile .` then `DEEPRESEARCH_SANDBOX_TESTS=1 cargo test -p deepresearch-core --test sandbox -- --ignored --nocapture`.

---

## Memory & Retrieval
- Default stub retriever keeps everything in-memory (safe for tests).
- Enable Qdrant + FastEmbed by wiring `SessionOptions::with_qdrant_retriever(url, collection, concurrency)` (and the matching `ResumeOptions`).
- Documents are ingested via `ingest_documents` or the CLI (`deepresearch-cli ingest --session <id> --path <docs> --qdrant-url http://localhost:6334` — gRPC endpoint).
- `HybridRetriever` stores vectors in Qdrant (dense cosine similarity) and constrains load with a semaphore.
- `FactCheckTask` sits between Analyst and Critic; configure it via `FactCheckSettings` (min confidence, verification attempts, simulated timeout).

```rust
let summary = run_research_session_with_options(
    SessionOptions::new(query)
        .with_qdrant_retriever("http://localhost:6334", "deepresearch", 8)
        .with_fact_check_settings(FactCheckSettings {
            min_confidence: 0.8,
            verification_count: 5,
            timeout_ms: 150,
        })
        .with_session_id(session_id.clone()),
).await?;

// Ingest supporting material
ingest_documents(IngestOptions {
    session_id: session_id.clone(),
    documents: vec![IngestDocument {
        id: Uuid::new_v4().to_string(),
        text: doc_text,
        source: Some("notes/report.txt".into()),
    }],
    retriever: RetrieverChoice::qdrant("http://localhost:6334", "deepresearch", 8),
}).await?;
```

---

## Extending the Pipeline
- **Branching:** Use the customiser hook to insert tasks or additional conditional edges.  
- **Parallelism:** Wrap child tasks with `graph_flow::FanOutTask` (see upstream examples) if you require concurrent retrieval.  
- **Persistence:** Replace `InMemorySessionStorage` with the `PostgresSessionStorage` from the crate when durability is required.  
- **Ingestion:** Use `deepresearch-cli ingest --session <id> --path <docs> --qdrant-url http://localhost:6334` to index local files into Qdrant (ensure port 6334 is exposed with `QDRANT__SERVICE__GRPC_PORT=6334`).
- **Evaluation:** Analyse nightly logs with `EvaluationHarness::analyze_log(...)` to track fact-check confidence and failures.
- **CI**: GitHub Actions enforces fmt/clippy/tests/snapshot/bench/API; see `docs/release/CI_GUIDE.md` for the full matrix.

Document any new context keys or task IDs in this file to keep downstream contributors aligned.

### Storage Backends & Session Control
- **In-memory (default):** `SessionOptions::new(query)` stores sessions only for the duration of the process.  
- **Postgres:** Compile with `--features postgres-session` (or enable in `Cargo.toml`) and call `SessionOptions::with_postgres_storage(database_url)` / `ResumeOptions::with_postgres_storage(...)` to persist sessions.  
- **Resume:** Use `ResumeOptions::new(session_id)` + `resume_research_session` to continue an existing workflow (CLI support via `deepresearch-cli resume`).
