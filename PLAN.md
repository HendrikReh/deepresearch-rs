# DeepResearch GraphFlow Roadmap

This plan tracks the new graph-first implementation. Update the checkboxes and add dated notes as work evolves.

---

## Objectives
- Model the Researcher → Analyst → Critic loop exclusively with `graph_flow`.
- Provide a runnable CLI demo returning a critic verdict string.
- Add tests that exercise the workflow end-to-end and per-task behaviour.

---

## Milestones

### M0 — Graph Foundation ✅
- [x] Create workspace with `deepresearch-core` (library) and `deepresearch-cli` (demo binary).
- [x] Implement Researcher/Analyst/Critic tasks using `graph_flow::Task`.
- [x] Build linear workflow with `GraphBuilder` and execute via `FlowRunner`.
- [x] Wire CLI to run `run_research_session` and print the critic verdict.

### M1 — Observability & Testing ✅
- [x] Add structured tracing (span per task) and log formatting defaults.
- [x] Provide integration test covering a full session loop.
- [x] Document context keys emitted by each task.

### M2 — Branching & Extensibility ✅
- [x] Introduce conditional edge (manual-review branch when `critique.confident == false`).
- [x] Add example of injecting custom task into the graph from client code.
- [x] Support custom session IDs and configurable inputs via public API (`SessionOptions`).

### M3 — Persistence & Replay ✅
- [x] Add docker-compose services for Qdrant + Postgres (local development stack).
- [x] Swap to `PostgresSessionStorage` behind a feature flag.
- [x] Implement helper to resume sessions and fetch latest `ExecutionResult`.
- [x] Add CLI argument to resume from an existing session.

### M4 — Memory & Retrieval Layer ✅
- [x] Integrate Qdrant client with per-session namespaces and async backpressure limits (feature `qdrant-retriever`).
- [x] Implement hybrid retrieval combining dense embedding + payload scoring (FastEmbed-backed).
- [x] Add document ingestion command that indexes local files via CLI.

### M5 — Fact-Checking & Evaluation ✅
- [x] Build configurable fact-check task honoring `min_confidence`, `verification_count`, `timeout_ms`.
- [x] Record confidence scores and source IDs in context for downstream reporting.
- [x] Provide nightly evaluation harness that reads log outputs and aggregates accuracy metrics.

### M6 — Explainability & Trace Serialization ✅
- [x] Emit `graph_flow` events into a TraceCollector and persist `trace.json` per session.
- [x] Generate reasoning graph summaries for CLI `--explain` and API responses.
- [x] Document trace schema and add tooling to render GraphViz/Mermaid.

### M7 — Interfaces (CLI & API) ✅
- [x] Flesh out CLI commands (`query`, `ingest`, `eval`, `explain`, `resume`, `purge`) with structured output.
- [x] Build Axum API server exposing `POST /query`, `GET /session/:id`, `POST /ingest`.
- [x] Enforce capacity limits (HTTP 429) and include explainability toggles in responses.
- [x] `/health` capacity reporting endpoint and CLI `bench` load tester to support tuning `DEEPRESEARCH_MAX_CONCURRENT_SESSIONS`.

### M8 — Security, Privacy & Logging ✅
- [x] Enforce env-only secrets, session purge, and data retention policies.
- [x] Implement JSON logging pipeline with rotation (`data/logs/<year>/<month>/session.jsonl`) and redaction.
- [x] Add audit logging when PII or secrets are stripped; document compliance posture.
- [x] Automatic log retention pruning, CLI purge log cleanup, and audit trails for redacted secrets.

### M9 — Performance & Release Gates ✅
- [x] Instrument spans for each task; capture latency metrics (median ≤150s, P95 ≤240s).
- [x] Add concurrency/backpressure tests ensuring semaphore/session caps hold under load.
- [x] Validate release criteria from PRD (≥80% fact verification, CLI/API stability, docs updated).
- [x] Prepare release checklist, tagging, and README/PRD alignment.
- [x] Provide CLI benchmarking utility (`bench`) for throughput tuning and latency measurement experiments.
- [x] `/health` monitoring, CLI bench latency gates (CI thresholds avg ≤350 ms / p95 ≤400 ms), and `docs/RELEASE_CHECKLIST.md` capturing performance + compliance verification.

---

## Cross-Cutting Tasks ✅
- [x] Establish testing harness (`cargo test --offline`) and expand CI documentation (`docs/CI_GUIDE.md`, CI workflow enforcing fmt/clippy/tests/snapshot/bench/API).
- [x] Maintain `AGENTS.md` when adding/removing context keys or tasks (updated with CI commands).
- [x] Keep `docs/TESTING_GUIDE.md` aligned with the active milestone.

---

*Last updated:* 2025-10-19
