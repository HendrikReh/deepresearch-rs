# Sandbox Operations Runbook

## Image Lifecycle

1. Build locally before promotion:
   ```bash
   docker build -t deepresearch-python-sandbox:latest -f containers/python-sandbox/Dockerfile .
   ```
2. Tag + push to registry:
   ```bash
   docker tag deepresearch-python-sandbox:latest registry.example.com/deepresearch/python-sandbox:<tag>
   docker push registry.example.com/deepresearch/python-sandbox:<tag>
   ```
3. Update deployment manifests / Helm values to reference the new tag.
4. Smoke test via `DEEPRESEARCH_SANDBOX_TESTS=1 cargo test -p deepresearch-core --test sandbox -- --ignored`.

## Security Patching

- Monitor upstream Python base image CVEs.
- Rebuild image after `apt-get` or dependency updates.
- Verify collector override stack uses latest `otel/opentelemetry-collector-contrib`.

## Emergency Response

- If sandbox failures spike:
  1. Check `sandbox_logs_total` (Prometheus metric) for status breakdown.
  2. Inspect `/var/log/deepresearch` for problematic scripts.
  3. Scale down sandbox usage via feature flag if necessary.
- If container fails to start:
  - Ensure Docker daemon accessible; check volume permissions.
  - Rebuild image with `docker build --no-cache`.

## Upgrade Checklist

- [ ] run `cargo check`
- [ ] run sandbox smoke test
- [ ] rebuild sandbox image
- [ ] update override stack + redeploy
- [ ] update `docs/OPERATIONS.md` if new steps introduced

