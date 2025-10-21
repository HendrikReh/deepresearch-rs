# Sandbox Override Compose Stack

This override brings up:
- `sandbox`: idle container using the hardened image (build locally first).
- `otel-collector`: filelog receiver scraping `/var/log/deepresearch`, exporting OTLP+Prometheus.
- `api`: DeepResearch API with metrics endpoint wired to the collector.
- `cli-runner`: convenience container to run CLI commands against the sandbox volume.

Usage:
```bash
# build local images first
make build-sandbox-image # or docker build -t deepresearch-python-sandbox:latest ...
cargo build -p deepresearch-api -p deepresearch-cli

# launch base services + override
COMPOSE_PROJECT_NAME=deepresearch \
  docker compose -f docker-compose.yml -f docker-compose.overrides/docker-compose.sandbox.yml up -d

# drop into cli container
docker compose -f docker-compose.yml -f docker-compose.overrides/docker-compose.sandbox.yml run --rm cli-runner bash
```

The collector publishes Prometheus metrics on `http://localhost:9464/metrics`. Point Prometheus or Grafana Agent at that endpoint to visualise `telemetry_sandbox_failure_streak` etc.
