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

### M1 — Observability & Testing
- [ ] Add structured tracing (span per task) and log formatting defaults.
- [ ] Provide integration test covering a full session loop.
- [ ] Document context keys emitted by each task.

### M2 — Branching & Extensibility
- [ ] Introduce conditional edge (e.g., route to manual-review task when `critique.confident == false`).
- [ ] Add example of injecting custom task into the graph from client code.
- [ ] Support custom session IDs and configurable inputs via public API.

### M3 — Persistence & Replay
- [ ] Swap to `PostgresSessionStorage` behind a feature flag.
- [ ] Implement helper to resume sessions and fetch latest `ExecutionResult`.
- [ ] Add CLI argument to resume from an existing session.

### M4 — Memory & Retrieval Layer
- [ ] Integrate Qdrant client with per-session namespaces and async backpressure limits.
- [ ] Implement hybrid retrieval combining dense + sparse scores (FastEmbed or equivalent).
- [ ] Add document ingestion command that indexes local files via CLI.

### M5 — Fact-Checking & Evaluation
- [ ] Build configurable fact-check task honoring `min_confidence`, `verification_count`, `timeout_ms`.
- [ ] Record confidence scores and source IDs in context for downstream reporting.
- [ ] Provide nightly evaluation harness that reads log outputs and aggregates accuracy metrics.

### M6 — Explainability & Trace Serialization
- [ ] Emit `graph_flow` events into a TraceCollector and persist `trace.json` per session.
- [ ] Generate reasoning graph summaries for CLI `--explain` and API responses.
- [ ] Document trace schema and add tooling to render GraphViz/Mermaid.

### M7 — Interfaces (CLI & API)
- [ ] Flesh out CLI commands (`query`, `ingest`, `eval`, `explain`, `resume`, `purge`) with structured output.
- [ ] Build Axum API server exposing `POST /query`, `GET /session/:id`, `POST /ingest`.
- [ ] Enforce capacity limits (HTTP 429) and include explainability toggles in responses.

### M8 — Security, Privacy & Logging
- [ ] Enforce env-only secrets, session purge, and data retention policies.
- [ ] Implement JSON logging pipeline with rotation (`data/logs/<year>/<month>/session.jsonl`) and redaction.
- [ ] Add audit logging when PII or secrets are stripped; document compliance posture.

### M9 — Performance & Release Gates
- [ ] Instrument spans for each task; capture latency metrics (median ≤150s, P95 ≤240s).
- [ ] Add concurrency/backpressure tests ensuring semaphore/session caps hold under load.
- [ ] Validate release criteria from PRD (≥80% fact verification, CLI/API stability, docs updated).
- [ ] Prepare release checklist, tagging, and README/PRD alignment.

---

## Cross-Cutting Tasks
- [ ] Establish testing harness (`cargo test --offline`) and add CI instructions.
- [ ] Maintain `AGENTS.md` when adding/removing context keys or tasks.
- [ ] Keep `docs/TESTING_GUIDE.md` aligned with the active milestone.

---

*Last updated:* 2025-10-18
