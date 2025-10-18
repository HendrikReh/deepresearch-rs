# DeepResearch Agent System — Implementation Guide

Reference for engineers and AI coding agents contributing to DeepResearch, a Rust workspace that orchestrates autonomous multi-agent research, analysis, and explainability.

---

## Table of Contents
- [System Snapshot](#system-snapshot)
- [Agent Roles and Collaboration Flow](#agent-roles-and-collaboration-flow)
- [Core Modules by Crate](#core-modules-by-crate)
- [Development Workflow](#development-workflow)
- [Engineering Standards](#engineering-standards)
- [Dependency Management and Feature Flags](#dependency-management-and-feature-flags)
- [Memory, Retrieval, and Explainability](#memory-retrieval-and-explainability)
- [Interfaces: CLI, API, and Web Tools](#interfaces-cli-api-and-web-tools)
- [Configuration and Secrets](#configuration-and-secrets)
- [Operations and Deployment](#operations-and-deployment)
- [Testing and Quality Gates](#testing-and-quality-gates)
- [Roadmap Alignment](#roadmap-alignment)
- [Quick Reference](#quick-reference)

---

## System Snapshot
- **Goal:** End-to-end autonomous research workflow with transparent reasoning, grounded citations, and configurable agents.
- **Workspace layout:** `deepresearch-rs/` hosts multiple crates (`core`, `cli`, `api`, `agents`), shared resources (`config.toml`, `tests/`, `PRD.md`, `AGENTS.md`).
- **Primary dependencies:** `rig-core`, `tokio`, `axum`, `qdrant-client`, `fastembed`, `clap`, `serde`, `serde_json`, `tracing`, `tracing-subscriber`, `thiserror`, `anyhow`, `reqwest`.
- **Feature toggles:** `openai`, `ollama`, `web-search` (MCP integration), `gui` (future Axum UI).
- **Supported surfaces:** CLI REPL, REST API, future Axum GUI.

---

## Agent Roles and Collaboration Flow

### Roles
| Role | Focus | Key Abilities | Core Types |
|------|-------|---------------|------------|
| Planner | Decompose user intent into DAG tasks | LLM reasoning, dependency tracking | `PlannerAgent`, `TaskGraph` |
| Researcher | Retrieve facts via web/local memory | MCP search, Qdrant hybrid retrieval | `AgentRole::Researcher`, `QdrantMemory` |
| Analyst | Synthesize and draft reports | Summarization, structure alignment | `AgentRole::Analyst`, `ReasoningGraph` |
| Critic | Fact-check and ensure coherence | Confidence scoring, gap detection | `FactChecker`, `ConfidenceScore` |

### Execution Flow
```
User Query
  │
Planner Agent builds DAG
  │
Rig Orchestrator executes nodes (topological order)
  ├─ Researcher agents in parallel (Tokio tasks)
  ├─ Analyst aggregates findings
  └─ Critic validates and annotates
  │
Result assembler outputs Markdown/JSON with trace + citations
```

Rig orchestration hinges on `rig_graph.rs`, leveraging `FuturesUnordered` for concurrency, resilience on per-node failures, and `TraceCollector` events to power explainability.

---

## Core Modules by Crate

| Crate/Path | Module | Summary | Notable Dependencies |
|------------|--------|---------|----------------------|
| `crates/deepresearch-core/src/planner.rs` | Planner | Query decomposition, DAG construction, role assignment | `rig-core` |
| `crates/deepresearch-agents/src/agent.rs` | Agents | Role definitions, LLM context, messaging | OpenAI/Ollama SDK |
| `crates/deepresearch-core/src/rig_graph.rs` | Orchestrator | DAG execution, event emission, failure handling | `rig-core`, `tokio` |
| `crates/deepresearch-core/src/memory_qdrant.rs` | Memory | Hybrid search, FastEmbed integration, namespaces | `qdrant-client`, `fastembed` |
| `crates/deepresearch-core/src/factcheck.rs` | Fact checker | Claim verification, confidence scoring | Agent outputs |
| `crates/deepresearch-core/src/explainability.rs` | Explainability | Trace capture, reasoning graph serialization | `serde_json`, graph events |
| `crates/deepresearch-cli/src/cli.rs` | CLI | REPL, command handling, session persistence | `clap`, `tokio`, `rustyline` |
| `crates/deepresearch-api/src/api.rs` | API server | Axum routes, request validation, auth stubs | `axum` |
| `crates/deepresearch-core/src/web_tools.rs` | Web tools | MCP integration, alt search providers | OpenAI MCP SDK |
| `crates/deepresearch-core/src/citation.rs` | Citations | Source tracking, inline citation generation | — |

For any new capability, define the types first, align with the crate’s public API surface, then implement logic with tests and documentation.

---

## Development Workflow

### Onboarding Checklist
- Install Rust toolchain (`rustup default stable`) and workspace prerequisites.
- Start Qdrant locally or via Docker for hybrid memory tests.
- Set required environment variables (`OPENAI_API_KEY`, `QDRANT_URL`).
- Review `PRD.md` for product constraints.

### Daily Commands
```bash
# Build all workspace crates
cargo build --workspace

# Focused build (example: CLI)
cargo build -p deepresearch-cli

# Run all tests
cargo test --workspace

# Integration tests with logging
cargo test --workspace -- --nocapture --test-threads=1

# Lint and enforce warnings
cargo clippy --workspace --all-targets -- -D warnings

# Format the workspace
cargo fmt --all

# Generate docs locally
cargo doc --workspace --open --no-deps
```

### Branch & Release Discipline
1. Create feature branch: `git checkout -b feature/<description>`.
2. Implement change + tests; update this guide if roles or modules change.
3. Run CI-equivalent locally: `cargo fmt`, `cargo clippy`, `cargo test`, `cargo build --release`.
4. Follow semantic versioning; bump crate versions when APIs break.
5. Tag releases: `git tag -a v0.x.y -m "<summary>"`.

---

## Engineering Standards

### Code Style
- `snake_case` for functions/vars, `PascalCase` for types, `SCREAMING_SNAKE_CASE` for constants.
- Suffix async helpers with `_async` when clarity matters.
- Keep modules focused; extract traits for external integrations to ease mocking.

### Error Handling
- Prefer crate-specific error enums via `thiserror`.
- Propagate with `?`; enrich context using `anyhow::Context` or `tracing::error!`.
- Avoid swallowing errors—log before returning.

### Async and Concurrency
- Use `tokio::spawn` or `FuturesUnordered` for parallel agent tasks.
- Guard CPU-bound work with `tokio::task::spawn_blocking`.
- Apply `tokio::select!` when cancellation or timeouts are required.

### Logging & Observability
```rust
use tracing::{debug, error, info};

info!(query = %query_text, "Starting research workflow");
debug!(agent = "Researcher", task_id = %task_id, "Retrieval step");
error!(%err, "Qdrant connection failed");
```
- Configure via `tracing-subscriber`; set `RUST_LOG=debug` for verbose runs.

### Documentation Expectations
- `///` doc comments for all public APIs, including arguments and return details.
- Provide runnable or `no_run` examples for complex flows (planner, orchestrator, memory).
- Update high-level module docs if behavior or dependencies shift.

---

## Dependency Management and Feature Flags

```toml
[dependencies]
rig-core = "0.x"         # Multi-agent orchestration
qdrant-client = "1.x"    # Vector DB client
fastembed = "3.x"        # Embedding generation
axum = "0.7"             # Web framework
tokio = { version = "1", features = ["full"] }
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tracing = "0.1"
tracing-subscriber = "0.3"
thiserror = "1"
anyhow = "1"
reqwest = { version = "0.11", features = ["json"] }
```

```toml
[features]
default = ["openai"]
openai = []              # Use OpenAI GPT-5
ollama = []              # Use local Ollama
web-search = ["mcp-sdk"] # Enable web search via MCP
gui = ["axum"]           # Future web UI
```

Run `cargo update` or `cargo outdated` to track dependency health; prefer minimal version bumps.

---

## Memory, Retrieval, and Explainability

### Qdrant Hybrid Memory
- Dense + sparse embeddings via `fastembed` (BM25/BM42, dense vectors).
- Per-user namespaces (`collection/user_id`) for isolation.
- `MemoryEntry` struct stores snippet text, metadata, embeddings, sources.
- Summarization policy should cap history growth; prefer streaming inserts to bulk loads.

### Fact Checking
- `FactChecker` validates claims and annotates outputs with `ConfidenceScore`.
- Configurable thresholds (`min_confidence`, retry counts) sourced from `config.toml`.
- Implement a self-judge loop to re-query sources when confidence dips.

### Explainability
- `TraceCollector` subscribes to orchestrator events: `Start`, `Finish`, `Message`.
- `ReasoningGraph` maps agent dialogue, dependencies, and citations.
- CLI flag `--explain` emits a summary; JSON export powers GUI/trace visualizations.
- Future: GraphViz/Mermaid rendering based on serialized trace data.

---

## Interfaces: CLI, API, and Web Tools

### CLI (`deepresearch-cli`)
- Commands: `query`, `ingest`, `eval`, `explain`, `resume`.
- REPL via `rustyline`; sessions persist to JSON/local DB.
- Example dev run:
```bash
cargo run -p deepresearch-cli -- query "Compare solar adoption in 2023 vs 2024"
```

### API (`deepresearch-api`)
- Built on `axum`.
- Routes:
  - `POST /query` → submit research task.
  - `GET /session/:id` → fetch session state.
  - `POST /ingest` → add documents to memory.
- Auth: token-based via `auth.json` (MVP); JWT/OAuth planned for v0.3.
- Ensure CORS configuration aligns with the upcoming GUI.

### Web Tools (`web_tools.rs`)
- Integrates OpenAI MCP search; supports alternative providers through feature flags.
- Normalize result formats and track rate limits with exponential backoff.
- Log provider usage for observability and billing.

---

## Configuration and Secrets

### Configuration File (`config.toml`)
```toml
[llm]
provider = "openai"      # or "ollama"
model = "gpt-5"
api_key_env = "OPENAI_API_KEY"

[qdrant]
url = "http://localhost:6333"
collection = "deepresearch"

[planner]
max_iterations = 10
confidence_threshold = 0.8

[factcheck]
verification_count = 3
min_confidence = 0.7

[logging]
level = "info"
```

Load configs with `serde` + `toml`:
```rust
use serde::Deserialize;

#[derive(Deserialize)]
struct Config {
    llm: LlmConfig,
    qdrant: QdrantConfig,
    // ...
}

let config: Config = toml::from_str(&fs::read_to_string("config.toml")?)?;
```

### Secrets Management
- Keep API keys in environment variables (`OPENAI_API_KEY`, `DEEPRESEARCH_CONFIG`).
- Do not commit credentials; prefer `.env` files excluded via `.gitignore`.
- Namespace Qdrant data per user for privacy; encrypt at rest in roadmap v0.3.

---

## Operations and Deployment

### Local Debugging
```bash
# Verbose logging
RUST_LOG=debug cargo run --bin deepresearch-cli -- query "test"

# Inspect orchestrator trace
deepresearch explain --last --format json > trace.json
```

### Qdrant Health Checks
```bash
curl http://localhost:6333/health
curl http://localhost:6333/collections
```

### Docker Compose
```yaml
version: "3.8"
services:
  qdrant:
    image: qdrant/qdrant:latest
    ports:
      - "6333:6333"
    volumes:
      - ./qdrant_data:/qdrant/storage

  deepresearch:
    build: .
    depends_on:
      - qdrant
    environment:
      - QDRANT_URL=http://qdrant:6333
      - OPENAI_API_KEY=${OPENAI_API_KEY}
    volumes:
      - ./data:/app/data
```

### Dockerfile
```dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release --workspace

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates
COPY --from=builder /app/target/release/deepresearch-cli /usr/local/bin/
CMD ["deepresearch-cli"]
```

### CI/CD (GitHub Actions)
```yaml
name: CI
on: [push, pull_request]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - run: cargo build --workspace
      - run: cargo clippy --workspace -- -D warnings
      - run: cargo test --workspace
      - run: cargo fmt --all -- --check
```

---

## Testing and Quality Gates

### Testing Layers
- **Unit tests:** `#[cfg(test)]` modules; leverage `#[tokio::test]` for async.
- **Integration tests:** `tests/` directory; exercise planner → orchestrator → outputs.
- **Mocks/Stubs:** Define traits for external services (LLM, Qdrant) to enable deterministic tests.
- **Benchmarks:** Optional via `cargo bench --workspace`; focus on retrieval latency.

Example async unit test:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn planner_builds_dag() {
        let planner = PlannerAgent::new();
        let graph = planner.plan("test query").await.unwrap();
        assert!(!graph.nodes().is_empty());
    }
}
```

### Quality Gates Before Merge
1. `cargo fmt --all`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`
4. `cargo build --workspace --release`
5. Add/update docs and this guide if APIs or behaviors change.

### Security & Safety
- Avoid `unsafe` unless absolutely necessary; document invariants when present.
- Run `cargo audit` regularly:
```bash
cargo install cargo-audit
cargo audit
```
- Store tokens in env vars; plan for encryption at rest (roadmap v0.3).

### Performance Tips
- Share state with `Arc<T>`; prefer `tokio::sync::RwLock`.
- Batch Qdrant queries to reduce round trips.
- Stream large payloads instead of buffering entire datasets.
- Profiling via `cargo flamegraph --bin deepresearch-cli -- query "test"`.

---

## Roadmap Alignment

### MVP (v0.1)
- Multi-agent coordination (Researcher, Analyst, Critic).
- Qdrant hybrid search with FastEmbed.
- Fact-checking with confidence scoring.
- CLI sessions with persistence.
- REST API delivering structured JSON + trace.

### v0.2 Targets
- Axum web GUI with Tailwind UI.
- Python toolchain integration for numerical analysis.
- Continual learning via behavioral tuning.

### v0.3 Vision
- Distributed orchestrator for larger DAGs.
- Encrypted memory at rest, stronger auth (JWT/OAuth).
- Evaluation dashboard for research quality metrics.

---

## Quick Reference

### Frequently Used Commands
```bash
# Full CI pass
cargo fmt --all && cargo clippy --workspace -- -D warnings && cargo test --workspace

# Run CLI query in dev mode
cargo run -p deepresearch-cli -- query "test query"

# Generate documentation
cargo doc --workspace --open --no-deps

# Update dependencies
cargo update

# Identify outdated dependencies
cargo outdated
```

### Environment Variables
- `OPENAI_API_KEY` — LLM provider credential.
- `QDRANT_URL` — Qdrant endpoint (default `http://localhost:6333`).
- `RUST_LOG` — Logging level (`trace`, `debug`, `info`, `warn`, `error`).
- `DEEPRESEARCH_CONFIG` — Overrides default config path.

### Workspace Structure
```
deepresearch-rs/
├── Cargo.toml
├── crates/
│   ├── deepresearch-core/
│   ├── deepresearch-cli/
│   ├── deepresearch-api/
│   └── deepresearch-agents/
├── tests/
├── config.toml
├── PRD.md
└── AGENTS.md
```

---

**Status:** Implementation ready. Use this guide whenever extending the DeepResearch agent platform.
