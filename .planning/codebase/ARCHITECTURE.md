<!-- refreshed: 2026-08-26 -->
# Architecture

**Analysis Date:** 2026-08-26

## System Overview

```text
┌─────────────────────────────────────────────────────────────┐
│                    Consumer / Adapter Layer                   │
├──────────────────┬─────────────────┬───────────────────────┤
│ Rust examples    │ Skill harness   │ Standalone Go CLI     │
│ `examples/`      │ `.claude/skills/`│ `golang/main.go`      │
└────────┬─────────┴────────┬────────┴───────────────────────┘
         │                 │                 │ separate pipeline
         ▼                 ▼                 ▼
┌──────────────────────────────────────────┐   ┌─────────────────┐
│ Rust public facade `src/lib.rs`           │   │ Go simulation   │
│ Core exports + public `arena` namespace   │   │ `golang/main.go` │
└────────────────────┬─────────────────────┘   └─────────────────┘
                     │
          ┌───────────┴──────────┐
          ▼                      ▼
┌────────────────────┐  ┌────────────────────────────────┐
│ Single-deck engine │  │ Competitive arena              │
│ `src/engine.rs`    │◄──│ `src/arena/{model,runner,...}` │
└─────────┬──────────┘  └────────────────────────────────┘
          │                      │
          └───────────┬───────────┘
                     ▼
┌────────────────────────────────────────────────────────────┐
│ Domain, policies, and reports                                  │
│ `src/{card,deck,win_condition,mulligan,metrics}.rs`            │
│ `src/arena/{mod,schedule,report}.rs`                           │
└─────────────────────────────────────────────────────────────┘
```

The reusable system is the Rust library rooted at `src/lib.rs`. The executable in `golang/main.go` independently implements a narrower two-card draw simulation and does not call into the Rust crate.

## Component Responsibilities

| Component | Responsibility | File |
|-----------|----------------|------|
| Crate facade | Defines module visibility and presents the stable core API through re-exports | `src/lib.rs` |
| Card model | Stores card identity, optional type, and free-form role tags | `src/card.rs` |
| Deck model | Owns immutable deck lists and creates shuffled/drawn copies for trials | `src/deck.rs` |
| Win strategies | Defines `WinCondition` and built-in composable tag predicates | `src/win_condition.rs` |
| Mulligan strategies | Defines keep and London-bottom policy extension points and defaults | `src/mulligan.rs` |
| Single-deck engine | Performs mulligans, turn draws, seeded trials, and parallel Monte Carlo execution | `src/engine.rs` |
| Core metrics | Represents trial outcomes and reduces outcome slices into aggregate statistics | `src/metrics.rs` |
| Arena contracts | Owns competitor, matchup, trial identity, outcome, error, and deterministic seed primitives | `src/arena/mod.rs` |
| Arena scheduling | Converts competitor registries into stable two-seat matchups | `src/arena/schedule.rs` |
| Arena models | Defines `MatchSimulator` and adapts two core trials into a goldfish race | `src/arena/model.rs` |
| Arena execution | Validates schedules, runs paired parallel trials, replays trials, and assembles reports | `src/arena/runner.rs` |
| Arena reporting | Incrementally records per-matchup statistics, examples, intervals, and standings | `src/arena/report.rs` |
| Rust adapters | Demonstrate construction and console reporting for core and arena APIs | `examples/two_card_combo.rs`, `examples/round_robin.rs` |
| Simulation skill | Supplies a decklist parser/reporting harness and user workflow around the Rust API | `.claude/skills/mindcrank-simulate/templates/simulate.rs`, `.claude/skills/mindcrank-simulate/SKILL.md` |
| Go executable | Parses CLI flags and runs its own worker/channel simulation pipeline | `golang/main.go` |

## Pattern Overview

**Overall:** Layered library with strategy-pattern domain policies, deterministic Monte Carlo pipelines, and a higher-level arena orchestration subsystem (`src/lib.rs`, `src/engine.rs`, `src/arena/mod.rs`).

**Key Characteristics:**
- Keep card representation deliberately shallow and tag-driven; richer behavior belongs behind `WinCondition`, `MulliganPolicy`, `BottomHeuristic`, or `MatchSimulator` traits in `src/win_condition.rs`, `src/mulligan.rs`, and `src/arena/model.rs`.
- Pass simulation inputs as borrowed configuration values. `Params<'a>` borrows a `Deck` and strategy traits in `src/engine.rs`, and `Competitor<'a>` embeds that plan without cloning the deck in `src/arena/mod.rs`.
- Separate scheduling, match semantics, parallel execution, and reduction across `src/arena/schedule.rs`, `src/arena/model.rs`, `src/arena/runner.rs`, and `src/arena/report.rs`.
- Derive independent RNG streams from stable identities so parallel worker count does not change seeded results in `src/engine.rs` and `src/arena/mod.rs`.
- Return plain data records from library code; console formatting remains in `examples/*.rs`, `.claude/skills/mindcrank-simulate/templates/simulate.rs`, and `golang/main.go`.

## Layers

**Public API Facade:**
- Purpose: Expose the reusable Rust surface while keeping implementation modules private.
- Location: `src/lib.rs`
- Contains: Re-exports for cards, decks, parameters, metrics, and strategy traits; the public `arena` namespace.
- Depends on: All private core modules under `src/*.rs` and public `src/arena/mod.rs`.
- Used by: `examples/*.rs`, `tests/*.rs`, and `.claude/skills/mindcrank-simulate/templates/simulate.rs`.

**Domain Model:**
- Purpose: Represent a deck as cloned tagged cards without embedding Magic rules.
- Location: `src/card.rs`, `src/deck.rs`
- Contains: `Card`, `Deck`, tag lookup, shuffle, draw, and bottom placement operations.
- Depends on: `std::collections::HashSet` in `src/card.rs` and `rand` in `src/deck.rs`.
- Used by: Strategies in `src/win_condition.rs` and `src/mulligan.rs`, execution in `src/engine.rs`, and consumer adapters in `examples/*.rs`.

**Strategy and Policy Layer:**
- Purpose: Make win detection and mulligan decisions replaceable without changing the execution loop.
- Location: `src/win_condition.rs`, `src/mulligan.rs`, `src/arena/model.rs`, `src/arena/schedule.rs`
- Contains: `WinCondition`, `MulliganPolicy`, `BottomHeuristic`, `MatchSimulator`, and `Schedule`, plus built-in implementations.
- Depends on: Domain values from `src/card.rs`, `src/deck.rs`, and arena contracts from `src/arena/mod.rs`.
- Used by: `src/engine.rs` and `src/arena/runner.rs` through `dyn Trait` references.

**Single-Deck Execution Layer:**
- Purpose: Turn a `Params` plan into one `TrialOutcome` or a parallel `Aggregate`.
- Location: `src/engine.rs`
- Contains: Configuration builders, London mulligan loop, bottoming normalization, turn draw loop, per-trial seed derivation, and Rayon dispatch.
- Depends on: Models and strategies from `src/{card,deck,mulligan,win_condition}.rs`, results from `src/metrics.rs`, and `rand`/`rayon` from `Cargo.toml`.
- Used by: Direct consumers through `src/lib.rs` and `GoldfishRaceModel` in `src/arena/model.rs`.

**Metrics and Reduction Layer:**
- Purpose: Preserve explicit misses and compute deterministic summary statistics.
- Location: `src/metrics.rs`, `src/arena/report.rs`
- Contains: `TrialOutcome`, `Aggregate`, `Record`, Wilson intervals, replay example selection, matchup reports, and standings.
- Depends on: Core outcomes from `src/metrics.rs` and arena trial records from `src/arena/mod.rs`.
- Used by: `src/engine.rs`, `src/arena/runner.rs`, examples, and integration tests.

**Arena Orchestration Layer:**
- Purpose: Run scheduled competitive comparisons over existing single-deck plans.
- Location: `src/arena/mod.rs`, `src/arena/runner.rs`
- Contains: Stable identities, validation, named seed streams, paired starting-seat trials, worker-local accumulators, replay, and typed failures.
- Depends on: Policies in `src/arena/{schedule,model}.rs`, reporting in `src/arena/report.rs`, and the single-deck API exposed by `src/lib.rs`.
- Used by: `examples/round_robin.rs` and `tests/arena.rs`.

**Adapter and Documentation Layer:**
- Purpose: Convert concrete deck questions into library calls and render results for humans.
- Location: `examples/*.rs`, `.claude/skills/mindcrank-simulate/`, `README.md`, `docs/interactive-simulation-plan.md`
- Contains: Runnable examples, a decklist parsing harness, API guidance, and architecture planning.
- Depends on: The public Rust API from `src/lib.rs` and `src/arena/mod.rs`.
- Used by: Developers and coding agents; no library module imports this layer.

**Standalone Go Pipeline:**
- Purpose: Provide a CLI for a fixed aggregate combo-draw scenario.
- Location: `golang/main.go`
- Contains: CLI configuration, validation, deck creation, shuffling, goroutine workers, channel reduction, and console output.
- Depends on: Go standard library only, as declared by `golang/go.mod`.
- Used by: Direct `go run` execution documented in `golang/README.md`; it has no dependency edge to `src/*.rs`.

## Data Flow

### Primary Single-Deck Request Path

1. A consumer builds tagged `Card` values, a `Deck`, and a `WinCondition`, then constructs `Params` (`examples/two_card_combo.rs:20`, `src/engine.rs:25`).
2. `monte_carlo` resolves a master seed and maps each trial index to a deterministic seed on Rayon (`src/engine.rs:254`).
3. `run_once_with_seed` shuffles a fresh deck copy, performs optional London mulligans, normalizes bottom choices, and hands the kept cards to the play loop (`src/engine.rs:67`).
4. `play_out` checks turn zero, draws into a cumulative hand through the horizon, and returns a win or explicit miss as `TrialOutcome` (`src/engine.rs:168`).
5. `Aggregate::from_outcomes` reduces all collected outcomes into rates, averages, and a draws-to-win distribution (`src/metrics.rs:53`).
6. The consumer formats the plain `Aggregate` (`examples/two_card_combo.rs:35`, `.claude/skills/mindcrank-simulate/templates/simulate.rs`).

### Arena Schedule and Run Flow

1. A consumer registers stable `Competitor` IDs around borrowed single-deck `Params` and calls `ArenaMonteCarlo::run` (`examples/round_robin.rs:23`, `src/arena/runner.rs:43`).
2. The runner validates unique competitor IDs, asks a `Schedule` for matchups, and validates matchup IDs and competitor indices (`src/arena/runner.rs:49`, `src/arena/runner.rs:181`).
3. `RoundRobin` sorts by competitor ID and emits every unordered pair with stable sequential `MatchupId` values (`src/arena/schedule.rs:15`).
4. The runner expands each matchup into paired trial jobs, derives matchup/sample seeds, alternates starting seat, and invokes `MatchSimulator::simulate` (`src/arena/runner.rs:59`, `src/arena/runner.rs:148`).
5. `GoldfishRaceModel` derives competitor-keyed streams, calls `run_once` for both plans, and compares their `turns_to_win` values under a tie policy (`src/arena/model.rs:48`).
6. Rayon worker-local `MatchupAccumulator` maps are merged incrementally; final matchup reports and standings are built without retaining every `TrialRecord` (`src/arena/runner.rs:62`, `src/arena/report.rs:111`).
7. The caller receives `Result<ArenaReport, ArenaError>` and renders the standings (`src/arena/runner.rs:113`, `examples/round_robin.rs:34`).

### Arena Replay Flow

1. A caller selects a stored `TrialId` from `OutcomeExamples` in `src/arena/report.rs` and supplies the resolved run seed to `ArenaMonteCarlo::replay` (`src/arena/runner.rs:122`).
2. Replay regenerates and validates the schedule, finds the stable `MatchupId`, and calls the same `execute_trial` path used by the batch run (`src/arena/runner.rs:130`).
3. `execute_trial` re-derives sample and competitor streams and returns the full `TrialRecord` for inspection (`src/arena/runner.rs:148`, `src/arena/mod.rs:92`).

### Standalone Go CLI Flow

1. `main` parses flags into `Config`, supplies a time-based seed when zero, validates the configuration, and calls `runScenario` (`golang/main.go:61`).
2. `runScenario` starts `runtime.NumCPU()` goroutines, gives each simulation index a deterministic RNG, and sends `Simulation` values over a channel (`golang/main.go:99`).
3. The main goroutine reduces channel values into `Results` and prints them (`golang/main.go:128`, `golang/main.go:95`).

**State Management:**
- Rust input state is borrowed and effectively immutable: each trial clones/shuffles `Deck` data, while mutable hand/library state stays local to `run_once_with_seed` in `src/engine.rs`.
- Core `monte_carlo` collects a `Vec<TrialOutcome>` before reduction in `src/engine.rs`; arena runs reduce worker-local `BTreeMap<MatchupId, MatchupAccumulator>` values in `src/arena/runner.rs`.
- Replay identity is explicit data (`MatchupId`, `TrialId`, resolved seed) in `src/arena/mod.rs` and `src/arena/report.rs`; there is no persisted database or service state.
- Go workers own per-simulation RNG/deck values and communicate immutable result records over channels in `golang/main.go`.

## Key Abstractions

**Tagged Card and Deck:**
- Purpose: Model only the card properties that a simulation question reads.
- Examples: `src/card.rs`, `src/deck.rs`
- Pattern: Value objects with builder methods; `Deck` exposes non-destructive public draw/shuffle behavior and crate-private mutation for trial-local libraries.

**`WinCondition`:**
- Purpose: Decide whether a cumulative hand has assembled its objective and rank relevant cards for mulligan bottoming.
- Examples: `src/win_condition.rs`, custom implementations in `tests/simulation.rs`
- Pattern: `Send + Sync` strategy trait with built-in `TwoCardSet`, `KOfTag`, and composite `AnyOf` implementations.

**Mulligan Policies:**
- Purpose: Separate keep decisions from bottom-card selection.
- Examples: `src/mulligan.rs`, custom policies in `tests/simulation.rs`
- Pattern: Independent `MulliganPolicy` and `BottomHeuristic` strategy traits; invalid bottom indices are repaired by the engine in `src/engine.rs`.

**Simulation Parameters:**
- Purpose: Bundle borrowed models and execution controls without owning or duplicating deck data.
- Examples: `Params<'a>` and `MonteCarloParams<'a>` in `src/engine.rs`
- Pattern: Copyable parameter objects with constructor defaults, fluent methods for common options, and public fields for remaining controls.

**Schedule and Match Simulator:**
- Purpose: Make matchup generation and match-resolution semantics independent of repetition and parallelism.
- Examples: `src/arena/schedule.rs`, `src/arena/model.rs`
- Pattern: `Send + Sync` strategy traits consumed through `&dyn Schedule` and `&dyn MatchSimulator` in `src/arena/runner.rs`.

**Stable Trial Identity and Named RNG Streams:**
- Purpose: Make samples replayable and prevent one subsystem's random consumption from perturbing another.
- Examples: `TrialId`, `TrialContext::stream_seed`, and `TrialContext::competitor_seed` in `src/arena/mod.rs`
- Pattern: Deterministic seed derivation from master seed, matchup ID, paired sample index, namespace, and competitor ID.

**Incremental Arena Accumulator:**
- Purpose: Aggregate large competitive runs without retaining every trial.
- Examples: `MatchupAccumulator` in `src/arena/report.rs`, Rayon fold/reduce in `src/arena/runner.rs`
- Pattern: Associative worker-local accumulation and merge, retaining only aggregate records and the lowest replayable example IDs.

## Entry Points

**Rust Crate API:**
- Location: `src/lib.rs`
- Triggers: Imported by another Rust crate, an example, or an integration test.
- Responsibilities: Expose `Card`, `Deck`, strategies, `run_once`, `monte_carlo`, metrics, and the `arena` module.

**Single-Trial API:**
- Location: `src/engine.rs:62`
- Triggers: Direct `run_once(&Params)` calls or `GoldfishRaceModel` in `src/arena/model.rs`.
- Responsibilities: Resolve a seed and execute one complete single-deck trial.

**Core Monte Carlo API:**
- Location: `src/engine.rs:254`
- Triggers: `monte_carlo(MonteCarloParams)` from a library consumer or `examples/two_card_combo.rs`.
- Responsibilities: Dispatch deterministic independent trials and return `Aggregate`.

**Arena Run and Replay API:**
- Location: `src/arena/runner.rs:43`, `src/arena/runner.rs:122`
- Triggers: `ArenaMonteCarlo::run` or `ArenaMonteCarlo::replay` from a Rust consumer.
- Responsibilities: Validate competitive inputs, execute scheduled trials, return reports, or reproduce a selected trial.

**Core Example Binary:**
- Location: `examples/two_card_combo.rs`
- Triggers: `cargo run --release --example two_card_combo` documented in `README.md`.
- Responsibilities: Build a concrete 99-card two-piece scenario, run one million trials, and print aggregate metrics.

**Arena Example Binary:**
- Location: `examples/round_robin.rs`
- Triggers: `cargo run --release --example round_robin` documented in `README.md`.
- Responsibilities: Compare three deck plans under a round-robin goldfish model and print standings.

**Agent Simulation Harness:**
- Location: `.claude/skills/mindcrank-simulate/templates/simulate.rs`
- Triggers: Copied to ignored `examples/sim_scratch.rs` per `.claude/skills/mindcrank-simulate/SKILL.md`.
- Responsibilities: Parse pasted decklists, apply tag groups, validate/report deck composition, invoke the Rust engine, and print confidence-aware results.

**Go CLI:**
- Location: `golang/main.go:61`
- Triggers: `go run .` from `golang/`, as documented in `golang/README.md`.
- Responsibilities: Parse primitive combo/deck counts and execute the standalone Go simulation.

## Architectural Constraints

- **Threading:** Core Rust trials use Rayon's global pool when `workers == 0` or a per-run bounded `ThreadPool` otherwise in `src/engine.rs`; arena trials use the same choice with incremental fold/reduce in `src/arena/runner.rs`. Strategy traits must remain `Send + Sync` in `src/{win_condition,mulligan}.rs` and `src/arena/{model,schedule}.rs`.
- **Global state:** There is no project-owned mutable global state in `src/*.rs`. Unseeded runs read external randomness through `rand::random`, and `workers == 0` relies on Rayon's process-global pool in `src/engine.rs` and `src/arena/runner.rs`.
- **Circular imports:** No circular module chain is detected. The arena depends inward on the core through `crate::run_once` in `src/arena/model.rs`; core modules do not depend on `src/arena/`.
- **Borrowed lifetimes:** `Params<'a>` borrows its deck and strategies in `src/engine.rs`, and `Competitor<'a>` stores `Params<'a>` in `src/arena/mod.rs`. Keep these referents alive for the complete run and do not return competitors that outlive them.
- **Determinism:** Preserve index-derived core seeds in `src/engine.rs` and identity-derived arena streams in `src/arena/mod.rs`/`src/arena/runner.rs`; consuming one shared mutable RNG across parallel jobs would break worker-count independence asserted by `tests/simulation.rs` and `tests/arena.rs`.
- **Two-seat arena:** `Matchup`, `MatchResult`, report arrays, paired starting seats, and `1 - seat` arithmetic encode exactly two competitors across `src/arena/mod.rs`, `src/arena/runner.rs`, and `src/arena/report.rs`.
- **Model scope:** The core accumulates cards in hand and checks tag predicates; it does not model mana, casting, zones, combat, or opposing interaction in `src/engine.rs`. `GoldfishRaceModel` compares independent completion turns in `src/arena/model.rs`.
- **Memory behavior:** Core `monte_carlo` stores one `TrialOutcome` per trial before aggregation in `src/engine.rs`; the arena intentionally retains only accumulators and selected example IDs in `src/arena/runner.rs` and `src/arena/report.rs`.
- **API boundary:** Internal modules under `src/*.rs` and `src/arena/*.rs` are private and surfaced through `src/lib.rs` and `src/arena/mod.rs`; add public API through those facades instead of importing implementation paths.

## Anti-Patterns

### Duplicated Simulation Engines

**What happens:** `golang/main.go` independently implements configuration, deck construction, shuffle, deterministic seeds, trial execution, and aggregation that overlap the canonical Rust flow in `src/{card,deck,engine,metrics}.rs`.
**Why it's wrong:** Behavior and metrics can diverge because fixes or invariants must be maintained in two pipelines; the Go executable cannot reuse Rust traits or the arena in `src/arena/`.
**Do this instead:** Add reusable simulation behavior to the Rust modules under `src/`, expose it through `src/lib.rs`, and add a consumer under `examples/`; touch `golang/main.go` only when deliberately maintaining the standalone Go CLI contract documented by `golang/README.md`.

### Scattered Two-Seat Arithmetic

**What happens:** Two-player cardinality is repeated as `[T; 2]`, `0..2`, `trial_index % 2`, and `1 - seat` across `src/arena/mod.rs`, `src/arena/runner.rs`, `src/arena/model.rs`, and `src/arena/report.rs`.
**Why it's wrong:** Seat-count behavior is implicit across scheduling, execution, model, validation, and reports; changing one location independently can create index errors or internally inconsistent statistics.
**Do this instead:** For two-player features, preserve the fixed-size invariant and update all four arena layers together with coverage in `tests/arena.rs`. For any variable-seat work, introduce one explicit contest/seat abstraction in `src/arena/mod.rs` before changing schedule, runner, model, or report representations.

## Error Handling

**Strategy:** Use explicit outcome data for expected simulation misses, typed `Result` errors for arena input/execution failures, and process termination only at executable adapter boundaries (`src/metrics.rs`, `src/arena/mod.rs`, `golang/main.go`).

**Patterns:**
- Represent a horizon miss as `TrialOutcome { won: false, turns_to_win: None }`, not as an error, in `src/engine.rs` and `src/metrics.rs`.
- Normalize hand size, draws per turn, mulligan count, and malformed bottom-heuristic output inside `src/engine.rs` so trial execution preserves hand-size invariants.
- Return `Result<ArenaReport, ArenaError>` and `Result<TrialRecord, ArenaError>` from `src/arena/runner.rs`; define human-readable typed variants in `src/arena/mod.rs`.
- Convert custom arena worker-pool construction failures into `ArenaError::WorkerPool` in `src/arena/runner.rs`; note that core `monte_carlo` uses `expect` for the corresponding build failure in `src/engine.rs`.
- Validate Go CLI inputs with `validateConfig` and terminate from `main` with `log.Fatalf` only in `golang/main.go`.

## Cross-Cutting Concerns

**Logging:** Library code under `src/` performs no logging. Human-readable output belongs to `examples/*.rs`, `.claude/skills/mindcrank-simulate/templates/simulate.rs`, and `golang/main.go`.
**Validation:** Core Rust preserves local invariants through clamping/normalization in `src/engine.rs`; arena validates competitor/matchup identity and indices in `src/arena/mod.rs` and `src/arena/runner.rs`; Go validates numeric configuration in `golang/main.go`.
**Authentication:** Not applicable; there is no network service, account, or external identity integration in `src/`, `examples/`, or `golang/`.

---

*Architecture analysis: 2026-08-26*
