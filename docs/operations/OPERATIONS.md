# DeepResearch Operations — Container Playbook

This guide documents every container-related workflow used across development, CI, and runtime validation. Run commands from the repository root unless noted otherwise.

---

## 1. Prerequisites

- Docker Engine 20.10+ (desktop or daemon)
- Optional: `docker compose` plug-in for local Qdrant/Postgres stack
- Sufficient disk space for images (~2 GB)

---

## 2. Hardened Python Sandbox Image

The sandbox image powers secure Python execution (Matplotlib, Graphviz, Mermaid). Build it any time dependencies change:

```bash
docker build -t deepresearch-python-sandbox:latest \
  -f containers/python-sandbox/Dockerfile \
  .
```

- `-t` tags the image for reuse by CLI/API/CI jobs.
- `-f` points to the custom Dockerfile.
- The trailing `.` is the build context (required).

To publish under a different tag (e.g., CI): change `-t deepresearch-python-sandbox:<tag>`.

### Smoke Test

With Docker running and the image built, run the optional sandbox validation:

```bash
docker build -t deepresearch-python-sandbox:latest \
  -f containers/python-sandbox/Dockerfile .
DEEPRESEARCH_SANDBOX_TESTS=1 \
DEEPRESEARCH_SANDBOX_IMAGE=deepresearch-python-sandbox:latest \
cargo test -p deepresearch-core --test sandbox -- --ignored --nocapture
```

The test exercises Matplotlib, Graphviz, and Mermaid output inside the sandbox and asserts artefacts are produced.

---

## 3. Local Retrieval Stack (Qdrant + Postgres)

Use the provided compose file when running hybrid retrieval or Postgres-backed sessions:

```bash
docker compose up -d          # start services
docker compose logs -f qdrant # tail Qdrant logs
docker compose down           # stop and remove containers
```

Default ports:
- Qdrant REST: `6333`
- Qdrant gRPC: `6334`
- Postgres: `5432`

Update `.env` or compose overrides if ports conflict with local services.

---

## 4. CI Sandbox Job

GitHub Actions builds and smoke-tests the sandbox image on every PR:

```yaml
docker build -t deepresearch-python-sandbox:ci -f containers/python-sandbox/Dockerfile .
DEEPRESEARCH_SANDBOX_TESTS=1 DEEPRESEARCH_SANDBOX_IMAGE=deepresearch-python-sandbox:ci \
  cargo test -p deepresearch-core --test sandbox -- --ignored --nocapture
```

When modifying the Dockerfile or test suite, replicate those commands locally to verify before pushing.

---

## 5. Sandbox Telemetry & Alerts

- Each sandbox run emits `telemetry.sandbox` events via `tracing` with `status`, `duration_ms`, `outputs`, and `failure_streak` fields. Route these to your observability pipeline by tailing stdout/stderr (e.g., use the OpenTelemetry Collector `filelog` receiver or Vector's `stdin` source) and forward to OTLP/Prometheus as needed.
- Consecutive failures increment the `failure_streak`. When the streak reaches 3, the runner logs an error-level event so alerting systems can page on persistent breakage.
- Downstream tasks set `math.alert_required=true` and `math.degradation_note` whenever a timeout/failure occurs. Surface these fields in dashboards to highlight degraded sessions (Grafana example: query `math_alert_required{service="deepresearch-core"}` and display the degradation note as a panel annotation).
- Recommended alert threshold: warn when `failure_streak >= 3` within a five-minute window, critical when `failure_streak >= 5`. Expose `math.alert_required` and `math.degradation_note` in dashboards (example Grafana query: `sum by(session_id) (math_alert_required{service="deepresearch-core"})`).

### Kubernetes OTEL Collector manifests (example)

```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: otel-collector-config
  namespace: deepresearch
data:
  collector.yaml: |
    receivers:
      filelog/sandbox:
        include: [/var/log/deepresearch/*.log]
        operators:
          - type: json_parser
            parse_from: body
          - type: filter
            expr: 'attributes["target"] == "telemetry.sandbox"'

    processors:
      batch: {}

    exporters:
      otlphttp/default:
        endpoint: http://otel-gateway:4318
      prometheus/default:
        endpoint: 0.0.0.0:9464

    service:
      pipelines:
        logs/sandbox:
          receivers: [filelog/sandbox]
          processors: [batch]
          exporters: [otlphttp/default, prometheus/default]
---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: otel-collector
  namespace: deepresearch
spec:
  replicas: 1
  selector:
    matchLabels:
      app: otel-collector
  template:
    metadata:
      labels:
        app: otel-collector
    spec:
      containers:
        - name: collector
          image: otel/opentelemetry-collector-contrib:0.96.0
          args: ["--config=/etc/otel/collector.yaml"]
          volumeMounts:
            - name: config
              mountPath: /etc/otel
            - name: sandbox-logs
              mountPath: /var/log/deepresearch
          ports:
            - containerPort: 9464
              name: metrics
            - containerPort: 4318
              name: otlphttp
      volumes:
        - name: config
          configMap:
            name: otel-collector-config
        - name: sandbox-logs
          hostPath:
            path: /var/log/deepresearch
            type: Directory
---
apiVersion: v1
kind: Service
metadata:
  name: otel-collector
  namespace: deepresearch
spec:
  selector:
    app: otel-collector
  ports:
    - name: otlphttp
      port: 4318
      targetPort: 4318
    - name: metrics
      port: 9464
      targetPort: 9464
```

Mount the sandbox log directory appropriately (hostPath, CSI, or sidecar log shipping) to feed the filelog receiver. Adjust exporters to match your production endpoints.

### Collector with transform processor

```yaml
receivers:
  filelog/sandbox:
    include: [/var/log/deepresearch/*.log]
    operators:
      - type: regex_parser
        regex: '^(?P<time>[^ ]+)\s+(?P<level>[^ ]+)\s+(?P<target>[^ ]+)\s+-\s+(?P<body>.*)$'
        timestamp:
          parse_from: time
          layout: '%Y-%m-%dT%H:%M:%S%.fZ'
        severity:
          parse_from: level
      - type: filter
        expr: 'attributes["target"] == "telemetry.sandbox"'

processors:
  batch: {}

exporters:
  otlphttp:
    endpoint: http://otel-gateway:4318

service:
  pipelines:
    logs/sandbox:
      receivers: [filelog/sandbox]
      processors: [batch]
      exporters: [otlphttp]
```

Feed the resulting log stream into your metric pipeline (for example with the collector's `transform` processor) to emit a counter on `failure_streak`. A simple Prometheus alert reads:

```yaml
alert: SandboxFailureBurst
expr: max_over_time(telemetry_sandbox_failure_streak[5m]) >= 3
for: 2m
labels:
  severity: warning
annotations:
  summary: "Sandbox failures >=3 in the last 5 minutes"
  description: "Investigate math tool degradation for job {{ $labels.job }}"
```

### Optional Prometheus container for local testing

To experiment locally, run Prometheus alongside the collector:

```bash
docker run -d --name prometheus \
  -p 9090:9090 \
  -v $(pwd)/ops/prometheus.yml:/etc/prometheus/prometheus.yml:ro \
  prom/prometheus:latest
```

Sample `ops/prometheus.yml` scraping the collector’s Prometheus exporter:

```yaml
global:
  scrape_interval: 15s

scrape_configs:
  - job_name: 'deepresearch-sandbox'
    static_configs:
      - targets: ['otel-gateway:9464']
```

Update the OTEL collector pipeline to add a Prometheus exporter that exposes metrics derived from the sandbox logs:

```yaml
exporters:
  prometheus:
    endpoint: 0.0.0.0:9464

service:
  pipelines:
    logs/sandbox:
      receivers: [filelog/sandbox]
      processors: [batch, transform/sandbox_metrics]
      exporters: [otlphttp, prometheus]

processors:
  transform/sandbox_metrics:
    error_mode: ignore
    log_statements:
      - context: log
        statements:
          - set(metric.telemetry_sandbox_failure_streak, attributes.failure_streak)
```

Now Prometheus scrapes the collector and stores the `telemetry_sandbox_failure_streak` metric. Load `http://localhost:9090` to graph the metric or verify the alert rule.

### Runtime OTEL meter provider (Rust example)

DeepResearch publishes sandbox metrics via the OpenTelemetry `Meter` API. To export them, install an OTLP meter provider in the process hosting the runtime:

```rust
use opentelemetry::{global, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{runtime, Resource};

pub fn init_meter_provider(service_name: &str, endpoint: &str) {
    let resource = Resource::new(vec![KeyValue::new("service.name", service_name)]);
    let provider = opentelemetry_otlp::new_pipeline()
        .metrics(runtime::Tokio)
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(endpoint),
        )
        .with_resource(resource)
        .with_period(std::time::Duration::from_secs(15))
        .build()
        .expect("failed to build OTLP metrics pipeline");

    global::set_meter_provider(provider);
}

pub fn shutdown_meter_provider() {
    global::shutdown_meter_provider();
}
```

Call `init_meter_provider` early in your binary (CLI/API) and `shutdown_meter_provider` during shutdown to flush metrics. The OTEL Collector manifest above can then scrape and forward the emitted metrics.

---

## 6. Image Hygiene

- List images: `docker images | grep deepresearch`
- Remove unused sandbox tags: `docker image rm deepresearch-python-sandbox:<tag>`
- Prune dangling layers after upgrades: `docker system prune`

---

## 7. Troubleshooting

| Issue | Symptoms | Remediation |
|-------|----------|-------------|
| Missing build context | `docker: 'docker buildx build' requires 1 argument` | Ensure the trailing `.` path is included in `docker build` commands. |
| Mermaid CLI failure | Sandbox test errors mentioning Chromium | Rebuild the image; ensure headless Chromium dependencies remain in the Dockerfile. |
| Permission errors on bind mount | Sandbox outputs missing / permission denied | Confirm Docker Desktop has access to the repo path, and the host user has write permissions. |
| Compose port conflicts | Services fail to start | Adjust ports in `docker-compose.yml` and update CLI/API environment variables. |

---

## 8. Reference Commands

| Purpose | Command |
|---------|---------|
| Build sandbox image | `docker build -t deepresearch-python-sandbox:latest -f containers/python-sandbox/Dockerfile .` |
| Run sandbox smoke | `DEEPRESEARCH_SANDBOX_TESTS=1 DEEPRESEARCH_SANDBOX_IMAGE=deepresearch-python-sandbox:latest cargo test -p deepresearch-core --test sandbox -- --ignored --nocapture` |
| Start retrieval stack | `docker compose up -d` |
| Stop retrieval stack | `docker compose down` |
| Tail Qdrant logs | `docker compose logs -f qdrant` |
| Remove sandbox image | `docker image rm deepresearch-python-sandbox:latest` |

Keep this playbook updated whenever new container workflows or automation hooks are introduced.
