# DeepResearch (GraphFlow Edition)

Minimal, graph-first implementation of the DeepResearch agent pipeline. All multi-agent orchestration is powered directly by [`graph_flow`](https://docs.rs/graph-flow/latest/graph_flow/).

---

## Workspace Layout

```
deepresearch-rs/
├── Cargo.toml
├── crates/
│   ├── deepresearch-core   # GraphFlow tasks + workflow runner
│   └── deepresearch-cli    # Demo binary that runs the workflow
├── docs/                   # Testing guide and supporting docs
├── AGENTS.md               # Developer reference
├── PLAN.md                 # Roadmap / milestone tracking
└── PRD.md                  # Product requirements
```

---

## Quick Start

```bash
# Format & lint
cargo fmt
cargo clippy --workspace --all-targets -- -D warnings

# Run tests (offline if dependencies are cached)
cargo test --workspace --offline

# Execute the demo workflow
cargo run --offline -p deepresearch-cli
```

This produces a critic verdict summarising the analyst’s findings and enumerating supporting sources.

---

## Milestone Status

| Milestone | Status | Summary |
|-----------|--------|---------|
| M0 — Graph Foundation | ✅ | Core Researcher → Analyst → Critic tasks wired via `graph_flow` |
| M1 — Observability & Testing | ✅ | Structured tracing, integration test, documented context keys |
| M2+ | 🚧 | See `PLAN.md` for upcoming work (branching, persistence, retrieval, etc.) |

Refer to `PLAN.md` for the full roadmap.

---

## Testing

See `docs/TESTING_GUIDE.md` for the complete matrix. Key commands:

```bash
cargo fmt
cargo clippy --workspace --all-targets -- -D warnings
cargo check --offline
cargo test --offline -p deepresearch-core critic_verdict_is_non_empty
```

---

## Context Keys

| Key | Notes |
|-----|-------|
| `query` | Original user prompt. |
| `research.findings` | Vector of bullet insights from the researcher. |
| `research.sources` | Source URIs attached to findings. |
| `analysis.output` | Structured summary (`AnalystOutput`). |
| `critique.confident` | Boolean confidence flag from critic. |
| `critique.verdict` | Human-readable verdict string. |

(See `AGENTS.md` for more details.)

---

## GitHub Actions

Basic CI is defined in `.github/workflows/ci.yml` (fmt, clippy, tests).

---

## Licensing

Released under the MIT License. See `LICENSE` for details.

