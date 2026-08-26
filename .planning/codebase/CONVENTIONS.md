# Coding Conventions

**Analysis Date:** 2026-08-26

## Naming Patterns

**Files:**
- Use lowercase `snake_case.rs` for Rust modules and integration tests, as in `src/win_condition.rs`, `src/arena/runner.rs`, and `tests/simulation.rs`.
- Keep a directory module's public surface in `mod.rs`, as in `src/arena/mod.rs`; place focused implementations beside it in `src/arena/model.rs`, `src/arena/report.rs`, `src/arena/runner.rs`, and `src/arena/schedule.rs`.
- Name Rust examples after the scenario in lowercase snake case, as in `examples/two_card_combo.rs` and `examples/round_robin.rs`.
- Use Go's conventional `_test.go` suffix for tests, as in `golang/main_test.go`; the standalone prototype remains a single `package main` in `golang/main.go`.

**Functions:**
- Use `snake_case` for Rust functions and methods (`run_once`, `monte_carlo`, `validate_competitors`) in `src/engine.rs` and `src/arena/mod.rs`.
- Use `new` for constructors and fluent `with_*` methods for optional configuration (`Params::new`, `with_seed`, `with_workers`, `with_tie_policy`) in `src/engine.rs`, `src/arena/runner.rs`, and `src/arena/model.rs`.
- Use domain verbs for trait behavior (`satisfied`, `keep`, `cards_to_bottom`, `matchups`, `simulate`) in `src/win_condition.rs`, `src/mulligan.rs`, and `src/arena/`.
- Use lower camel case for Go functions (`runScenario`, `createDeck`, `validateConfig`) and `TestXxx` for Go tests in `golang/main.go` and `golang/main_test.go`.

**Variables:**
- Use descriptive `snake_case` locals in Rust (`opening_lands`, `draws_per_turn`, `master_seed`) in `src/engine.rs`; reserve short names for mathematical formulas (`n`, `p`, `z`) in `src/arena/report.rs`.
- Encode units or roles in names where ambiguity is possible (`trials_per_matchup`, `competitor_indices`, `starting_seat`) in `src/arena/runner.rs` and `src/arena/mod.rs`.
- Use lower camel case in Go (`workerCount`, `openingWinCount`, `numComboPieces`) in `golang/main.go`.

**Types:**
- Use `UpperCamelCase` nouns for Rust structs and enums (`TrialOutcome`, `ArenaMonteCarlo`, `OutcomeReason`) in `src/metrics.rs` and `src/arena/`.
- Name behavioral Rust traits after the role they represent (`WinCondition`, `MulliganPolicy`, `Schedule`, `MatchSimulator`) in `src/win_condition.rs`, `src/mulligan.rs`, and `src/arena/`.
- Use tuple newtypes for stable identifiers (`MatchupId`, `TrialId`) and derive ordering/equality traits needed by deterministic collections in `src/arena/mod.rs`.
- Use `UpperCamelCase` for Go structs (`Config`, `Results`, `Simulation`) in `golang/main.go`; their unexported fields stay lower camel case.

## Code Style

**Formatting:**
- Format all Rust with the stable `rustfmt` component declared in `rust-toolchain.toml`; no `rustfmt.toml` is present, so use standard rustfmt defaults.
- Check Rust formatting with `cargo fmt --all -- --check`, the repository-wide command recorded in `docs/interactive-simulation-plan.md`.
- Format Go with `gofmt`; `golang/main.go` and `golang/main_test.go` follow tab-indented Go formatting, and no separate formatter configuration is present under `golang/`.
- Keep chained Rust calls one method per line when they exceed the formatter width, as in the Monte Carlo setup in `tests/simulation.rs` and arena runs in `tests/arena.rs`.

**Linting:**
- Use the stable Clippy component declared in `rust-toolchain.toml`; there is no repository-specific `clippy.toml` or crate-level lint override in `src/lib.rs`.
- Treat every Clippy warning as an error with `cargo clippy --all-targets --all-features -- -D warnings`, matching `docs/interactive-simulation-plan.md`.
- Rely on Go compiler checks plus `go test ./...` and `gofmt`; no `golangci-lint` configuration is present under `golang/`.
- Prefer iterator adapters and checked arithmetic where they express intent (`checked_mul`, `map`, `fold`, `reduce`) in `src/arena/runner.rs`.

## Import Organization

**Order:**
1. Put Rust standard-library imports first (`std::collections`, `std::fmt`, `std::sync`) as in `src/engine.rs`, `src/arena/mod.rs`, and `tests/simulation.rs`.
2. Add a blank line, then import external crates (`rand`, `rayon`, `approx`) as in `src/engine.rs`, `src/deck.rs`, and `tests/simulation.rs`.
3. Add a blank line, then import crate-local items with `crate::{...}` or sibling items with `super::{...}` as in `src/mulligan.rs` and `src/arena/runner.rs`.
4. In integration tests and examples, import the public `mindcrank` API after standard-library imports, as in `tests/arena.rs` and `examples/round_robin.rs`.
5. Keep Go standard-library imports in one gofmt-sorted block, as in `golang/main.go` and `golang/main_test.go`; there are no third-party Go dependencies in `golang/go.mod`.

**Path Aliases:**
- No custom path aliases are configured in `Cargo.toml` or `golang/go.mod`.
- Inside Rust modules, use `crate::` for crate-root dependencies and `super::` for arena sibling exports, following `src/deck.rs` and `src/arena/model.rs`.
- Consumers should import re-exported public types from `mindcrank` or `mindcrank::arena`, following `tests/simulation.rs` and `tests/arena.rs`, instead of reaching into private module paths.

## Error Handling

**Patterns:**
- Model fallible arena operations with `Result<T, ArenaError>` and a typed error enum in `src/arena/mod.rs`; add new arena validation failures as variants there.
- Implement `std::fmt::Display` and `std::error::Error` for public Rust error types, following `ArenaError` in `src/arena/mod.rs`.
- Convert low-level failures at the boundary with `map_err`, and create domain errors with `ok_or`, as in `src/arena/runner.rs`.
- Validate caller-supplied indices, IDs, trial counts, and model outputs before indexing or aggregating them in `src/arena/runner.rs` and `src/arena/mod.rs`.
- Represent expected absence with `Option` (`turns_to_win`, confidence intervals, example trials) in `src/metrics.rs` and `src/arena/report.rs`; do not use sentinel numeric values.
- Keep the infallible single-deck API defensive by normalizing bounds (`max`, `min`, `saturating_sub`) in `src/engine.rs` and `src/deck.rs`.
- Reserve `expect` for an internal invariant with an actionable message; the only library example is thread-pool construction in `src/engine.rs`. Arena pool construction is instead propagated as `ArenaError::WorkerPool` from `src/arena/runner.rs`.
- In the Go prototype, return `error` from validation and scenario functions in `golang/main.go`; terminate only at the CLI boundary in `main` with `log.Fatalf`.

## Logging

**Framework:** No library logging framework is configured in `Cargo.toml`; the Rust library under `src/` emits no logs.

**Patterns:**
- Keep reusable Rust library code silent and return structured values or errors from `src/engine.rs` and `src/arena/runner.rs`.
- Print user-facing results only in executable examples with `println!`, as in `examples/two_card_combo.rs` and `examples/round_robin.rs`.
- In the Go CLI, use `fmt.Printf` for normal output and `log.Fatalf` for unrecoverable top-level errors in `golang/main.go`; do not log from simulation helpers.

## Comments

**When to Comment:**
- Use comments to explain domain constraints and non-obvious reproducibility choices, such as London mulligan semantics in `src/engine.rs`, paired sampling in `src/arena/runner.rs`, and stable hash non-security intent in `src/arena/mod.rs`.
- Explain why a fallback or policy exists, not each mechanical statement; `normalized_bottom_indices` in `src/engine.rs` documents why invalid custom heuristic output is completed.
- Keep short field comments next to metrics whose semantics are easy to misread, such as winning-trial-only averages in `src/metrics.rs` and play/draw records in `src/arena/report.rs`.

**JSDoc/TSDoc:**
- Not applicable. Use Rustdoc `//!` for crate/module overviews in `src/lib.rs` and `src/arena/mod.rs`.
- Use Rustdoc `///` on public traits, structs, methods, and semantically significant fields throughout `src/engine.rs`, `src/mulligan.rs`, and `src/arena/`.
- Use conventional Go doc comments above named types and helpers in `golang/main.go`; comments for exported Go identifiers should begin with the identifier name.

## Function Design

**Size:** Keep helpers focused on one transformation or validation step, as with `partition_hand` in `src/engine.rs`, `execute_trial` in `src/arena/runner.rs`, and `wilson_interval` in `src/arena/report.rs`. Extract seed derivation, validation, and aggregation rather than embedding them into entry points.

**Parameters:**
- Borrow Rust inputs with slices and references (`&[Card]`, `&Deck`, `&dyn Schedule`) in `src/deck.rs`, `src/engine.rs`, and `src/arena/runner.rs`.
- Use explicit lifetimes only where configuration stores borrowed trait objects, as in `Params<'a>` in `src/engine.rs` and `Competitor<'a>` in `src/arena/mod.rs`.
- Accept flexible owned text and collections at construction boundaries with `impl Into<String>` and `IntoIterator`, following `Card` in `src/card.rs` and `Deck::put_on_bottom` in `src/deck.rs`.
- Group many related simulation inputs into configuration structs (`Params`, `MonteCarloParams`, `Config`) in `src/engine.rs` and `golang/main.go`.

**Return Values:**
- Return domain records from computation (`TrialOutcome`, `Aggregate`, `ArenaReport`) in `src/engine.rs`, `src/metrics.rs`, and `src/arena/runner.rs`.
- Return `Result` for invalid external configuration or runtime setup in `src/arena/runner.rs` and `golang/main.go`.
- Return `Option` for genuinely missing measurements rather than defaulting them, as in `Aggregate` in `src/metrics.rs` and `MatchupReport` in `src/arena/report.rs`.

## Module Design

**Exports:**
- Keep implementation modules private and curate the crate API with `pub use` in `src/lib.rs`; arena consumers receive a second curated surface from `src/arena/mod.rs`.
- Use `pub(crate)` for helpers shared only within the crate, such as `derive_seed` in `src/arena/mod.rs` and `MatchupAccumulator` in `src/arena/report.rs`.
- Keep state private when callers should use behavior (`Deck.cards` in `src/deck.rs`), while public configuration/result structs expose fields for ergonomic setup and inspection in `src/engine.rs` and `src/metrics.rs`.
- Use small `Send + Sync` traits as extension points (`WinCondition`, `BottomHeuristic`, `MatchSimulator`, `Schedule`) in `src/win_condition.rs`, `src/mulligan.rs`, and `src/arena/`.

**Barrel Files:**
- `src/lib.rs` is the crate-level public barrel and `src/arena/mod.rs` is the arena-level barrel; add new public types to the appropriate explicit `pub use` list.
- No wildcard re-exports are used in `src/lib.rs` or `src/arena/mod.rs`; preserve explicit API curation.

---

*Convention analysis: 2026-08-26*
