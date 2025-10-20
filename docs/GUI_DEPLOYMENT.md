# GUI Managed-Container Deployment Playbook

This playbook documents how to build, configure, and operate the Axum-based GUI as a managed container service. It complements the quick-start instructions in `README.md` and unblocks DevOps hand-off by capturing the end-to-end pipeline, required environment knobs, and monitoring hooks.

## Build & Publish Pipeline
- **Compile GUI assets.** Run `npm ci --prefix crates/deepresearch-gui/web` followed by `npm run build --prefix crates/deepresearch-gui/web`. The baked Vite bundle is emitted to `crates/deepresearch-gui/web/dist` and copied into the container at build time.
- **Produce the runtime image.** Invoke `docker build -f crates/deepresearch-gui/Dockerfile -t <registry>/deepresearch-gui:<tag> .`. The multi-stage Dockerfile compiles the Rust binary in release mode and enables the GUI flag by default.
- **Smoke test locally.** Execute the container with `docker run --rm -p 8080:8080 <image>`. Confirm `/health/live`, `/health/ready`, and `/api/sessions` respond as expected before publishing.
- **Publish to registry.** Push the verified image to your container registry (`docker push <registry>/deepresearch-gui:<tag>`). Capture the digest for deployment manifests.

## Runtime Configuration

| Variable | Default | Purpose |
|----------|---------|---------|
| `GUI_ENABLE_GUI` | `false` | Must be `true` (or set `GUI_AUTH_TOKEN`) to serve GUI + API routes. |
| `GUI_AUTH_TOKEN` | _unset_ | Optional bearer token required on `/api/*` and stream endpoints when provided. |
| `GUI_LISTEN_ADDR` | `0.0.0.0:8080` | Socket the service binds to; override for sidecars or non-standard ports. |
| `GUI_MAX_CONCURRENCY` | Host CPU count | Limits concurrent workflow executions; exposed via health metrics. |
| `GUI_DEFAULT_TRACE` | `true` | Enables trace capture by default; set to `false` to opt-in per session. |
| `GUI_ASSETS_DIR` | `<repo>/crates/deepresearch-gui/web/dist` | Location of the built frontend assets on disk. |
| `GUI_STORAGE` | `inmemory` | Switch to `postgres` (requires `--features postgres-session`) for durable sessions. |
| `GUI_POSTGRES_URL` / `DATABASE_URL` | _required when `GUI_STORAGE=postgres`_ | Connection string for Postgres-backed session storage. |
| `GUI_SESSION_NAMESPACE` | _unset_ | Prepends a namespace to session IDs for multi-tenant deployments. |
| `GUI_OTEL_ENDPOINT` | _unset_ | Optional hint for ops tooling. When set, the service emits `telemetry.gui` tracing events annotated with the endpoint so an external subscriber (e.g., OpenTelemetry sidecar) can forward spans. |

> **Prompt rule:** Incoming queries are automatically prefixed with `use context7` to satisfy the global prompt contract; upstream clients should avoid duplicating the prefix.

## Deployment Checklist
1. **Roll out secrets/config:** Render the environment variables above via your secret manager or Helm/Kustomize values file. Ensure the auth token (if used) matches downstream clients.
2. **Apply manifests:** Reference the published image digest in your Kubernetes `Deployment` (or equivalent). Mount the compiled assets if you supply them externally; otherwise rely on the baked bundle.
3. **Expose networking:** Publish port `8080` through your ingress or service mesh. The GUI requires HTTP/S access to `/`, `/api/*`, `/health/*`, and `/api/sessions/:id/stream` (Server-Sent Events).
4. **Gate concurrency:** Monitor `metrics.available_permits`; if it frequently drops to zero, adjust `GUI_MAX_CONCURRENCY` and underlying resource limits.

## Monitoring & Alerting
- **Health probes:** Configure liveness on `/health/live` (expects `200 OK`) and readiness on `/health/ready` (returns `503` if the GUI is disabled or capacity is exhausted).
- **Session telemetry:** The GUI emits structured `telemetry.gui` tracing events (`session_started`, `session_completed`, `session_failed`) with `session_id`, concurrency gauges, and manual-review flags. When `GUI_OTEL_ENDPOINT` is set, the endpoint value is included so platform teams can route traces to an external collector.
- **Stream observers:** SSE subscriptions increase the `stream_opened`/`stream_closed` counters. Alert if active subscribers spike or streams churn rapidly—this usually indicates GUI disconnects or networking issues.
- **Event stream:** `/api/sessions/:id/stream` emits JSON-encoded SSE events (`started`, `completed`, `error`). Watch for `error` events or repeated reconnects to detect failures early.
- **Capacity metrics:** Every response embeds `metrics` showing `max_concurrency`, `available_permits`, `running_sessions`, and `total_sessions`. Feed these into Grafana/Datadog dashboards for saturation alerts.
- **Explainability signals:** The trace endpoint now returns fact-check confidence, critic verdict confidence, per-task latency buckets, and manual-review indicators. Fold these into downstream QA dashboards when analysing regressions.

## Operations Runbook
- **Start a session:** `curl -XPOST :8080/api/sessions -H 'content-type: application/json' -H 'authorization: Bearer <token>' -d '{"query":"What is the roadmap impact?"}'`.
- **Stream progress:** `curl -N :8080/api/sessions/<id>/stream` to watch SSE updates; responses contain the final summary and trace availability once completed.
- **Trace retrieval:** `GET /api/sessions/<id>/trace` returns the full summary, trace events, and optional explainability payloads for audit trails.
- **Scale down & cleanup:** Shutdown the pods, then remove any Postgres sessions or local logs if the deployment is ephemeral.

Document updates should accompany changes to deployment tooling, environment variables, or operational procedures to keep DevOps aligned.
