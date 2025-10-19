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

---

## v0.2 Roadmap (Planned)

### Objectives
- Ship an Axum-based GUI that visualizes sessions, reasoning graphs, and evidence in real time.
- Integrate a Python-powered math/stats toolchain accessible from the research workflow.
- Stand up a continual learning loop that tunes agent behaviour using captured traces.
- Preserve parity across interfaces (CLI/API/GUI) and enforce the “use context7” prompt rule.

### M10 — Axum GUI Foundations
**Target Window:** Weeks 1–3 of the v0.2 cycle (shift if infra blockers arise)

#### Backend & Infrastructure
- [x] Scaffold the standalone `deepresearch-gui` crate with Axum routing, shared session orchestrator wiring, and Tailwind/Vite asset pipeline. *(Owner: Platform)*
- [ ] Ship managed-container bootstrap: config loader, env-secret wiring, `use context7` prompt enforcement, health/readiness probes, and OpenTelemetry hooks. *(Owner: Platform)*
- [ ] Publish container image + Helm chart draft; integrate with deployment playbook (Immediate Follow-Up #1). *(Owner: DevOps)*

#### Frontend UX & Realtime Flows
- [x] Build chat + evidence panels with SSE/WebSocket streaming, Markdown renderer parity, and context key surface mirroring CLI/API. *(Owner: Frontend)*
- [ ] Provide initial reasoning graph canvas (static JSON viewer) wired to TraceCollector JSON endpoint to unblock M11 visualization upgrades. *(Owner: Frontend)*
- [ ] Implement auth/session selector supporting in-memory & Postgres storage, capacity guardrails, and managed-container session namespace switching. *(Owner: Frontend + Platform)*

#### Quality, Tooling & Documentation
- [ ] Add smoke/integration tests for HTTP endpoints, WebSocket flow, and asset build checks; gate via CI. *(Owner: QA)*
- [x] Document local GUI iteration workflow (dev server, Tailwind watch, Docker override) and update onboarding guide. *(Owner: Docs)*
- [ ] Define acceptance checklist covering end-to-end happy path, health probes, auth gating, and reporting to release readiness dashboard. *(Owner: QA + PM)*

#### Dependencies & Coordination
- **Prereqs:** Immediate Follow-Up #1 draft, storage feature flag parity, TraceCollector JSON endpoint finalized (from M11).
- **Hand-offs:** Container artifact + doc set to DevOps for managed deployment pilot; streaming API contract shared with CLI/API teams.

#### Acceptance Criteria
- GUI crate builds and runs via `cargo run -p deepresearch-gui` and container entrypoint.
- Managed deployment manifests validated in staging cluster with green health/readiness probes.
- Live chat stream renders markdown responses, sources, and context keys; reasoning trace viewer loads latest session JSON.
- Authentication + session selection works against both in-memory and Postgres storage with enforced concurrency caps.
- CI job suite covers HTTP/WebSocket smoke tests and front-end asset compile; docs updated for contributors and operators.

### M11 — GUI Explainability & Observability
- [ ] Render reasoning DAGs and trace timelines inside the GUI using the existing `TraceCollector` output.
- [ ] Surface per-task metrics (latency, retries, confidence) and highlight manual-review branches.
- [ ] Expose explainability toggles (`--explain` parity) and downloadable trace artifacts.
- [ ] Instrument frontend telemetry (OpenTelemetry exporter) and connect to tracing stack.

### M12 — Python Tool Integration (Math & Stats)
- [ ] Design the Python execution bridge (evaluate `pyo3`, `python-subprocess`, or microservice) with sandboxing + timeout guarantees.
- [ ] Implement a `MathToolTask` that routes numeric sub-queries through the Python runtime and returns structured outputs.
- [ ] Add regression tests covering numerical workloads (e.g., regression, summary stats) and failure fallbacks.
- [ ] Document tooling setup (venv management, dependency pinning) and extend CI to validate the Python path offline.

### M13 — Continual Learning & Behavioural Tuning
- [ ] Define data pipeline that snapshots session traces/logs into a training corpus with consent + retention controls.
- [ ] Build offline evaluation harness that scores new model checkpoints before promotion.
- [ ] Automate weekly tuning job (CLI command + GitHub Action) producing performance deltas and release notes.
- [ ] Update governance docs covering review gates, safety tests, and rollback procedures.

### M14 — v0.2 Release Readiness
- [ ] Update PRD, README, and docs (`docs/CI_GUIDE.md`, `docs/TESTING_GUIDE.md`, GUI quickstart) to reflect new interfaces.
- [ ] Establish acceptance criteria (GUI usability ≥4/5, math-tool coverage, continual learning cadence) and add snapshot tests.
- [ ] Run end-to-end load tests across CLI/API/GUI to confirm latency and concurrency targets.
- [ ] Prepare release checklist including packaging, changelog, and deployment playbooks for GUI + Python services.

### Cross-Cutting Initiatives (v0.2)
- [ ] Enforce the “use context7” prefix across CLI, API, and GUI prompts (validation + developer guidance).
- [ ] Expand monitoring dashboards with GUI metrics, Python tool health, and tuning job status.
- [ ] Ensure security/privacy posture carries over to new components (auth, rate limiting, PII redaction).
- [ ] Keep `AGENTS.md` and context-key registry in sync with new tasks and explainability fields.

### Immediate Follow-Ups
- [ ] Document the GUI managed-container deployment playbook (build pipeline, env vars, monitoring hooks).
- [ ] Draft the Python 3.13 service contract covering API surface, timeout limits, and security model; publish in docs.
- [ ] Provide docker-compose overrides that launch the GUI and Python service together for local full-stack testing.

#### Resolved Decisions
- GUI ships as a standalone `deepresearch-gui` crate/binary.
- Optimize the initial GUI build for managed container deployments.
- Python tooling will run via an external service standardized on Python 3.13.

#### Open Questions
- _(none pending)_
