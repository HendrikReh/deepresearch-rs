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
- Consolidate to Parquet:
  ```bash
  cargo run -p data-pipeline -- data/pipeline/raw data/pipeline/curated
  ```
- Filters out sessions where `consent_provided=false`.
- Output: `data/pipeline/curated/sessions_<timestamp>.parquet`.

### Nightly Automation
- GitHub Actions workflow: `.github/workflows/data-pipeline.yml`
  - Runs daily at 03:00 UTC
  - Uploads curated Parquet as artifact (consume or ship to storage backend).
- TODO: integrate with storage backend (S3, GCS) and retention policy.

### Ground Truth Enrichment
- Tag sessions with taxonomy labels using CLI tool (future work).
- Ensure retention/redaction policies align with governance requirements.

## Upgrade Checklist
- [ ] `cargo check`
- [ ] Sandbox smoke & integration tests
- [ ] Rebuild sandbox image
- [ ] Run data pipeline consolidation (manual or nightly job)
- [ ] Update runbook/docs with changes

