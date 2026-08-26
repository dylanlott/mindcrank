# Codebase Concerns

**Analysis Date:** 2026-08-26

## Tech Debt

**Duplicated Rust and Go simulators:**
- Issue: The Rust library and the standalone Go program implement separate deck construction, shuffling, seeding, trial execution, and aggregation paths with different semantics and metrics.
- Files: `src/engine.rs`, `src/metrics.rs`, `golang/main.go`, `golang/main_test.go`
- Impact: A correctness or reproducibility fix must be made twice, and results from the Go executable are not directly comparable with Rust results because the Go path has no mulligans, horizon misses, tag model, or arena.
- Fix approach: Designate the Rust crate as the canonical engine and either remove the Go program, mark it explicitly as a reference prototype, or make the Go executable consume a shared specification and cross-language golden vectors.

**Unchecked public simulation parameters:**
- Issue: Callers mutate public fields on `Params`, `MonteCarloParams`, and `ArenaMonteCarlo`; there is no validated configuration type or fallible constructor for deck size, hand size, draw count, turn horizon, trial count, or worker count.
- Files: `src/engine.rs`, `src/arena/runner.rs`, `.claude/skills/mindcrank-simulate/reference.md`
- Impact: Invalid values are accepted, silently rewritten, or fail only during resource allocation, making caller mistakes hard to distinguish from intentional edge cases.
- Fix approach: Add `validate()`/`try_new()` APIs with a typed error enum, keep builder methods fallible where needed, and document explicit behavior for zero trials, empty decks, short decks, and zero draws.

**Fixed two-seat arena representation:**
- Issue: Matchups, records, examples, tie resolution, and accumulator arithmetic are hard-coded as two-element arrays with expressions such as `1 - seat`.
- Files: `src/arena/mod.rs`, `src/arena/model.rs`, `src/arena/report.rs`, `src/arena/runner.rs`, `docs/interactive-simulation-plan.md`
- Impact: The arena cannot represent Commander pods or any variable-seat contest without a cross-cutting public API refactor; every new report field risks duplicating the two-seat assumption.
- Fix approach: Follow the contest/seating generalization specified in `docs/interactive-simulation-plan.md`: validated dynamic seat collections, contest/sample/seating trial IDs, generic accumulator loops, and compatibility wrappers for two-player callers.

**Custom models cannot report failures:**
- Issue: `MatchSimulator::simulate` returns `MatchOutcome` directly, so illegal actions, exhausted safety limits, or model-specific failures can only panic or be disguised as a simulated result.
- Files: `src/arena/model.rs`, `src/arena/runner.rs`, `docs/interactive-simulation-plan.md`
- Impact: One bad custom model can terminate a parallel run or contaminate statistics; callers cannot distinguish simulation failure from a legitimate draw.
- Fix approach: Return `Result<MatchOutcome, SimulationError>`, attach the `TrialId`/resolved seed to errors, and make the parallel reducer stop without recording the failed trial.

**Copied harness is outside compilation and test targets:**
- Issue: The user-facing simulation workflow copies a substantial parser and reporter from a hidden template into a gitignored example; the source template is not built by `cargo test --all-targets`.
- Files: `.claude/skills/mindcrank-simulate/SKILL.md`, `.claude/skills/mindcrank-simulate/templates/simulate.rs`, `.gitignore`
- Impact: Parser and statistics regressions can ship while the repository's Rust test, Clippy, and formatting checks remain green.
- Fix approach: Move reusable parsing/reporting into a normal crate module or checked example, add integration tests with Arena/Moxfield-style fixtures, and generate the scratch harness as a thin caller.

## Known Bugs

**Odd arena trial counts are not play/draw balanced:**
- Symptoms: `ArenaMonteCarlo::new(2n + 1)` schedules one extra even-indexed trial with seat 0 starting, despite the runner and README describing paired, balanced trials.
- Files: `src/arena/runner.rs`, `README.md`, `tests/arena.rs`
- Trigger: Run any non-empty schedule with an odd `trials_per_matchup`, such as `ArenaMonteCarlo::new(1)`.
- Workaround: Supply an even trial count; a code fix should reject odd counts or redefine the input as sample count and always expand each sample into both seatings.

**Zero hand/draw settings are silently changed to one:**
- Symptoms: Setting `Params.hand_size = 0` still draws one opening card, and setting `Params.draws_per_turn = 0` still draws one card per turn.
- Files: `src/engine.rs`, `.claude/skills/mindcrank-simulate/reference.md`
- Trigger: Set either public field to zero and call `run_once` or `monte_carlo`.
- Workaround: Avoid zero values; a code fix should preserve a documented zero meaning or return a validation error instead of applying `.max(1)`.

**Core random seeds are discarded when callers omit a seed:**
- Symptoms: `run_once` and `monte_carlo` generate random seeds internally, but neither `TrialOutcome` nor `Aggregate` exposes the resolved seed, so a surprising run cannot be reproduced.
- Files: `src/engine.rs`, `src/metrics.rs`
- Trigger: Call `run_once` or `monte_carlo` with both seed fields set to `None`.
- Workaround: Always pass and log an explicit seed; a code fix should return a run report containing the resolved master seed, matching `ArenaReport` in `src/arena/report.rs`.

**Replay accepts trials that never belonged to the run:**
- Symptoms: `ArenaMonteCarlo::replay` uses `trial_id.trial_index` without checking it against `trials_per_matchup`, so it can synthesize a deterministic record for an out-of-range trial ID.
- Files: `src/arena/runner.rs`, `src/arena/mod.rs`, `tests/arena.rs`
- Trigger: Create a runner with `trials_per_matchup = 2` and replay a known matchup with `trial_index >= 2`.
- Workaround: Replay only IDs emitted by `ArenaReport`; a code fix should add an `UnknownTrial`/`InvalidTrialIndex` error and validate the bound.

**Custom schedules can pair a competitor with itself:**
- Symptoms: Matchup validation checks only bounds and duplicate matchup IDs; `[index, index]` is accepted as a two-player matchup and produces misleading records.
- Files: `src/arena/runner.rs`, `src/arena/mod.rs`, `src/arena/schedule.rs`
- Trigger: Implement `Schedule` and return a `Matchup` whose two `competitor_indices` are equal.
- Workaround: Custom schedules must enforce distinct seats themselves; a code fix should reject duplicate competitor indices within each matchup.

**Template reports misleading confidence at boundary rates:**
- Symptoms: The scratch harness uses an unbounded normal/Wald margin; zero wins prints `0.00% (+/- 0.00pp, 95% CI)`, and zero trials also produces `NaN` in the cumulative rate division.
- Files: `.claude/skills/mindcrank-simulate/templates/simulate.rs`, `src/arena/report.rs`
- Trigger: Use a condition with no wins, or set `TRIALS` to zero in the copied harness.
- Workaround: Keep `TRIALS > 0` and do not interpret the boundary interval literally; a code fix should share the Wilson interval implementation from `src/arena/report.rs` and handle an empty aggregate explicitly.

**Go scenario validation is not enforced by the scenario runner:**
- Symptoms: `runScenario` returns `error` but never calls `validateConfig`; a direct call with `deckSize < 7` reaches `deck[:7]` and panics instead of returning an error.
- Files: `golang/main.go`, `golang/main_test.go`
- Trigger: Call `runScenario` directly with an invalid `Config` instead of going through `main`.
- Workaround: Call `validateConfig` first; a code fix should validate at the `runScenario` boundary and reserve `main` for error presentation.

## Security Considerations

**Unbounded resource controls:**
- Risk: If deck size, trials, or worker count are exposed through a service or other untrusted front end, callers can request deep allocations, very long CPU runs, or an excessive Rayon thread pool, causing denial of service.
- Files: `src/deck.rs`, `src/engine.rs`, `src/arena/runner.rs`, `golang/main.go`, `.claude/skills/mindcrank-simulate/templates/simulate.rs`
- Current mitigation: Arena total-trial multiplication uses `checked_mul`, arena worker-pool construction returns `ArenaError`, and the Go CLI rejects non-positive run counts; there are no upper bounds or cancellation budgets.
- Recommendations: Enforce application-level maxima, estimate work before execution, support cancellation/time budgets, and return typed allocation/pool errors from both Rust runners.

**Library extensions execute unsandboxed caller code in parallel:**
- Risk: Implementations of `WinCondition`, `MulliganPolicy`, `BottomHeuristic`, `Schedule`, and `MatchSimulator` can block, panic, perform I/O, or mutate interior state; `Send + Sync` provides thread-safety typing but not isolation.
- Files: `src/win_condition.rs`, `src/mulligan.rs`, `src/arena/schedule.rs`, `src/arena/model.rs`
- Current mitigation: Rust type safety prevents ordinary data races, competitor/matchup outputs receive structural validation, and no `unsafe` blocks are present in `src/`.
- Recommendations: Treat extensions as trusted code, document that boundary explicitly, return errors from model hooks, and add host-level timeout/isolation if extensions ever become user-supplied plugins.

## Performance Bottlenecks

**Every trial deep-clones the full deck:**
- Problem: `Deck::shuffle` clones every `Card`; each card clone duplicates its `String`, optional type, and `HashSet<String>` tags before shuffling.
- Files: `src/deck.rs`, `src/card.rs`, `src/engine.rs`, `src/arena/model.rs`
- Cause: Simulation state owns `Card` values rather than shuffling compact indices/references into an immutable deck definition; each mulligan repeats the full clone.
- Improvement path: Store immutable card definitions once and shuffle `usize` indices or compact card IDs, then keep hands/libraries as index vectors. Benchmark one million 99-card trials before and after.

**Single-deck Monte Carlo retains every outcome:**
- Problem: `monte_carlo` collects a `Vec<TrialOutcome>` for all trials and only then builds `Aggregate`, making memory usage O(trials).
- Files: `src/engine.rs`, `src/metrics.rs`
- Cause: Aggregation is batch-oriented even though every metric can be merged incrementally.
- Improvement path: Add a mergeable accumulator and use Rayon's `fold`/`reduce`, following the bounded accumulator pattern in `src/arena/runner.rs` and `src/arena/report.rs`.

**Win conditions rescan a growing hand:**
- Problem: Each turn checks the full accumulated hand; `TwoCardSet` scans twice, `KOfTag` scans once, and `AnyOf` repeats scans for every child condition.
- Files: `src/engine.rs`, `src/win_condition.rs`, `src/deck.rs`
- Cause: The trait accepts only a hand slice and has no incremental state or tag-count cache.
- Improvement path: Keep per-tag counts in trial state or add an optional stateful evaluator while preserving the simple slice-based trait for small simulations.

**Go allocates and shuffles a full deck for each job:**
- Problem: The default Go CLI executes 10,000,000 simulations, constructing a new `rand.Rand` and appending/shuffling a new card slice for every simulation.
- Files: `golang/main.go`, `golang/README.md`
- Cause: Worker-local buffers and reusable deck storage are not used.
- Improvement path: Reuse worker-local deck buffers, represent cards compactly, benchmark allocation counts, or route the CLI through the optimized Rust engine.

## Fragile Areas

**London mulligan policy context:**
- Files: `src/engine.rs`, `src/mulligan.rs`, `.claude/skills/mindcrank-simulate/reference.md`, `tests/simulation.rs`
- Why fragile: `MulliganPolicy::keep` cannot see the mulligan number or final-hand status, while the engine force-keeps after the configured limit and normalizes invalid bottom indices behind the policy's back. Small API changes can alter seeded results and mulligan statistics.
- Safe modification: Extract a tested opening-hand state object carrying attempt number, hand, library, bottom count, and resolved seed; preserve seeded golden outcomes when refactoring.
- Test coverage: Tests cover one mulligan, zero-mulligan keeping, and one custom bottomer, but not invalid/duplicate bottom indices, multiple mulligans, short decks, or policy behavior by attempt.

**Arena accumulator assumes validated binary outcomes:**
- Files: `src/arena/report.rs`, `src/arena/runner.rs`, `src/arena/mod.rs`
- Why fragile: Arithmetic such as `1 - starter` and array indexing is safe only because `execute_trial` constructs starter values and validates winner seats before recording. Reusing `MatchupAccumulator` or generalizing seats without updating every invariant can panic or misattribute records.
- Safe modification: Keep validation at the runner boundary, replace binary arithmetic with validated seat collections, and add reconciliation assertions (`games == wins + losses + draws`) in tests.
- Test coverage: No direct tests exercise invalid winner seats, duplicate matchup IDs, invalid competitor indices, self-pairings, or accumulator reconciliation after parallel merges.

**Seeded behavior is coupled to dependency implementation:**
- Files: `src/engine.rs`, `src/arena/mod.rs`, `Cargo.toml`, `Cargo.lock`, `.claude/skills/mindcrank-simulate/reference.md`
- Why fragile: Trial stream derivation is local and stable, but the actual shuffle uses `rand::rngs::StdRng`; no output-version identifier or cross-version golden sequence is stored with reports.
- Safe modification: Treat seeded equality as a same-build contract, or adopt an explicitly versioned RNG algorithm and include engine/RNG version metadata in replayable reports.
- Test coverage: Tests compare worker counts inside one dependency build but do not pin a golden shuffle/outcome across dependency or toolchain upgrades.

**Decklist parser uses permissive heuristics:**
- Files: `.claude/skills/mindcrank-simulate/templates/simulate.rs`, `.claude/skills/mindcrank-simulate/SKILL.md`
- Why fragile: Any non-digit line changes section state, overflowed counts silently become one via `unwrap_or(1)`, and everything after the last `(` is stripped as printing metadata even when parentheses are part of a card name.
- Safe modification: Return structured parse errors/warnings with line numbers, recognize explicit header/export grammars, validate counts, and preserve names unless a full set/collector suffix matches.
- Test coverage: The template parser has no automated tests and is not compiled by the normal Cargo targets.

## Scaling Limits

**Round-robin arena growth:**
- Current capacity: Work is `n(n-1)/2 × trials_per_matchup`, with all matchup reports retained; the example uses 3 competitors and 100,000 trials per matchup in `examples/round_robin.rs`.
- Limit: `checked_mul` prevents `usize` overflow, but there is no preflight warning before quadratic schedules consume CPU and O(n²) report memory.
- Scaling path: Expose projected matchups/games before execution, require confirmation or a configured budget above a threshold, and add sampled schedules for large registries.
- Files: `src/arena/schedule.rs`, `src/arena/runner.rs`, `src/arena/report.rs`, `examples/round_robin.rs`

**Single-deck trial volume:**
- Current capacity: Documentation recommends 100,000 to 1,000,000 Rust trials, while each trial clones the deck and the runner retains all outcomes.
- Limit: CPU scales with trial count × deck clone/evaluation work, and memory scales linearly with trial count before aggregation.
- Scaling path: Stream aggregation, compact card representation, benchmark-driven budgets, and optional progress/cancellation callbacks.
- Files: `src/engine.rs`, `src/deck.rs`, `.claude/skills/mindcrank-simulate/SKILL.md`

## Dependencies at Risk

**Moving stable Rust toolchain:**
- Risk: `Cargo.toml` declares an MSRV of 1.97, but `rust-toolchain.toml` selects the moving `stable` channel and the repository has no checked-in CI workflow to test either MSRV or upcoming stable changes.
- Impact: Builds, Clippy results, formatting, or seeded dependency behavior can change when a developer updates the installed stable toolchain.
- Migration plan: Pin the repository toolchain when bit-for-bit reproducibility matters and add CI jobs for the declared MSRV plus current stable.
- Files: `Cargo.toml`, `rust-toolchain.toml`, `Cargo.lock`

**No automated dependency policy/audit:**
- Risk: Rust dependencies are locked, but no vulnerability, license, or stale-dependency policy is defined alongside the manifest; the Go module has no external dependencies but also no repository-wide automation.
- Impact: A vulnerable transitive release or incompatible toolchain/dependency update may go unnoticed until a manual check.
- Migration plan: Add a CI workflow with RustSec auditing and dependency/license policy, plus `cargo test`, Clippy, formatting, `go test -race`, and `go vet` gates.
- Files: `Cargo.toml`, `Cargo.lock`, `golang/go.mod`, `.gitignore`

## Missing Critical Features

**Interactive and multiplayer game model:**
- Problem: Competitive results are fixed two-player goldfish races. There is no shared game state, mana, stack, interaction, protection, combat, hidden-information view, pilot, or four-player seating balance.
- Blocks: Arena output cannot answer realistic Commander matchup questions; it can only compare independent assembly timing, as documented by the model itself.
- Files: `src/arena/model.rs`, `src/arena/mod.rs`, `docs/interactive-simulation-plan.md`, `README.md`

**Model-quality and run-provenance metadata:**
- Problem: Single-deck aggregates do not include resolved seed, engine/dependency version, deck fingerprint, parameter snapshot, or warnings about the goldfish abstraction.
- Blocks: Persisted results cannot be independently reproduced or audited unless the calling application records all inputs out of band.
- Files: `src/metrics.rs`, `src/engine.rs`, `.claude/skills/mindcrank-simulate/templates/simulate.rs`

**First-class decklist ingestion:**
- Problem: Decklist parsing exists only inside a copy-and-edit agent template, not as a tested library or CLI API.
- Blocks: Applications must duplicate permissive parsing/tagging logic and cannot rely on structured validation for library size, sections, counts, or unmatched tags.
- Files: `.claude/skills/mindcrank-simulate/templates/simulate.rs`, `.claude/skills/mindcrank-simulate/SKILL.md`, `src/lib.rs`

## Test Coverage Gaps

**Rust parameter and engine boundaries:**
- What's not tested: Empty/short decks, zero hand size, zero draws per turn, draws larger than the library, multiple/maximum mulligans, malformed bottom-index output, zero-trial core aggregation, and worker-pool failure behavior.
- Files: `src/engine.rs`, `src/deck.rs`, `src/mulligan.rs`, `tests/simulation.rs`
- Risk: Silent coercions, panic paths, and seeded-result changes can escape the six single-deck integration tests.
- Priority: High

**Arena validation and balance boundaries:**
- What's not tested: Odd trial counts, invalid winner seats, duplicate matchup IDs, invalid/self competitor indices, total-trial overflow, out-of-range replay IDs, custom-model panics/errors, and Wilson interval numeric cases.
- Files: `src/arena/runner.rs`, `src/arena/report.rs`, `tests/arena.rs`
- Risk: Custom schedules/models and large-run edge cases can produce biased reports, phantom replays, or failures despite the seven arena tests passing.
- Priority: High

**Simulation skill harness:**
- What's not tested: Arena/Moxfield parsing variants, section transitions, set/collector suffixes, parentheses in card names, invalid/overflow counts, unmatched tag warnings, zero trials, confidence interval boundaries, and cumulative curves when `draws_per_turn > 1` or the final draw is partial.
- Files: `.claude/skills/mindcrank-simulate/templates/simulate.rs`, `.claude/skills/mindcrank-simulate/SKILL.md`
- Risk: The primary end-user workflow can report a malformed deck or misleading statistics while all compiled targets pass.
- Priority: High

**Go negative and outcome paths:**
- What's not tested: Most `validateConfig` branches, direct invalid calls to `runScenario`, opening-hand wins, last-card wins, result averages, zero/negative seeds, and worker behavior at different CPU counts.
- Files: `golang/main.go`, `golang/main_test.go`
- Risk: Panics and metric drift can go unnoticed; the race detector only establishes that the three covered tests are race-free.
- Priority: Medium

**Validation baseline:**
- What's tested: `cargo test --all-targets` passes 13 integration tests, `cargo clippy --all-targets --all-features -- -D warnings` passes, `cargo fmt --all -- --check` passes, and `go test -race ./...` plus `go vet ./...` pass.
- Files: `tests/simulation.rs`, `tests/arena.rs`, `golang/main_test.go`, `Cargo.toml`, `golang/go.mod`
- Risk: The clean baseline covers deterministic happy paths but does not reduce the priorities of the explicit boundary gaps above.
- Priority: Informational

---

*Concerns audit: 2026-08-26*
