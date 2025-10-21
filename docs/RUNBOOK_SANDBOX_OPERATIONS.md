# Sandbox & Data Pipeline Runbook

## Sandbox Image Lifecycle

1. **Build locally**
   ```bash
   docker build -t deepresearch-python-sandbox:latest -f containers/python-sandbox/Dockerfile .
   ```
2. **Tag + push**
   ```bash
   docker tag deepresearch-python-sandbox:latest registry.example.com/deepresearch/python-sandbox:<tag>
   docker push registry.example.com/deepresearch/python-sandbox:<tag>
   ```
3. **Update manifests** (Helm/compose overrides) to use new tag.
4. **Smoke tests**
   ```bash
   DEEPRESEARCH_SANDBOX_TESTS=1 cargo test -p deepresearch-core --test sandbox -- --ignored --nocapture
   DEEPRESEARCH_SANDBOX_TESTS=1 cargo test -p deepresearch-core --test integration_sandbox -- --ignored --nocapture
   ```

## Security Patching

- Monitor base image CVEs quarterly.
- Rebuild image after dependency updates; bump collector image (`otel/opentelemetry-collector-contrib`).
- Document changes in release notes + this runbook.

## Runtime Overrides

- `docker-compose.overrides/` provides sandbox/API/GUI + OTEL collector + Prometheus examples.
- Start stack:
  ```bash
  COMPOSE_PROJECT_NAME=deepresearch \
    docker compose -f docker-compose.yml -f docker-compose.overrides/docker-compose.sandbox.yml up -d
  ```
- Use `cli-runner` container for ad-hoc CLI tests.

## Telemetry & Alerting

- Sandbox emits `telemetry.sandbox` logs with status/duration/failure streak.
- OTEL collector config: `ops/otel/collector.yaml` (filelog receiver → OTLP/Prometheus exporters).
- Prometheus config + alert rule: `ops/prometheus.yml`, `ops/alerts/sandbox_alerts.yml` (`SandboxFailureBurst`).
- Dashboard hint: plot `telemetry_sandbox_failure_streak` and surface `math.alert_required`.

## Incident Response

1. **Sandbox failures spike**: check Prometheus alert, inspect `/var/log/deepresearch`, consider throttling math tool usage.
2. **Container start failure**: ensure Docker access, verify volume permissions, rebuild image.
3. **Data pipeline issues**: tenant on-call reviews nightly GitHub Action logs (`Data Pipeline Nightly`).

## Data Pipeline (M13)

### Raw Records
- Core workflow writes JSONL under `data/pipeline/raw/<DATE>.jsonl` via `persist_session_record`.
- Each record includes: query, verdict, manual-review flag, math status/outputs, trace path, consent flag (future).

### Consolidation Utility
- Consolidate raw JSONL to curated snapshot (plus optional Postgres insert):
  ```bash
  cargo run -p data-pipeline -- \
    --raw-dir data/pipeline/raw \
    --output-dir data/pipeline/curated \
    --postgres-url $DATABASE_URL   # optional
  ```
- Filters out sessions where `consent_provided=false`.
- Output: `data/pipeline/curated/sessions_<timestamp>.json` (and DB rows when configured).

### Nightly Automation
- GitHub Actions workflow: `.github/workflows/data-pipeline.yml`
  - Runs daily at 03:00 UTC
  - Uploads curated JSON snapshot as artifact (consume or ship to storage backend).
  - Inserts records into Postgres when `PIPELINE_DATABASE_URL` secret provided.
- TODO: integrate with storage backend (S3, GCS) and retention policy.

### Ground Truth Enrichment
- Tag sessions with taxonomy labels using CLI tool (future work).
- Ensure retention/redaction policies align with governance requirements.

## Evaluation Harness (M13)

- Manual replay:
  ```bash
  cargo run -p eval-harness -- \
    --input data/pipeline/curated/sessions_latest.json \
    --limit 20 \
    --replay cargo --replay run --replay --offline --replay -p --replay deepresearch-cli --replay query --replay --format --replay json
  ```
- Generates `data/eval/latest/report.json` with overall metrics, per-bucket breakdowns, and per-session deltas.
- Investigate non-zero `verdict_changed`/`math_status_changed` counts before promoting a new build.
- Weekly GitHub Action: `.github/workflows/evaluation.yml` (Mondays 04:00 UTC) replays the latest snapshot and uploads report artefact.
- Future work: enforce regression thresholds (fail pipeline when deltas exceed tolerances) and export to analytics dashboards/storage.

## Upgrade Checklist
- [ ] `cargo check`
- [ ] Sandbox smoke & integration tests
- [ ] Rebuild sandbox image
- [ ] Run data pipeline consolidation (manual or nightly job)
- [ ] Execute evaluation harness on curated snapshot
- [ ] Update runbook/docs with changes
