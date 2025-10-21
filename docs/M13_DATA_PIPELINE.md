# M13 Data Pipeline — Session Record Contract

## Goal
Establish a durable, consent-aware session dataset capturing agent verdicts, math outputs, telemetry, and taxonomy required for continual learning.

## Record Schema (JSON)
| Field | Type | Description |
|-------|------|-------------|
| `session_id` | string | Unique identifier for the workflow run |
| `timestamp` | string (RFC3339) | Completion time when record persisted |
| `query` | string | Original user query (auto-prefixed with `use context7`) |
| `verdict` | string | Critic verdict (`critique.verdict`) |
| `requires_manual_review` | bool | Whether manual-review branch triggered |
| `math_status` | string | `math.status` (`success`, `failure`, `timeout`, `skipped`) |
| `math_alert_required` | bool | Mirrors `math.alert_required` |
| `math_stdout` / `math_stderr` | string | Captured math tool output (truncated) |
| `math_outputs[]` | array | Artefact summary (path, kind, bytes length) |
| `trace_path` | string? | Local path to persisted trace file |
| `sandbox_failure_streak` | number | Consecutive sandbox failures at time of record |
| `domain_label` | string? | (Future) Domain taxonomy label |
| `confidence_bucket` | string? | (Future) Bucketed verdict confidence |
| `consent_provided` | bool? | Flag enabling training usage (default true) |

## Storage Strategy
- Raw records: `data/pipeline/raw/<YYYY-MM-DD>.jsonl` (append-only, configurable via `DEEPRESEARCH_PIPELINE_DIR`).
- Curated store: stream timestamped JSON snapshots in `data/pipeline/curated/` via the `data-pipeline` crate. Snapshots are written incrementally — no more buffering the entire dataset in memory.
- Optional Postgres ingestion uses batched inserts (`--batch-size`, default 1000) to keep memory usage predictable while sustaining throughput.
- Retention & consent enforcement handled downstream.

## Tooling Overview
- `persist_session_record` (core crate) writes JSONL on session completion.
- `data-pipeline` crate streams raw records, filters on consent, enriches taxonomy labels, writes a pretty-printed JSON snapshot, and batch-inserts into Postgres when `--postgres-url` is supplied. Key flags:
  - `--batch-size <N>` — tune Postgres batch flush size (defaults to 1000, minimum 1).
  - `--snapshot-alias <NAME>` — maintain a rolling symlink (defaults to `sessions_latest.json`).
- `.github/workflows/data-pipeline.yml` runs nightly and uploads curated JSON artefacts (including the alias) for downstream consumers.
- Future: taxonomy enrichment + outcome labels integrated during consolidation.

## Security / Compliance
- Raw JSONL inherits existing log retention; curated store subject to governed retention.
- Ensure artefacts with sensitive content either excluded or redacted before export.
- Maintain audit trail of access to curated datasets.
