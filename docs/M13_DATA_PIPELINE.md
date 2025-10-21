# M13 Data Pipeline — Session Record Contract

## Goal
Establish a durable, consent-aware session dataset capturing agent verdicts, math outputs, and telemetry required for continual learning and behavioural tuning.

## Record Schema (JSON)
| Field | Type | Description |
|-------|------|-------------|
| `session_id` | string | Unique identifier for the workflow run |
| `timestamp` | string (RFC3339) | Completion time captured when record is written |
| `query` | string | Original user query (auto-prefixed with `use context7`) |
| `verdict` | string | Final critic verdict (`critique.verdict`) |
| `requires_manual_review` | bool | Whether manual branch triggered |
| `math_status` | string | `math.status` captured from `MathToolTask` |
| `math_alert_required` | bool | True when sandbox degraded (`math.alert_required`) |
| `math_outputs` | array<object> | Subset of `math.outputs` with `path`, `kind`, and base64 `bytes` (optional) |
| `math_stdout` / `math_stderr` | string | Captured stdout/stderr from math task |
| `sandbox_failure_streak` | integer | Consecutive failure streak at completion |
| `consent_provided` | bool | Whether session eligible for training (future flag) |
| `trace_path` | string | Local path to persisted trace (if available) |

## Storage Strategy
- Records appended to JSONL under `data/pipeline/raw/<YYYY-MM-DD>.jsonl`
- Directory configurable via `DEEPRESEARCH_PIPELINE_DIR`
- High-water mark trimming handled by downstream ETL job (future milestone)

## Downstream Utility (M13 Scope)
- Merge raw JSONL → governed Parquet/CSV (`data/curated/`) with consent filter
- Attach taxonomy labels (domain, confidence tier, manual review) during consolidation
- Provide CLI/automation entrypoint (GitHub Action TBD)

## Security & Compliance
- Raw logs respect existing retention defaults; governed store inherits consent flag
- Sensitive fields (PII, secrets) already redacted upstream (see sandbox runbook)
- Access controlled via existing log storage ACLs

## Open Questions
- Should math artefacts be stored inline (base64) or persisted to object storage with references?
- How to handle large outputs / binary artefacts without bloating JSONL?
- Governance workflow for deleting records when consent revoked?
