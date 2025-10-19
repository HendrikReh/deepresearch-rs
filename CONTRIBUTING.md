# Contributing to DeepResearch

Thanks for your interest in improving DeepResearch! This document explains how to set up a development environment, follow the coding standards, and submit high-quality contributions.

## 1. Prerequisites

- **Rust**: latest stable toolchain via [`rustup`](https://rustup.rs)
- **Node.js**: v20+ (CI uses 23.x) for building the GUI bundle
- **Docker** (optional): for running Qdrant/Postgres locally
- Access to the required LLM provider (e.g., `OPENAI_API_KEY`)

## 2. Environment Setup

1. Clone the repository:
   ```bash
   git clone https://github.com/your-org/deepresearch-rs.git
   cd deepresearch-rs
   ```
2. Install Rust dependencies:
   ```bash
   cargo fetch
   ```
3. (GUI) Install Node dependencies:
   ```bash
   npm install --prefix crates/deepresearch-gui/web
   npm ci --prefix crates/deepresearch-gui/web
   ```
4. Optional services:
   ```bash
   docker-compose up -d    # Qdrant + Postgres
   ```

## 3. Development Workflow

- Create a feature branch from `v0.2.x` (or the active release branch).
- Prefer small, focused commits with meaningful messages.
- Keep documentation (`README.md`, `docs/*.md`) aligned with your changes.

### Formatting & Linting
```bash
cargo fmt
cargo clippy --workspace --all-targets -- -D warnings
```

### Tests & Benchmarks
```bash
cargo test --workspace --all-targets -- --nocapture
cargo test --offline -p deepresearch-core finalize_summary_snapshot -- --nocapture
RUST_LOG=warn cargo run --offline -p deepresearch-cli bench "Dev bench" --sessions 8 --concurrency 4 --format json
```

### GUI Build (if applicable)
```bash
npm ci --prefix crates/deepresearch-gui/web
npm run build --prefix crates/deepresearch-gui/web
GUI_ENABLE_GUI=true cargo run -p deepresearch-gui
```

## 4. Pull Request Checklist

- [ ] Tests pass locally (`cargo test`, GUI build succeeds if modified).
- [ ] Clippy warnings resolved (`cargo clippy -- -D warnings`).
- [ ] Formatting applied (`cargo fmt`).
- [ ] Documentation updated (README, CI/Testing guides, changelog if necessary).
- [ ] No unrelated files or generated artefacts committed (check `git status`).

## 5. Code Style & Guidelines

- Follow idiomatic Rust patterns; prefer `?` for error propagation.
- Keep functions small and focused; add documentation comments for public APIs.
- Avoid panics in library code—bubble errors with `anyhow::Result` or custom types.
- Use feature flags (`postgres-session`, `qdrant-retriever`) to gate optional dependencies.

For frontend changes:
- Stick to the existing Tailwind design tokens; avoid inline styles when possible.
- Co-locate React components under `crates/deepresearch-gui/web/src/`; prefer functional components + hooks.

## 6. Filing Issues

If you encounter a bug or have a feature request:
1. Search existing issues to avoid duplicates.
2. Include reproduction steps, logs, or screenshots where relevant.
3. Label issues appropriately (bug, enhancement, docs, etc.).

## 7. Code of Conduct

We follow the [Contributor Covenant](https://www.contributor-covenant.org/). Please report unacceptable behaviour to the maintainers via the contact information listed in the repository.

---

Thank you for helping make DeepResearch better! 🙌
