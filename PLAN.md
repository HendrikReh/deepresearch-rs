# DeepResearch Roadmap (v0.2.1)

This plan aligns with the consolidated PRD (`PRD.md`) and preserves the history of completed GraphFlow milestones. Update checkboxes and notes as work evolves.

---

## Objectives
- Deliver faithful, audience-fit explanations (global + local) with traceable sources and knowledge-limit disclosures.
- Provide process transparency across planner, orchestrator, retrieval, critique, and UX surfaces.
- Achieve AIS-backed attribution, faithfulness probes, and counterfactual nudges for claims.
- Maintain provenance artefacts (PROV-O / OpenLineage), logging, and disclosures aligned with NIST AI RMF / EU AI Act guidance.

---

## Completed Milestones (Historical)

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

## Cross-Cutting Tasks ✅
- [x] Establish testing harness (`cargo test --offline`) and expand CI documentation (`docs/CI_GUIDE.md`, CI workflow enforcing fmt/clippy/tests/snapshot/bench/API).
- [x] Maintain `AGENTS.md` when adding/removing context keys or tasks (updated with CI commands).
- [x] Keep `docs/TESTING_GUIDE.md` aligned with the active milestone.

### M10 — Axum GUI Foundations

#### Backend & Infrastructure
- [x] Scaffold the standalone `deepresearch-gui` crate with Axum routing, shared session orchestrator wiring, and Tailwind/Vite asset pipeline. *(Owner: Platform)*
- [x] Ship managed-container bootstrap: config loader, env-secret wiring, `use context7` prompt enforcement, health/readiness probes, and OpenTelemetry hooks. *(Owner: Platform)*
- [x] Publish container image + Helm chart draft; integrate with deployment playbook (Immediate Follow-Up #1). *(Owner: DevOps)*

#### Frontend UX & Realtime Flows
- [x] Build chat + evidence panels with SSE/WebSocket streaming, Markdown renderer parity, and context key surface mirroring CLI/API. *(Owner: Frontend)*
- [x] Provide initial reasoning graph canvas (static JSON viewer) wired to TraceCollector JSON endpoint to unblock M11 visualization upgrades. *(Owner: Frontend)*
- [x] Implement auth/session selector supporting in-memory & Postgres storage, capacity guardrails, and managed-container session namespace switching. *(Owner: Frontend + Platform)*

#### Quality, Tooling & Documentation
- [x] Add smoke/integration tests for HTTP endpoints, WebSocket flow, and asset build checks; gate via CI. *(Owner: QA)*
- [x] Document local GUI iteration workflow (dev server, Tailwind watch, Docker override) and update onboarding guide. *(Owner: Docs)*
- [x] Define acceptance checklist covering end-to-end happy path, health probes, auth gating, and reporting to release readiness dashboard. *(Owner: QA + PM)*

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

#### Visualization Experience
- [x] Render an interactive reasoning DAG sourced from `TraceCollector` JSON (pan/zoom, task hover details, branch highlighting).
- [x] Add a chronological trace timeline that streams updates live via the existing SSE endpoint and annotates manual-review decisions.
- [x] Persist the final trace view alongside the summary so resumed sessions load historical explainability data.

#### Metrics & Telemetry
- [x] Surface per-task metrics (latency, retries, confidence) in both the DAG inspector and a dedicated metrics panel.
- [x] Emit GUI runtime metrics and logs through OpenTelemetry when `GUI_OTEL_ENDPOINT` is set; include session ids, task ids, and stream health counters.
- [x] Integrate capacity and latency gauges into the existing monitoring dashboards (Grafana/Datadog hand-off).

#### UX Controls & Export
- [x] Provide explainability toggles matching CLI/API (`--explain`, markdown, mermaid, graphviz) and ensure queries continue to auto-prefix `use context7`.
- [x] Enable download buttons for trace artifacts (JSON, markdown, mermaid) and guard access behind the optional bearer token.
- [x] Add a session comparison mode that diffs two runs on metrics and verdicts to support QA/analyst reviews.

#### Quality & Documentation
- [x] Extend GUI integration tests to cover explainability toggles, download endpoints, and DAG rendering fallbacks.
- [x] Update `docs/GUI_ACCEPTANCE.md` with the new explainability scenarios and add troubleshooting notes for telemetry configuration.
- [x] Provide operator guidance on sizing OTEL exporters and alert thresholds for stream failure rates.

#### Dependencies & Coordination
- **Prereqs:** TraceCollector JSON schema finalized (M10 follow-up), OTEL collector availability, metrics taxonomy aligned with API/CLI surfaces.
- **Hand-offs:** Visualization contract shared with CLI/API teams; telemetry pipeline validated with Platform observability group.

#### Acceptance Criteria
- GUI renders the reasoning DAG and live timeline for active sessions and shows persisted views for completed runs.
- Metrics panel surfaces latency, retry counts, confidence, and manual-review flags per task with export capability.
- Explainability toggles mirror CLI/API behaviour, maintain the `use context7` prompt prefix, and gated downloads function with auth.
- OTEL spans/metrics flow to the configured collector with documentation covering setup, dashboards, and alerts.
- Automated tests and updated docs cover explainability flows end-to-end and are exercised in CI.

_Key artefacts (retained for reference):_ GraphTrace JSON, trace explain toggles, docs updates (`GUI_ACCEPTANCE.md`, `GUI_DEPLOYMENT.md`, `TESTING_GUIDE.md`).

---

## Active Milestones

### M12 — Python Tool Integration (Math & Stats)
**Target Window:** Weeks 7–9 of v0.2 cycle

- Architecture & Platform
  - [ ] Decide embedded `pyo3` vs. sidecar microservice (sandboxing, quotas, restart semantics)
  - [ ] Define `MathToolRequest` / `MathToolResponse` schema + error taxonomy (Rust ↔ Python)
  - [ ] Package strategy (Docker image + optional virtualenv) with reproducible builds
- Workflow Integration
  - [ ] Implement `MathToolTask` that preserves `use context7` prefix and writes `math.*` context keys
  - [ ] Route Researcher/Analyst through math tool; ensure Critic consumes structured outputs (values, units, confidence)
  - [ ] Add timeout / retry / graceful degradation path when Python is unavailable
- Reliability & Operations
  - [ ] Health + readiness probes for Python service (latency/error/concurrency metrics via OTEL)
  - [ ] Docker-compose overrides for GUI + API + Python + data services (local full-stack)
  - [ ] Runbooks for dependency upgrades, vulnerability patches, independent scaling
- Quality & Testing
  - [ ] Integration tests for representative math workloads (regression, summary stats, error propagation)
  - [ ] Extend CI: `pytest` suite + Rust↔Python smoke test (`cargo test --offline`)
  - [ ] Snapshot critical math responses to catch precision/formatting regressions
- Documentation & Enablement
  - [ ] Contributor guide for Python service (venv, lint, formatting)
  - [ ] API examples for math-heavy queries (CLI/API/GUI)
  - [ ] Security guidance (sandboxing limits, allowed libraries, secret injection)
- Dependencies & Coordination
  - [ ] Align Math Tool API with analytics stakeholders; schedule security review prior to rollout
- Acceptance Criteria
  - [ ] End-to-end sessions use math tool (CLI/API/GUI) with outputs in analyst/critic steps
  - [ ] Failure modes (timeouts/invalid input) degrade gracefully with alerts
  - [ ] CI validates Python + Rust suites; build artifacts reproducible
  - [ ] Docs + runbooks enable engineers/ops to operate and troubleshoot the Python service

### M13 — Continual Learning & Behavioural Tuning
**Target Window:** Weeks 10–12 of v0.2 cycle (post math-tool rollout)

- Data Pipeline & Curation
  - [ ] Automated ETL ingesting traces, verdicts, metrics, math outputs, respecting consent/retention
  - [ ] Taxonomise sessions (domain/confidence/manual review) with outcome labels for supervised tuning
  - [ ] Enforce retention, redaction, audit logging for compliance
- Evaluation Harness
  - [ ] Offline evaluator replaying sessions against new checkpoints (verdict deltas, confidence shifts)
  - [ ] Statistical tests (bootstrap significance) gating promotions
  - [ ] Visual dashboards summarising evaluation runs for PM/QA
- Automation & Ops
  - [ ] CLI command + GitHub Action orchestrating weekly tuning job (data fetch, train/evaluate, metrics, artefacts)
  - [ ] Observability hooks alert on tuning job failure or metric regression
  - [ ] Handle large datasets via batching/distributed execution
- Governance & Documentation
  - [ ] Update governance docs with review gates, safety guardrails, rollback procedures, consent flows
  - [ ] Author guidance for interpreting evaluation reports and residual risk analyses
  - [ ] Contributor guide for adding evaluation scenarios/metrics
- Dependencies & Coordination
  - [ ] Ensure telemetry/math context keys stabilised (M11 prerequisite)
  - [ ] Secure compute resources for training jobs; align with analytics/security stakeholders
- Acceptance Criteria
  - [ ] Governed training corpus continually populated, respecting consent + retention
  - [ ] Evaluation harness yields statistically sound comparisons with dashboards for decisions
  - [ ] Weekly job runs automatically, surfaces alerts, publishes release-ready reports
  - [ ] Governance docs + contribution guides support controlled promotion & rollback

---

## Backlog by PRD Section

### 0 / 1 / 2 — Strategy & Personas
- [x] Document XAI objectives, non-goals, audience personas, and use-case (`explain --claim <ID>`)

### 3 — Interaction Model Enhancements
- [x] CLI explainability switches (`--explain`, `--claim`, provenance dump, card exporters)
- [ ] API: expand schema with `explanations[]`, per-claim AIS, provenance, `evaluation{ ais, faithfulness }` (pending final API release)
- [ ] GUI (post-v0.2): interactive evidence highlights, counterfactual explorer sliders, claim drill-downs

### 4 — Input / Output Schema
- [x] Define enriched output payload (global/local explanations, provenance blocks, evaluation bundle)
- [ ] Implement API/CLI serialization for new fields (depends on AIS engine delivery)

### 5 — Core Capabilities (delta)
- [x] Planner emits rationale summaries + explainable action trait scaffold
- [x] Hypothesis loop registers claims with coverage thresholds
- [ ] Attribution Verifier sub-role (AIS + faithfulness probes wiring in core workflow)
- [x] Qdrant shard metadata (`source_url`, spans, dense/sparse scores, usage references)
- [ ] Late interaction reranker (ColBERT) influence logging
- [ ] Evaluation export with AIS/faithfulness metrics (`evaluation.json`)
- [ ] Per-sentence citation enforcement with support levels in final renderer
- [ ] Counterfactual generator for numeric/threshold claims

### 6 — System Architecture & Provenance
- [x] XAI trace collector integrated with orchestrator pipeline (GUI + CLI)
- [ ] Full provenance store (PROV-O / OpenLineage emitter modules)
- [ ] Documented integration contracts (Planner↔Executor, Agents↔Memory, FactCheck↔Retrieval) — _drafted; publish to `AGENTS.md`_

### 7 — CLI & API Design
- [x] Core CLI surface (query/ingest/eval/explain/resume/purge) retained
- [ ] Extend CLI help/docs for model/data cards, claim explanations (needs doc update)
- [ ] API claim endpoint + provenance export (blocked on AIS implementation)

### 8 — Explainability Methods
- [ ] Implement AIS scoring engine
- [ ] Faithfulness probes (deletion/insertion AUC, leave-one-out)
- [ ] Counterfactual explanation service
- [ ] Document limits (no raw CoT, highlight distinction between faithfulness vs. plausibility)

### 9 — Memory & Persistence
- [x] Namespaced Qdrant storage with dense+sparse vectors, metadata, retention policy
- [ ] Implement embedding encryption at rest (future)

### 10 — Evaluation & Logging
- [x] JSON log schema + rotation (historical)
- [ ] Nightly aggregate job producing evaluation dashboard inputs (ties into M13)
- [ ] Coverage/faithfulness metrics gating CI (future advanced milestone)

### 11 — LLM Integration
- [x] External / local engine matrix documented
- [ ] Configure policy module to redact chain-of-thought while exposing tool I/O (implementation pass pending)

### 12 — Privacy, Security & Compliance
- [x] Baseline safeguards (secret handling, purge, retention)
- [ ] User-facing AI disclosure banner (CLI/API/GUI)
- [ ] Synthetic content labelling
- [ ] Retention >= 6 months configurable (default currently 90 days in logs)
- [ ] Optional ISO/IEC 42001-aligned governance pack

### 13 — Performance & Concurrency
- [x] Latency/backpressure guardrails (historical)
- [ ] Session-level explainability budget enforcement (ablation/counterfactual caps in executor)
- [ ] Cache for AIS/faithfulness results keyed by `(claim_hash, evidence_hash)`

### 14 — Explainability Model
- [x] Trace event typing (`PlanRationale`, `ToolCall`, etc.)
- [x] CLI/GUI overlay for support levels (timeline/metrics toggles implemented in M11)
- [ ] Merge legacy `Start/Finish/Message` trace schema with new event types in docs

### 15 — Deployment
- [x] Managed container playbook (GUI) with telemetry guidance
- [ ] Compose add-ons for PROV/OpenLineage exporters and evaluation artefact volumes
- [ ] OFFLINE mode disclaimers + AIS N/A annotations

### 16 — Roadmap Integration
- [x] Historical v0.1 / v0.2 / v0.3 milestones recorded for continuity
- [ ] Sync roadmap updates with `PLAN.md` after M12/M13 refinements

### 17 — Success Criteria
- [x] Record baseline targets (AIS coverage, faithfulness AUC, provenance completeness, usability rating)
- [ ] Instrument metrics collection to report on success criteria automatically

### 18 — Developer Tasks
- [ ] Implement new modules (`xai_trace.rs`, `provenance.rs`, `ais.rs`, `faithfulness.rs`, `counterfactuals.rs`, `cards.rs`, `hybrid_retrieval.rs`)
- [ ] Schema migration for Qdrant (sparse vectors, retrieval hashes, used_by_claims backfill)
- [ ] CLI/API feature delivery (cards, explain claim, provenance export)

### 19 — UX Guidelines
- [x] Document explanation authoring patterns (decision → why → evidence → limits/what-ifs)
- [ ] Align Business Analyst vs. Research Developer views in GUI/CLI docs

---

## Status Summary
- ✅ Foundations (M0–M11) complete; GUI explainability shipped.
- 🔄 M12 & M13 scheduled for v0.2 cycle.
- 🔁 Backlog tracks AIS/faithfulness/counterfactual engines, provenance exporters, governance disclosures, and evaluation automation.
