# Sandbox Security Review Packet

## Overview
- **Image**: `deepresearch-python-sandbox` (Python 3.11 slim base)
- **Purpose**: Execute math/plotting workloads in an isolated container for DeepResearch agents.
- **Consumers**: CLI, API, GUI pipelines, optional automation.

## Hardening Summary
- Base image: `python:3.11-slim-bullseye` with security updates applied during build.
- Packages: Matplotlib, NetworkX, Pandas/Polars, Graphviz, Mermaid CLI + Chromium deps.
- Non-root `sandbox` user; no password.
- Container runtime flags (enforced by orchestrator / compose overrides):
  - `--cap-drop=ALL` + minimal `--cap-add` (CHOWN, SETUID, SETGID, FOWNER).
  - `--security-opt no-new-privileges`.
  - `--read-only` rootfs with tmpfs for `/tmp`, `/var/tmp`, `/run`.
  - `--network none`.
  - CPU/memory limits configurable (defaults 2 CPUs / 2 GiB).
- Volume access: Bind-mounted workspace only (`/workspace` or override stack). Logs collected through `/var/log/deepresearch` (tmpfs by default).

## Telemetry & Alerting
- Structured logs: `telemetry.sandbox` events capture status, duration, exit code, failure streak.
- Metrics: OTEL `Meter` integration exposes `sandbox_runs_total`, `sandbox_duration_ms`, `sandbox_alerts_total` (requires external meter provider).
- Prometheus integration sample provided (`ops/prometheus.yml` + alerts) covering `SandboxFailureBurst` >= 3 failures within 5 minutes.

## Operational Runbook
- Image build/publish process (`docs/operations/RUNBOOK_SANDBOX_OPERATIONS.md`).
- Docker Compose overrides for local + staging validation (`docker-compose.overrides/`).
- OTEL collector manifests (`ops/otel/collector.yaml`) + Prometheus alerts.

## Threat Model Checklist
- [x] Host escape mitigated via rootless user, dropped capabilities, readonly fs.
- [x] Network egress disabled by default (`--network none`).
- [x] Resource exhaustion bounded by `--cpus` / `--memory`. (Limits adjustable per env.)
- [x] Logging / metrics available for detection of anomalous behavior.
- [x] Pending: Security sign-off meeting (capture date / attendees).

## Review Agenda
1. Walkthrough of container build + dependencies.
2. Runtime enforcement (compose overrides / orchestrator code).
3. Monitoring & alerting pathway (telemetry, Prometheus alert).
4. Upgrade / patch cadence.
5. Outstanding risks / future work (e.g., inline signature verification, dependency SBOM).

## Next Actions
- Schedule review with Platform + Security (owner: [Hendrik Reh](hendrik.reh˛blacksmith-consulting.ai)).
- Collect test evidence: sandbox smoke + integration run logs, Prometheus alert screenshot.
- Update this document with meeting minutes + decision (approve/changes required).
