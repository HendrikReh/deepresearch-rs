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
**Target Window:** Weeks 4–6 of the v0.2 cycle (follows M10 rollout)

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

### M12 — Python Tool Integration (Math & Stats)
**Target Window:** Weeks 7–9 of the v0.2 cycle (begins once M11 telemetry is stable)

#### Architecture & Platform
- [ ] Finalize the Python execution strategy (embedded `pyo3` module vs. sidecar microservice). Decision must include sandboxing, resource quotas, and restart semantics.
- [ ] Define the Math Tool API contract (`MathToolRequest`, `MathToolResponse`, error taxonomy) and publish protobuf/JSON schema shared across Rust + Python.
- [ ] Provide a hardened packaging story (Docker image and optional virtualenv bootstrap) with dependency pinning and reproducible builds.

#### Workflow Integration
- [ ] Implement `MathToolTask` in `deepresearch-core` that dispatches numeric sub-queries, ensures the `use context7` prefix is preserved in upstream prompts, and records outputs under `math.*` context keys.
- [ ] Add tool routing logic to the Researcher/Analyst workflow so fact-check and critic steps consume structured math outputs (values, units, confidence).
- [ ] Support timeout, retry, and graceful degradation paths (fallback to explanatory text when Python service is unavailable).

#### Reliability & Operations
- [ ] Introduce health and readiness probes for the Python service, plus metrics (invocation latency, error rate, concurrency) exported via OpenTelemetry.
- [ ] Provide docker-compose overrides that run GUI + API + Python tool + data services for local full-stack validation.
- [ ] Document runbooks for dependency upgrades, vulnerability patches, and scaling the Python component independently.

#### Quality & Testing
- [ ] Add integration tests that execute representative math workloads (time-series regression, summary stats, error propagation) and verify critic behaviour on failures.
- [ ] Extend CI to run Python unit tests (`pytest`) and a Rust-to-Python smoke test under `cargo test --offline`.
- [ ] Snapshot high-value math responses to catch regressions in numerical precision or formatting.

#### Documentation & Enablement
- [ ] Update contributor docs with development workflow (venv management, linting, formatting) for the Python service.
- [ ] Publish API examples showing how CLI/API clients submit math-intensive queries and receive structured answers.
- [ ] Provide security guidance covering sandboxing limits, allowed libraries, and how secrets are injected (env vars vs. secret manager).

#### Dependencies & Coordination
- **Prereqs:** M11 telemetry instrumentation operational, OTEL collector sized for additional spans, DevOps ready with Python build pipeline.
- **Hand-offs:** Math Tool API shared with analytics stakeholders; security review scheduled before production rollout.

#### Acceptance Criteria
- Research sessions can invoke the math tool end-to-end (CLI/API/GUI) with results surfaced to analyst/critic steps and logged under new context keys.
- Failure scenarios (timeouts, invalid input) produce safe fallbacks and alerts without blocking the overall workflow.
- CI covers Python unit/integration tests alongside Rust suites, and deployment artifacts (images/venv) are reproducible.
- Documentation and runbooks enable engineers and operators to build, deploy, monitor, and troubleshoot the Python service.

### M13 — Continual Learning & Behavioural Tuning
**Target Window:** Weeks 10–12 of the v0.2 cycle (post math-tool rollout)

#### Data Pipeline & Curation
- [ ] Stand up an automated ETL that ingests session traces, verdicts, metrics, and math tool outputs into a governed training corpus with consent/retention flags.
- [ ] Classify sessions by taxonomy (domain, confidence, manual review) and capture outcome labels needed for supervised tuning.
- [ ] Implement retention, redaction, and audit logging to prove compliance with privacy policies.

#### Evaluation Harness
- [ ] Build an offline evaluator that replays captured sessions against new model checkpoints, collecting verdict deltas, confidence shifts, and regression indicators.
- [ ] Include statistical tests (e.g., bootstrap significance) to decide if model changes meet promotion thresholds.
- [ ] Provide visualization dashboards summarizing evaluation runs for PM/QA review.

#### Automation & Ops
- [ ] Create a CLI command (and GitHub Action) that orchestrates weekly tuning: fetch data, train/evaluate, output metrics, and produce release artifacts.
- [ ] Integrate with observability stack to alert on tuning job failures or metric regressions.
- [ ] Ensure the job handles large datasets via batching or distributed execution as needed.

#### Governance & Documentation
- [ ] Update governance docs with review gates, safety guardrails, rollback procedures, and sign-off requirements for promoting new checkpoints.
- [ ] Document how to interpret evaluation reports, including residual risk analysis and fallback plans.
- [ ] Provide contributor guidance for adding new evaluation scenarios or metrics.

#### Dependencies & Coordination
- **Prereqs:** M10–M12 telemetry/math context keys stable, storage layer sized for historical session retention, access to compute for training jobs.
- **Hand-offs:** Evaluation harness outputs shared with stakeholders; governance changes reviewed with legal/compliance.

#### Acceptance Criteria
- Training corpus pipelines continuously populate governed datasets, respecting consent and retention policies.
- Evaluation harness produces repeatable, statistically sound comparisons with dashboards for decision makers.
- Weekly tuning job runs automatically, surfaces alerts on failure, and generates release-ready reports.
- Governance documentation and contributor guides enable controlled promotion and rollback of tuned behaviours.

### M14 — v0.2 Release Readiness
**Target Window:** Weeks 13–15 of the v0.2 cycle (final stabilization sprint)

#### Release Documentation & Communication
- [ ] Refresh PRD, README, and product docs (`docs/CI_GUIDE.md`, `docs/TESTING_GUIDE.md`, GUI quickstart) to capture GUI explainability, math tool integration, and continual learning flows.
- [ ] Publish a versioned release notes draft including highlights, breaking changes, migration guidance, and the enforced `use context7` prompt rule.
- [ ] Produce operator runbooks consolidating deployment steps for API, GUI, Python tool, and evaluation jobs.

#### Validation & Testing
- [ ] Execute end-to-end load tests across CLI/API/GUI with the math tool enabled, confirming latency, concurrency caps, and error budgets.
- [ ] Complete GUI usability study (target ≥4/5) and document findings plus remediation tasks if thresholds are missed.
- [ ] Freeze and update snapshot/acceptance tests covering new explainability screens, math outputs, and evaluation dashboards.

#### Packaging & Deployment
- [ ] Build and sign release artifacts (Rust binaries, Docker images, Python package) with reproducible provenance metadata.
- [ ] Validate deployment manifests/playbooks for cloud (Helm/Kustomize) and local (docker-compose) environments, ensuring feature flags align.
- [ ] Verify upgrade/migration steps from v0.1 → v0.2, including data migrations, session resume compatibility, and config diffs.

#### Operational Readiness
- [ ] Confirm monitoring dashboards aggregate GUI, Python tool, and tuning job metrics with alert thresholds set and rehearsed.
- [ ] Run incident response tabletop covering math tool failure, telemetry outage, and tuning job regression scenarios.
- [ ] Ensure support/on-call documentation is updated with escalation paths and known issues.

#### Governance & Sign-off
- [ ] Collect approvals from product, security, and compliance stakeholders using the updated governance checklist.
- [ ] Finalize the release checklist (including smoke tests, rollback plan, communication timeline) and obtain sign-off before GA.
- [ ] Schedule post-launch monitoring windows and success metric reviews.

#### Dependencies & Coordination
- **Prereqs:** M10–M13 deliverables accepted; docs from prior milestones merged; observability stack stable post-M11; tuning job pipelines ready.
- **Hand-offs:** Release notes and deployment runbooks delivered to DevOps and Support; marketing brief aligned with product messaging.

#### Acceptance Criteria
- All documentation, runbooks, and release notes reflect v0.2 capabilities and reference the `use context7` requirement.
- Load/acceptance tests demonstrate performance and usability targets across interfaces with math tool and continual learning enabled.
- Deployment artifacts and manifests pass validation for both cloud and local stacks with verified upgrade paths.
- Monitoring, incident response, and support materials are signed off; governance approvals complete and recorded.

### Cross-Cutting Initiatives (v0.2)
- [ ] Expand monitoring dashboards with GUI metrics, Python tool health, and tuning job status.
- [ ] Ensure security/privacy posture carries over to new components (auth, rate limiting, PII redaction).
- [ ] Keep `AGENTS.md` and context-key registry in sync with new tasks and explainability fields.

### Immediate Follow-Ups
- [x] Document the GUI managed-container deployment playbook (build pipeline, env vars, monitoring hooks).
- [ ] Draft the Python 3.13 service contract covering API surface, timeout limits, and security model; publish in docs.
- [ ] Provide docker-compose overrides that launch the GUI and Python service together for local full-stack testing.

#### Resolved Decisions
- GUI ships as a standalone `deepresearch-gui` crate/binary.
- Optimize the initial GUI build for managed container deployments.
- Python tooling will run via an external service standardized on Python 3.13.

#### Open Questions
- _(none pending)_
