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
5. **Explainability Surfaces**
   - Confirm timeline, per-task metrics, and reasoning graph render once the session completes.
   - Toggle Markdown / Graph / Graphviz visibility and download each artifact (markdown, mermaid, graphviz) for archival.
   - Verify manual-review banner appears when the critic flags `final.requires_manual=true`.
   - Inspect raw trace JSON and reasoning steps for parity with CLI/API explain outputs.
6. **Downloads & Comparison**
   - Use the comparison selector to load a second session; check summary diff, metric delta table, and timeline drill-down.
   - Ensure clearing the comparison resets the tables and hides comparison timeline.
   - Validate downloaded artifacts respect the `use context7` prefix in upstream prompts.
7. **Postgres / Namespace Smoke (if feature enabled)**
   - `GUI_STORAGE=postgres GUI_POSTGRES_URL=... GUI_SESSION_NAMESPACE=gui-demo`.
   - Run a session and verify the session ID is namespaced (`gui-demo::...`).
   - Restart the service and resume the session by selecting it from the directory.
8. **Telemetry & Logs**
   - With `GUI_OTEL_ENDPOINT` unset, confirm standard structured logs appear.
   - Set `GUI_OTEL_ENDPOINT=http://collector:4317`, tail logs for `telemetry.gui` events (`session_started`, `session_completed`, `stream_opened`), and ensure your deployment-side subscriber forwards them to the tracing stack.
9. **CI Matrix**
   - `cargo fmt`
   - `cargo clippy --workspace --all-targets -- -D warnings`
   - `cargo test --workspace --all-targets -- --nocapture`
   - `RUST_LOG=warn cargo run --offline -p deepresearch-cli bench "GUI acceptance" --sessions 6 --concurrency 3 --format json`

Document verification results alongside the PR or release entry.
