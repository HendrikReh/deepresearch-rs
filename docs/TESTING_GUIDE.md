# Testing Guide — GraphFlow Milestone 1

Milestone 1 focuses on validating the new GraphFlow-based workflow (Researcher → Analyst → Critic). Use this guide to exercise the system and record verification steps.

---

## Scope
- Task behaviour implemented in `crates/deepresearch-core/src/tasks.rs`.
- Workflow orchestration in `crates/deepresearch-core/src/workflow.rs`.
- CLI execution path (`crates/deepresearch-cli/src/main.rs`).

---

## Recommended Test Matrix

| Area | Test | Command / Notes |
|------|------|-----------------|
| Formatting | Ensure consistent style | `cargo fmt` |
| Linting | Enforce Clippy lints | `cargo clippy --workspace --all-targets -- -D warnings` |
| Compilation | Offline build (no network) | `cargo check --offline` |
| Workflow smoke test | Run demo session | `cargo run -p deepresearch-cli --offline` |
| Task unit tests | (to be added) per-task validation | Add `#[cfg(test)]` blocks under `tasks.rs` |
| Integration | End-to-end session assertion | `cargo test --offline -p deepresearch-core critic_verdict_is_non_empty` |

---

## Manual Verification Steps
1. Run the CLI and confirm the output includes:
   - Critic verdict line.
   - Summary section.
   - Enumerated sources.
2. Inspect logs (set `RUST_LOG=info`) to confirm each task executes in order.
3. Verify `critique.confident` toggles when you edit task logic (e.g., make Analyst omit sources).

---

## Future Automation Checklist
- [x] Add async test asserting `run_research_session` returns a non-empty verdict.
- [ ] Snapshot-test the critic output string for regression detection.
- [ ] Instrument tracing spans and assert they appear during test runs.

Update this document whenever test coverage expands or new milestones introduce additional checks.
