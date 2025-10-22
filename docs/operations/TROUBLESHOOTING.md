# DeepResearch Troubleshooting Guide

This document covers common issues encountered during development and deployment of DeepResearch, along with their solutions.

---

## Docker Sandbox (M12 — Secure Code Execution)

### Issue: Sandbox tests failing with Puppeteer/Chromium errors

**Symptoms:**
```
Error: Failed to launch the browser process!
rosetta error: failed to open elf at /lib64/ld-linux-x86-64.so.2
```
or
```
Error: Could not find Chrome (ver. 131.0.6778.204)
```
or
```
[ERROR:zygote_host_impl_linux.cc(127)] No usable sandbox!
```

**Root Cause:**

The `mermaid-cli` (`mmdc`) tool requires Puppeteer with Chrome/Chromium to render diagrams. Several issues can occur:

1. **Architecture mismatch**: Puppeteer may download x86_64 Chrome binaries when running on ARM64 (Apple Silicon)
2. **Missing Chrome**: Puppeteer can't find the Chrome binary
3. **Nested sandbox**: Chromium's internal sandbox conflicts with Docker's container isolation

**Solution:**

The fix is implemented in `containers/python-sandbox/Dockerfile`:

1. **Install system Chromium** (architecture-native):
   ```dockerfile
   apt-get install -y --no-install-recommends nodejs chromium chromium-driver
   ```

2. **Configure Puppeteer to use system Chromium**:
   ```dockerfile
   ENV PUPPETEER_EXECUTABLE_PATH=/usr/bin/chromium \
       PUPPETEER_SKIP_CHROMIUM_DOWNLOAD=true \
       PUPPETEER_ARGS="--no-sandbox,--disable-setuid-sandbox,--disable-dev-shm-usage"
   ```

3. **Create mmdc wrapper** to pass security flags:
   ```dockerfile
   RUN set -eux; \
       echo '{"args": ["--no-sandbox", "--disable-setuid-sandbox", "--disable-dev-shm-usage"]}' > /etc/mermaid-puppeteer.json; \
       mmdc_path=$(which mmdc); \
       mv "$mmdc_path" "${mmdc_path}.real"; \
       echo '#!/bin/sh' > "$mmdc_path"; \
       echo "exec ${mmdc_path}.real --puppeteerConfigFile /etc/mermaid-puppeteer.json \"\$@\"" >> "$mmdc_path"; \
       chmod +x "$mmdc_path"
   ```

**Verification:**

After rebuilding the image:
```bash
docker build -t deepresearch-python-sandbox:latest -f containers/python-sandbox/Dockerfile .
DEEPRESEARCH_SANDBOX_TESTS=1 cargo test -p deepresearch-core --test sandbox -- --ignored --nocapture
```

Expected output:
```
test sandbox_generates_expected_artifacts ... ok
```

**Debug Tips:**

To see actual error output from failed sandbox runs, the test now prints stdout/stderr when exit code is non-zero. Check the output after `=== SANDBOX STDERR ===` marker.

---

## Build Issues

### Issue: Cargo build fails with dependency conflicts

**Solution:**
```bash
# Clear cargo cache and rebuild
cargo clean
rm -rf ~/.cargo/registry/index/*
cargo build --workspace
```

### Issue: Docker image build times out or fails

**Symptoms:**
- Build hangs during `apt-get` or `npm install` steps
- Network timeout errors

**Solution:**
```bash
# Increase Docker resources (Docker Desktop → Settings → Resources)
# Recommended: 4+ CPUs, 8GB+ memory

# Or build with explicit timeout
docker build --network=host -t deepresearch-python-sandbox:latest -f containers/python-sandbox/Dockerfile .
```

---

## Test Failures

### Issue: Integration tests fail with "session not found"

**Symptoms:**
```
Error: session <UUID> not found in storage
```

**Solution:**

Ensure you're using consistent storage backends:
```bash
# In-memory (default) - sessions don't persist between runs
cargo test --workspace --all-targets

# Postgres-backed (persistent)
docker-compose up -d
DATABASE_URL=postgres://deepresearch:deepresearch@localhost:5432/deepresearch \
  cargo test --features postgres-session --workspace --all-targets
```

### Issue: Snapshot tests fail after intentional output changes

**Symptoms:**
```
snapshot assertion for 'finalize_summary' failed
```

**Solution:**

Update snapshots deliberately:
```bash
INSTA_UPDATE=always cargo test --offline -p deepresearch-core finalize_summary_snapshot -- --nocapture
```

**Warning:** Only use `INSTA_UPDATE=always` when you've intentionally changed the baseline output format.

---

## CLI/API Issues

### Issue: CLI query returns empty results

**Symptoms:**
```bash
cargo run -p deepresearch-cli query "test" --format json
# Returns: {"summary": "", "claims": []}
```

**Solution:**

Check retriever configuration:
```bash
# Verify Qdrant is running
curl http://localhost:6333/health

# Run with retriever enabled
cargo run -F qdrant-retriever -p deepresearch-cli query "test" --qdrant-url http://localhost:6334
```

### Issue: API returns 429 (Too Many Requests)

**Symptoms:**
```bash
curl http://localhost:8080/query
# {"error": "max concurrent sessions exceeded"}
```

**Solution:**

Adjust concurrency limits:
```bash
# Set environment variable before starting API
DEEPRESEARCH_MAX_CONCURRENT_SESSIONS=20 cargo run -p deepresearch-api
```

---

## GUI Issues

### Issue: GUI assets not loading / 404 errors

**Symptoms:**
- Blank page when visiting `http://localhost:8080`
- Browser console shows 404 for `/dist/index.js`

**Solution:**

Build the frontend bundle first:
```bash
npm ci --prefix crates/deepresearch-gui/web
npm run build --prefix crates/deepresearch-gui/web
cargo run -p deepresearch-gui
```

### Issue: GUI streaming events not appearing

**Symptoms:**
- Timeline remains empty during query execution
- No real-time updates

**Solution:**

Check SSE endpoint manually:
```bash
curl -N http://localhost:8080/api/stream/<SESSION_ID>
# Should return event-stream data
```

If no events appear, verify `trace.enabled` is set in the session context.

---

## Performance Issues

### Issue: Slow test execution / timeouts

**Symptoms:**
```
test bench_latency_threshold ... timeout after 60s
```

**Solution:**

Run in release mode for performance-critical tests:
```bash
cargo test --release -p deepresearch-core bench_latency_threshold
```

Or adjust test timeouts in code (see `TESTING_GUIDE.md` for benchmark thresholds).

### Issue: High memory usage during batch operations

**Symptoms:**
- Process killed by OOM
- Docker container exits with code 137

**Solution:**

Reduce concurrency or batch sizes:
```bash
# CLI bench with lower concurrency
cargo run -p deepresearch-cli bench "test" --sessions 4 --concurrency 2

# Docker: increase memory limit
docker run --memory=4g deepresearch-python-sandbox:latest
```

---

## Security & Privacy

### Issue: Logs contain sensitive information

**Symptoms:**
- API keys visible in `data/logs/` JSONL files

**Solution:**

Verify redaction is working:
```bash
cargo test -p deepresearch-core logging::tests::session_logging_sanitizes_and_persists

# Check log files manually
grep -r "sk-" data/logs/
# Should return: [REDACTED]
```

If redaction fails, update the regex patterns in `crates/deepresearch-core/src/logging.rs`.

---

## Qdrant / Vector DB Issues

### Issue: Connection refused to Qdrant

**Symptoms:**
```
Error: Connection refused (os error 61)
```

**Solution:**

Ensure Qdrant is running and accessible:
```bash
docker-compose up -d
docker ps | grep qdrant

# Verify gRPC port (6334) is exposed
curl http://localhost:6333/health
lsof -i :6334
```

### Issue: Collection not found

**Symptoms:**
```
Error: Collection 'deepresearch' not found
```

**Solution:**

Create collection before ingesting:
```bash
curl -X PUT http://localhost:6333/collections/deepresearch \
  -H 'Content-Type: application/json' \
  -d '{
    "vectors": {
      "size": 384,
      "distance": "Cosine"
    }
  }'
```

Or use the CLI ingest command which auto-creates collections:
```bash
cargo run -F qdrant-retriever -p deepresearch-cli ingest \
  --session demo \
  --path ./docs \
  --qdrant-url http://localhost:6334
```

---

## Evaluation Harness Issues

### Issue: Harness requires `--input`

**Symptoms:**
```
error: the following required arguments were not provided:
  --input <INPUT>
```

**Solution:** Always point to a curated snapshot:
```bash
cargo run -p eval-harness -- \
  --input data/pipeline/curated/sessions_latest.json \
  --output-dir data/eval/latest
```

### Issue: Snapshot missing (`No such file or directory`)

**Symptoms:**
```
Error: open data/pipeline/curated/sessions_latest.json
Caused by:
    No such file or directory (os error 2)
```

**Root Cause:** The consolidation job has not produced a curated snapshot yet.

**Solution:**
1. Generate raw session logs by running the CLI/API at least once:
   ```bash
   cargo run -p deepresearch-cli -- query "use context7 baseline sanity" --format json
   ```
2. Consolidate the raw JSONL files:
   ```bash
   cargo run -p data-pipeline -- \
     --raw-dir data/pipeline/raw \
     --output-dir data/pipeline/curated
   ```
3. Re-run the harness against the new `sessions_latest.json`.

### Issue: `evaluation thresholds exceeded`

**Symptoms:**
```
Error: evaluation thresholds exceeded: verdict changed 2 > allowed 0
```

**Root Cause:** Verdict/math/manual deltas exceeded the configured `--max-*` limits or the bootstrap confidence interval crossed the permitted proportion.

**Solution:**
1. Review artefacts under `data/eval/latest/`:
   - `report.json` / `report.md` for counts, CIs, and p-values.
   - `dashboard.html` for a visual summary of guardrails and bucket metrics.
   - `deltas/` JSONL batches for per-session diffs.
2. Investigate the drift and remediate regressions. For exploratory runs you may temporarily relax thresholds (e.g., `--max-verdict-delta 2`), but keep stricter limits for promotion gating.

---

## CI/CD Issues

### Issue: GitHub Actions CI failing on sandbox tests

**Symptoms:**
```
Error: Docker daemon not available in CI
```

**Solution:**

The CI workflow has a separate `sandbox` job that runs sandbox tests with Docker-in-Docker. Ensure:

1. The `sandbox` job depends on the main `build` job:
   ```yaml
   sandbox:
     runs-on: ubuntu-latest
     needs: build
   ```

2. The sandbox image tag matches:
   ```yaml
   env:
     DEEPRESEARCH_SANDBOX_IMAGE: "deepresearch-python-sandbox:ci"
   ```

3. Tests are marked as `#[ignore]` and run explicitly:
   ```bash
   cargo test -p deepresearch-core --test sandbox -- --ignored --nocapture
   ```

---

## Getting Help

If you encounter an issue not covered here:

1. **Check existing issues**: [GitHub Issues](https://github.com/HendrikReh/deepresearch-rs/issues)
2. **Review logs**: Enable debug logging with `RUST_LOG=debug cargo run ...`
3. **Run diagnostics**:
   ```bash
   cargo fmt
   cargo clippy --workspace --all-targets -- -D warnings
   cargo check --offline
   ```
4. **Create a minimal reproduction** and open a new issue with:
   - Rust version (`rustc --version`)
   - OS/architecture
   - Full error output
   - Steps to reproduce

---

*Last updated: 2025-02-14*
