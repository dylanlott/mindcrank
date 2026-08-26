# Technology Stack

**Analysis Date:** 2026-08-26

## Languages

**Primary:**
- Rust, edition 2024 with minimum compiler 1.97 - Core library, arena engine, examples, and integration tests in `Cargo.toml`, `src/lib.rs`, `src/arena/mod.rs`, `examples/`, and `tests/`.
- Rust crate type `lib` - The default Cargo target is the reusable `mindcrank` library exported from `src/lib.rs`; runnable Rust programs are examples under `examples/` rather than a production server or default binary.

**Secondary:**
- Go 1.22 module language level - Standalone command-line Monte Carlo simulator and its tests in `golang/go.mod`, `golang/main.go`, and `golang/main_test.go`.
- Markdown - User, agent, and design documentation in `README.md`, `golang/README.md`, `docs/interactive-simulation-plan.md`, and `.claude/skills/mindcrank-simulate/`.

## Runtime

**Environment:**
- Native Rust executable/library code compiled with stable Rust; the repository sets `channel = "stable"` in `rust-toolchain.toml` and requires Rust 1.97 or newer in `Cargo.toml`.
- The analyzed development environment resolves the repository toolchain to `rustc 1.97.1` and `cargo 1.97.1`; the source-of-truth requirements remain `Cargo.toml` and `rust-toolchain.toml`.
- Native Go command compiled or run with Go 1.22-compatible tooling from `golang/go.mod`; the command entry point is `golang/main.go`.
- Both implementations are CPU-bound local processes: Rust uses Rayon worker pools in `src/engine.rs` and `src/arena/runner.rs`, while Go uses goroutines sized from `runtime.NumCPU()` in `golang/main.go`.

**Package Manager:**
- Cargo 1.97-compatible workflow for the Rust crate, configured by `Cargo.toml` and `rust-toolchain.toml`.
- Lockfile: present as `Cargo.lock` (lock format version 4); use locked dependency resolution for reproducible builds.
- Go modules at language level 1.22, configured by `golang/go.mod`.
- Go lock/checksum file: not applicable because `golang/go.mod` declares no third-party modules and the repository has no `golang/go.sum`.

## Frameworks

**Core:**
- No web, GUI, persistence, or application framework is used; `src/lib.rs` exposes a plain Rust library API composed from structs, traits, and functions.
- Rayon 1.12.0 - Data-parallel execution framework for independent Monte Carlo trials in `src/engine.rs` and arena matchups in `src/arena/runner.rs`.
- Go standard library - The companion CLI uses `flag`, `math/rand`, `runtime`, `sync`, and console packages directly in `golang/main.go`; `golang/go.mod` has no external dependencies.

**Testing:**
- Rust built-in test harness through Cargo - Integration suites live in `tests/simulation.rs` and `tests/arena.rs`; crate doctests are rooted at `src/lib.rs`.
- approx 0.5.1 - Floating-point assertions used by `tests/simulation.rs`, declared under `[dev-dependencies]` in `Cargo.toml`.
- Go built-in `testing` package - Unit tests live beside the Go command in `golang/main_test.go`.

**Build/Dev:**
- Cargo - Builds the library and runs examples defined by source layout in `src/lib.rs` and `examples/`; package metadata is in `Cargo.toml`.
- rustfmt and Clippy - Required stable-toolchain components in `rust-toolchain.toml`; use them for formatting and linting Rust changes.
- Go toolchain - Builds, runs, formats, and tests the self-contained module rooted at `golang/go.mod`.
- Release-mode Cargo examples - Performance-sensitive simulations are run with `cargo run --release --example ...` as documented in `README.md` and `.claude/skills/mindcrank-simulate/SKILL.md`.

## Key Dependencies

**Critical:**
- rand 0.10.2 - Supplies `StdRng`, seeding, shuffling, and fallback random master seeds in `src/deck.rs`, `src/engine.rs`, and `Cargo.toml`.
- rayon 1.12.0 - Supplies parallel iterators and optional bounded thread pools in `src/engine.rs`, `src/arena/runner.rs`, and `Cargo.toml`.
- Rust standard library collections - `HashSet` and `BTreeMap` implement tags, metrics, reports, and deterministic ordering in `src/card.rs`, `src/engine.rs`, `src/metrics.rs`, `src/arena/report.rs`, and `src/arena/runner.rs`.
- Go standard library only - Command parsing, deterministic PRNG streams, validation, concurrency, and output all remain within `golang/main.go`; no external package appears in `golang/go.mod`.

**Infrastructure:**
- getrandom 0.4.3 (transitive through rand) - Provides platform entropy behind unseeded Rust runs, locked in `Cargo.lock`; simulations switch to deterministic seeded streams in `src/engine.rs` once the master seed is selected.
- rayon-core 1.13.0 and crossbeam packages (transitive through Rayon) - Back the worker pools used by `src/engine.rs` and `src/arena/runner.rs`, with exact versions locked in `Cargo.lock`.
- No database driver, HTTP client/server, serialization framework, cloud SDK, authentication SDK, or telemetry package is declared in `Cargo.toml` or `golang/go.mod`.

## Configuration

**Environment:**
- Rust simulation settings are typed values, not environment variables: configure `Params` and `MonteCarloParams` from `src/engine.rs`, following runnable composition in `examples/two_card_combo.rs` and `examples/round_robin.rs`.
- Rust deck-simulation harness settings are compile-time constants in `.claude/skills/mindcrank-simulate/templates/simulate.rs`; the workflow copies the template to the ignored `examples/sim_scratch.rs` path listed in `.gitignore`.
- Go settings are command-line flags (`deck-size`, `lands`, `combos`, `required`, `runs`, and `seed`) parsed in `golang/main.go` and documented in `golang/README.md`.
- No `.env` file is present and neither `src/` nor `golang/` reads environment variables; runtime configuration requires no secret values in `Cargo.toml` or `golang/go.mod`.

**Build:**
- `Cargo.toml` defines package version 0.1.0, Rust edition 2024, minimum Rust 1.97, library dependencies, and test-only dependencies.
- `Cargo.lock` pins the complete Rust dependency graph; preserve it for application-style reproducibility even though `src/lib.rs` is a library target.
- `rust-toolchain.toml` selects stable Rust and installs `rustfmt` and `clippy` components.
- `golang/go.mod` defines the isolated `mindcrank` Go module at Go 1.22 with no dependency blocks.
- No feature flags, Cargo workspace members beyond the root package, build scripts, custom profiles, or target-specific dependencies are configured in `Cargo.toml`.

## Platform Requirements

**Development:**
- Install stable Rust 1.97 or newer with Cargo, rustfmt, and Clippy according to `Cargo.toml` and `rust-toolchain.toml`.
- Install Go 1.22 or newer to build and test `golang/go.mod` and `golang/main.go`.
- Run `cargo test --locked` from the repository root for `tests/` and `go test ./...` from `golang/`; both commands pass against the analyzed tree.
- Use a multicore CPU for throughput because `src/engine.rs`, `src/arena/runner.rs`, and `golang/main.go` all parallelize independent simulations; seeded runs remain reproducible across worker counts where asserted by `tests/simulation.rs`, `tests/arena.rs`, and `golang/main_test.go`.

**Production:**
- No hosted deployment target, container image, service process, or platform manifest exists; consumers embed the library exported by `src/lib.rs` or run local examples under `examples/`.
- The Go deliverable is a local CLI from `golang/main.go`, invoked with `go run .` or compiled with the standard Go toolchain as documented in `golang/README.md`.
- Optimized Rust workloads should use Cargo release mode as prescribed in `README.md` and `.claude/skills/mindcrank-simulate/SKILL.md`.

---

*Stack analysis: 2026-08-26*
