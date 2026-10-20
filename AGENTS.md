# AGENTS.md

DéLOG is a GPU-accelerated drone flight-log and live-telemetry analyzer
(egui/wgpu desktop app) written in Rust, with optional embedded-Python
(pyo3) scripting. Rust workspace, edition 2024, toolchain pinned in
`rust-toolchain.toml` (1.96.0). All third-party versions are pinned once in
`[workspace.dependencies]` in the root `Cargo.toml`; member crates inherit
with `workspace = true`. `Cargo.lock` is committed; keep builds `--locked`.

## Build

```bash
cargo build --release --locked -p delog-app              # with Python scripting
cargo build --release --locked -p delog-app --no-default-features  # no scripting
```

Scripting builds link against the system CPython (3.12 expected) and require
NumPy; the `delog-script` crate's `python` feature is off by default so
`cargo build --workspace` needs no Python toolchain.

## Checks before integration (mirror of CI, .github/workflows/ci.yml)

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo clippy --workspace --all-targets --locked --no-default-features -- -D warnings
cargo test --workspace --locked --exclude delog-script -- --test-threads=1
cargo test -p delog-script --locked -- --test-threads=1
```

Always use `--test-threads=1` for Rust tests: parallel runs can crash on
systems whose Vulkan loader is unstable under concurrent headless GPU init.
Run the relevant package's tests during implementation; run the full matrix
before integration.

## Fuzzing

`fuzz/` is a detached cargo-fuzz workspace (needs nightly + clang):
`cargo +nightly fuzz run <target>` from `fuzz/`. Targets: `bin_record`,
`mavlink_framing`, `ulog_defs_data`.

## Crate ownership

Put changes in the owning crate (see docs/ARCHITECTURE_AND_STYLE.md for the
full contract):

| Crate | Owns |
| --- | --- |
| `delog-core` | IDs, canonical microsecond time, chunks/snapshots, ingest, diagnostics |
| `delog-parsers` | format sniffing, ULog / ArduPilot BIN / tlog decoding |
| `delog-stream` | live MAVLink links, frame reading, raw recording |
| `delog-flow` | dataflow graph model, commands, evaluation, publishing |
| `delog-api` | stable native catalog and app-control contracts |
| `delog-script` | embedded-Python execution, adapters to native contracts |
| `delog-cache` | f32 render caches (the One Copy), min/max pyramid, decimation |
| `delog-render` | wgpu pipelines, buffer manager, 2D/3D draw passes |
| `delog-parquet-format` | versioned structured Parquet schema codec |
| `delog-app` | egui UI, orchestration, persistence, user-visible behavior |

Dependencies point toward the smallest owner of a concept. Shared domain
types do not go in `delog-app`; shared egui widgets go in
`delog-app::ui::components`. Do not reroute existing diagnostics/control
paths when deduplicating.

## Style rules

- `cargo fmt` and Clippy (`-D warnings`) are mandatory; no warnings allowed.
- New third-party deps are added to `[workspace.dependencies]` only; member
  crates use `workspace = true`.
- Errors: typed causes via `thiserror` in the owning crate (no shared
  catch-all). Messages are lowercase, no final period. User-facing errors
  use `Display`, not `{:?}`. Before refactoring an existing error type,
  tests must pin its message, `Error::source`, conversions, and bounds.
- Poisoned mutexes are fatal; never silently recover poison.
- Domain IDs are newtypes; unit boundaries carry `_us`, `_ms`, `_rad`,
  `_deg`, `_m` suffixes. `delog-core::time` owns canonical time rules.
- Comments explain invariants and reasons, not the next line. `//!` for
  module contracts, `///` for public items, `// SAFETY:` before unsafe.
- UI: reuse `ui::icons`, `ui::components`, `ui::design_tokens` and the
  active theme; keep labels/error context at call sites.
- Hot paths (ingest, cache, render): bounded queues, short lock scopes, no
  new dynamic dispatch or allocations for deduplication. Algorithm changes
  need a before/after benchmark (criterion is available).

## Tests

Test names describe behavior. Short unit tests sit beside the code; larger
fixtures and public boundaries use a sibling `tests.rs` or integration test.
Assert real results and error paths, not source text. Route changes need
explicit route-contract tests. Python-side scripts in `scripts/` are user
examples, not the test suite.

## System dependencies (Linux)

eframe/wgpu and serialport need: `libxkbcommon-dev libwayland-dev
libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libgl1-mesa-dev
libudev-dev`. Scripting builds also need `libpython3-dev` (CPython 3.12)
plus `numpy scipy bottleneck cffi` (versions pinned in CI).

## Docker dev environment

The repo ships a ready-made dev container (`docker/dev.Dockerfile`,
`compose.yaml`): Debian + CPython 3.12.3, the toolchain pinned in
`rust-toolchain.toml`, the X11/GL system libs, and the CI-pinned Python
packages. The host GPU is passed through (`gpus: all`), and `target/` plus
the cargo registry persist in named volumes.

```bash
docker compose build
docker compose run --rm dev <command>        # e.g. cargo clippy --workspace
docker compose up -d                        # then: docker compose exec dev bash
```

Wrappers in `scripts/dev/` (build the image automatically if missing):

- `scripts/dev/in.sh <command...>` — run any command in the container
- `scripts/dev/check.sh` — the full pre-integration matrix below (fmt,
  both clippy feature sets, both test commands), green/red summary
- `scripts/dev/app.sh [--release]` — launch the GUI via X11 forwarding

Running the app GUI from the container uses X11 forwarding (DISPLAY and
/tmp/.X11-unix are mounted); on Wayland hosts run `xhost +local:` first
(app.sh does it best-effort). All commands below assume this container when
the host has no Rust toolchain. When tests initialize wgpu, run them with
`--test-threads=1`.
