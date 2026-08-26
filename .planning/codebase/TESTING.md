# Testing Patterns

**Analysis Date:** 2026-08-26

## Test Framework

**Runner:**
- Rust's built-in `cargo test`/`libtest` runner from the stable 1.97 toolchain declared in `rust-toolchain.toml`.
- Rust configuration: package and dev dependencies in `Cargo.toml`; no dedicated test-runner configuration file is present.
- Go's standard `testing` package under Go module version 1.22 in `golang/go.mod`.
- Go configuration: `golang/go.mod`; no third-party assertion or test-runner dependencies are present.

**Assertion Library:**
- Use Rust standard macros (`assert!`, `assert_eq!`) throughout `tests/simulation.rs` and `tests/arena.rs`.
- Use `approx` 0.5.1 from `Cargo.toml` for floating-point assertions, as demonstrated by `assert_abs_diff_eq!` in `tests/simulation.rs`.
- Use `testing.T` checks and `t.Fatal`/`t.Fatalf` in `golang/main_test.go`.

**Run Commands:**
```bash
cargo test                         # Run the Rust library and integration tests documented in README.md
cargo test --all-targets           # Run all Rust tests and compile test harnesses for examples
cargo test monte_carlo_is_reproducible_across_worker_counts  # Run one Rust test by name
cd golang && go test ./...         # Run all Go tests in the standalone module
cd golang && go test -run TestCreateDeckUsesDeckSize ./...    # Run one Go test by name
```

**Quality Gate Commands:**
```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets
cd golang && test -z "$(gofmt -d .)" && go test ./...
```
- The Rust quality-gate commands are recorded in `docs/interactive-simulation-plan.md`; the basic test command is also documented in `README.md`.
- No watch-mode command or watcher dependency is configured in `Cargo.toml` or `golang/go.mod`.

## Test File Organization

**Location:**
- Rust behavior tests are separate integration-test crates under `tests/`: single-deck behavior in `tests/simulation.rs` and competitive arena behavior in `tests/arena.rs`.
- No `#[cfg(test)]` modules or `#[test]` functions are embedded under `src/`; tests intentionally exercise the public surface re-exported by `src/lib.rs` and `src/arena/mod.rs`.
- Go tests are co-located with the standalone command in `golang/main_test.go` and use `package main`, allowing direct access to unexported helpers in `golang/main.go`.

**Naming:**
- Name Rust test files after the feature area (`tests/simulation.rs`, `tests/arena.rs`).
- Name Rust test functions in behavior-oriented `snake_case`, such as `misses_are_not_reported_as_slow_wins` in `tests/simulation.rs` and `duplicate_competitor_ids_are_rejected` in `tests/arena.rs`.
- Name Go tests `Test<Function><Behavior>`, as in `TestRunScenarioDeterministicWithSeed` and `TestValidateConfigRejectsRequiredZero` in `golang/main_test.go`.

**Structure:**
```text
tests/
├── arena.rs          # Public-API integration tests for schedules, models, reports, errors, and replay
└── simulation.rs     # Public-API integration tests for cards, decks, mulligans, and Monte Carlo

golang/
├── main.go           # Standalone Go implementation
└── main_test.go      # Co-located same-package unit tests
```

## Test Structure

**Suite Organization:**
```rust
#[test]
fn duplicate_competitor_ids_are_rejected() {
    let deck = inert_deck();
    let win = KOfTag::new("missing", 1);
    let params = Params::new(&deck, &win);
    let competitors = vec![
        Competitor::new("same", params),
        Competitor::new("same", params),
    ];

    let error = ArenaMonteCarlo::new(2)
        .run(&competitors, &RoundRobin, &StartingPlayerWins)
        .unwrap_err();

    assert_eq!(error, ArenaError::DuplicateCompetitorId("same".into()));
}
```
- This arrange/act/assert pattern is used in `tests/arena.rs`: build small domain fixtures, invoke one public operation, and assert the complete result or error.

**Patterns:**
- Construct minimal decks and policies inline so the causal input is visible next to each assertion in `tests/simulation.rs`.
- Use fixed seeds for every stochastic regression that expects exact equality in `tests/simulation.rs`, `tests/arena.rs`, and `golang/main_test.go`.
- Compare full aggregates/reports when determinism is the contract, as in `monte_carlo_is_reproducible_across_worker_counts` and `results_and_replay_are_reproducible_across_worker_counts`.
- Assert both aggregate totals and per-seat details for arena behavior in `tests/arena.rs`; this catches bookkeeping errors that a single win-rate assertion would miss.
- Exercise empty and boundary behavior explicitly (`zero_trials_produces_empty_records_and_standings_entries`, `keeping_at_zero_mulligans_bottoms_nothing`) in `tests/arena.rs` and `tests/simulation.rs`.
- There is no shared setup/teardown lifecycle; Rust fixtures are plain helper functions in `tests/arena.rs`, and Go tests create local `Config` values in `golang/main_test.go`.

## Mocking

**Framework:** No mocking library is configured in `Cargo.toml` or `golang/go.mod`. Use small hand-written Rust test doubles that implement public traits.

**Patterns:**
```rust
struct StartingPlayerWins;

impl MatchSimulator for StartingPlayerWins {
    fn simulate(
        &self,
        _competitors: &[Competitor<'_>],
        _matchup: &Matchup,
        context: TrialContext,
    ) -> MatchOutcome {
        MatchOutcome::winner(context.starting_seat, 3, OutcomeReason::TurnOrderTieBreak)
    }
}
```
- `StartingPlayerWins` in `tests/arena.rs` isolates runner scheduling and aggregation from the real goldfish model.
- `MullOnce` and `BottomTagged` in `tests/simulation.rs` implement `MulliganPolicy` and `BottomHeuristic` to force precise mulligan paths.
- `MullOnce` uses `AtomicUsize` in `tests/simulation.rs` because the production trait requires `Send + Sync`; test doubles must honor the same concurrency contract.

**What to Mock:**
- Replace strategy boundaries when the test targets orchestration: `MatchSimulator`, `MulliganPolicy`, or `BottomHeuristic` from `src/arena/model.rs` and `src/mulligan.rs`.
- Make test doubles deterministic and minimal, returning a fixed outcome or selecting a known tag, following `tests/arena.rs` and `tests/simulation.rs`.

**What NOT to Mock:**
- Do not mock `Deck`, `Card`, win conditions, schedule validation, aggregation, or seed derivation; instantiate the real public types as the integration tests do in `tests/simulation.rs` and `tests/arena.rs`.
- Do not mock RNG APIs. Supply fixed seeds through `Params::with_seed`, `MonteCarloParams::with_seed`, or `ArenaMonteCarlo::with_seed` and compare reproducible outputs, as in both Rust test files.
- Do not replace Go worker or RNG behavior; `TestRunScenarioDeterministicWithSeed` in `golang/main_test.go` validates the real concurrent path twice with the same configuration.

## Fixtures and Factories

**Test Data:**
```rust
fn inert_deck() -> Deck {
    Deck::new(vec![Card::new("Filler"); 20])
}

let mut frequent_cards = vec![Card::new("Threat").with_tag("win"); 5];
frequent_cards.extend(vec![Card::new("Filler"); 35]);
let frequent_deck = Deck::new(frequent_cards);
```

**Location:**
- Keep a reusable fixture local to the feature test file when it is broadly useful there, as with `inert_deck` in `tests/arena.rs`.
- Keep scenario-specific decks inline in the test that owns their meaning, as in `tests/simulation.rs` and the reproducibility test in `tests/arena.rs`.
- Keep test-only simulators and policies beside the tests that use them in `tests/arena.rs` and `tests/simulation.rs`; there is no global fixture directory.
- In Go, build explicit `Config` literals and a locally seeded `rand.Rand` in `golang/main_test.go`.

## Coverage

**Requirements:** No coverage threshold, CI coverage job, `cargo-tarpaulin`, `llvm-cov`, or Go coverage configuration is present in `Cargo.toml`, `golang/go.mod`, or the repository root.

**View Coverage:**
```bash
# Not configured by the repository.
# Rust coverage requires adding an approved coverage tool before a canonical command exists.
cd golang && go test -cover ./...  # Standard Go coverage is available without extra dependencies
```
- The Rust suite contains 13 integration tests across `tests/simulation.rs` and `tests/arena.rs`.
- The Go suite contains 3 tests in `golang/main_test.go`.
- The test suite verifies major public flows but does not expose or enforce a line/branch percentage in repository configuration.

## Test Types

**Unit Tests:**
- Rust source-local unit tests are not used under `src/`; the smallest tests in `tests/simulation.rs` still compile as external integration tests and exercise public types.
- Go same-package unit tests cover deterministic scenario execution, configuration rejection, and deck composition in `golang/main_test.go`.

**Integration Tests:**
- `tests/simulation.rs` covers public composition of win conditions, non-destructive deck drawing, London mulligan behavior, miss semantics, and worker-count reproducibility.
- `tests/arena.rs` covers round-robin scheduling, paired trials, race resolution, tie policies, replay, validation errors, empty runs, and deterministic parallel aggregation.
- `cargo test --all-targets` also builds test harnesses for `examples/two_card_combo.rs` and `examples/round_robin.rs`, but those files contain no test functions.

**E2E Tests:**
- Not used. No subprocess tests exercise the printed output or command-line flags of `golang/main.go`, `examples/two_card_combo.rs`, or `examples/round_robin.rs`.
- No browser, network, or external-service test framework is configured in `Cargo.toml` or `golang/go.mod`.

## Common Patterns

**Async Testing:**
```rust
let single = monte_carlo(
    MonteCarloParams::new(params, 5_000)
        .with_seed(42)
        .with_workers(1),
);
let parallel = monte_carlo(
    MonteCarloParams::new(params, 5_000)
        .with_seed(42)
        .with_workers(4),
);
assert_eq!(single, parallel);
```
- No async runtime is used. Test concurrency by comparing seeded single-worker and multi-worker results, following `tests/simulation.rs` and `tests/arena.rs`.
- Go concurrency is exercised through the real worker pool in `runScenario`; repeat a fixed-seed run and compare the full `Results` value as in `golang/main_test.go`.

**Error Testing:**
```rust
let error = ArenaMonteCarlo::new(2)
    .run(&competitors, &RoundRobin, &StartingPlayerWins)
    .unwrap_err();
assert_eq!(error, ArenaError::DuplicateCompetitorId("same".into()));
```
- Assert the exact typed Rust error variant and payload, as in `tests/arena.rs`.

```go
if err := validateConfig(cfg); err == nil {
	t.Fatal("expected required=0 to be rejected")
}
```
- For Go validation, assert that invalid inputs return an error in `golang/main_test.go`; add message assertions only when message text is part of the CLI contract.

---

*Testing analysis: 2026-08-26*
