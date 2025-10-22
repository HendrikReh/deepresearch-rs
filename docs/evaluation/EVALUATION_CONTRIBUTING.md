# Evaluation Harness Contribution Guide

This guide outlines the workflow for extending the evaluation harness with new scenarios, metrics, and artefacts.

## 1. Define the Scenario
- Describe the hypothesis you want to test (e.g., domain-specific regression, math alert sensitivity).
- Identify the source corpus (curated snapshot shard, Postgres filter, or ad-hoc dataset).
- Decide whether the scenario runs as part of the weekly CI, a nightly cron, or an ad-hoc playbook.

## 2. Extend the Harness
- Add metric aggregation logic in `crates/eval-harness/src/main.rs`. Store counts in `AggregateMetrics` and update `StatisticsReport` if the metric should surface in guardrails.
- Update the JSON/Markdown writers so new metrics appear in artefacts. Include short descriptions for PM/QA readers.
- If the metric should gate promotions, wire it into the threshold checker and expose CLI flags (follow the pattern used for verdict/math/manual metrics).

## 3. Update Automation
- Modify `.github/workflows/evaluation.yml` when new flags or artefacts are required.
- For heavy scenarios, consider adding dedicated jobs or shards. Use descriptive artifact names (e.g., `evaluation-report-domain-finance`).

## 4. Document Behaviour
- Update `docs/evaluation/M13_EVALUATION.md` with the new metric, interpretation guidance, and any CLI examples.
- Note dashboard updates (table columns, panels) so PM/QA know where to look.
- If the change alters governance expectations, cross-link the relevant section in `docs/governance/M13_GOVERNANCE.md`.

## 5. Add Tests
- Unit tests should cover aggregation and formatting logic (e.g., check bootstrap calculations, markdown tables).
- Integration tests can replay a small curated fixture to ensure the metric populates correctly. Place fixtures under `crates/eval-harness/tests/fixtures/`.

## 6. Review Checklist
- [ ] New CLI flags documented and have sensible defaults.
- [ ] JSON/Markdown schema changes communicated to downstream consumers.
- [ ] Dashboard render verified locally (`data/eval/latest/dashboard.html`).
- [ ] Updated artefacts uploaded by CI (inspect workflow logs).
- [ ] Documentation and changelog entries created.

Following this flow keeps evaluation expansions predictable and auditable.
