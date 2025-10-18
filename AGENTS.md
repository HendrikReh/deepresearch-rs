# DeepResearch Agent Pipeline — Developer Guide

This repo hosts a fresh graph-first implementation of DeepResearch. All agent behaviour is composed with the [`graph_flow`](https://docs.rs/graph-flow/latest/graph_flow/) crate; there are no bespoke orchestrators or ad-hoc DAG executors.

---

## System Snapshot
- **Workflow:** Researcher → Analyst → Critic tasks executed through `graph_flow`.
- **Crates:**  
  - `deepresearch-core` — reusable tasks and workflow helpers.  
  - `deepresearch-cli` — demo binary that runs the default research session.
- **Primary dependencies:** `graph-flow`, `tokio`, `anyhow`, `tracing`.

---

## Module Guide

| Path | Purpose | Notes |
|------|---------|-------|
| `crates/deepresearch-core/src/tasks.rs` | Implements `ResearchTask`, `AnalystTask`, `CriticTask` (`graph_flow::Task`) | Stores intermediate state in `Context` keys like `research.*`, `analysis.*`, `critique.*` |
| `crates/deepresearch-core/src/workflow.rs` | Builds the workflow graph and runs sessions via `FlowRunner` | Uses `InMemorySessionStorage` and loops until `ExecutionStatus::Completed` |
| `crates/deepresearch-cli/src/main.rs` | Initializes tracing and runs a sample session for a hard-coded query | Prints the critic verdict + summary string returned from `run_research_session` |

---

## How the Graph Runs
1. `build_graph()` registers the three agent tasks on a `GraphBuilder` and adds edges `researcher -> analyst -> critic`.  
2. A new session is created with `Session::new_from_task(...)`; the query is stored in the session `Context`.  
3. `FlowRunner::run` executes until the critic ends the workflow (`NextAction::End`).  
4. The final report is assembled from context keys (`analysis.output`, `critique.verdict`, `critique.confident`).

Agents may extend their behaviour by reading/writing new context keys; the workflow automatically carries state across tasks thanks to `graph_flow::Context`.

---

## Shared Context Keys

| Key | Producer | Type | Purpose |
|-----|----------|------|---------|
| `query` | `ResearchTask` (seeded via workflow) | `String` | User prompt driving the session. |
| `research.findings` | `ResearchTask` | `Vec<String>` | Bullet insights gathered during retrieval. |
| `research.sources` | `ResearchTask` | `Vec<String>` | Source URIs backing the findings. |
| `analysis.output` | `AnalystTask` | `AnalystOutput` (summary/highlight/sources) | Structured synthesis consumed by the critic. |
| `critique.confident` | `CriticTask` | `bool` | Indicates whether automated checks pass. |
| `critique.verdict` | `CriticTask` | `String` | Human-readable verdict surfaced to the end user. |

All tasks emit tracing spans (`task.research`, `task.analyst`, `task.critic`) and attach structured fields (query, counts, confidence) for observability.

---

## Development Workflow

```bash
cargo fmt                    # format
cargo check --offline        # build without hitting crates.io
cargo run -p deepresearch-cli
```

Add new tasks by implementing `graph_flow::Task` and registering them in `build_graph()`. Prefer `NextAction::ContinueAndExecute` for straight-line execution and `NextAction::End` or `WaitForInput` for pauses.

---

## Extending the Pipeline
- **Branching:** Swap the critic edge for `add_conditional_edge` to fork based on `Context` state.  
- **Parallelism:** Wrap child tasks with `graph_flow::FanOutTask` (see upstream examples) if you require concurrent retrieval.  
- **Persistence:** Replace `InMemorySessionStorage` with the `PostgresSessionStorage` from the crate when durability is required.

Document any new context keys or task IDs in this file to keep downstream contributors aligned.
