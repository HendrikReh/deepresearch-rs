# M13 Evaluation Harness — Design Notes

## Goals
- Replay historic sessions using latest DeepResearch build.
- Measure differences in final verdict, math status, and confidence buckets.
- Produce rollout-ready reports (JSON + Markdown) summarising impacts per domain/confidence bucket.

## Inputs
- Curated session dataset (`data/pipeline/curated/sessions_*.json`) or Postgres `session_records` table.
- Optional filters: date range, domain label, consent flag (must be `true`).

## Outputs
- `data/eval/latest/report.json` — structured metrics per domain/confidence.
- `data/eval/latest/report.md` — human-readable summary for PM/QA.
- Exit status failure when verdict delta exceeds configured tolerance (gates promotions).

## Key Metrics
- `verdict_changed_pct` — percentage of sessions whose critic verdict changed.
- `math_alert_increase` — count of sessions with new `math_alert_required=true`.
- `confidence_shift_distribution` — histogram of positive/negative shifts.
- `manual_review_delta` — diff in manual review rate.

## High-Level Algorithm
1. Load baseline records (Parquet or Postgres).
2. For each record:
   - Re-run session via CLI/API (headless, sandbox-enabled) capturing new summary.
   - Compute deltas (verdict change, math status change, confidence bucket movement).
3. Aggregate metrics by `domain_label`, `confidence_bucket`.
4. Emit JSON report + Markdown summary.

## Tooling Plan
- `crates/eval-harness` binary:
  - Input flags: `--parquet`, `--postgres`, `--limit`, `--output-dir`, `--ci-mode` (fail on regression).
  - Runs sessions sequentially (initial implementation) with optional concurrency.
  - Uses pipeline’s persisted data to reconstruct prompts, call CLI/API.
- Automation: future GitHub Action (weekly) referencing this tool.

## Open Questions
- How to seed domain/confidence labels for legacy records? (Manual tagging / heuristics TBD.)
- Should we snapshot “expected” outputs per session to detect text diffs beyond verdict? (Future work.)
