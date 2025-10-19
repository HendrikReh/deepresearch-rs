# Testing Guide — GraphFlow Milestones 1–4

Milestones 1–4 cover the core GraphFlow workflow (Researcher → Analyst → Critic) plus the new hybrid retrieval layer backed by FastEmbed and Qdrant. Use this guide to exercise the system and record verification steps.

---

## Scope
- Task behaviour implemented in `crates/deepresearch-core/src/tasks.rs`.
- Workflow orchestration in `crates/deepresearch-core/src/workflow.rs`.
- CLI execution path (`crates/deepresearch-cli/src/main.rs`).
- Retrieval adapters (`crates/deepresearch-core/src/memory/`).

---

## Recommended Test Matrix

| Area | Test | Command / Notes |
|------|------|-----------------|
| Formatting | Ensure consistent style | `cargo fmt` |
| Linting | Enforce Clippy lints | `cargo clippy --workspace --all-targets -- -D warnings` |
| Compilation | Offline build (no network) | `cargo check --offline` |
| Workflow smoke test | Run demo session | `cargo run -p deepresearch-cli --offline query "Test prompt"` |
| Task unit tests | (to be added) per-task validation | Add `#[cfg(test)]` blocks under `tasks.rs` |
| Integration | End-to-end session assertion | `cargo test --offline -p deepresearch-core critic_verdict_is_non_empty` |
| Branching Integration | Manual-review path triggers | `cargo test --offline -p deepresearch-core manual_review_branch_triggers` |
| Resume Integration | Resume existing session returns summary | `cargo test --offline -p deepresearch-core resume_session_returns_summary` |
| Retrieval Ingestion | Index docs into Qdrant | `cargo run -F qdrant-retriever -p deepresearch-cli ingest --session demo --path ./docs --qdrant-url http://localhost:6334` |
| Explainability | CLI `--explain` prints reasoning summary and writes trace | `cargo run --offline -p deepresearch-cli run --session test --explain` |
| Evaluation Metrics | Aggregate fact-check logs | `cargo test -p deepresearch-core eval::tests::evaluation_harness_aggregates_confidence` |

---

## Container Health Checks

*Start services:* `docker-compose up -d` (requires Docker Desktop).

*Verify Qdrant REST:* `curl http://localhost:6333/health` → expect `{"status":"ok"}`.
*Verify gRPC port:* Confirm `6334` is exposed (e.g., `lsof -i :6334`) when using the hybrid retriever.

*Verify Postgres connectivity:*
```bash
docker compose ps
# identify the postgres container name (e.g., deepresearch-rs-postgres-1)
docker exec -it <container> psql -U deepresearch -d deepresearch -c "SELECT 1;"
```

*Shutdown:* `docker-compose down` when finished.

---

## Manual Verification Steps
1. Run the CLI (`deepresearch-cli query`) and confirm the output includes:
   - Critic verdict line.
   - Summary section.
   - Enumerated sources.
2. Inspect logs (set `RUST_LOG=info`) to confirm each task executes in order.
3. Verify `factcheck.confidence` and `factcheck.passed` appear in the output (and call the evaluator on captured logs when doing manual QA).
4. Verify `critique.confident` toggles when you edit task logic (e.g., make Analyst omit sources).
5. Run the CLI with `--explain` and confirm both the rendered reasoning summary and the persisted `data/traces/<session>.json` payload.

---

## Future Automation Checklist
- [x] Add async test asserting `run_research_session` returns a non-empty verdict.
- [ ] Snapshot-test the critic output string for regression detection.
- [ ] Instrument tracing spans and assert they appear during test runs.

Update this document whenever test coverage expands or new milestones introduce additional checks.
