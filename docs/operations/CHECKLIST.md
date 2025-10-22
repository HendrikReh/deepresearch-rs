# Command Execution Checklist

## Use Case Playbooks

### Start the Local Stack

- [ ] `docker-compose up -d` — `docs/getting-started/USAGE.md:23`; `docs/getting-started/TESTING_GUIDE.md:109`
- [ ] `docker compose ps` — `docs/getting-started/USAGE.md:26`; `docs/getting-started/USAGE.md:181`
- [ ] `curl http://localhost:6333/health` — `docs/getting-started/TESTING_GUIDE.md:110`; `docs/operations/TROUBLESHOOTING.md:163`
- [ ] `lsof -i :6334` — `docs/getting-started/TESTING_GUIDE.md:111`; `docs/getting-started/USAGE.md:179`

### Run a CLI Research Session (with explainability)

- [ ] `cargo run --offline -p deepresearch-cli query "Map critical minerals policy responses" --explain --explain-format mermaid --trace-dir data/custom-traces` — `docs/getting-started/USAGE.md:89`
- [ ] `cargo run --offline -p deepresearch-cli explain <SESSION_ID> --include-summary` — `docs/getting-started/USAGE.md:62`
- [ ] `cargo run --offline -p deepresearch-cli eval data/logs/demo.jsonl --format json` — `docs/getting-started/USAGE.md:65`
- [ ] `cargo run --offline -p deepresearch-cli purge <SESSION_ID>` — `docs/release/RELEASE_CHECKLIST.md:23`

### Build and Launch the GUI Dashboard

- [ ] `npm ci --prefix crates/deepresearch-gui/web` — `README.md:66`; `docs/getting-started/GUI_DEPLOYMENT.md:6`
- [ ] `npm run build --prefix crates/deepresearch-gui/web` — `README.md:67`; `docs/getting-started/GUI_DEPLOYMENT.md:6`
- [ ] `cargo run -p deepresearch-gui -- --gui-enabled` — `README.md:70`
- [ ] `curl -s http://localhost:8080/health/live | jq` — `docs/getting-started/GUI_ACCEPTANCE.md:10`

## Setup & Repo

- [ ] `cargo fetch` — `CONTRIBUTING.md:21`
- [ ] `cargo install --path crates/deepresearch-cli` — `README.md:36`
- [ ] `git clone https://github.com/your-org/deepresearch-rs.git` — `CONTRIBUTING.md:16`
- [ ] `git status` — `CONTRIBUTING.md:65`; `docs/release/RELEASE_CHECKLIST.md:7`
- [ ] `git tag vX.Y.Z && git push origin vX.Y.Z` — `docs/release/RELEASE_CHECKLIST.md:52`
- [ ] `make build-sandbox-image # or docker build -t deepresearch-python-sandbox:latest ...` — `docker-compose.overrides/README.md:12`

## GUI Assets

- [ ] `npm ci && npm run build` — `docs/release/CI_GUIDE.md:5`
- [ ] `npm ci --prefix crates/deepresearch-gui/web` — `CONTRIBUTING.md:26`; `CONTRIBUTING.md:54`; `README.md:66`; `docs/release/CI_GUIDE.md:20`; `docs/getting-started/GUI_ACCEPTANCE.md:7`; `docs/getting-started/GUI_DEPLOYMENT.md:6`; `docs/getting-started/TESTING_GUIDE.md:22`; `docs/operations/TROUBLESHOOTING.md:199`
- [ ] `npm install` — `docs/operations/TROUBLESHOOTING.md:94`
- [ ] `npm install --prefix crates/deepresearch-gui/web` — `CONTRIBUTING.md:25`; `docs/getting-started/GUI_ACCEPTANCE.md:6`; `docs/getting-started/TESTING_GUIDE.md:22`
- [ ] `npm run build --prefix crates/deepresearch-gui/web` — `CONTRIBUTING.md:55`; `README.md:67`; `docs/release/CI_GUIDE.md:21`; `docs/getting-started/GUI_ACCEPTANCE.md:8`; `docs/getting-started/GUI_DEPLOYMENT.md:6`; `docs/getting-started/TESTING_GUIDE.md:22`; `docs/operations/TROUBLESHOOTING.md:200`

## Format & Lint

- [ ] `cargo clippy -- -D warnings` — `CONTRIBUTING.md:62`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` — `CONTRIBUTING.md:42`; `README.md:137`; `docs/release/CI_GUIDE.md:23`; `docs/release/CI_GUIDE.md:7`; `docs/getting-started/GUI_ACCEPTANCE.md:37`; `docs/release/RELEASE_CHECKLIST.md:8`; `docs/getting-started/TESTING_GUIDE.md:24`; `docs/operations/TROUBLESHOOTING.md:434`; `docs/getting-started/USAGE.md:47`
- [ ] `cargo fmt` — `CONTRIBUTING.md:41`; `CONTRIBUTING.md:63`; `docs/release/CI_GUIDE.md:22`; `docs/getting-started/GUI_ACCEPTANCE.md:36`; `docs/release/RELEASE_CHECKLIST.md:8`; `docs/getting-started/TESTING_GUIDE.md:23`; `docs/operations/TROUBLESHOOTING.md:433`; `docs/getting-started/USAGE.md:46`
- [ ] `cargo fmt # format` — `AGENTS.md:108`
- [ ] `cargo fmt --all` — `README.md:136`
- [ ] `cargo fmt --all -- --check` — `docs/release/CI_GUIDE.md:6`

## Build & Check

- [ ] `cargo build --workspace` — `docs/operations/TROUBLESHOOTING.md:88`
- [ ] `cargo build -p deepresearch-api -p deepresearch-cli` — `docker-compose.overrides/README.md:13`
- [ ] `cargo check` — `docs/operations/RUNBOOK_SANDBOX_OPERATIONS.md:102`
- [ ] `cargo check --offline` — `docs/getting-started/TESTING_GUIDE.md:25`; `docs/operations/TROUBLESHOOTING.md:435`
- [ ] `cargo check --offline # build without hitting crates.io` — `AGENTS.md:109`
- [ ] `cargo clean` — `docs/operations/TROUBLESHOOTING.md:86`

## Core Tests

- [ ] `INSTA_UPDATE=always cargo test --offline -p deepresearch-core finalize_summary_snapshot` — `docs/getting-started/TESTING_GUIDE.md:29`
- [ ] `INSTA_UPDATE=always cargo test --offline -p deepresearch-core finalize_summary_snapshot -- --nocapture` — `docs/operations/TROUBLESHOOTING.md:141`
- [ ] `cargo test` — `CONTRIBUTING.md:61`
- [ ] `cargo test --features postgres-session --workspace --all-targets` — `docs/operations/TROUBLESHOOTING.md:127`
- [ ] `cargo test --offline` — `PLAN.md:75`; `docs/getting-started/TESTING_GUIDE.md:143`
- [ ] `cargo test --offline --workspace --all-targets -- --nocapture` — `docs/getting-started/TESTING_GUIDE.md:30`
- [ ] `cargo test --offline --workspace --all-targets -- --nocapture # mirrors CI` — `AGENTS.md:111`
- [ ] `cargo test --offline -p deepresearch-core critic_verdict_is_non_empty` — `docs/getting-started/TESTING_GUIDE.md:38`
- [ ] `cargo test --offline -p deepresearch-core finalize_summary_snapshot` — `docs/release/RELEASE_CHECKLIST.md:9`; `docs/getting-started/TESTING_GUIDE.md:126`; `docs/getting-started/TESTING_GUIDE.md:29`
- [ ] `cargo test --offline -p deepresearch-core finalize_summary_snapshot -- --nocapture` — `AGENTS.md:112`; `CONTRIBUTING.md:48`; `docs/release/CI_GUIDE.md:25`; `docs/release/CI_GUIDE.md:9`
- [ ] `cargo test --offline -p deepresearch-core manual_review_branch_triggers` — `docs/getting-started/TESTING_GUIDE.md:42`; `docs/getting-started/TESTING_GUIDE.md:45`
- [ ] `cargo test --offline -p deepresearch-gui --test http -- --nocapture` — `README.md:139`
- [ ] `cargo test --release -p deepresearch-core bench_latency_threshold` — `docs/operations/TROUBLESHOOTING.md:235`
- [ ] `cargo test --workspace --all-targets` — `docs/operations/TROUBLESHOOTING.md:122`
- [ ] `cargo test --workspace --all-targets -- --nocapture` — `CONTRIBUTING.md:47`; `README.md:138`; `docs/release/CI_GUIDE.md:24`; `docs/release/CI_GUIDE.md:8`; `docs/getting-started/GUI_ACCEPTANCE.md:38`; `docs/release/RELEASE_CHECKLIST.md:8`; `docs/getting-started/TESTING_GUIDE.md:26`
- [ ] `cargo test -p deepresearch-core eval::tests::evaluation_harness_aggregates_confidence` — `docs/release/RELEASE_CHECKLIST.md:47`; `docs/getting-started/TESTING_GUIDE.md:63`
- [ ] `cargo test -p deepresearch-core logging::tests::session_logging_sanitizes_and_persists` — `docs/getting-started/TESTING_GUIDE.md:90`; `docs/operations/TROUBLESHOOTING.md:270`
- [ ] `cargo test -p deepresearch-gui --test http -- --nocapture` — `docs/getting-started/TESTING_GUIDE.md:27`

## Extended Tests & Sandbox

- [ ] `DEEPRESEARCH_SANDBOX_TESTS=1 DEEPRESEARCH_SANDBOX_IMAGE=deepresearch-python-sandbox:ci cargo test -p deepresearch-core --test sandbox -- --ignored --nocapture` — `docs/release/CI_GUIDE.md:31`
- [ ] `DEEPRESEARCH_SANDBOX_TESTS=1 DEEPRESEARCH_SANDBOX_IMAGE=deepresearch-python-sandbox:latest cargo test -p deepresearch-core --test sandbox -- --ignored --nocapture` — `docs/operations/OPERATIONS.md:331`
- [ ] `DEEPRESEARCH_SANDBOX_TESTS=1 cargo test -p deepresearch-core --test integration_sandbox -- --ignored --nocapture` — `docs/operations/RUNBOOK_SANDBOX_OPERATIONS.md:18`; `docs/getting-started/TESTING_GUIDE.md:132`
- [ ] `DEEPRESEARCH_SANDBOX_TESTS=1 cargo test -p deepresearch-core --test sandbox -- --ignored --nocapture` — `AGENTS.md:118`; `docs/operations/RUNBOOK_SANDBOX_OPERATIONS.md:17`; `docs/getting-started/TESTING_GUIDE.md:100`; `docs/getting-started/TESTING_GUIDE.md:28`; `docs/operations/TROUBLESHOOTING.md:65`
- [ ] `cargo test -p deepresearch-core --test sandbox -- --ignored` — `docs/release/CI_GUIDE.md:13`
- [ ] `cargo test -p deepresearch-core --test sandbox -- --ignored --nocapture` — `docs/operations/OPERATIONS.md:40`; `docs/operations/TROUBLESHOOTING.md:420`

## Service Processes

- [ ] `DEEPRESEARCH_MAX_CONCURRENT_SESSIONS=20 cargo run -p deepresearch-api` — `docs/operations/TROUBLESHOOTING.md:182`
- [ ] `GUI_ENABLE_GUI=true cargo run -p deepresearch-gui` — `CONTRIBUTING.md:56`
- [ ] `cargo run --offline -p deepresearch-api` — `docs/getting-started/TESTING_GUIDE.md:80`; `docs/getting-started/USAGE.md:190`
- [ ] `cargo run --offline -p deepresearch-api &` — `README.md:53`; `docs/release/CI_GUIDE.md:27`; `docs/release/RELEASE_CHECKLIST.md:16`
- [ ] `cargo run -p deepresearch-cli` — `AGENTS.md:110`
- [ ] `cargo run -p deepresearch-gui` — `PLAN.md:101`; `docs/operations/TROUBLESHOOTING.md:201`
- [ ] `cargo run -p deepresearch-gui -- --gui-enabled` — `README.md:70`

## CLI Queries

- [ ] `RUST_LOG=info cargo run --offline -p deepresearch-cli query "Tracing check"` — `docs/getting-started/TESTING_GUIDE.md:41`
- [ ] `cargo run --offline -F postgres-session -p deepresearch-cli query "Resume via pg" --session pg-demo` — `docs/getting-started/TESTING_GUIDE.md:54`
- [ ] `cargo run --offline -p deepresearch-cli query "Baseline market scan"` — `docs/getting-started/TESTING_GUIDE.md:37`
- [ ] `cargo run --offline -p deepresearch-cli query "Compare EV supply chains"` — `docs/getting-started/USAGE.md:50`
- [ ] `cargo run --offline -p deepresearch-cli query "Explainability" --explain --format json` — `docs/getting-started/TESTING_GUIDE.md:68`
- [ ] `cargo run --offline -p deepresearch-cli query "How are sodium-ion batteries tracking?" --explain` — `docs/getting-started/USAGE.md:86`
- [ ] `cargo run --offline -p deepresearch-cli query "Interfaces" --format json` — `docs/getting-started/TESTING_GUIDE.md:75`
- [ ] `cargo run --offline -p deepresearch-cli query "Log test"` — `docs/getting-started/TESTING_GUIDE.md:92`
- [ ] `cargo run --offline -p deepresearch-cli query "Map critical minerals policy responses" --explain --explain-format mermaid --trace-dir data/custom-traces` — `docs/getting-started/USAGE.md:89`
- [ ] `cargo run --offline -p deepresearch-cli query "Release sanity" --format text` — `docs/release/RELEASE_CHECKLIST.md:13`
- [ ] `cargo run --offline -p deepresearch-cli query "Resume test" --session demo-resume` — `docs/getting-started/TESTING_GUIDE.md:49`
- [ ] `cargo run --offline -p deepresearch-cli query "Where are sodium-ion deployments accelerating?" --format json --explain --explain-format mermaid` — `docs/getting-started/USAGE.md:53`
- [ ] `cargo run -F qdrant-retriever -p deepresearch-cli query "Hybrid retrieval" --session demo --qdrant-url http://localhost:6334` — `docs/getting-started/TESTING_GUIDE.md:60`
- [ ] `cargo run -F qdrant-retriever -p deepresearch-cli query "Release retrieval" --session release --qdrant-url http://localhost:6334` — `docs/release/RELEASE_CHECKLIST.md:45`
- [ ] `cargo run -F qdrant-retriever -p deepresearch-cli query "Run a Qdrant-backed session" --session demo --qdrant-url http://localhost:6334` — `docs/getting-started/USAGE.md:120`
- [ ] `cargo run -F qdrant-retriever -p deepresearch-cli query "test" --qdrant-url http://localhost:6334` — `docs/operations/TROUBLESHOOTING.md:166`
- [ ] `cargo run -p deepresearch-cli -- query "use context7 baseline sanity" --format json` — `docs/operations/TROUBLESHOOTING.md:364`
- [ ] `cargo run -p deepresearch-cli query "test" --format json` — `docs/operations/TROUBLESHOOTING.md:154`

## CLI Explain / Resume / Ingest

- [ ] `cargo run --offline -F postgres-session -p deepresearch-cli purge <SESSION_ID>` — `docs/getting-started/USAGE.md:69`
- [ ] `cargo run --offline -p deepresearch-cli eval data/logs/demo.jsonl --format json` — `docs/getting-started/USAGE.md:65`
- [ ] `cargo run --offline -p deepresearch-cli eval data/logs/sample.jsonl --format text` — `docs/getting-started/TESTING_GUIDE.md:65`
- [ ] `cargo run --offline -p deepresearch-cli explain <SESSION> --include-summary` — `docs/getting-started/TESTING_GUIDE.md:76`
- [ ] `cargo run --offline -p deepresearch-cli explain <SESSION_ID> --format text --explain-format graphviz` — `docs/getting-started/USAGE.md:96`
- [ ] `cargo run --offline -p deepresearch-cli explain <SESSION_ID> --include-summary` — `docs/getting-started/USAGE.md:62`
- [ ] `cargo run --offline -p deepresearch-cli purge <SESSION>` — `docs/getting-started/TESTING_GUIDE.md:77`; `docs/getting-started/TESTING_GUIDE.md:95`
- [ ] `cargo run --offline -p deepresearch-cli purge <SESSION_ID>` — `docs/release/RELEASE_CHECKLIST.md:23`
- [ ] `cargo run --offline -p deepresearch-cli resume <SESSION_ID>` — `docs/getting-started/USAGE.md:59`
- [ ] `cargo run --offline -p deepresearch-cli resume demo-resume` — `docs/getting-started/TESTING_GUIDE.md:49`
- [ ] `cargo run -F qdrant-retriever -p deepresearch-cli ingest --session demo --path ./docs --qdrant-url http://localhost:6334` — `docs/getting-started/TESTING_GUIDE.md:59`; `docs/operations/TROUBLESHOOTING.md:325`; `docs/getting-started/USAGE.md:111`
- [ ] `cargo run -F qdrant-retriever -p deepresearch-cli ingest --session release --path ./docs --qdrant-url http://localhost:6334` — `docs/release/RELEASE_CHECKLIST.md:44`
- [ ] `deepresearch-cli ingest --session <id> --path <docs> --qdrant-url http://localhost:6334` — `AGENTS.md:125`; `AGENTS.md:159`
- [ ] `deepresearch-cli purge` — `docs/getting-started/USAGE.md:263`
- [ ] `deepresearch-cli purge <SESSION>` — `docs/getting-started/TESTING_GUIDE.md:125`
- [ ] `deepresearch-cli resume` — `AGENTS.md:168`

## CLI Benchmarks

- [ ] `RUST_LOG=warn cargo run --offline -p deepresearch-cli bench "CI bench" --sessions 8 --concurrency 4 --format json` — `docs/release/CI_GUIDE.md:26`
- [ ] `RUST_LOG=warn cargo run --offline -p deepresearch-cli bench "Dev bench" --sessions 8 --concurrency 4 --format json` — `CONTRIBUTING.md:49`
- [ ] `RUST_LOG=warn cargo run --offline -p deepresearch-cli bench "GUI acceptance" --sessions 6 --concurrency 3 --format json` — `docs/getting-started/GUI_ACCEPTANCE.md:39`
- [ ] `RUST_LOG=warn cargo run --offline -p deepresearch-cli bench "Release bench" --sessions 24 --concurrency 6 --format json` — `docs/release/RELEASE_CHECKLIST.md:27`
- [ ] `RUST_LOG=warn cargo run --offline -p deepresearch-cli bench "Stress-test battery policy query" --sessions 24 --concurrency 6 --format json` — `docs/getting-started/USAGE.md:72`
- [ ] `cargo run --offline -p deepresearch-cli bench "CI bench" --sessions 8 --concurrency 4 --format json` — `docs/release/CI_GUIDE.md:10`
- [ ] `cargo run --offline -p deepresearch-cli bench "Capacity tuning" --sessions 12 --concurrency 4` — `docs/getting-started/TESTING_GUIDE.md:78`
- [ ] `cargo run -p deepresearch-cli bench "test" --sessions 4 --concurrency 2` — `docs/operations/TROUBLESHOOTING.md:251`
- [ ] `deepresearch-cli bench …` — `docs/getting-started/USAGE.md:264`

## Data Pipeline & Evaluation

- [ ] `cargo run -p data-pipeline -- --raw-dir data/pipeline/raw --output-dir data/pipeline/curated` — `docs/evaluation/M13_EVALUATION.md:57`; `docs/operations/TROUBLESHOOTING.md:368`
- [ ] `cargo run -p data-pipeline -- --raw-dir data/pipeline/raw --output-dir data/pipeline/curated --postgres-url $DATABASE_URL # optional` — `docs/operations/RUNBOOK_SANDBOX_OPERATIONS.md:59`
- [ ] `cargo run -p eval-harness -- --input data/pipeline/curated/sessions_latest.json --output-dir data/eval/latest` — `docs/operations/TROUBLESHOOTING.md:345`
- [ ] `cargo run -p eval-harness -- --input data/pipeline/curated/sessions_latest.json --output-dir data/eval/latest --limit 50 --max-verdict-delta 0 --bootstrap-samples 2000 --batch-size 200 --delta-sample-limit 250 --replay cargo --replay run --replay --offline --replay -p --replay deepresearch-cli --replay query --replay --format --replay json` — `docs/operations/RUNBOOK_SANDBOX_OPERATIONS.md:83`
- [ ] `cargo run -p eval-harness -- --input data/pipeline/curated/sessions_latest.json --output-dir data/eval/latest --replay cargo --replay run --replay --offline --replay -p --replay deepresearch-cli --replay query --replay --format --replay json --max-verdict-delta 0 --max-math-delta 5 --max-manual-delta 5 --bootstrap-samples 2000 --bootstrap-alpha 0.05 --batch-size 200 --delta-sample-limit 250 --shard-count 1 --shard-index 0` — `docs/evaluation/M13_EVALUATION.md:36`

## Direct deepresearch binary

- [ ] `deepresearch explain --claim <ID>` — `PRD.md:63`
- [ ] `deepresearch explain --claim C1` — `README.md:47`
- [ ] `deepresearch explain --last` — `README.md:46`
- [ ] `deepresearch purge --session <ID>` — `PRD.md:193`
- [ ] `deepresearch query --session acme-q4 --sources web,local --depth detailed --explain "Compare Q4 revenue growth of top battery manufacturers"` — `README.md:39`
- [ ] `deepresearch trace --prov --out prov.json` — `README.md:48`

## API & Health Checks

- [ ] `cat <<'DOCS' | curl -s http://localhost:8080/ingest -H 'content-type: application/json' -d @-` — `docs/getting-started/USAGE.md:223`
- [ ] `curl -N :8080/api/sessions/<id>/stream` — `docs/getting-started/GUI_DEPLOYMENT.md:44`
- [ ] `curl -N http://localhost:8080/api/stream/<SESSION_ID>` — `docs/operations/TROUBLESHOOTING.md:214`
- [ ] `curl -X DELETE http://localhost:6333/collections/deepresearch` — `docs/getting-started/USAGE.md:182`
- [ ] `curl -X PUT http://localhost:6333/collections/deepresearch -H 'Content-Type: application/json' -d '{` — `docs/operations/TROUBLESHOOTING.md:313`
- [ ] `curl -XPOST :8080/api/sessions -H 'content-type: application/json' -H 'authorization: Bearer <token>' -d '{"query":"What is the roadmap impact?"}'` — `docs/getting-started/GUI_DEPLOYMENT.md:43`
- [ ] `curl -i http://localhost:8080/health/ready` — `docs/getting-started/GUI_ACCEPTANCE.md:11`
- [ ] `curl -s "http://localhost:8080/session/<SESSION>?explain=true&include_summary=true"` — `docs/getting-started/TESTING_GUIDE.md:70`
- [ ] `curl -s "http://localhost:8080/session/<SESSION_ID>?explain=true&explain_format=graphviz&include_summary=true"` — `docs/getting-started/USAGE.md:220`
- [ ] `curl -s http://127.0.0.1:8080/query -H 'content-type: application/json' -d '{` — `README.md:54`
- [ ] `curl -s http://localhost:8080/health` — `docs/getting-started/USAGE.md:213`
- [ ] `curl -s http://localhost:8080/health | jq` — `docs/release/RELEASE_CHECKLIST.md:17`; `docs/getting-started/TESTING_GUIDE.md:82`
- [ ] `curl -s http://localhost:8080/health | jq .` — `docs/release/CI_GUIDE.md:28`
- [ ] `curl -s http://localhost:8080/health/live | jq` — `docs/getting-started/GUI_ACCEPTANCE.md:10`
- [ ] `curl -s http://localhost:8080/query -H 'content-type: application/json' -d '{"query":"API integration", "explain":true}' | jq` — `docs/getting-started/TESTING_GUIDE.md:83`
- [ ] `curl -s http://localhost:8080/query -H 'content-type: application/json' -d '{"query":"Release API check","explain":true}' | jq` — `docs/release/RELEASE_CHECKLIST.md:18`
- [ ] `curl http://localhost:6333/health` — `docs/getting-started/TESTING_GUIDE.md:110`; `docs/operations/TROUBLESHOOTING.md:163`; `docs/operations/TROUBLESHOOTING.md:298`
- [ ] `curl http://localhost:6333/health # → {"status":"ok"}` — `docs/getting-started/USAGE.md:29`
- [ ] `curl http://localhost:8080/query` — `docs/operations/TROUBLESHOOTING.md:173`

## Docker Compose Stack

- [ ] `docker compose` — `docs/operations/OPERATIONS.md:10`
- [ ] `docker compose -f docker-compose.yml -f docker-compose.overrides/docker-compose.sandbox.yml run --rm cli-runner bash` — `docker-compose.overrides/README.md:20`
- [ ] `docker compose -f docker-compose.yml -f docker-compose.overrides/docker-compose.sandbox.yml up -d` — `docker-compose.overrides/README.md:17`; `docs/operations/RUNBOOK_SANDBOX_OPERATIONS.md:33`
- [ ] `docker compose down` — `docs/operations/OPERATIONS.md:333`
- [ ] `docker compose down # stop and remove containers` — `docs/operations/OPERATIONS.md:54`
- [ ] `docker compose logs -f qdrant` — `docs/operations/OPERATIONS.md:334`
- [ ] `docker compose logs -f qdrant # tail Qdrant logs` — `docs/operations/OPERATIONS.md:53`
- [ ] `docker compose ps` — `docs/getting-started/USAGE.md:181`; `docs/getting-started/USAGE.md:26`
- [ ] `docker compose up -d` — `docs/operations/OPERATIONS.md:332`
- [ ] `docker compose up -d # start services` — `docs/operations/OPERATIONS.md:52`
- [ ] `docker-compose down` — `docs/getting-started/TESTING_GUIDE.md:113`; `docs/getting-started/USAGE.md:244`; `docs/getting-started/USAGE.md:38`
- [ ] `docker-compose up -d` — `docs/release/RELEASE_CHECKLIST.md:43`; `docs/getting-started/TESTING_GUIDE.md:109`; `docs/getting-started/TESTING_GUIDE.md:52`; `docs/getting-started/TESTING_GUIDE.md:58`; `docs/operations/TROUBLESHOOTING.md:125`; `docs/operations/TROUBLESHOOTING.md:294`; `docs/getting-started/USAGE.md:23`
- [ ] `docker-compose up -d # Qdrant + Postgres` — `CONTRIBUTING.md:30`

## Docker & Sandbox

- [ ] `DEEPRESEARCH_SANDBOX_TESTS=1 docker build -t deepresearch-python-sandbox:latest -f containers/python-sandbox/Dockerfile .` — `docs/getting-started/TESTING_GUIDE.md:28`
- [ ] `docker build` — `docs/operations/OPERATIONS.md:319`
- [ ] `docker build --network=host -t deepresearch-python-sandbox:latest -f containers/python-sandbox/Dockerfile .` — `docs/operations/TROUBLESHOOTING.md:103`
- [ ] `docker build -f crates/deepresearch-gui/Dockerfile -t <registry>/deepresearch-gui:<tag> .` — `docs/getting-started/GUI_DEPLOYMENT.md:7`
- [ ] `docker build -t deepresearch-python-sandbox:ci -f containers/python-sandbox/Dockerfile .` — `docs/release/CI_GUIDE.md:30`
- [ ] `docker build -t deepresearch-python-sandbox:latest -f containers/python-sandbox/Dockerfile .` — `AGENTS.md:118`; `docs/operations/OPERATIONS.md:20`; `docs/operations/OPERATIONS.md:330`; `docs/operations/OPERATIONS.md:36`; `docs/operations/RUNBOOK_SANDBOX_OPERATIONS.md:7`; `docs/getting-started/TESTING_GUIDE.md:131`; `docs/getting-started/TESTING_GUIDE.md:99`; `docs/operations/TROUBLESHOOTING.md:64`
- [ ] `docker exec -it <pg-container> psql -U deepresearch -d deepresearch -c "SELECT 1;"` — `docs/getting-started/TESTING_GUIDE.md:112`
- [ ] `docker image rm deepresearch-python-sandbox:<tag>` — `docs/operations/OPERATIONS.md:310`
- [ ] `docker image rm deepresearch-python-sandbox:latest` — `docs/operations/OPERATIONS.md:335`
- [ ] `docker images | grep deepresearch` — `docs/operations/OPERATIONS.md:309`
- [ ] `docker logs deepresearch-rs-qdrant-1` — `docs/getting-started/USAGE.md:181`
- [ ] `docker ps | grep qdrant` — `docs/operations/TROUBLESHOOTING.md:295`
- [ ] `docker push <registry>/deepresearch-gui:<tag>` — `docs/getting-started/GUI_DEPLOYMENT.md:9`
- [ ] `docker push registry.example.com/deepresearch/python-sandbox:<tag>` — `docs/operations/RUNBOOK_SANDBOX_OPERATIONS.md:12`
- [ ] `docker run --memory=4g deepresearch-python-sandbox:latest` — `docs/operations/TROUBLESHOOTING.md:254`
- [ ] `docker run --rm -p 8080:8080 <image>` — `docs/getting-started/GUI_DEPLOYMENT.md:8`
- [ ] `docker run -d --name prometheus -p 9090:9090 -v $(pwd)/ops/prometheus.yml:/etc/prometheus/prometheus.yml:ro prom/prometheus:latest` — `docs/operations/OPERATIONS.md:229`
- [ ] `docker run -p 6333:6333 -p 6334:6334 -e QDRANT__SERVICE__GRPC_PORT=6334 qdrant/qdrant:latest` — `docs/getting-started/USAGE.md:36`
- [ ] `docker system prune` — `docs/operations/OPERATIONS.md:311`
- [ ] `docker tag deepresearch-python-sandbox:latest registry.example.com/deepresearch/python-sandbox:<tag>` — `docs/operations/RUNBOOK_SANDBOX_OPERATIONS.md:11`

## Cleanup Commands

- [ ] `rm -rf .fastembed_cache` — `docs/getting-started/USAGE.md:250`
- [ ] `rm -rf data/qdrant` — `docs/getting-started/USAGE.md:182`
- [ ] `rm -rf data/qdrant data/postgres` — `docs/getting-started/USAGE.md:247`
- [ ] `rm -rf ~/.cargo/registry/index/*` — `docs/operations/TROUBLESHOOTING.md:87`

## Diagnostics

- [ ] `lsof` — `docs/getting-started/USAGE.md:15`
- [ ] `lsof -i :6334` — `docs/getting-started/TESTING_GUIDE.md:111`; `docs/operations/TROUBLESHOOTING.md:299`; `docs/getting-started/USAGE.md:179`
- [ ] `lsof -i :6334 # expect docker-proxy or qdrant` — `docs/getting-started/USAGE.md:32`
