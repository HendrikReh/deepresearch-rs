# DeepResearch XAI PRD (v0.2.1)

## Table of Contents
- [1. Vision](#1-vision)
- [2. Strategic Goals](#2-strategic-goals)
- [3. Explainability Principles](#3-explainability-principles)
- [4. Personas & Use Cases](#4-personas--use-cases)
- [5. Interaction Surfaces](#5-interaction-surfaces)
- [6. Inputs & Outputs](#6-inputs--outputs)
- [7. System Architecture](#7-system-architecture)
- [8. Core Capabilities](#8-core-capabilities)
  - [8.1 Planner & Dynamic Reasoning](#81-planner--dynamic-reasoning)
  - [8.2 Hypothesis & Claim Management](#82-hypothesis--claim-management)
  - [8.3 Multi-Agent Collaboration](#83-multi-agent-collaboration)
  - [8.4 Memory & Retrieval](#84-memory--retrieval)
  - [8.5 Evaluation & Faithfulness](#85-evaluation--faithfulness)
  - [8.6 Citation & Provenance](#86-citation--provenance)
  - [8.7 Explainability Surfaces](#87-explainability-surfaces)
  - [8.8 Future Math & Quant Modules](#88-future-math--quant-modules)
- [9. Explainability Programme](#9-explainability-programme)
- [10. Memory & Data Governance](#10-memory--data-governance)
- [11. Evaluation & Observability](#11-evaluation--observability)
- [12. LLM Integration Framework](#12-llm-integration-framework)
- [13. Privacy, Security & Compliance](#13-privacy-security--compliance)
- [14. Performance & Operational Excellence](#14-performance--operational-excellence)
- [15. Deployment & Packaging](#15-deployment--packaging)
- [16. Roadmap](#16-roadmap)
- [17. Success Metrics](#17-success-metrics)
- [18. Delivery Workstreams](#18-delivery-workstreams)
- [19. UX Writing & Artefact Guidelines](#19-ux-writing--artefact-guidelines)
- [Appendix A — References](#appendix-a--references)
- [Implementation Tip (Rust)](#implementation-tip-rust)
- [Status](#status)

---

## 1. Vision
DeepResearch is our flagship demonstration of **Explainable AI in action**: a Rust-first, multi-agent workflow that not only answers complex business questions but *proves* how it reached every conclusion. The product’s core mandate is to showcase best-in-class XAI—faithful reasoning, traceable evidence, and user-ready disclosures—without sacrificing speed or autonomy. We position DeepResearch as:
- A living case study for clients evaluating AI adoption with strict transparency requirements.
- An engineering reference for building graph-driven, multi-agent systems with explainability baked into every layer.
- A testbed for emerging governance expectations (NIST AI RMF, EU AI Act, ISO/IEC 42001).

## 2. Strategic Goals
1. Provide decision-ready research reports with verifiable, audience-fit explanations.
2. Expose process transparency across planner, orchestrator, retrieval, critique, and UX surfaces.
3. Guarantee attribution to identified sources using AIS scoring and provenance standards.
4. Maintain compliance-ready artefacts (PROV-O, OpenLineage, model/data cards, audit logs).
5. Deliver a portable, modular codebase illustrating explainable multi-agent orchestration in Rust.

## 3. Explainability Principles
We anchor design and delivery on the four NISTIR 8312 principles:
- **Explanation**: Every answer includes “what” and “why”, plus evidence and limits.
- **Meaningful**: Explanation format adapts to the intended persona (analyst vs. developer).
- **Explanation Accuracy**: Rationales faithfully reflect actual computation and evidence.
- **Knowledge Limits**: The system discloses uncertainty, assumptions, and missing data.
Additionally, we align with EU AI Act transparency/logging expectations and AIS (Attributable to Identified Sources) methodology.

## 4. Personas & Use Cases
- **Business Analyst** – wants concise decision rationale, highlighted evidence, and “what would change the conclusion?” insights.
- **Research Developer** – inspects plan evolution, tool I/O, retrieval hits, and ablations to extend the system.
- **AI Consultant** – needs audit artefacts (provenance, logs, model/data cards) for due diligence demos.

**Hero Use Case**: `deepresearch explain --claim <ID>` returns supporting snippets, provenance handles, AIS verdict, confidence history, and counterfactual nudges for numeric/threshold claims.

## 5. Interaction Surfaces
| Interface | Explainability Features |
| --- | --- |
| CLI | `--explain`, `--claim`, `trace --prov`, `report --model-card`, `report --data-card` |
| API | `explanations[]`, per-claim AIS metadata, `provenance`, `evaluation{ ais, faithfulness }` |
| GUI (Axum + Tailwind, v0.2) | Streaming evidence highlights, plan graph overlays, counterfactual explorer |

## 6. Inputs & Outputs
### Inputs
- Natural-language prompts with optional metadata (sources, depth, explainability flag).
- CLI session controls (`--session`, `--resume`, `--purge`).

### Outputs
- Markdown report (default) plus structured JSON schema.
- Explainability payload:
  - `explanations.global`: plan rationale, trade-offs, knowledge limits.
  - `explanations.local[]`: claim ID, rationale summary, evidence handles (source IDs + spans), AIS score, faithfulness metrics, optional counterfactuals.
  - `provenance`: PROV-O / OpenLineage fragments.
  - `evaluation`: AIS coverage, faithfulness (deletion/insertion AUC), latency, outcome.

## 7. System Architecture
### 7.1 Flow
```
User Query
   │
Planner Agent ──► Plan Rationale
   │
Rig Orchestrator ──► XAI Trace Collector
   │
Researcher  ──► Retrieval (hybrid) ──► Evidence Store (Qdrant)
Analyst     ──► Synthesis ──► Draft Claims
Critic      ──► AIS + Faithfulness Probes
Attribution Verifier ──► Claim Verdicts
   │
Result Assembler ──► Report + Explanations + Provenance
```

### 7.2 Components
| Component | Responsibility | Tooling |
| --- | --- | --- |
| Graph-Flow Executor | Execute/monitor DAG tasks | `graph-flow` |
| Planner Agent | Build task graph, assign roles | Custom module |
| Agents | Role-specific LLM sub-tasks | OpenAI GPT-5 / Ollama |
| Secure Sandbox Runner | Execute untrusted Python code for math/stats tooling with strict isolation | Docker (`python:3.11-slim`), Bollard/CLI wrapper, hardened runtime flags |
| Vector DB | Memory + retrieval | Qdrant (dense + sparse) |
| Hybrid Embedding Engine | Dense + BM42 vectors | FastEmbed (optional ColBERT reranker) |
| XAI Trace Collector | Capture plan/tool/retrieval events | Custom module |
| CLI / API / GUI | User surfaces | `clap`, Axum, Tailwind |

**Integration Notes**
- Planner ↔ Executor: planner returns `TaskGraph`; executor validates DAG before run.
- Agents ↔ Memory: async traits with backpressure (Semaphore cap 8) to protect Qdrant latency.
- Fact Checker ↔ Retrieval: `claim_id`, expected sources; fall back to cached embeddings on failure.
- Trace Collector flushes events if consumers don’t ack within 500 ms.
- Node failures emit `TaskError { reason, retryable }` with exponential backoff retries.
- Sandbox orchestrator prepares per-run volumes, enforces read-only/rootless execution, captures outputs, and emits health metrics.

### 7.3 Provenance Schemas
- **PROV-O**: `prov:Entity` (source/snippet/embedding), `prov:Activity` (retrieval/synthesis), `prov:Agent` (Researcher/Analyst/Critic).
- **OpenLineage (optional)**: job = session/plan, run = execution instance, datasets = corpora/web artefacts.

## 8. Core Capabilities
### 8.1 Planner & Dynamic Reasoning
- Decomposes prompts into DAGs; updates strategy as facts arrive.
- Nodes implement `ExplainableAction` for rationale, evidence handles, provenance.

### 8.2 Hypothesis & Claim Management
- Hypotheses registered as claims with evidence sets and confidence.
- Loop halts when confidence + coverage thresholds met or timeouts fire.

### 8.3 Multi-Agent Collaboration
- Roles: Researcher, Analyst, Critic, Attribution Verifier.
- Independent LLM contexts (GPT-5 or Ollama) with shared memory.
- Message logging yields explainable dialogue trace; Tokio orchestrates sequential/parallel execution.

### 8.4 Memory & Retrieval
- Qdrant shards store `source_url`, spans, dense/sparse scores, `insert_ts`, hashes, `used_by_claims[]`.
- Hybrid retrieval logs influence (dense vs. sparse contributions per claim).

### 8.5 Evaluation & Faithfulness
- AIS scoring (full/partial/none support) per claim.
- Faithfulness probes: deletion/insertion AUC, leave-one-out.
- Counterfactual explanations for numeric/threshold claims.
- Baseline guardrails: confidence ≥0.75 with up to 3 retrieval cycles (configurable `factcheck.*`).

### 8.6 Citation & Provenance
- Enforce per-sentence inline citations with support levels.^
- Provenance handles (source hash + span offset) for auditability.

### 8.7 Explainability Surfaces
- Reasoning DAG overlays support levels and evidence flow.
- PROV/O and OpenLineage exports for audits.
- CLI `--explain` renders plan graph; GUI highlights evidence and counterfactual sliders (v0.2).

### 8.8 Python Math Sandbox
- Hardened Docker sandbox executes Python tooling (formulas, stats, plotting) with pre-baked libraries (Matplotlib, NetworkX, Pandas, Mermaid CLI).
- Rust orchestrator mounts ephemeral workspace, captures outputs (PNG/SVG/PDF) and error taxonomy.
- Roadmap: expand to additional quant libraries, sidecar retriever ingestion, and cached artefact reuse.

## 9. Explainability Programme
- **Global explanations**: plan rationale, trade-offs, knowledge limits.
- **Local explanations**: claim rationales, evidence, AIS score, faithfulness metrics, counterfactuals.
- **Faithfulness vs. Plausibility**: critic reports both; avoids persuasive but unfaithful rationales.
- **Governance artefacts**: PROV/OpenLineage exports, model/data cards.

## 10. Memory & Data Governance
- Namespaced Qdrant storage; session state persisted to local store.
- Manual re-indexing via CLI; future encryption at rest.
- Retention defaults: embeddings 30 days (configurable), logs 90 days (recommended ≥6 months).

## 11. Evaluation & Observability
- Runtime metrics JSON (`session_id`, `query`, `claims[]`, coverage, latency, outcome).
- Faithfulness metrics (deletion/insertion AUC) and AIS coverage tracked per run.
- Artefacts: `graph_trace.json`, `prov.json`, `openlineage.json`, `model_card.md`, `datasheet.md`.
- Logs rotate weekly (`data/logs/<year>/<month>/session.jsonl`).

## 12. LLM Integration Framework
| Mode | Engine | Interface | Policy |
| --- | --- | --- | --- |
| External | OpenAI GPT-5 | Official Rust SDK | Default |
| Local | Ollama runtime | HTTP API | Deployment configurable |
| Failover | None | Fail-fast | Log + user notice |

- Expose summaries + tool I/O; never raw chain-of-thought.
- Critic self-check loop verifies AIS support, flags unfaithful passages.

## 13. Privacy, Security & Compliance
- CLI/API/GUI disclose AI usage; label synthetic content.
- Per-user purge (`deepresearch purge --session <ID>`).
- Environment-only secrets; config forbids inline credentials.
- Default trace retention 30 days; logs 90 days; policies configurable for EU AI Act readiness.
- Secure Docker sandbox disables networking, drops Linux capabilities, enforces read-only rootfs with tmpfs scratch space, and logs sandbox health metrics.
- Hooks for ISO/IEC 42001-style governance.

## 14. Performance & Operational Excellence
- Tokio-based concurrency; semaphores cap hybrid searches.
- Latency targets: median ≤150 s, P95 ≤240 s (CI fails if avg >350 ms or P95 >400 ms).
- Backpressure: maximum five concurrent sessions (HTTP 429 with retry hints beyond cap).
- Explainability budget caps ablations/reranks; caches results keyed by `(claim_hash, evidence_hash)`.

## 15. Deployment & Packaging
- Components: Rust binary (agent + CLI + API), Qdrant, optional Ollama.
- Docker Compose template for local/full-stack setups (includes hardened Python sandbox image and optional Mermaid sidecar).
- Optional OpenLineage emitter + provenance/evaluation volumes.
- `OFFLINE_MODE=true` disables web retrieval and marks AIS as N/A.
- Managed container playbook documents build pipeline, telemetry hooks, monitoring.

## 16. Roadmap
**Explainability Track**
| Phase | Version | Scope |
| --- | --- | --- |
| MVP | v0.1.X | AIS scoring, faithfulness probes, PROV export, hybrid influence logging, model/data cards |
| v0.2 + GUI | Interactive evidence highlights, counterfactual explorer, OpenLineage, late-interaction visualisation, Python math explainer |
| v0.3 Advanced | Automated explanation QA in CI, explanation-drift alerts, configurable EU AI Act disclosure packs |

**Functional Track**
| Phase | Version | Scope |
| --- | --- | --- |
| MVP | v0.1 | Multi-agent coordination, local corpora ingestion, cross-session memory, fact-checking |
| v0.2 | GUI dashboard, Python tool integration, behavioural tuning loop |
| v0.3 | Distributed graphs, encrypted memory, JWT/OAuth + evaluation dashboard |

## 17. Success Metrics
- AIS coverage ≥85%, zero unsupported high-impact claims.
- Faithfulness metrics meet domain-specific thresholds (median deletion/insertion AUC).
- 100% provenance coverage for tool calls & retrieval hits.
- User testing: analysts rate explanations ≥4/5 for clarity/actionability.
- Core delivery goals: rapid one-shot research, >80% fact verification, resumable sessions.

## 18. Delivery Workstreams
- `xai_trace.rs` (event schema, persistence)
- `provenance.rs` (PROV-O fragments, OpenLineage emitter)
- `ais.rs` (claim segmentation, citation alignment, scoring)
- `faithfulness.rs` (deletion/insertion + LOO probes, metrics)
- `counterfactuals.rs` (numeric counterfactual engine)
- `cards.rs` (model/data card exporters)
- `hybrid_retrieval.rs` (dense + BM42 + optional ColBERT logging)
- Qdrant schema migration (sparse vectors, retrieval hashes, `used_by_claims` backfill)
- CLI/API enhancements for cards, claim explanations, provenance export
- Docker sandbox image + orchestrator (image hardening, Bollard wrapper, integration tests)

## 19. UX Writing & Artefact Guidelines
- Lead with decision, explain why, cite evidence, then state limits/what-ifs.
- Mark model inferences vs. direct citations.
- Tailor Business Analyst view (concise narrative) vs. Research Developer view (detailed artefacts).

## Appendix A — References
- NIST AI RMF 1.0
- NISTIR 8312
- EU AI Act transparency/logging expectations
- ICO & Alan Turing Institute (explaining AI decisions)
- AIS (Attributable to Identified Sources)
- Faithfulness vs. Plausibility (Jacovi & Goldberg)
- Counterfactual Explanations (Wachter et al.)
- Hybrid Retrieval (Qdrant + BM42 + ColBERT)
- PROV-O / OpenLineage
- Model Cards & Datasheets

## Implementation Tip (Rust)
Start with high-signal explainability elements: AIS scoring, PROV export, and hybrid retrieval influence logging. Wire `ExplainableAction` into the rig executor so each node emits `ProvFragment`s; assemble `prov.json` and `graph_trace.json` at the end to unlock GUI/CLI explainability quickly while heavier ablations remain budget-controlled.

## Status
✅ Specification locked for implementation. Next step: translate module list into issues across `agent.rs`, `planner.rs`, `graph_executor.rs`, `factcheck.rs`, `cli.rs`, `memory_qdrant.rs`.
