# Testing Guide — Milestones M0–M8

This guide consolidates recommended verification steps for the DeepResearch stack through milestone M8. Use it to validate local changes, run regression suites, and sanity‑check production‑critical paths (CLI, API, logging, explainability).

---

## 1. Prerequisites

- **Rust** toolchain (`rustup`, latest stable)
- **Node.js** 20+ (CI uses 23.x) for building the GUI bundle
- Docker Desktop (for Qdrant/Postgres stack)
- Optional: `just`, `jq`, `hey`/`wrk` for benchmarking, `curl`

> Assumption: all commands executed from repo root (`/Volumes/Halle4/projects/deepresearch-rs`).

---

## 2. Baseline Checks (applies to every milestone)

| Goal | Command | Notes |
|------|---------|-------|
| Frontend build | `npm install --prefix crates/deepresearch-gui/web` (first time)<br>`npm ci --prefix crates/deepresearch-gui/web`<br>`npm run build --prefix crates/deepresearch-gui/web` | Build and update the GUI assets before running `deepresearch-gui` |
| Format | `cargo fmt` | Required before committing |
| Lint | `cargo clippy --workspace --all-targets -- -D warnings` | Ensures clean build |
| Offline build smoke test | `cargo check --offline` | Catches missing workspace deps |
| Full test suite | `cargo test --workspace --all-targets -- --nocapture` | Runs unit + integration + logging tests |
| GUI smoke tests | `cargo test -p deepresearch-gui --test http -- --nocapture` | Verifies health/auth guards, SSE stream payload (manual-review flag), metrics/timeline trace response, and API wiring |
| Snapshot regression | `cargo test --offline -p deepresearch-core finalize_summary_snapshot` | Guards finalize/critic output formatting (use `INSTA_UPDATE=always cargo test --offline -p deepresearch-core finalize_summary_snapshot` to refresh deliberately) |
| Offline harness | `cargo test --offline --workspace --all-targets -- --nocapture` | Mirrors CI test matrix locally |

---

## 3. Milestone-Specific Verification

### M0 – Graph Foundation
- Ensure linear workflow runs end-to-end: `cargo run --offline -p deepresearch-cli query "Baseline market scan"`
- Integration tests: `cargo test --offline -p deepresearch-core critic_verdict_is_non_empty`

### M1 – Observability & Testing
- Confirm tracing spans present: `RUST_LOG=info cargo run --offline -p deepresearch-cli query "Tracing check"`
- Integration coverage: `cargo test --offline -p deepresearch-core manual_review_branch_triggers`

### M2 – Branching & Extensibility
- Verify manual-review branch: edit analyst sources to zero (optional), or run `cargo test --offline -p deepresearch-core manual_review_branch_triggers`
- API customizer smoke (if modifying workflows) by running unit/integration tests after hooking a custom task

### M3 – Persistence & Replay
- Resume support: `cargo run --offline -p deepresearch-cli query "Resume test" --session demo-resume` then `cargo run --offline -p deepresearch-cli resume demo-resume`
- Postgres path (requires docker-compose stack):
  ```bash
  docker-compose up -d
  DATABASE_URL=postgres://deepresearch:deepresearch@localhost:5432/deepresearch \
    cargo run --offline -F postgres-session -p deepresearch-cli query "Resume via pg" --session pg-demo
  ```

### M4 – Memory & Retrieval (Qdrant/FastEmbed)
1. Start stack: `docker-compose up -d`
2. Ingest docs: `cargo run -F qdrant-retriever -p deepresearch-cli ingest --session demo --path ./docs --qdrant-url http://localhost:6334`
3. Run with retriever: `cargo run -F qdrant-retriever -p deepresearch-cli query "Hybrid retrieval" --session demo --qdrant-url http://localhost:6334`

### M5 – Fact-Checking & Evaluation
- Unit coverage: `cargo test -p deepresearch-core eval::tests::evaluation_harness_aggregates_confidence`
- Manual: run CLI query with verbose logging (`RUST_LOG=info`) and confirm `factcheck.*` context values appear in output
- Optional: create a JSONL log and run `cargo run --offline -p deepresearch-cli eval data/logs/sample.jsonl --format text`

### M6 – Explainability & Trace Serialization
- CLI explain: `cargo run --offline -p deepresearch-cli query "Explainability" --explain --format json`
- Ensure `data/traces/<session>.json` written and sanitised
- API explain route: `curl -s "http://localhost:8080/session/<SESSION>?explain=true&include_summary=true"`

### M7 – Interfaces (CLI & API)
- CLI commands:
  ```bash
  cargo run --offline -p deepresearch-cli query "Interfaces" --format json
  cargo run --offline -p deepresearch-cli explain <SESSION> --include-summary
  cargo run --offline -p deepresearch-cli purge <SESSION>
  cargo run --offline -p deepresearch-cli bench "Capacity tuning" --sessions 12 --concurrency 4
  ```
- API routes (requires server running via `cargo run --offline -p deepresearch-api`):
  ```bash
  curl -s http://localhost:8080/health | jq
  curl -s http://localhost:8080/query \
    -H 'content-type: application/json' \
    -d '{"query":"API integration", "explain":true}' | jq
  ```
- Observe health output while bench is executing to confirm `active_sessions` increments and HTTP 429 returned when capacity exceeded.

### M8 – Security, Privacy & Logging
- Redaction & audit test (already automated): `cargo test -p deepresearch-core logging::tests::session_logging_sanitizes_and_persists`
- Manual verification:
  1. Set `DEEPRESEARCH_LOG_DIR=.tmp/logs` and run `cargo run --offline -p deepresearch-cli query "Log test"`
  2. Inspect `.tmp/logs/<year>/<month>/session.jsonl` for `[REDACTED]` replacing tokens
  3. Confirm `.tmp/logs/<year>/<month>/audit.jsonl` contains corresponding entries
  4. Run `cargo run --offline -p deepresearch-cli purge <SESSION>` and verify session entry, trace file, and audit record removed
- Retention: set `DEEPRESEARCH_LOG_RETENTION_DAYS=0` and rerun logging test; directory should be pruned automatically.

---

## 4. Container Health Checklist (M4+)

(unchanged from previous version)

- Start stack: `docker-compose up -d`
- Qdrant REST: `curl http://localhost:6333/health`
- Qdrant gRPC port: `lsof -i :6334`
- Postgres connectivity: `docker exec -it <pg-container> psql -U deepresearch -d deepresearch -c "SELECT 1;"`
- Shutdown: `docker-compose down`

---

## 5. Manual QA Workflow

1. Run CLI query; verify verdict, summary, sources present.
2. Inspect `RUST_LOG=info` output for task span order.
3. Validate fact-check context (`factcheck.confidence`, `factcheck.passed`).
4. Force manual-review scenario (edit or patch) and ensure CLI outputs manual-review summary.
5. Use `--explain` / `explain` subcommand to inspect reasoning trace and persisted trace file.
6. Exercise API `/query`, `/session/:id`, `/ingest`, `/health` and check 429 throttling by exceeding `DEEPRESEARCH_MAX_CONCURRENT_SESSIONS`.
7. Trigger `deepresearch-cli purge <SESSION>` and confirm session traces + logs deleted.
8. Re-run the snapshot test (`cargo test --offline -p deepresearch-core finalize_summary_snapshot`) after modifying summary formatting to ensure expected output.

---

## 6. Future Automation Wishlist

- [x] Async integration test ensuring `run_research_session` returns non-empty verdict
- [x] Snapshot tests for critic/finalize output to guard against formatting regressions
- [ ] Automated span assertion (verify key tracing spans emitted)
- [ ] CI workflow running `cargo test --offline` and linting matrix

Update this document whenever milestone coverage expands or tooling changes.
