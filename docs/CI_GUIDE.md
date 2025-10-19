# CI Guide — DeepResearch

The automated GitHub Actions workflow (`.github/workflows/ci.yml`) enforces the following gates on every PR and push:

1. **Frontend build** — `npm ci && npm run build` in `crates/deepresearch-gui/web`
2. **Formatting** — `cargo fmt --all -- --check`
3. **Linting** — `cargo clippy --workspace --all-targets -- -D warnings`
4. **Unit + integration tests** — `cargo test --workspace --all-targets -- --nocapture`
5. **Snapshot guard** — `cargo test --offline -p deepresearch-core finalize_summary_snapshot -- --nocapture`
6. **Bench latency check** — `cargo run --offline -p deepresearch-cli bench "CI bench" --sessions 8 --concurrency 4 --format json`
   - Fails if average latency > 350 ms, p95 latency > 400 ms, or any benchmark run fails
7. **API smoke** — launches `deepresearch-api`, hits `/health` and `/query`, then shuts down

## Running the CI matrix locally

Run the commands below from the repo root to mirror the workflow:

```bash
npm ci --prefix crates/deepresearch-gui/web
npm run build --prefix crates/deepresearch-gui/web
cargo fmt
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets -- --nocapture
cargo test --offline -p deepresearch-core finalize_summary_snapshot -- --nocapture
RUST_LOG=warn cargo run --offline -p deepresearch-cli bench "CI bench" --sessions 8 --concurrency 4 --format json
cargo run --offline -p deepresearch-api &
curl -s http://localhost:8080/health | jq .
kill $!
```

> Tips:
> - Use `RUST_LOG=warn` (or `error`) during bench runs to minimise log noise.
> - Install `jq` and `bc` locally to mirror the CI parsing logic.
> - Use `INSTA_UPDATE=always` with the snapshot command only when intentionally updating the baseline.

## Updating the workflow

- Add new checks to `.github/workflows/ci.yml`
- Reflect additions in this guide and `docs/TESTING_GUIDE.md`
- Ensure new commands have suitable offline flags or caching to keep CI fast
