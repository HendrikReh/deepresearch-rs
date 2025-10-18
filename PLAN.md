# DeepResearch Implementation Plan (v0.1)

Structured execution plan derived from `PRD.md` for delivering the DeepResearch MVP (v0.1). Update this file as work progresses (mark checkboxes, add notes, refine scope).

---

## 1. Objectives & Success Criteria
- Deliver multi-agent research orchestration that meets PRD Sections 5–13.
- Ship CLI & API surfaces with session persistence and explainability toggles (`PRD.md §3-4, §7`).
- Achieve ≥80% verified claims on benchmark suite and median end-to-end latency ≤150s (`PRD.md §12, §16`).
- Provide auditable reasoning graph, logging, and citation trail (`PRD.md §5.7-5.8, §9, §13`).

---

## 2. Dependencies & Environment
- Rust stable toolchain, `cargo` workspace set up (`PRD.md §1`).
- Running Qdrant instance with configured namespaces (`PRD.md §5.4-5.5, §8`).
- LLM provider credentials (OpenAI GPT-5 or Ollama) available via env vars (`PRD.md §4, §10, §11`).
- Optional: OpenTelemetry collector if instrumentation feature flag is enabled (`PRD.md §12`).

---

## 3. Milestones & Workstreams

### Milestone 0 — Foundations & Infrastructure
- [x] **Workspace scaffolding**: ensure crate structure (`core`, `agents`, `cli`, `api`) matches PRD (§1, §6).
- [x] **Config loader**: implement `config.toml` parsing with validation (`PRD.md §4, §11`).
- [x] **Tracing + logging setup**: initialize `tracing` subscriber, log formatting, OpenTelemetry hook stub (`PRD.md §12`).
- [x] **Error taxonomy**: define shared `DeepResearchError`, `TaskError` enums (`PRD.md §6.2, §12`).
- [x] **Security baseline**: enforce env-only secrets, session purge CLI command skeleton (`PRD.md §11`).

### Milestone 1 — Planner & Orchestration Core
- [ ] **Task graph types**: implement `PlannerAgent`, `TaskGraph`, validation (cycle detection) (`PRD.md §5.1, §6.2`).
- [ ] **Rig orchestrator integration**: topological execution, retry/backoff, concurrency limits (5 sessions) (`PRD.md §5.3, §12`).
- [ ] **Event bus**: emit explainability events (`Event::Start/Finish/Message`) to collector (`PRD.md §6.2, §13`).
- [ ] **Agent role scaffolding**: define `Researcher`, `Analyst`, `Critic` contexts and message contracts (`PRD.md §5.3`).

### Milestone 2 — Retrieval, Memory, and Fact-Checking
- [ ] **Qdrant client wrapper**: async traits with semaphore guard (8 concurrent searches) (`PRD.md §5.4-5.5, §6.2`).
- [ ] **FastEmbed integration**: dense + sparse embedding pipeline, summarization policy stub (`PRD.md §5.4-5.5`).
- [ ] **Hybrid search API**: combine semantic + keyword scores, return ranked sources (`PRD.md §5.5`).
- [ ] **Fact checker engine**: implement configurable thresholds (`min_confidence=0.75`, `verification_count=3`, `timeout_ms=20000`) with iterative retrieval (`PRD.md §5.6`).
- [ ] **Citation manager**: assign source IDs, inline references, source metadata block (`PRD.md §5.7`).

### Milestone 3 — Explainability & Logging
- [ ] **Trace collector service**: capture events, persist to `graph_trace.json`, enforce 500ms ack fallback (`PRD.md §5.8, §6.2, §13`).
- [ ] **Reasoning graph serializer**: DAG -> JSON with nodes, edges, agent messages (`PRD.md §5.8, §13`).
- [ ] **Logging pipeline**: JSON schema writer, storage rotation (`data/logs/<year>/<month>/session.jsonl`), redaction (`PRD.md §9`).
- [ ] **Evaluation metrics job (stub)**: compute accuracy, citation recall, latency summary for nightly batch (`PRD.md §9, §16`).

### Milestone 4 — Interfaces & User Experience
- [ ] **CLI commands**: `query`, `ingest`, `eval`, `explain`, `resume`, `purge` with REPL support and explain flag (`PRD.md §3, §7, §11`).
- [ ] **API server**: Axum routes (`POST /query`, `GET /session/:id`, `POST /ingest`), schema validation, HTTP 429 on capacity (`PRD.md §3, §4, §12`).
- [ ] **Response formatting**: Markdown + JSON output, include `partial_sources` flag when fallback triggered (`PRD.md §4, §12`).
- [ ] **Session persistence**: namespace-aware storage, resume logic, manual re-indexing command (`PRD.md §5.4, §8`).

### Milestone 5 — Performance, Security, and Quality Gates
- [ ] **Latency instrumentation & tests**: measure median/P95, enforce fallback after 20s external search (`PRD.md §12`).
- [ ] **Concurrency / backpressure tests**: ensure semaphore and session caps hold under load (`PRD.md §12`).
- [ ] **Privacy & purge workflows**: validate session purge command, audit logging for redactions (`PRD.md §11`).
- [ ] **Benchmark suite**: build representative research scenarios to validate ≥80% fact verification (`PRD.md §16`).
- [ ] **Documentation updates**: update `AGENTS.md`, API schema, user guides as features land (`PRD.md §15 exit criteria`).

---

## 4. Cross-Cutting Tasks
- [ ] **Testing strategy**: unit (`#[cfg(test)]`), integration (`tests/`), async harness via `tokio::test`, mocking LLM/Qdrant (`PRD.md §5, §12`).  
- [ ] **CI pipeline**: ensure `cargo fmt`, `cargo clippy -D warnings`, `cargo test`, and optional `cargo audit` run per milestone (`PRD.md §15, §16`).
- [ ] **Observability feature flag**: allow enabling OpenTelemetry exporter without code changes (`PRD.md §12`).
- [ ] **Roadmap tracking**: update PLAN.md after each milestone; revisit scope for v0.2/v0.3 items when MVP stabilizes (`PRD.md §15`).

---

## 5. Risk Register & Mitigations
- **LLM rate limits**: implement exponential backoff and provider failover (OpenAI → Ollama) (`PRD.md §5.3, §10`).  
- **Qdrant availability**: caching layer for recent embeddings; degrade to read-only mode when writes fail (`PRD.md §5.4-5.5`).  
- **Explainability performance**: large traces may exceed memory; stream events to disk when channel backlog > N (`PRD.md §13`).  
- **Security compliance**: review redaction + retention before v0.1 release to avoid privacy regressions (`PRD.md §11`).  
- **Latency regression**: add performance tests to CI and monitor instrumentation output (`PRD.md §12`).

---

## 6. Tracking & Updates
- Use checkbox state (`[ ]` → `[x]`) to record completion.  
- Add dated notes under relevant tasks when scope changes or blockers arise.  
- For newly discovered work, append subtasks beneath the appropriate milestone or cross-cutting section.  
- Review PLAN.md weekly to ensure alignment with PRD success criteria.

---

*Last updated:* 2024-11-24
