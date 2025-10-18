# DeepResearch Testing Guide — Milestone 1

This guide documents the verification strategy for Milestone 1 (“Planner & Orchestration Core”) as defined in `PLAN.md`. It complements the Project PRD by detailing the specific test matrix, tooling, and sign-off criteria required before progressing to Milestone 2.

---

## Scope Under Test
- `PlannerAgent`, `TaskGraph`, and cycle validation logic (`PRD.md §5.1`, `PLAN.md Milestone 1`).
- Graph-flow executor execution layer with retry/backoff and session concurrency caps (`PRD.md §5.3`, §12).
- Explainability event bus emitting `Event::Start`, `Event::Finish`, `Event::Message` (`PRD.md §5.8`, §13).
- Agent role scaffolding for `Researcher`, `Analyst`, `Critic` message contracts (`PRD.md §5.3`).

---

## Test Matrix

| Area | Test Type | Description | Tooling |
|------|-----------|-------------|---------|
| Task Graph Validation | Unit | Ensure DAG creation rejects cycles, missing nodes, and duplicated IDs | `cargo test -p deepresearch-core planner::tests` |
| Planner Role Assignment | Unit | Verify planner assigns roles per node and respects `max_iterations` & `confidence_threshold` | Same as above |
| Orchestrator Execution | Integration | Run small task graphs (parallel & sequential) verifying retries, exponential backoff, and stop-on-terminal failures | `cargo test -p deepresearch-core orchestrator::tests` |
| Session Concurrency Cap | Integration | Spawn more than 5 concurrent sessions and assert 6th returns capacity error | `cargo test -p deepresearch-api` or dedicated integration harness |
| Event Bus | Unit/Integration | Confirm events published per task lifecycle, 500ms ack timeout flushes to disk | `cargo test -p deepresearch-core explainability::tests` |
| Agent Message Contract | Unit | Validate placeholder agent handlers parse/produce structured messages, even if stubbed | `cargo test -p deepresearch-agents` |

---

## Test Execution Workflow
1. **Pre-checks**: `cargo fmt --all` and `cargo clippy --workspace --all-targets -- -D warnings`.
2. **Unit Tests**: `cargo test --workspace --lib` to cover module-level logic.
3. **Integration Suites**:
   - Core orchestrator flows: `cargo test --test milestone1_integration`.
   - API capacity behavior (if routes implemented): `cargo test -p deepresearch-api`.
4. **End-to-End Demo**: `cargo run --example milestone1_demo` to verify complete workflow.
5. **Concurrency and Timing**: Use `tokio::time::pause` / `advance` in tests to simulate backoff without real waits.
6. **Explainability Verification**: Validate JSON trace output schema using `serde_json::from_str` within tests.
7. **Artifacts**: Store test traces in `target/test-artifacts/` with run timestamp for manual inspection.

---

## Tooling & Utilities
- **Tokio test harness**: `#[tokio::test(flavor = "multi_thread", worker_threads = 2)]` for async tests.
- **Mocking**: Implement trait-based mocks for LLM & Qdrant clients to avoid network calls.
- **Property Tests (optional)**: Use `proptest` for stress-testing DAG generation.
- **Linting & Coverage** (optional for milestone but recommended):
  - `cargo llvm-cov --workspace` (requires installing `cargo-llvm-cov`).
  - Target ≥85% coverage on planner/orchestrator modules before merge.

---

## Acceptance Criteria for Milestone 1
- ✅ All planned tests are implemented and pass (19 tests: 10 unit + 3 agent + 6 integration).
- ✅ New functionality documented in `AGENTS.md` and relevant module-level rustdoc.
- ✅ PLAN.md milestone 1 checklist updated with `[x]` and completion notes.
- ✅ Zero clippy warnings with `-- -D warnings` flag.
- ✅ Interactive demo (`milestone1_demo`) showcasing all core capabilities.
- ✅ No critical `TODO`/`unwrap` within orchestrator/planner execution path (stubs clearly marked).

---

## Residual Risks & Follow-ups
- Performance under large DAGs deferred to Milestone 5 performance testing.
- Real agent LLM/Qdrant integration happens in Milestone 2; current tests rely on mocks.
- GUI explainability path still pending (ties into future roadmap milestones).

---

---

## Quick Reference Commands

```bash
# Run all tests
cargo test --workspace

# Run integration tests only
cargo test --test milestone1_integration

# Run interactive demo
cargo run --example milestone1_demo

# Verify code quality
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings

# Full CI check
cargo fmt --all && \
cargo clippy --workspace --all-targets -- -D warnings && \
cargo test --workspace && \
cargo build --workspace --release
```

---

*Last updated:* 2025-10-18
