# External Integrations

**Analysis Date:** 2026-08-26

## APIs & External Services

**Runtime network services:**
- Not detected - The Rust dependency set in `Cargo.toml` contains only `rand`, `rayon`, and test-only `approx`; runtime modules under `src/` import no HTTP, RPC, socket, cloud, or vendor SDK.
- Not detected - The Go module in `golang/go.mod` has no third-party requirements, and `golang/main.go` imports only local computation, concurrency, CLI, timing, and console packages from the Go standard library.
- Inputs are in-process Rust values in `src/engine.rs` and `src/arena/mod.rs`, compile-time deck data in `examples/`, or CLI flags in `golang/main.go`; no endpoint or remote API is required.

**Package registries (build time only):**
- crates.io index - Cargo resolves the third-party Rust packages declared in `Cargo.toml`; registry sources and exact versions are recorded in `Cargo.lock`.
  - SDK/Client: Cargo, configured through `Cargo.toml` and `Cargo.lock`.
  - Auth: none required by repository configuration; no registry credential file is part of the codebase described by `Cargo.toml`.
- Go module proxy/service - Not used by the current module because `golang/go.mod` contains only the module name and Go version and has no external requirements.
  - SDK/Client: Go module tooling for `golang/go.mod`.
  - Auth: not applicable to the dependency-free module in `golang/go.mod`.

**Agent tooling:**
- Claude Code-compatible repository skill - `.claude/skills/mindcrank-simulate/SKILL.md` directs an agent to copy `.claude/skills/mindcrank-simulate/templates/simulate.rs` into an ignored example and invoke Cargo locally.
  - SDK/Client: no SDK; the skill composes the public Rust API exported by `src/lib.rs`.
  - Auth: none; `.claude/skills/mindcrank-simulate/` contains instructions, reference material, and a local Rust template only.

**Operating system facilities:**
- Local CPU threads - Rayon pools in `src/engine.rs` and `src/arena/runner.rs`, plus goroutines in `golang/main.go`, use host CPU resources without a remote scheduler.
- Local randomness/time - Unseeded Rust runs select a master seed through `rand::random` in `src/engine.rs` and `src/arena/runner.rs`; unseeded Go runs use `time.Now().UnixNano()` in `golang/main.go`.

## Data Storage

**Databases:**
- Not detected - Simulation state is stored in `Vec`, `HashSet`, and `BTreeMap` values across `src/deck.rs`, `src/card.rs`, `src/metrics.rs`, and `src/arena/report.rs`; `Cargo.toml` declares no database client or ORM.
- Not detected - The Go simulator holds decks, jobs, and results in slices, structs, and channels in `golang/main.go`; `golang/go.mod` declares no database module.

**File Storage:**
- Local filesystem is used only for source/configuration and Cargo build outputs represented by `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, and the ignored `/target/` rule in `.gitignore`.
- Runtime simulation code performs no file reads or writes under `src/`, `examples/`, or `golang/`; Rust examples print aggregates in `examples/two_card_combo.rs` and `examples/round_robin.rs`, while the Go CLI prints results in `golang/main.go`.
- The agent workflow copies `.claude/skills/mindcrank-simulate/templates/simulate.rs` to the explicitly ignored `examples/sim_scratch.rs` path documented in `.claude/skills/mindcrank-simulate/SKILL.md` and `.gitignore`.

**Caching:**
- No application cache is present; trial results are accumulated in memory in `src/metrics.rs`, `src/arena/report.rs`, and `golang/main.go`.
- Cargo build artifacts may be cached locally under the ignored `/target/` directory configured by `.gitignore`; this is build tooling, not runtime application caching.

## Authentication & Identity

**Auth Provider:**
- None - The public API in `src/lib.rs`, Rust examples in `examples/`, and Go CLI in `golang/main.go` run without users, sessions, accounts, credentials, or authorization checks.
  - Implementation: not applicable; `Cargo.toml` and `golang/go.mod` contain no authentication or identity dependencies.

## Monitoring & Observability

**Error Tracking:**
- None - `Cargo.toml` and `golang/go.mod` declare no telemetry, crash-reporting, tracing, or hosted error-tracking client.

**Logs:**
- Rust examples use direct stdout output through `println!` in `examples/two_card_combo.rs` and `examples/round_robin.rs`; the library under `src/` does not emit structured logs or metrics to an external sink.
- The Go command uses `fmt` for status/results and `log.Fatalf` for fatal validation/runtime errors in `golang/main.go`; output remains local stdout/stderr.
- Simulation measurements are returned as typed `Aggregate` and arena report structures from `src/metrics.rs` and `src/arena/report.rs`; persistence and monitoring are consumer-owned rather than integrations in this repository.

## CI/CD & Deployment

**Hosting:**
- None - The repository defines a Rust library in `Cargo.toml`, local examples in `examples/`, and a local Go command in `golang/main.go`; no server entry point, container definition, infrastructure manifest, or hosted platform configuration is present.

**CI Pipeline:**
- None detected - Test commands are documented for local execution in `README.md`, `.claude/skills/mindcrank-simulate/SKILL.md`, and `golang/README.md`; no CI workflow configuration accompanies `Cargo.toml` or `golang/go.mod`.
- Current verification uses `cargo test --locked` for `tests/simulation.rs` and `tests/arena.rs`, and `go test ./...` for `golang/main_test.go`; both suites pass against the analyzed tree.

## Environment Configuration

**Required env vars:**
- None - Rust runtime parameters are fields/builders in `src/engine.rs`, and repository examples configure them in `examples/two_card_combo.rs` and `examples/round_robin.rs`.
- None - Go runtime parameters are command-line flags parsed in `golang/main.go`, not values loaded from the environment.
- No `.env` file is present in the repository, and the implementation under `src/` and `golang/` contains no environment-variable reads.

**Secrets location:**
- Not applicable - No secret-bearing integration is configured in `Cargo.toml`, `golang/go.mod`, `src/`, or `golang/`.
- Package resolution uses public registry metadata locked in `Cargo.lock`; repository configuration does not name or require a package-registry secret.

## Webhooks & Callbacks

**Incoming:**
- None - There is no HTTP listener or callback handler in `src/` or `golang/main.go`, and no server framework is declared in `Cargo.toml` or `golang/go.mod`.

**Outgoing:**
- None - The library returns in-memory results from `src/engine.rs` and `src/arena/runner.rs`; Rust examples and the Go CLI only write human-readable output in `examples/` and `golang/main.go`.
- Trait callbacks such as `WinCondition`, `MulliganPolicy`, and `BottomHeuristic` exported by `src/lib.rs` are local in-process extension points, not network callbacks or third-party webhooks.

---

*Integration audit: 2026-08-26*
