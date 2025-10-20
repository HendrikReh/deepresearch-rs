# GUI Acceptance Checklist

Use this checklist before merging GUI-affecting changes or promoting the v0.2 release:

1. **Build Assets**
   - `npm install --prefix crates/deepresearch-gui/web`
   - `npm ci --prefix crates/deepresearch-gui/web`
   - `npm run build --prefix crates/deepresearch-gui/web`
2. **Health & Readiness Probes**
   - `curl -s http://localhost:8080/health/live | jq`
   - `curl -i http://localhost:8080/health/ready`
   - When `GUI_ENABLE_GUI=false`, readiness returns `503`.
3. **Authentication Gate**
   - Launch with `GUI_AUTH_TOKEN=secret` and confirm `/api/sessions` fails without `Authorization: Bearer secret`.
4. **Session Lifecycle**
   - Start a new session via GUI form; observe streaming events and final summary.
   - Confirm capacity widget updates (`available_permits` decrements while running).
   - Select an existing session from the directory and verify status rehydrates.
5. **Trace Surfaces**
   - Ensure Markdown summary, reasoning steps, and raw trace JSON render after completion.
6. **Postgres / Namespace Smoke (if feature enabled)**
   - `GUI_STORAGE=postgres GUI_POSTGRES_URL=... GUI_SESSION_NAMESPACE=gui-demo`.
   - Run a session and verify the session ID is namespaced (`gui-demo::...`).
   - Restart the service and resume the session by selecting it from the directory.
7. **Telemetry & Logs**
   - With `GUI_OTEL_ENDPOINT` unset, confirm standard structured logs appear.
   - If exporting to an OTLP collector, verify traces are received (feature flag required).
8. **CI Matrix**
   - `cargo fmt`
   - `cargo clippy --workspace --all-targets -- -D warnings`
   - `cargo test --workspace --all-targets -- --nocapture`
   - `RUST_LOG=warn cargo run --offline -p deepresearch-cli bench "GUI acceptance" --sessions 6 --concurrency 3 --format json`

Document verification results alongside the PR or release entry.
