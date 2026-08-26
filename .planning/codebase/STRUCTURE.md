# Codebase Structure

**Analysis Date:** 2026-08-26

## Directory Layout

```text
mindcrank/
├── .claude/
│   └── skills/mindcrank-simulate/       # Agent workflow, API reference, and scratch harness
├── .planning/
│   ├── codebase/                        # Generated GSD codebase maps
│   └── threads/                         # Persistent project discussion/context artifacts
├── docs/
│   └── interactive-simulation-plan.md   # Design plan for interactive arena evolution
├── examples/
│   ├── two_card_combo.rs                # Single-deck Rust example binary
│   └── round_robin.rs                   # Competitive arena Rust example binary
├── golang/
│   ├── go.mod                           # Independent Go module manifest
│   ├── main.go                          # Standalone Go CLI and simulation implementation
│   ├── main_test.go                     # Go tests
│   └── README.md                        # Go-specific usage and interpretation
├── src/
│   ├── lib.rs                           # Rust crate facade and re-exports
│   ├── card.rs                          # Tagged card value
│   ├── deck.rs                          # Deck container and draw/shuffle operations
│   ├── engine.rs                        # Single-trial and Monte Carlo execution
│   ├── metrics.rs                       # Core trial and aggregate results
│   ├── mulligan.rs                      # Mulligan/bottom policy traits and defaults
│   ├── win_condition.rs                 # Win-condition trait and implementations
│   └── arena/
│       ├── mod.rs                       # Arena facade, shared types, validation, seeds
│       ├── model.rs                     # Match simulator trait and goldfish model
│       ├── runner.rs                    # Parallel run/replay orchestration
│       ├── schedule.rs                  # Schedule trait and round robin
│       └── report.rs                    # Accumulation, intervals, matchup reports, standings
├── tests/
│   ├── simulation.rs                    # Core Rust integration tests
│   └── arena.rs                         # Arena Rust integration tests
├── Cargo.toml                                # Rust package/dependency manifest
├── Cargo.lock                                # Locked Rust dependency graph
├── rust-toolchain.toml                       # Stable toolchain plus rustfmt/clippy
├── README.md                                 # Primary Rust project documentation
└── .gitignore                                # Rust build output and scratch example exclusions
```

## Directory Purposes

**`src/`:**
- Purpose: Contains the reusable Rust library and all shipped simulation logic.
- Contains: Flat core modules plus the layered arena subsystem in `src/arena/`.
- Key files: `src/lib.rs`, `src/engine.rs`, `src/metrics.rs`, `src/arena/mod.rs`, `src/arena/runner.rs`.

**`src/arena/`:**
- Purpose: Composes single-deck plans into competitive two-seat schedules, models, parallel runs, replay, and reports.
- Contains: One file per arena responsibility; internal files stay private and are re-exported by `src/arena/mod.rs`.
- Key files: `src/arena/mod.rs`, `src/arena/model.rs`, `src/arena/schedule.rs`, `src/arena/runner.rs`, `src/arena/report.rs`.

**`examples/`:**
- Purpose: Holds runnable Rust binaries that demonstrate public API use and perform concrete simulations.
- Contains: One self-contained scenario per snake_case `.rs` file, including `examples/two_card_combo.rs` and `examples/round_robin.rs`.
- Key files: `examples/two_card_combo.rs`, `examples/round_robin.rs`; ignored scratch work belongs at `examples/sim_scratch.rs` per `.gitignore`.

**`tests/`:**
- Purpose: Exercises the public Rust API as an external consumer and pins architectural invariants.
- Contains: Core behavior in `tests/simulation.rs` and arena scheduling/determinism/replay/error behavior in `tests/arena.rs`.
- Key files: `tests/simulation.rs`, `tests/arena.rs`.

**`.claude/skills/mindcrank-simulate/`:**
- Purpose: Packages an agent-facing workflow for turning pasted decklists into Rust simulations.
- Contains: Operational instructions in `.claude/skills/mindcrank-simulate/SKILL.md`, API details in `.claude/skills/mindcrank-simulate/reference.md`, and a reusable executable template in `.claude/skills/mindcrank-simulate/templates/simulate.rs`.
- Key files: `.claude/skills/mindcrank-simulate/SKILL.md`, `.claude/skills/mindcrank-simulate/templates/simulate.rs`.

**`golang/`:**
- Purpose: Contains an independent Go implementation and CLI for a narrower combo-draw Monte Carlo scenario.
- Contains: A single `package main`, module metadata, tests, and Go-specific documentation.
- Key files: `golang/main.go`, `golang/main_test.go`, `golang/go.mod`, `golang/README.md`.

**`docs/`:**
- Purpose: Holds design documentation that is not compiled into the Rust crate.
- Contains: The interactive arena design plan at `docs/interactive-simulation-plan.md`.
- Key files: `docs/interactive-simulation-plan.md`.

**`.planning/`:**
- Purpose: Stores GSD planning and codebase-intelligence artifacts rather than runtime code.
- Contains: Generated maps in `.planning/codebase/` and persistent context under `.planning/threads/`.
- Key files: `.planning/codebase/ARCHITECTURE.md`, `.planning/codebase/STRUCTURE.md`, `.planning/threads/commander-pod-simulations-with-recursive-response-priority.md`.

## Key File Locations

**Entry Points:**
- `src/lib.rs`: Public Rust library facade; add module declarations and public re-exports here.
- `src/engine.rs`: Public `run_once` and `monte_carlo` execution entry points.
- `src/arena/runner.rs`: Public `ArenaMonteCarlo::run` and `ArenaMonteCarlo::replay` entry points.
- `examples/two_card_combo.rs`: Runnable core-engine example.
- `examples/round_robin.rs`: Runnable arena example.
- `.claude/skills/mindcrank-simulate/templates/simulate.rs`: Source template copied into a runnable scratch example.
- `golang/main.go`: Independent Go CLI executable entry point.

**Configuration:**
- `Cargo.toml`: Rust package metadata, Rust 1.97 floor, and `rand`/`rayon`/`approx` dependency declarations.
- `Cargo.lock`: Exact Rust dependency resolution for reproducible repository builds.
- `rust-toolchain.toml`: Stable Rust channel and required `rustfmt`/`clippy` components.
- `golang/go.mod`: Independent Go module and Go 1.22 language version.
- `.gitignore`: Excludes `/target/` and generated `examples/sim_scratch.rs`.

**Core Logic:**
- `src/card.rs`: Tagged card value and fluent tag/type construction.
- `src/deck.rs`: Deck ownership, shuffle, draw, and bottom placement.
- `src/win_condition.rs`: Core simulation objective extension point.
- `src/mulligan.rs`: Opening-hand decision and bottoming extension points.
- `src/engine.rs`: Single-deck state machine and parallel Monte Carlo dispatcher.
- `src/metrics.rs`: Single-deck output contracts and aggregate reduction.
- `src/arena/mod.rs`: Shared arena identities, outcomes, errors, validation, and seed derivation.
- `src/arena/schedule.rs`: Matchup generation.
- `src/arena/model.rs`: Match resolution semantics.
- `src/arena/runner.rs`: Arena execution and replay.
- `src/arena/report.rs`: Arena accumulation and report generation.
- `golang/main.go`: Separate Go simulation core and CLI adapter in one file.

**Testing:**
- `tests/simulation.rs`: Public-facing tests for core composition, deck immutability, mulligans, misses, and deterministic parallelism.
- `tests/arena.rs`: Public-facing tests for schedules, seat balance, tie policies, replay, validation, zero trials, and deterministic parallelism.
- `golang/main_test.go`: Go tests for seed determinism, configuration validation, and deck construction.

**Documentation:**
- `README.md`: Canonical Rust overview, examples, model boundary, and extension guidance.
- `.claude/skills/mindcrank-simulate/SKILL.md`: Prescriptive deck-simulation workflow.
- `.claude/skills/mindcrank-simulate/reference.md`: Detailed public Rust API reference and preserved invariants.
- `golang/README.md`: Standalone Go implementation usage and result interpretation.
- `docs/interactive-simulation-plan.md`: Design plan; do not treat its target types as implemented code.

## Naming Conventions

**Files:**
- Use snake_case for Rust source, test, and example files: `src/win_condition.rs`, `examples/two_card_combo.rs`, `tests/simulation.rs`.
- Use responsibility nouns for core modules: `src/card.rs`, `src/deck.rs`, `src/metrics.rs`; use orchestration roles within the arena: `src/arena/model.rs`, `src/arena/runner.rs`, `src/arena/schedule.rs`, `src/arena/report.rs`.
- Use Rust module roots named `lib.rs` and `mod.rs` only for facades: `src/lib.rs`, `src/arena/mod.rs`.
- Use standard Go package file names inside `golang/`: implementation in `golang/main.go`, related tests in `golang/main_test.go`.
- Use UPPERCASE Markdown filenames for generated codebase maps under `.planning/codebase/`, including `.planning/codebase/ARCHITECTURE.md` and `.planning/codebase/STRUCTURE.md`.

**Directories:**
- Use lowercase single-purpose directories: `src/arena/`, `examples/`, `tests/`, `docs/`, `golang/`.
- Mirror the Rust public namespace with a subdirectory only when a subsystem has several cohesive files, as `src/arena/` maps to `mindcrank::arena` through `src/arena/mod.rs`.
- Keep agent skills under a named bundle: `.claude/skills/mindcrank-simulate/`, with reusable code under its `templates/` child.

**Rust Symbols:**
- Use PascalCase nouns for structs, enums, and traits: `MonteCarloParams` in `src/engine.rs`, `WinCondition` in `src/win_condition.rs`, `ArenaReport` in `src/arena/report.rs`.
- Use snake_case verbs for functions and builder methods: `run_once` in `src/engine.rs`, `with_tie_policy` in `src/arena/model.rs`, `build_standings` in `src/arena/report.rs`.
- Give traits capability-oriented names without an `I` prefix: `MulliganPolicy` in `src/mulligan.rs`, `Schedule` in `src/arena/schedule.rs`, `MatchSimulator` in `src/arena/model.rs`.
- Give result and identity types explicit suffixes: `TrialOutcome` in `src/metrics.rs`, `TrialRecord`, `TrialId`, and `MatchupId` in `src/arena/mod.rs`.

## Where to Add New Code

**New Core Domain Value:**
- Primary code: Add `src/<noun>.rs`, declare it privately in `src/lib.rs`, and re-export only the intended public values from `src/lib.rs`.
- Tests: Add public-API coverage to `tests/simulation.rs`; create another snake_case file under `tests/` only when the behavior forms a distinct subsystem comparable to `tests/arena.rs`.

**New Single-Deck Win Condition:**
- Built-in implementation: Add the type and `WinCondition` implementation to `src/win_condition.rs`, then re-export it from `src/lib.rs`.
- Tests: Add composition and edge-case coverage to `tests/simulation.rs`.
- One-off scenario logic: Keep a local custom `WinCondition` above `main` in a file under `examples/`, following `.claude/skills/mindcrank-simulate/reference.md`.

**New Mulligan or Bottoming Policy:**
- Built-in implementation: Add it beside the relevant trait in `src/mulligan.rs` and re-export it from `src/lib.rs`.
- Tests: Exercise kept hand size and bottom placement in `tests/simulation.rs`.

**New Core Metric:**
- Result fields and reduction: Update `TrialOutcome`/`Aggregate` and `Aggregate::from_outcomes` together in `src/metrics.rs`.
- Population point: Record the raw value in the trial paths in `src/engine.rs`.
- Tests and reporting: Add assertions in `tests/simulation.rs` and update consumers in `examples/two_card_combo.rs` and `.claude/skills/mindcrank-simulate/templates/simulate.rs` when the metric is user-facing.

**New Arena Schedule:**
- Implementation: Add the `Schedule` implementation to `src/arena/schedule.rs`; export the public type through `src/arena/mod.rs`.
- Tests: Add stable identity, invalid-index, and coverage assertions to `tests/arena.rs`.

**New Arena Match Model:**
- Implementation: Add the `MatchSimulator` implementation to `src/arena/model.rs`; add shared public outcomes/contracts to `src/arena/mod.rs` only when they apply across models.
- Tests: Add deterministic outcome, tie, replay, and invalid-winner assertions to `tests/arena.rs`.
- Example: Add a snake_case runnable binary under `examples/` when the model needs a concrete demonstration.

**New Arena Report Field:**
- Contracts and reduction: Update public report structs plus `MatchupAccumulator` in `src/arena/report.rs`; populate required raw data from `TrialRecord` in `src/arena/mod.rs` or `src/arena/runner.rs`.
- Tests: Assert both per-matchup and standings behavior in `tests/arena.rs`.

**New Arena Component/Module:**
- Implementation: Add `src/arena/<responsibility>.rs`, declare it with a private `mod` in `src/arena/mod.rs`, and expose public API with `pub use` from `src/arena/mod.rs`.
- Integration: Keep scheduling in `src/arena/schedule.rs`, match semantics in `src/arena/model.rs`, dispatch/replay in `src/arena/runner.rs`, and aggregation in `src/arena/report.rs` rather than mixing responsibilities.

**New Runnable Simulation:**
- Durable example: Add `examples/<scenario_name>.rs` and document its `cargo run --release --example <scenario_name>` command in `README.md` when it is part of the supported project surface.
- Disposable deck analysis: Copy `.claude/skills/mindcrank-simulate/templates/simulate.rs` to ignored `examples/sim_scratch.rs` and follow `.claude/skills/mindcrank-simulate/SKILL.md`.

**New Go CLI Behavior:**
- Implementation: Keep `package main` behavior in `golang/main.go` and tests in `golang/main_test.go`.
- Boundary: Do not place reusable Rust library features in `golang/`; implement them under `src/` and expose them through `src/lib.rs`.

**Utilities:**
- Core-only helper: Keep a private function in the owning module, as seed mixing stays in `src/engine.rs` and report math stays in `src/arena/report.rs`.
- Arena-shared helper: Put it in `src/arena/mod.rs` with `pub(crate)` visibility only when multiple arena modules need it, following `validate_competitors` and `derive_seed` in `src/arena/mod.rs`.
- Public helper: Add it to the narrowest owning module and re-export through `src/lib.rs` or `src/arena/mod.rs`; avoid a generic `utils.rs` because no such catch-all exists in `src/`.

## Special Directories

**`target/`:**
- Purpose: Cargo build products generated from `Cargo.toml`.
- Generated: Yes.
- Committed: No; `/target/` is excluded by `.gitignore`.

**`.planning/codebase/`:**
- Purpose: Generated architecture, structure, stack, quality, testing, integration, and concern maps for GSD workflows.
- Generated: Yes.
- Committed: Intended as project planning artifacts; files under `.planning/` are tracked elsewhere in the repository, including `.planning/threads/commander-pod-simulations-with-recursive-response-priority.md`.

**`.planning/threads/`:**
- Purpose: Persistent project-context and discussion artifacts used by planning workflows.
- Generated: Yes.
- Committed: Yes; `.planning/threads/commander-pod-simulations-with-recursive-response-priority.md` is tracked.

**`.claude/skills/mindcrank-simulate/`:**
- Purpose: Redistributable agent workflow and simulation scaffold around the crate.
- Generated: No.
- Committed: Yes; `.claude/skills/mindcrank-simulate/SKILL.md`, `.claude/skills/mindcrank-simulate/reference.md`, and `.claude/skills/mindcrank-simulate/templates/simulate.rs` are tracked.

**`examples/`:**
- Purpose: Cargo example binaries and the designated scratch-simulation location.
- Generated: Mixed; `examples/two_card_combo.rs` and `examples/round_robin.rs` are authored, while `examples/sim_scratch.rs` is copied from the skill template.
- Committed: Authored examples are committed; `examples/sim_scratch.rs` is excluded by `.gitignore`.

**`golang/`:**
- Purpose: Self-contained Go module and executable, separate from the Rust crate.
- Generated: No.
- Committed: Yes; `golang/go.mod`, `golang/main.go`, `golang/main_test.go`, and `golang/README.md` are tracked.

---

*Structure analysis: 2026-08-26*
