# M13 Evaluation Harness — Design Notes

## Goals
- Replay historic sessions using latest DeepResearch build.
- Measure differences in final verdict, math status, and confidence buckets.
- Produce rollout-ready reports (JSON + Markdown) summarising impacts per domain/confidence bucket.

## Inputs
- Curated session dataset (`data/pipeline/curated/sessions_*.json`) or Postgres `session_records` table.
- Optional filters: date range, domain label, consent flag (must be `true`).

## Outputs
- `data/eval/latest/report.json` — structured metrics (counts + confidence intervals) per domain/confidence bucket.
- `data/eval/latest/report.md` — human-readable summary for PM/QA including statistical guardrails.
- `data/eval/latest/dashboard.html` — interactive snapshot for stakeholders (tables, metadata, sampled deltas).
- `data/eval/latest/deltas/` — JSONL batches of per-session deltas when `--batch-size` > 0.
- Non-zero exit on threshold breach or statistically-significant regressions.

## Key Metrics & Guardrails
- `verdict_changed_pct` — share of sessions whose critic verdict changed. Guarded by `--max-verdict-delta` and bootstrap CI.
- `math_alert_increase` — count of sessions with new `math_alert_required=true` (maps to incident alerts).
- `confidence_shift_distribution` — histogram of positive/negative shifts for reporting dashboards.
- `manual_review_delta` — diff in manual review rate (tracks safety/load implications).
- Bootstrap significance: configurable via `--bootstrap-samples` and `--bootstrap-alpha`. Harness fails when the lower bound of the CI breaches configured tolerance.

## High-Level Algorithm
1. Stream raw session records from curated JSON snapshot (consent-filtered, taxonomy-enriched).
2. Optionally sharded replay (`--shard-count`, `--shard-index`) bounds dataset size per run.
3. Re-run each session headlessly via the CLI, capturing verdict/math/manual deltas.
4. Aggregate deltas overall and by `domain_label` / `confidence_bucket`.
5. Compute bootstrap confidence intervals + binomial p-values vs configured thresholds.
6. Emit JSON, Markdown, and HTML artefacts; persist sampled deltas and batch files for inspection.

## CLI Reference
```bash
cargo run -p eval-harness -- \
  --input data/pipeline/curated/sessions_latest.json \
  --output-dir data/eval/latest \
  --replay cargo --replay run --replay --offline --replay -p --replay deepresearch-cli --replay query --replay --format --replay json \
  --max-verdict-delta 0 \
  --max-math-delta 5 \
  --max-manual-delta 5 \
  --bootstrap-samples 2000 \
  --bootstrap-alpha 0.05 \
  --batch-size 200 \
  --delta-sample-limit 250 \
  --shard-count 1 \
  --shard-index 0
```

- **Batching** — `--batch-size` controls when per-session delta JSONL batches flush to disk (set `0` to disable). `--delta-sample-limit` bounds how many deltas are embedded inline in the report.
- **Sharding** — run the harness on large corpora by splitting work across shards. Each shard writes its own metrics; totals can be combined by summing counts before recomputing proportions.
- **Thresholds** — `--max-*-delta` caps raw counts. Harness also fails if the bootstrap CI lower bound exceeds the tolerated proportion, ensuring statistically significant regressions block promotion.

### Troubleshooting
- **`--input` required** — The harness never guesses which snapshot to use. Always pass a curated file explicitly (typically `data/pipeline/curated/sessions_latest.json`).
- **Snapshot missing** — Run the consolidation step first: `cargo run -p data-pipeline -- --raw-dir data/pipeline/raw --output-dir data/pipeline/curated`. Generate raw records beforehand via the CLI/API so the pipeline has data to ingest.
- **Immediate failure (`evaluation thresholds exceeded`)** — This means verdict/math/manual deltas exceeded configured limits or the bootstrap CI flagged a regression. Inspect `data/eval/latest/report.json`, `report.md`, `dashboard.html`, or the JSONL files under `data/eval/latest/deltas/` to understand which sessions drifted. Either resolve the regression or temporarily relax the relevant `--max-*` threshold for exploratory runs.

## Automation
- `.github/workflows/evaluation.yml` executes weekly and on demand. It now uploads `report.json`, `report.md`, `dashboard.html`, and the `deltas/` directory for deeper analyses.
- Failures bubble up when:
  - Configured delta limits are exceeded.
  - Bootstrap confidence intervals show significant regressions.
  - Replay command exits non-zero.

## Interpreting Results
- **Overall table** — Start with the “Statistical Guardrails” section in `report.md` / `dashboard.html`. Non-zero p-values near 0 coupled with CI bounds over the threshold warrant blocking rollout.
- **Bucket analysis** — Drill into domain/confidence buckets to isolate regressions. The HTML dashboard surfaces percentages and raw counts side-by-side for quick triage.
- **Sampled deltas** — Use the preview in the dashboard to spot qualitative shifts. Full batches are stored under `deltas/` for ad-hoc queries.
- **Manual follow-up** — When `math_alert_required` increases or manual review deltas spike, raise an incident in Ops and trigger sandbox health checks.

## Extending Metrics
- Add new metrics by updating the aggregator in `crates/eval-harness/src/main.rs` and documenting them here.
- Include a bootstrap test (or analytic alternative) for each regression gate to keep gating logic rigorous.
- Update `.github/workflows/evaluation.yml` to add new artefacts or thresholds so the weekly run exercises them automatically.
