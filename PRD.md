# 🧠 **DeepResearch Agent — Product Requirements Document (PRD v0.1)**  

## 1. Product Overview  
**Purpose:**  
DeepResearch is a Rust-based, multi-agent system that autonomously gathers, analyzes, and synthesizes information on complex business queries with full traceability and explainability. It demonstrates advanced AI-native research orchestration and serves as a flagship project showcasing Hendrik Reh’s consulting and engineering capabilities.

**Core Value Proposition:**
- Autonomously conduct deep, multi-step research using structured reasoning and fact-checked synthesis.
- Provide transparent reasoning graphs and source traceability.
- Serve as a reference implementation of **agentic orchestration in Rust** using `graph-flow`, `Qdrant`, and `Axum`.

**Primary Domain:**  
Business intelligence and analysis — stock research, market reports, and trend synthesis.

---

## 2. Target Users & Use Cases  

**Personas:**  
- **Business Analyst** – asks one-shot analytical questions (“Compare Q4 revenue growth of top battery manufacturers”).  
- **Research Developer** – extends or automates the agent for custom domains.  
- **AI Consultant** – demonstrates multi-agent reasoning and explainability for client projects.

**Persona Stories & Success Signals:**  
- **Business Analyst**  
  - *Trigger:* Requests a comparative market insight via CLI.  
  - *Happy Path:* Report returns < 3 minutes with top-line summary, KPI table, and ≥ 3 cited sources.  
  - *Signal:* Analyst shares report with leadership without manual edits; follow-up queries reuse saved session.  
- **Research Developer**  
  - *Trigger:* Calls REST API with custom source preferences for sector-specific analysis.  
  - *Happy Path:* API responds with deterministic JSON schema, including reasoning graph reference and memory usage stats.  
  - *Signal:* Developer integrates DeepResearch into internal tooling without additional orchestration logic.  
- **AI Consultant**  
  - *Trigger:* Demos GUI session to client showcasing multi-agent explainability.  
  - *Happy Path:* Live run highlights planner decisions, critique loop, and cited insights; exportable trace shared post-demo.  
  - *Signal:* Client approves pilot engagement based on transparency and replayability of the session.

**Primary Use Case:**  
One-shot deep query producing a structured, cited report that synthesizes findings from web and local sources.

---

## 3. Interaction Model  

| Interface | Interaction Style | Output Format | Notes |
|------------|------------------|---------------|-------|
| CLI | REPL + commands | Markdown (default) / Plain text / JSON | Persist sessions across restarts |
| API | JSON request/response | JSON schema | For programmatic access |
| GUI (Axum web app, future scope) | Multi-panel dashboard (chat + evidence pane + graphs) | Markdown + charts + tables | Long-running sessions, responsive Tailwind UI |

---

## 4. Input & Output Formats  

### Input  
- Natural-language prompts (CLI/API/GUI)  
- Optional metadata (e.g. preferred sources, report depth)

**Example CLI Invocation:**  
```bash
deepresearch query \
  --session acme-q4 \
  --sources web,local \
  --depth detailed \
  "Compare Q4 revenue growth of top battery manufacturers"
```

**Example REST Payload:**  
```json
{
  "query": "Compare Q4 revenue growth of top battery manufacturers",
  "session_id": "acme-q4",
  "preferences": {
    "sources": ["web", "local"],
    "report_depth": "detailed",
    "explainability": true
  },
  "constraints": {
    "max_duration_ms": 180000,
    "confidence_threshold": 0.8
  }
}
```

### Output  
- **Default:** Markdown report  
- **Structure:** `summary`, `sources`, `metadata`, `confidence_scores`  
- **JSON schema:** standardized for programmatic consumption  
- **Explainability Toggle:** `--explain` flag renders reasoning graph and confidence trace  

**Example JSON Response Snippet:**  
```json
{
  "session_id": "acme-q4",
  "summary": "Battery manufacturers X and Y outperformed peers with 12% and 9% YoY revenue growth.",
  "sections": [
    {"title": "Market Overview", "content": "...", "citations": [1, 2]},
    {"title": "Competitive Landscape", "content": "...", "citations": [3]}
  ],
  "confidence_scores": {"overall": 0.81},
  "sources": [
    {"id": 1, "type": "web", "uri": "https://example.com/report"},
    {"id": 2, "type": "local", "uri": "qdrant://acme/notes"}
  ],
  "reasoning_trace_uri": "s3://deepresearch/traces/acme-q4.json"
}
```

---

## 5. Core Capabilities & Features  

### 5.1 Dynamic Reasoning & Task Planning
- **Planner Agent** decomposes queries into sub-tasks and constructs a graph-flow DAG of actions.
- Reasoning is data-driven: planner iteratively updates strategy as new facts arrive.  

### 5.2 Iterative Hypothesis Testing  
- Agents form and test hypotheses (“Does Company A outperform Company B in margin?”).  
- Cycles continue until confidence threshold is reached or timeout triggered.

### 5.3 Multi-Agent Collaboration  
- Agents spawn dynamically (per task).  
- Roles: `Researcher`, `Analyst`, `Critic`.  
- Each agent has its own LLM context (OpenAI GPT-5 via SDK or local Ollama).  
- All share a common memory pool (Qdrant + session context).  
- Explicit message logging between agents → explainable dialogue trace.  
- Sequential (Analyst → Critic) and parallel (Researcher threads) execution under Tokio.

### 5.4 Memory & Contextual Awareness  
- Persistent cross-session memory (Qdrant embeddings).  
- Stores: conversation embeddings, retrieved facts, and source snippets.  
- Semantic retrieval enables context recall across sessions.  
- Summarization policy limits memory growth (least-used info summarized after threshold).

### 5.5 Knowledge Retrieval & Hybrid Search  
- Qdrant collection with dense and sparse vectors (BM25/BM42 via FastEmbed).  
- Hybrid ranking combines semantic and keyword relevance.  
- Sources:  
  - **Web** via MCP (search.openai.com, Tavily, Brave API)  
  - **Local** corpora (PDFs, docs, databases) manually indexed via CLI  
  - **APIs** (e.g. Wikipedia; future domain APIs)  

### 5.6 Evaluation & Fact-Checking  
- Runtime fact-checker verifies claims ≥ confidence threshold **0.75** with up to **3** retrieval cycles (defaults configurable via `config.toml`).  
- Annotates each statement with `confidence_score` and `source_id`.  
- Built-in self-judge module automates evaluation (accuracy, citation recall).  
- Logs exported as JSON (`session_id`, `query`, `steps`, `sources`, `scores`, `latency`, `outcome`).  
- Configuration keys:  
  - `factcheck.min_confidence` (default `0.75`)  
  - `factcheck.verification_count` (default `3`)  
  - `factcheck.timeout_ms` (default `20000`)

### 5.7 Citation Management  
- Track and list sources used in final answers.  
- Inline citation style only (e.g. [1], [2]); no APA/MLA/BibTeX export for now.  

### 5.8 Explainability & Visualization  
- Reasoning graph (DAG of tasks + agent message flow).  
- Graph serializable to JSON for CLI debug and GUI visualization.  
- CLI: `--explain` flag renders graph structure and decision log.  
- GUI (future): interactive graph view showing task dependencies and agent communications.  

### 5.9 Math & Statistical Analysis  
- (Deferred to Python tool integration in future scope.)  

---

## 6. System Architecture  

### 6.1 High-Level Flow  
```
User Query
   │
Planner Agent
   │ builds
   ▼
Graph-Flow Executor  ──►  Dynamic DAG (nodes=actions, edges=dependencies)
   │
   ├── Researcher Subgraph (web + local retrieval)
   ├── Analyst Subgraph   (synthesis, summarization)
   └── Critic Subgraph    (fact-check, consistency)
   │
   ▼
Result Assembler → Markdown/JSON report + reasoning trace
```

### 6.2 Key Components
| Component | Responsibility | Framework / Tool |
|------------|----------------|------------------|
| **Graph-Flow Executor** | Execute and monitor DAG tasks | `graph-flow` |
| **Planner Agent** | Build task graph, assign roles | Custom module |
| **Agents** | Perform sub-tasks (LLM-driven) | GPT-5 / Ollama SDK |
| **Vector DB** | Memory + retrieval | Qdrant (dense + sparse) |
| **Hybrid Embedding Engine** | Generate BM25/BM42 and dense embeddings | FastEmbed |
| **Web Tool** | Search APIs via MCP SDK | OpenAI MCP |
| **Explainability Collector** | Capture events (start/finish/message) | Custom trace module |
| **CLI Interface** | Commands & session control | `clap` / `tokio` async REPL |
| **API Server** | REST interface | Axum |
| **GUI (future)** | Dashboard view + graph visualization | Axum + Tailwind |

**Integration Contracts & Operational Notes:**
- Planner ↔ Graph-Flow Executor: trait boundary returning `TaskGraph` with explicit dependency list; executor validates DAG structure before execution.
- Agents ↔ Memory: all reads/writes wrapped in async traits with backpressure (`Semaphore` capped at 8 concurrent hybrid searches) to protect Qdrant latency.
- Fact Checker ↔ Retrieval: fact-check requests include `claim_id`, `expected_sources`, and fallback to cached embeddings when remote search fails.
- Explainability Collector emits structured events (`Event::Start`, `Event::Finish`, `Event::Message`) appended to `trace_sink` channel; consumers must ack within 500 ms or events are flushed to disk.
- Error propagation: node failures bubble with `TaskError` (reason, retryable flag). Retryable nodes are automatically rescheduled up to 2 times with exponential backoff starting at 1 second.

---

## 7. CLI Design  

**Persistent Sessions:**  
- Session resume: `deepresearch resume --session <ID>`  

**Commands:**  
```bash
deepresearch query "compare solar panel growth 2023 vs 2024"
deepresearch ingest ./docs
deepresearch eval ./logs/session.json
deepresearch explain --last
```

**Session Store:** per-user namespace in Qdrant; auth via `auth.json` (token-based).

---

## 8. Memory & Persistence  

- Per-user namespace in Qdrant.  
- Embeddings encrypted at rest (future scope).  
- Session state (serialized graph + context) stored in local database or JSON file.  
- Manual re-indexing of local corpora via CLI.  

---

## 9. Evaluation & Logging  

- Integrated runtime evaluation (self-judge loop).  
- Standardized JSON log schema:  
  ```json
  {
    "session_id": "uuid",
    "query": "string",
    "steps": ["string"],
    "sources": ["url"],
    "scores": {"claim_1": 0.92},
    "latency": "ms",
    "outcome": "success|failure"
  }
  ```
- Logs used for post-hoc analysis and continuous evaluation.
- Storage & retention: logs persisted to `data/logs/<year>/<month>/session.jsonl`, rotated weekly; retained for 90 days unless flagged for red-team analysis.  
- Access control: CLI/API require `ROLE_ANALYST` capability to export logs; sensitive fields (PII, API keys) are redacted before write.  
- Evaluation pipeline: nightly batch job aggregates accuracy, citation recall, latency distribution; metrics feed into v0.3 dashboard backlog.

---

## 10. LLM Integration  

| Mode | Engine | SDK / Interface | Policy |
|------|---------|-----------------|---------|
| **External** | OpenAI GPT-5 | Official SDK via Rust client | Default |
| **Local** | Ollama runtime | HTTP API | Configured per runtime |
| **Failover** | None | Fail fast on error | (log & return message) |

---

## 11. Privacy & Security  

- Per-user memory namespaces.  
- Local API token auth (`auth.json`).  
- Data retention: session traces stored for 30 days by default; users can purge via `deepresearch purge --session <ID>`.  
- Secrets: API keys loaded from environment variables; config loader forbids inline secrets in TOML.  
- Compliance guardrails: redact PII before persisting to Qdrant; add audit log entry when sensitive fields are dropped.  
- Future: JWT/OAuth integration + embedding encryption (AES-256) as part of v0.3 roadmap.

---

## 12. Performance & Concurrency  

- Async execution with Tokio.  
- Parallel search requests and agent threads.  
- Graceful error handling per node → graph continues where possible.  
- Target latency: median end-to-end query ≤ 150 seconds with Qdrant reachable; P95 ≤ 240 seconds.  
- Fallback behavior: if external search exceeds 20 seconds, switch to cached embeddings and flag report with `partial_sources: true`.  
- Instrumentation: `tracing` spans emitted per DAG node, aggregated via `tracing-subscriber` + OpenTelemetry exporter (optional feature flag).  
- Capacity planning: orchestrator limits concurrent sessions to 5; beyond that, CLI/API return HTTP 429 with retry-after hint.

---

## 13. Explainability Model  

- All graph-flow execution events (`Start`, `Finish`, `Message`) emitted to `TraceCollector`.
- Collector aggregates → `graph_trace.json`.
- Graph structure:
  - Nodes = actions or agent sub-tasks
  - Edges = dependencies and message flows
- CLI: `--explain` prints summary tree; GUI: renders interactive DAG.

---

## 14. Deployment  

- Local execution or Docker container.  
- Components:  
  - Rust binary (agent + CLI + API)  
  - Qdrant service  
  - Configurable Ollama runtime (optional)  
- Docker Compose template defines network and volume mounts.  
- Offline mode supported (no web search).  

---

## 15. Roadmap  

| Phase | Version | Scope |
|-------|----------|-------|
| **MVP** | v0.1 (this PRD) | Multi-agent coordination, local corpora ingestion, cross-session memory, fact-checking module |
| **v0.2** | + GUI (Axum chat dashboard), Python tool integration for math/stats, continual learning via behavioral tuning |
| **v0.3** | Distributed multi-agent graphs, encrypted memory, advanced auth (JWT/OAuth) + evaluation dashboard |

**Phase Exit Criteria:**  
- **v0.1:** All core commands (`query`, `ingest`, `explain`, `resume`) stable; fact-checker meets ≥80% verified claims on benchmark suite; documentation updated (`AGENTS.md`, API schema).  
- **v0.2:** GUI passes usability test (score ≥4/5 with 3 pilot users); Python tool integration handles ≥2 numerical workloads; continual learning pipeline delivers weekly model update reports.  
- **v0.3:** Encrypted memory enabled by default; JWT/OAuth passes security review; evaluation dashboard displays real-time KPIs (accuracy, latency) with alerting on SLA breaches.

---

## 16. Success Criteria  

- End-to-end one-shot research query executed with explainable graph and structured report.  
- Accurate fact-checking (> 80 % verified claims).  
- Repeatable research sessions (resumable via CLI).  
- Demonstrates Rust-based multi-agent orchestration and AI explainability to prospective clients.  

---

✅ **Status:** Specification locked for implementation.
**Next step:** Translate modules into implementation tasks (`agent.rs`, `planner.rs`, `graph_executor.rs`, `factcheck.rs`, `cli.rs`, `memory_qdrant.rs`).  

*Last updated:* 2025-10-18