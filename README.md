# DeepResearch

[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Build Status](https://img.shields.io/github/actions/workflow/status/hendrikreh/deepresearch-rs/ci.yml?branch=master)](https://github.com/hendrikreh/deepresearch-rs/actions)

> Autonomous multi-agent research system with explainable reasoning, hybrid memory, and fact-checked synthesis — built in Rust.

DeepResearch is a production-grade, multi-agent orchestration platform that autonomously gathers, analyzes, and synthesizes information on complex business queries. It demonstrates advanced AI-native research workflows with full traceability, transparent reasoning graphs, and grounded citations.

**Key Features:**
- 🧠 **Multi-Agent Coordination** — Planner, Researcher, Analyst, and Critic agents collaborate via DAG orchestration
- 🔍 **Hybrid Search** — Dense + sparse vector retrieval (BM25/BM42) powered by Qdrant and FastEmbed
- ✅ **Fact-Checking** — Runtime claim verification with configurable confidence thresholds
- 📊 **Explainability** — Complete reasoning graphs and agent dialogue traces
- 🔒 **Privacy-First** — Per-user memory namespaces, PII redaction, secrets management
- ⚡ **High Performance** — Async execution with Tokio, parallel agent tasks, graceful degradation

---

## Quick Start

### Prerequisites
- Rust 1.75+ ([install](https://rustup.rs/))
- Qdrant vector database ([Docker setup](#docker-deployment))
- OpenAI API key or local Ollama runtime

### Installation

```bash
# Clone the repository
git clone https://github.com/hendrikreh/deepresearch-rs.git
cd deepresearch-rs

# Build all workspace crates
cargo build --workspace --release

# Run CLI
cargo run -p deepresearch-cli -- --help
```

### Basic Usage

```bash
# Set your API key
export OPENAI_API_KEY=sk-...

# Start Qdrant (via Docker)
docker run -p 6333:6333 qdrant/qdrant

# Run a research query
cargo run -p deepresearch-cli -- query "Compare Q4 revenue growth of top battery manufacturers"

# Resume a previous session
cargo run -p deepresearch-cli -- resume --session acme-q4

# Explain the reasoning
cargo run -p deepresearch-cli -- explain --last
```

---

## Architecture

DeepResearch is built as a Rust workspace with four primary crates:

```
deepresearch-rs/
├── crates/
│   ├── deepresearch-core     # Shared types, config, memory, orchestration
│   ├── deepresearch-agents   # Agent roles and LLM integration
│   ├── deepresearch-cli      # Interactive REPL and commands
│   └── deepresearch-api      # REST API server (Axum)
├── tests/                    # Integration tests
├── config.toml              # Runtime configuration
├── PRD.md                   # Product requirements
└── AGENTS.md                # Implementation guide
```

### Agent Workflow

```
User Query
  │
  ├─► Planner decomposes into DAG tasks
  │
  ├─► Rig Orchestrator executes nodes
  │    ├─► Researcher agents (parallel web + local search)
  │    ├─► Analyst synthesizes findings
  │    └─► Critic validates and fact-checks
  │
  └─► Result Assembler → Markdown/JSON + reasoning trace
```

**Tech Stack:**
- [rig-core](https://github.com/0xPlaygrounds/rig) — Multi-agent orchestration
- [Qdrant](https://qdrant.tech/) — Vector database for hybrid search
- [FastEmbed](https://github.com/Anush008/fastembed-rs) — Embedding generation (BM25/dense)
- [Axum](https://github.com/tokio-rs/axum) — Web framework
- [Tokio](https://tokio.rs/) — Async runtime
- [Clap](https://github.com/clap-rs/clap) — CLI parsing

---

## Configuration

Create a `config.toml` file in the project root:

```toml
[llm]
provider = "openai"  # or "ollama"
model = "gpt-4"
api_key_env = "OPENAI_API_KEY"

[qdrant]
url = "http://localhost:6333"
collection = "deepresearch"

[planner]
max_iterations = 10
confidence_threshold = 0.8

[factcheck]
verification_count = 3
min_confidence = 0.75
timeout_ms = 20000

[logging]
level = "info"  # trace, debug, info, warn, error
```

**Environment Variables:**
- `OPENAI_API_KEY` — LLM provider credential
- `QDRANT_URL` — Qdrant endpoint (overrides config)
- `RUST_LOG` — Logging level
- `DEEPRESEARCH_CONFIG` — Custom config path

---

## Docker Deployment

### Docker Compose

```bash
# Start all services
docker-compose up -d

# View logs
docker-compose logs -f deepresearch

# Stop services
docker-compose down
```

Example `docker-compose.yml`:

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
    ports:
      - "3000:3000"
```

---

## API Reference

### REST Endpoints

```bash
# Submit a research query
POST /query
{
  "query": "Compare Q4 revenue growth of top battery manufacturers",
  "session_id": "acme-q4",
  "preferences": {
    "sources": ["web", "local"],
    "report_depth": "detailed",
    "explainability": true
  }
}

# Retrieve session
GET /session/:id

# Ingest documents
POST /ingest
{
  "path": "./docs",
  "recursive": true
}

# Health check
GET /health
```

See full [API documentation](docs/api.md) for request/response schemas.

---

## Development

### Building & Testing

```bash
# Full CI check
cargo fmt --all && \
cargo clippy --workspace --all-targets -- -D warnings && \
cargo test --workspace && \
cargo build --workspace --release

# Run specific crate tests
cargo test -p deepresearch-core

# Integration tests with logging
cargo test --workspace -- --nocapture --test-threads=1

# Generate documentation
cargo doc --workspace --open --no-deps
```

### Code Standards

- **Style:** Follow Rust naming conventions (snake_case, PascalCase)
- **Errors:** Use `thiserror` for custom error types, propagate with `?`
- **Async:** Prefer `async fn`, use `tokio::spawn` for parallel tasks
- **Logging:** Structured logs via `tracing` crate
- **Testing:** Unit tests in `#[cfg(test)]`, integration tests in `tests/`
- **Docs:** `///` rustdoc comments for all public APIs

See [AGENTS.md](AGENTS.md) for detailed implementation guidelines.

---

## Roadmap

| Version | Status | Features |
|---------|--------|----------|
| **v0.1 (MVP)** | 🚧 In Progress | Multi-agent coordination, hybrid search, fact-checking, CLI/API |
| **v0.2** | 📋 Planned | Web GUI (Axum + Tailwind), Python tool integration, continual learning |
| **v0.3** | 💡 Future | Distributed graphs, encrypted memory, JWT/OAuth, evaluation dashboard |

**Current Focus:** Completing MVP implementation with stable core modules and comprehensive testing.

---

## Contributing

We welcome contributions! Please follow these guidelines:

1. **Fork & Clone** — Create a feature branch from `master`
2. **Follow Standards** — Review [AGENTS.md](AGENTS.md) for code style and patterns
3. **Write Tests** — Include unit and integration tests for new features
4. **Run CI Checks** — Format, lint, test, and build before submitting
5. **Submit PR** — Provide clear description with context and motivation

### Reporting Issues

- **Bugs:** Use the [bug report template](.github/ISSUE_TEMPLATE/bug_report.md)
- **Features:** Use the [feature request template](.github/ISSUE_TEMPLATE/feature_request.md)
- **Questions:** Open a [discussion](https://github.com/hendrikreh/deepresearch-rs/discussions)

---

## License

This project is licensed under the MIT License — see the [LICENSE](LICENSE) file for details.

---

## Acknowledgments

Built by [Hendrik Reh](https://github.com/hendrikreh) as a reference implementation of production-grade multi-agent systems in Rust.

**Powered by:**
- Anthropic Claude (planning & analysis)
- OpenAI GPT-4 (LLM integration)
- Qdrant (vector search)
- Rust ecosystem (tokio, axum, clap, serde, tracing)

---

## Resources

- 📖 [Product Requirements (PRD.md)](PRD.md)
- 🛠️ [Implementation Guide (AGENTS.md)](AGENTS.md)
- 📚 [API Documentation](docs/api.md)
- 🎯 [Benchmarks](docs/benchmarks.md)
- 💬 [Discussions](https://github.com/hendrikreh/deepresearch-rs/discussions)

---

**Status:** v0.1 (MVP) — Active Development

For questions or consulting inquiries, reach out via [GitHub Discussions](https://github.com/hendrikreh/deepresearch-rs/discussions).
