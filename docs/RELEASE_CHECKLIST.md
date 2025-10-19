# Release Checklist — DeepResearch

Use this checklist before cutting a milestone release (M9 scope). All commands run from repo root unless noted. Update the PRD with outcomes (latency, accuracy) as needed.

## 1. Pre-flight

- Ensure working tree is clean (`git status`).
- Confirm `cargo fmt`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace --all-targets -- --nocapture` succeed.
- Re-run snapshot guard: `cargo test --offline -p deepresearch-core finalize_summary_snapshot`.

## 2. Functional Verification

- CLI happy-path: `cargo run --offline -p deepresearch-cli query "Release sanity" --format text`.
- API smoke:
  ```bash
  cargo run --offline -p deepresearch-api &
  curl -s http://localhost:8080/health | jq
  curl -s http://localhost:8080/query \
    -H 'content-type: application/json' \
    -d '{"query":"Release API check","explain":true}' | jq
  ```
  Confirm 200 responses, `capacity.active_sessions` changes, and trace/explanation fields present.
- Purge workflow: `cargo run --offline -p deepresearch-cli purge <SESSION_ID>` (use an existing session) and verify trace + log entries removed.

## 3. Performance Gates

- Run the load tester: `RUST_LOG=warn cargo run --offline -p deepresearch-cli bench "Release bench" --sessions 24 --concurrency 6 --format json`.
- Confirm:
  - `avg_latency_ms` ≤ 350
  - `p95_latency_ms` ≤ 400
  - `failure_count` == 0
- Observe `/health` while bench runs to verify 429s do not appear under the configured `DEEPRESEARCH_MAX_CONCURRENT_SESSIONS`. Adjust environment variables if necessary.

## 4. Logging & Compliance

- Set `DEEPRESEARCH_LOG_DIR=.tmp/release-logs` and run a query. Inspect `session.jsonl` for `[REDACTED]` placeholders and `audit.jsonl` for entries.
- Ensure `DEEPRESEARCH_LOG_RETENTION_DAYS` is set appropriately in deployment manifests.

## 5. Retrieval & Fact-Checking

- (If Qdrant enabled) run ingestion + query smoke:
  ```bash
  docker-compose up -d
  cargo run -F qdrant-retriever -p deepresearch-cli ingest --session release --path ./docs --qdrant-url http://localhost:6334
  cargo run -F qdrant-retriever -p deepresearch-cli query "Release retrieval" --session release --qdrant-url http://localhost:6334
  ```
- Capture fact-check metrics: `cargo test -p deepresearch-core eval::tests::evaluation_harness_aggregates_confidence`.

## 6. Finalize

- Update `CHANGELOG.md` / release notes with benchmark stats (avg/p95 latency, bench command used).
- Tag the release after merging (`git tag vX.Y.Z && git push origin vX.Y.Z`).
