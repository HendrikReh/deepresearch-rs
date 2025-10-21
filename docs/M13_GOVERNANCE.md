# M13 Governance Playbook

The continual-learning loop introduces new training data, evaluation artefacts, and promotion gates. This playbook documents the guardrails that must be satisfied before shipping a tuned checkpoint.

## Review Gates
- **Data pipeline completion** — Confirm the nightly `Data Pipeline Nightly` job produced `sessions_<timestamp>.json` and the `sessions_latest.json` alias. Ensure consent filtering succeeded (no zero-length snapshot).
- **Evaluation harness green** — Weekly `.github/workflows/evaluation.yml` must succeed. The harness enforces raw delta limits and bootstrap-tested significance. A red run automatically blocks the release checklist.
- **Dashboard sign-off** — PM/QA reviews `dashboard.html` for the latest run. Highlight buckets with high deltas, math alerts, or manual-review spikes. Log sign-off in the release issue.
- **Sandbox health** — Inspect `math.alert_required` trends and sandbox alerts (`SandboxFailureBurst`). Remediate degradation before proceeding.

## Safety Guardrails
- Bootstrap confidence intervals ensure statistically meaningful regressions block promotion. Do not override guardrails without a documented risk waiver.
- Math tool telemetry is propagated via `math.alert_required`, `math.retry_recommended`, and downstream analyst flags. Investigate any increase before continuing.
- Manual review deltas signal higher analyst load. Engage with the safety liaison if the rate exceeds the agreed SLA.

## Rollback Procedures
1. **Revert model artefacts** — Promote the previous checkpoint in the model registry and redeploy inference endpoints.
2. **Restore evaluation reports** — Archive the failing harness artefacts, then restore the last known-good `data/eval/latest` bundle so dashboards reflect an approved state.
3. **Purge training data** — If the failure stems from corrupted or non-consented data, remove the offending JSONL files and regenerate the curated snapshot.
4. **Incident logging** — File an operations incident summarising trigger, mitigation, and follow-up tasks.

## Consent & Retention
- The data pipeline strips sessions where `consent_provided != true` before generating curated snapshots or inserting into Postgres.
- Snapshots are immutable; retention is governed by the storage policy documented in `docs/OPERATIONS.md` (default 90 days for raw JSONL, configurable for curated exports).
- Downstream consumers must reference `sessions_latest.json` to inherit the latest consent-filtered dataset.

## Compute Resource Planning
- Coordinate with Analytics/Security prior to large replays:
  - **CPU budget** — Evaluation harness shards can be distributed across nodes (`--shard-count`/`--shard-index`). Record planned concurrency and expected runtime.
  - **GPU usage** — Training jobs should target the shared tuning pool. Reserve capacity via the weekly analytics stand-up and document slot IDs in the runbook.
  - **Storage** — Ensure S3/GCS buckets have >25% headroom before dropping new curated snapshots or evaluation batches.
- Capture approvals (stakeholder + timeframe) in the release issue. No compute-intensive jobs should run without signed-off capacity plans.

## Residual Risk Assessment
- Use the “Statistical Guardrails” table from `report.md` to summarise regression risk.
- Document outstanding manual-review increases, math-alert spikes, or domains with material deltas. Attach the sampled deltas or batch files that support the narrative.
- Record mitigation or follow-up actions (e.g., retraining, data cleanup) and owners.

## Contacts
- **Ops On-Call** — Responds to sandbox or pipeline failures.
- **Safety Liaison** — Reviews manual-review and math-alert regressions.
- **Analytics Lead** — Approves compute resource plans and tuning schedules.

Keep this playbook updated as new governance requirements land. Every release should link to the specific sections above in the changelog or release issue.
