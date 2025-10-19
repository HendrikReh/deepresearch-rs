# Usage Guide — DeepResearch CLI

This guide walks through running the DeepResearch workflow locally, enabling the FastEmbed + Qdrant retriever, and diagnosing common setup issues.

---

## 1. Prerequisites

- **Rust** (stable, via `rustup`).
- **Docker Desktop** (or compatible runtime) for the bundled Qdrant/Postgres stack.
- **Network access** the first time you run ingestion so FastEmbed can download the ONNX model (~127 MiB).

Optional:
- `curl` for health checks.
- `lsof`/`netstat` to confirm ports.

---

## 2. Start the Local Stack

```bash
# Launch Qdrant (REST 6333, gRPC 6334) and Postgres
docker-compose up -d

# Inspect running services (optional)
docker compose ps

# Verify Qdrant REST endpoint
curl http://localhost:6333/health      # → {"status":"ok"}

# Verify gRPC port is exposed
lsof -i :6334                          # expect docker-proxy or qdrant
```

> If you run Qdrant manually, ensure you publish both ports:
> `docker run -p 6333:6333 -p 6334:6334 -e QDRANT__SERVICE__GRPC_PORT=6334 qdrant/qdrant:latest`

Shutdown when finished with `docker-compose down`.

---

## 3. Core CLI Commands

```bash
# Format & lint
cargo fmt
cargo clippy --workspace --all-targets -- -D warnings

# Run a new research session (text output)
cargo run --offline -p deepresearch-cli query "Compare EV supply chains"

# Run with JSON output and an embedded mermaid trace
cargo run --offline -p deepresearch-cli query "Where are sodium-ion deployments accelerating?" \
  --format json \
  --explain \
  --explain-format mermaid

# Resume an existing session
cargo run --offline -p deepresearch-cli resume <SESSION_ID>

# Render the stored trace without re-running tasks
cargo run --offline -p deepresearch-cli explain <SESSION_ID> --include-summary

# Aggregate evaluation metrics from a JSONL log
cargo run --offline -p deepresearch-cli eval data/logs/demo.jsonl --format json

# Purge a session from Postgres storage (requires the docker-compose stack)
DATABASE_URL=postgres://deepresearch:deepresearch@localhost:5432/deepresearch \
  cargo run --offline -F postgres-session -p deepresearch-cli purge <SESSION_ID>

# Benchmark session throughput at a given concurrency
RUST_LOG=warn cargo run --offline -p deepresearch-cli bench "Stress-test battery policy query" \
  --sessions 24 \
  --concurrency 6 \
  --format json
```

Every command supports `--format text|json`; text mode prints a human-readable summary, while JSON mode returns a structured payload (bench responses report latency stats alongside success/failure counts).

### Explainability Output (`--explain`)

Use the built-in explainability flags to capture task-level traces and render reasoning graphs:

```bash
# Markdown summary (default)
cargo run --offline -p deepresearch-cli query "How are sodium-ion batteries tracking?" --explain

# Mermaid graph (wraps output in ```mermaid fences)
cargo run --offline -p deepresearch-cli query \
  "Map critical minerals policy responses" \
  --explain \
  --explain-format mermaid \
  --trace-dir data/custom-traces

# Retrieve an existing explanation without re-running tasks
cargo run --offline -p deepresearch-cli explain <SESSION_ID> --format text --explain-format graphviz
```

- `--explain` (or the `explain` subcommand) enables the trace collector, prints the formatted summary, and persists `trace.json` per session (defaults to `data/traces/<session>.json`).
- `--explain-format` accepts `markdown`, `mermaid`, or `graphviz`, matching the helpers on `SessionOutcome`.
- `--trace-dir` overrides the output directory; the folder is created on demand.

Each persisted file is an array of `TraceEvent` objects with `task_id`, `message`, and `timestamp_ms`. These events feed into `TraceSummary::render_mermaid()` / `render_graphviz()` for downstream visualization.

---

## 4. Enable Hybrid Retrieval (FastEmbed + Qdrant)

1. **Ingest supporting documents** into a session namespace:
   ```bash
   cargo run -F qdrant-retriever -p deepresearch-cli ingest \
     --session demo \
     --path ./docs \
     --qdrant-url http://localhost:6334
   ```
   This downloads the FastEmbed model on first run. Subsequent ingestions reuse `.fastembed_cache/`.

2. **Run the workflow with Qdrant-backed memory:**
   ```bash
   cargo run -F qdrant-retriever -p deepresearch-cli query \
     "Run a Qdrant-backed session" \
     --session demo \
     --qdrant-url http://localhost:6334
   ```

3. **Programmatic usage:** In Rust, call
   ```rust
   SessionOptions::new("Solar adoption outlook")
       .with_session_id("demo-session")
       .with_qdrant_retriever("http://localhost:6334", "deepresearch", 8);
   ```
   The helper uses gRPC under the hood; make sure the port is reachable.

---

## 5. Configure Fact-Check Behaviour

The fact-check task runs after the analyst and before the critic. Tweak its behaviour when constructing options:

```rust
use deepresearch_core::{FactCheckSettings, SessionOptions};

let options = SessionOptions::new("Energy storage resilience")
    .with_fact_check_settings(FactCheckSettings {
        min_confidence: 0.85,
        verification_count: 5,
        timeout_ms: 150,
    });
```

- `min_confidence` — minimum confidence required to avoid manual review.
- `verification_count` — how many sources to sample.
- `timeout_ms` — simulated wait before completing the fact-check (useful when modelling external calls).

The task stores results under `factcheck.*` context keys (`confidence`, `verified_sources`, `passed`, `notes`) for downstream reporting.

---

## 6. Evaluation Harness

Analyse JSONL logs to track nightly fact-check metrics:

```rust
use deepresearch_core::EvaluationHarness;

let metrics = EvaluationHarness::analyze_log("logs/factcheck.jsonl")?;
println!("{}", metrics.summary());
```

Entries with malformed JSON are skipped (emitting a `debug!` log). Failures are recorded by session ID.

---

## 7. Troubleshooting

| Symptom | Likely Cause | Fix |
|---------|--------------|-----|
| `Unknown error h2 protocol error` when ingesting/running | Connecting to Qdrant REST port (6333) instead of gRPC (6334) | Point `--qdrant-url` (or code) to `http://localhost:6334` and ensure the container exposes `QDRANT__SERVICE__GRPC_PORT=6334`. |
| `Failed to obtain server version. Unable to check compatibility.` | gRPC port blocked or service restarting | Wait for Qdrant to finish booting; verify `lsof -i :6334`. |
| FastEmbed download retried every run | `.fastembed_cache/` missing or cleared | Allow the first run to complete; subsequent runs reuse the cache. Add `.fastembed_cache` to `.gitignore` (already configured). |
| `curl` health check fails | Container not running or port in use | `docker compose ps`, check logs with `docker logs deepresearch-rs-qdrant-1`. |
| Need to start fresh | Stale vectors or schema changes | `curl -X DELETE http://localhost:6333/collections/deepresearch` or remove the volume (`rm -rf data/qdrant`). |

---

## 8. REST API Quickstart

```bash
# Start the Axum server (in-memory storage by default)
cargo run --offline -p deepresearch-api

# Configure via environment variables (optional)
export DEEPRESEARCH_API_ADDR=0.0.0.0:8080
export DEEPRESEARCH_TRACE_DIR=data/traces
export DEEPRESEARCH_QDRANT_URL=http://localhost:6334
export DEEPRESEARCH_QDRANT_COLLECTION=deepresearch
export DEEPRESEARCH_MAX_CONCURRENT_SESSIONS=5
```

### Endpoints

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/health` | Returns capacity counters (max, available, active) and retrieval mode. |
| `POST` | `/query` | Runs a research session and returns the summary + optional explanation. |
| `GET` | `/session/:id` | Fetches the latest session report without mutating state. |
| `POST` | `/ingest` | Indexes documents for the configured retriever (Qdrant optional). |

### Sample Requests

```bash
# Check current capacity and retriever mode
curl -s http://localhost:8080/health

# Run a session with an embedded markdown explanation
echo '{"query":"Assess regional battery incentives","explain":true}' \
  | curl -s http://localhost:8080/query -H 'content-type: application/json' -d @-

# Retrieve an existing session (graphviz explanation)
curl -s "http://localhost:8080/session/<SESSION_ID>?explain=true&explain_format=graphviz&include_summary=true"

# Ingest supporting material
cat <<'DOCS' | curl -s http://localhost:8080/ingest \
  -H 'content-type: application/json' \
  -d @-
{
  "session_id": "demo",
  "documents": [
    {"text": "Lithium demand grows 20% YoY", "source": "notes/lithium.txt"},
    {"text": "Sodium-ion pilots ramp in 2025", "source": "notes/sodium.txt"}
  ]
}
DOCS
```

Errors return JSON with an `error` field and HTTP status codes (`404` when a session is missing, `429` when capacity is exhausted, `500` for unexpected failures).

---

## 9. Clean-up

```bash
# Stop containers
docker-compose down

# Remove local data (optional)
rm -rf data/qdrant data/postgres

# Clear FastEmbed cache (optional)
rm -rf .fastembed_cache
```

With the retrieval layer active, the Researcher task will pull real documents from Qdrant, and the critic summary will enumerate the ingested sources. Refer back to `docs/TESTING_GUIDE.md` for verification steps and `AGENTS.md` for deeper architectural context.

---

## 10. Logging & Retention

- Session completions append redacted JSON lines to `data/logs/<year>/<month>/session.jsonl`; high-risk tokens (`api_key=…`, `bearer …`, `sk-…`) are replaced with `[REDACTED]` and mirrored into `audit.jsonl` for compliance reviews.
- Configure the log root and retention policy via environment variables:
  - `DEEPRESEARCH_LOG_DIR` (default `data/logs`).
  - `DEEPRESEARCH_LOG_RETENTION_DAYS` (default `90`; set to `0` to disable automated pruning).
- `deepresearch-cli purge` now removes the session ledger (logs + traces) alongside storage state so data deletion requests stay compliant.
- Run `deepresearch-cli bench …` while watching `GET /health` to tune `DEEPRESEARCH_MAX_CONCURRENT_SESSIONS` before 429 throttling kicks in.
