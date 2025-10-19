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

# Run the default workflow (in-memory sessions)
cargo run --offline -p deepresearch-cli run --query "Compare EV supply chains"

# Resume an existing session
cargo run --offline -p deepresearch-cli resume --session <uuid>

# Use Postgres-backed storage (requires docker-compose stack)
DATABASE_URL=postgres://deepresearch:deepresearch@localhost:5432/deepresearch \
  cargo run --offline -F postgres-session -p deepresearch-cli run --session $(uuidgen)
```

The CLI prints the critic verdict, analyst summary, key insight, and enumerated sources.

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
   cargo run -F qdrant-retriever -p deepresearch-cli run \
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

## 8. Clean-up

```bash
# Stop containers
docker-compose down

# Remove local data (optional)
rm -rf data/qdrant data/postgres

# Clear FastEmbed cache (optional)
rm -rf .fastembed_cache
```

With the retrieval layer active, the Researcher task will pull real documents from Qdrant, and the critic summary will enumerate the ingested sources. Refer back to `docs/TESTING_GUIDE.md` for verification steps and `AGENTS.md` for deeper architectural context.
