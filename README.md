# DeepResearch — Explainable Multi-Agent Research in Rust

DeepResearch is a graph-driven, Rust-first AI Research Assistant that proves every conclusion it delivers. Built atop `graph-flow`, Axum, and Qdrant, the system showcases how to operationalise **Explainable AI (XAI)** end-to-end: autonomous research, transparent reasoning, traceable evidence, and governance-ready artefacts.

[![CI](https://github.com/HendrikReh/deepresearch-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/your-org/deepresearch-rs/actions/workflows/ci.yml)
![Version](https://img.shields.io/badge/version-0.2.13-informational.svg)
[![Rust Edition](https://img.shields.io/badge/Rust-2024-blue.svg)](https://www.rust-lang.org/)
[![OpenAI](https://img.shields.io/badge/OpenAI-Integration-brightgreen.svg)](https://openai.com)
[![Collaboration](https://img.shields.io/badge/Collaboration-Welcome-orange.svg)](CONTRIBUTING.md)
[![License](https://img.shields.io/badge/License-GPL--3.0--or--later-purple.svg)](LICENSE)
![XAI Badge](https://img.shields.io/badge/XAI-NISTIR%208312%20Aligned-blueviolet)

---

## Why DeepResearch?
Traditional AI demos optimise for flashy answers. DeepResearch optimises for **trust**.
- **Audience-fit explanations**: Every report summarises “what, why, and what would change” for Business Analysts, while exposing full evidence trails for developers and consultants.
- **Faithful reasoning**: Agents log plan rationales, tool calls, retrieval hits, and ablations so explanations reflect what actually happened.
- **Governance ready**: Generates AIS-backed citations, PROV-O/OpenLineage provenance, model/data cards, and audit-friendly logs aligned with NIST AI RMF and EU AI Act transparency guidance.

---

## Highlights
- **Graph-first orchestration** — Researcher → Analyst → Critic (plus Attribution Verifier) modelled entirely with `graph-flow`.
- **Explainability everywhere** — Global and per-claim rationales, AIS support levels, faithfulness probes, counterfactual nudges.
- **Provenance exports** — PROV-O fragments, optional OpenLineage payloads, model/data cards, structured evaluation logs.
- **Hybrid retrieval** — Dense (FastEmbed) + BM42 sparse vectors in Qdrant, with influence logging.
- **Evaluation harness** — Runtime metrics (coverage, faithfulness AUC, latency) plus CLI/CI utilities.
- **Multi-surface delivery** — CLI commands, Axum API, Axum/Tailwind GUI (streaming evidence, timeline, metrics).

---

## Quick Start (CLI)
```bash
# 1. Install dependencies (Rust 1.75+, Node 20+ for GUI assets)
cargo install --path crates/deepresearch-cli

# 2. Run a query with explainability enabled
deepresearch query \
  --session acme-q4 \
  --sources web,local \
  --depth detailed \
  --explain "Compare Q4 revenue growth of top battery manufacturers"

# 3. Inspect explanations and provenance
deepresearch explain --last
deepresearch explain --claim C1
deepresearch trace --prov --out prov.json
```

### REST API
```bash
cargo run --offline -p deepresearch-api &
curl -s http://127.0.0.1:8080/query \
  -H 'content-type: application/json' \
  -d '{
        "query": "Assess sodium-ion vs lithium-ion incentives",
        "preferences": {"explainability": true}
      }' | jq
```
Response payload includes `explanations`, `claims[]` with AIS metadata, `provenance`, and `evaluation` blocks.

### GUI (Preview)
```bash
# Build frontend bundle (first run)
npm ci --prefix crates/deepresearch-gui/web
npm run build --prefix crates/deepresearch-gui/web

# Launch streaming dashboard (timeline, metrics, evidence highlights)
cargo run -p deepresearch-gui -- --gui-enabled
```
Visit `http://localhost:8080` to watch the reasoning graph evolve in real time.

---

## Architecture at a Glance
```
User Query
   │
Planner Agent ──► Plan Rationale
   │
Rig Orchestrator ──► XAI Trace Collector
   │
Researcher  ──► Hybrid Retrieval (Qdrant)
Analyst     ──► Synthesis → Draft Claims
Critic      ──► AIS + Faithfulness Probes
Attribution Verifier ──► Claim Verdicts
   │
Result Assembler ──► Report + Explanations + Provenance
```
Key components:
- `deepresearch-core`: planner, agents, explainability traits, provenance fragments.
- `deepresearch-cli`: workflows, ingestion, evaluation, explain/prov exports.
- `deepresearch-api`: Axum REST server with explainability toggles.
- `deepresearch-gui`: Axum/Tailwind dashboard (streaming evidence, timeline, metrics).
- Qdrant (dense + BM42 sparse vectors) + optional ColBERT reranker for influence analysis.

---

## Explainability Pillars
| Pillar | Implementation |
| --- | --- |
| **Meaningful Explanations** | Global plan summary + per-claim rationales tuned for persona |
| **Faithfulness** | Deletion/insertion AUC, leave-one-out ablations, counterfactual deltas |
| **Attribution** | AIS scoring (full/partial/none) with provenance handles (source hash + span) |
| **Knowledge Limits** | Confidence bands, coverage metrics, explicit “what would change?” responses |
| **Governance** | PROV-O/OpenLineage exports, model/data cards, audit logs (JSON) |

---

## Compliance & Governance Readiness
- **NIST AI RMF & NISTIR 8312** — Explanations designed for Explanation, Meaningful, Explanation Accuracy, Knowledge Limits.
- **EU AI Act** — AI disclosures, logging retention, synthetic content labelling hooks ready.
- **ISO/IEC 42001 alignment** — Optional governance artefacts (policy templates, audit trails).

---

## Roadmap Snapshot
| Milestone | Status | Highlights |
| --- | --- | --- |
| M0–M11 | ✅ | Graph foundations, observability, persistence, memory, explainability, CLI/API + GUI (Axum) |
| **M12** – Math Tool Integration |   | Python sidecar (`MathToolTask`), structured math outputs, CI `pytest`, CLI/API/GUI support |
| **M13** – Continual Learning & Governance |   | Session ETL, evaluation harness, weekly tuning job, governance docs & dashboards |
| Future |   | Explanation QA in CI, explanation drift alerts, configurable disclosure packs |

Full plan: [`PLAN.md`](PLAN.md)

---

## Contributing
We welcome contributions that deepen explainability:
1. Fork & clone the repo.
2. Pick an item from [`PLAN.md`](PLAN.md) or the issues list (look for `xai`/`help wanted` labels).
3. Follow the testing matrix:
   ```bash
   cargo fmt --all
   cargo clippy --workspace --all-targets -- -D warnings
   cargo test --workspace --all-targets -- --nocapture
   cargo test --offline -p deepresearch-gui --test http -- --nocapture
   ```
4. Submit a PR with context; include screenshots for GUI changes.

---

## Documentation References
- **Getting Started (`docs/getting-started/`)**: [Usage](docs/getting-started/USAGE.md), [Testing Guide](docs/getting-started/TESTING_GUIDE.md), [GUI Deployment](docs/getting-started/GUI_DEPLOYMENT.md), [GUI Acceptance](docs/getting-started/GUI_ACCEPTANCE.md).
- **Operations (`docs/operations/`)**: [Operations Guide](docs/operations/OPERATIONS.md), [Sandbox Runbook](docs/operations/RUNBOOK_SANDBOX_OPERATIONS.md), [Troubleshooting](docs/operations/TROUBLESHOOTING.md), [Security Review](docs/operations/SECURITY_REVIEW_SANDBOX.md), [Operational Checklist](docs/operations/CHECKLIST.md).
- **Evaluation (`docs/evaluation/`)**: [Evaluation Playbook](docs/evaluation/M13_EVALUATION.md), [Data Pipeline](docs/evaluation/M13_DATA_PIPELINE.md), [Evaluation Contributing](docs/evaluation/EVALUATION_CONTRIBUTING.md).
- **Governance (`docs/governance/`)**: [Governance Overview](docs/governance/M13_GOVERNANCE.md).
- **Release (`docs/release/`)**: [Release Checklist](docs/release/RELEASE_CHECKLIST.md), [CI Guide](docs/release/CI_GUIDE.md).

---

## License
GNU GPL v3 (or later). See [`LICENSE`](LICENSE).

---

> “Explainable AI shouldn’t be an afterthought. DeepResearch proves you can design it in from day one.”
