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

## 5. Image Hygiene

- List images: `docker images | grep deepresearch`
- Remove unused sandbox tags: `docker image rm deepresearch-python-sandbox:<tag>`
- Prune dangling layers after upgrades: `docker system prune`

---

## 6. Troubleshooting

| Issue | Symptoms | Remediation |
|-------|----------|-------------|
| Missing build context | `docker: 'docker buildx build' requires 1 argument` | Ensure the trailing `.` path is included in `docker build` commands. |
| Mermaid CLI failure | Sandbox test errors mentioning Chromium | Rebuild the image; ensure headless Chromium dependencies remain in the Dockerfile. |
| Permission errors on bind mount | Sandbox outputs missing / permission denied | Confirm Docker Desktop has access to the repo path, and the host user has write permissions. |
| Compose port conflicts | Services fail to start | Adjust ports in `docker-compose.yml` and update CLI/API environment variables. |

---

## 7. Reference Commands

| Purpose | Command |
|---------|---------|
| Build sandbox image | `docker build -t deepresearch-python-sandbox:latest -f containers/python-sandbox/Dockerfile .` |
| Run sandbox smoke | `DEEPRESEARCH_SANDBOX_TESTS=1 DEEPRESEARCH_SANDBOX_IMAGE=deepresearch-python-sandbox:latest cargo test -p deepresearch-core --test sandbox -- --ignored --nocapture` |
| Start retrieval stack | `docker compose up -d` |
| Stop retrieval stack | `docker compose down` |
| Tail Qdrant logs | `docker compose logs -f qdrant` |
| Remove sandbox image | `docker image rm deepresearch-python-sandbox:latest` |

Keep this playbook updated whenever new container workflows or automation hooks are introduced.
