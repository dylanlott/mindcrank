# Plan: four-player Commander pods with recursive responses

## Objective

Extend the competitive arena from fixed two-player goldfish races to
deterministic four-player Commander pods, then add a coarse interactive turn
model with a stack-like priority loop. Interaction and protection must support
arbitrarily deep response chains; the gameplay limit is cards and mana, not a
hard-coded number of responses.

This remains an abstract simulation engine. It should model the strategic
shape of Commander games without implementing comprehensive Magic card text,
layers, replacement effects, or the complete rules engine.

## Current baseline

Commit `e5d2a28` introduced:

- `Competitor`, fixed two-seat `Matchup`, `MatchSimulator`, and `TrialContext`
- balanced two-player `RoundRobin` scheduling
- paired Monte Carlo samples with deterministic, competitor-keyed RNG streams
- `GoldfishRaceModel`
- matchup reports, standings, Wilson intervals, and replayable `TrialId`s
- seven arena integration tests and a three-deck example

The present arena assumes exactly two seats in its schedule, accumulator,
reports, starting-position balancing, and goldfish model. Those assumptions
must be removed before the interactive model is added.

## Locked design decisions

1. **Four-player pods are first-class.** The generalized core may support any
   positive seat count, but Commander scheduling validates exactly four.
2. **Responses are recursive.** A counter or protection effect can itself be
   answered. There is no gameplay response-depth setting.
3. **Protection is not a special terminal step.** Interaction and protection
   both add typed effects to the same stack. Their card roles differ for pilot
   strategy and reporting, not for stack semantics.
4. **Priority determines resolution.** Players pass or add a response in turn;
   all living players passing resolves the top stack item. An empty stack plus
   all passes closes the window.
5. **All 24 seat permutations are the default for a fixed four-player pod.**
   This balances starting seat, clockwise priority order, and targeting order.
6. **One Monte Carlo sample expands into every selected seating.** Deck-specific
   shuffles remain identical across those seatings through named RNG streams.
7. **Invalid model behavior fails loudly.** Illegal actions and safety-limit
   exhaustion produce a typed simulation error; they are never silently counted
   as draws.
8. **Hidden information stays hidden.** A pilot sees its own hand and all public
   state, but never an opponent's hand or library ordering.
9. **The existing two-player workflow remains supported.** `RoundRobin` and
   `GoldfishRaceModel` continue to work through the generalized contest types.
10. **Incremental reduction remains mandatory.** The Monte Carlo runner does
    not retain every trial; only aggregates and selected replay IDs are kept.

## Scope

### Included

- variable-size contest primitives and reports
- four-player fixed and all-combinations pod schedules
- deterministic full seat-permutation balancing
- multiplayer goldfish races as a regression baseline
- shared opening-hand and London-mulligan initialization
- per-player zones, lands, mana, and turn state
- stack items, priority, passes, recursive responses, and LIFO resolution
- coarse `WinAttempt` and `Counter` effects
- linear and threat-aware pilots
- typed response metrics and replay traces
- a runnable four-player pod example

### Deferred

- exhaustive card text and the complete Magic rules engine
- combat damage and individual player elimination
- replacement/prevention effects, layers, split second, and special actions
- political agreements, threat assessment learned from data, or table talk
- teams, partner rules, commander tax, commander damage, and color identity
- sideboards, best-of-three matches, tournament pairings, and league state
- statistically balanced sampled pod schedules for very large deck pools

## Target architecture

```text
ArenaMonteCarlo
├── Schedule
│   ├── RoundRobin                 two-player compatibility
│   ├── FixedPod                   exactly four selected competitors
│   └── FourPlayerCombinations     every C(n, 4) pod
├── SeatingPolicy
│   ├── Canonical                  one seating, useful for tests/replay
│   ├── Cyclic                     four rotations, faster approximation
│   └── AllPermutations            24 seatings, Commander default
├── ContestSimulator
│   ├── GoldfishRaceModel          multiplayer baseline
│   └── InteractiveTurnModel
├── Pilot registry
│   ├── LinearPilot
│   └── ThreatAwarePilot
└── Reducers
    ├── ContestReport
    ├── seat-conditioned records
    ├── standings
    └── response metrics/examples
```

The scheduling layer selects who participates. The seating layer selects their
order. The simulator resolves one game. Monte Carlo decides how many random
samples to execute. These remain separate extension points.

## Core data-model changes

The exact names may adjust during implementation, but the responsibilities are
fixed:

```rust
pub struct Contest {
    pub id: ContestId,
    /// Canonical competitor order, independent of seating.
    pub competitor_indices: Vec<usize>,
}

pub struct Seating {
    /// seat -> index within Contest::competitor_indices
    pub contest_slots: Vec<usize>,
}

pub struct TrialId {
    pub contest_id: ContestId,
    pub sample_index: usize,
    pub seating_index: usize,
}

pub struct TrialContext {
    pub id: TrialId,
    pub sample_seed: u64,
    pub seating: Seating,
}

pub enum ContestResult {
    Winner { seat: usize },
    Draw,
}
```

`Matchup`, `MatchupId`, and `MatchSimulator` should remain as compatibility
aliases or thin wrappers when that does not complicate the generalized core.
Because the crate is still `0.1`, a small documented API correction is
acceptable, but avoid needless churn.

Reports change from fixed arrays to seat-aligned vectors:

```rust
pub struct ContestReport {
    pub competitor_ids: Vec<String>,
    pub records: Vec<Record>,
    pub records_by_seat: Vec<Vec<Record>>,
    pub examples: OutcomeExamples,
    pub response_metrics: ResponseMetrics,
}
```

Every vector length must be validated against the contest seat count before
parallel execution begins.

## Deterministic sampling and seating

For four-player `AllPermutations`, one sample produces 24 games:

```text
contest seed = derive(master seed, stable contest identity)
sample seed  = derive(contest seed, sample index)
seating      = lexicographic permutation[seating index]
deck stream  = derive(sample seed, "competitor", competitor ID)
pilot stream = derive(sample seed, "pilot", competitor ID)
```

Deck and pilot streams do not include the seating index. That is deliberate:
the same random sample is replayed through every seat order, reducing variance
when measuring starting-position and priority-order effects.

The public Monte Carlo configuration should describe **samples per contest**,
not ambiguous trials per matchup. Reports expose both sample count and expanded
game count. Replay IDs identify all three dimensions directly.

For `n` registered decks:

- `FixedPod` creates one contest.
- `FourPlayerCombinations` creates `C(n, 4)` contests.
- each contest executes `samples × seating_count` games.
- callers must receive the projected game count before execution so accidental
  combinatorial explosions are visible.

## Interactive state

```rust
pub struct PlayerState {
    pub competitor_index: usize,
    pub library: Deck,
    pub hand: Vec<Card>,
    pub battlefield: Vec<Card>,
    pub graveyard: Vec<Card>,
    pub lands_in_play: usize,
    pub available_mana: usize,
    pub alive: bool,
}

pub struct StackItem {
    pub id: StackItemId,
    pub controller: usize,
    pub source: Card,
    pub role: CardRole,
    pub effect: Effect,
    pub countered: bool,
}

pub enum Effect {
    WinAttempt,
    Counter { target: StackItemId },
}

pub struct InteractiveGameState {
    pub turn: usize,
    pub active_seat: usize,
    pub priority_seat: usize,
    pub consecutive_passes: usize,
    pub players: Vec<PlayerState>,
    pub stack: Vec<StackItem>,
}
```

The first `CardEvaluator` may interpret tags such as `land`, `threat`,
`interaction`, `protection`, and `cost:N`. The evaluator returns typed profiles;
the state machine never parses tags directly.

`protection` and `interaction` cards can both yield `Effect::Counter`. Their
roles are retained on `StackItem` for pilot preferences and metrics. This
permits chains such as:

```text
WinAttempt(A)
  Counter(B -> WinAttempt)
    Counter(A -> Counter(B))       protection
      Counter(C -> protection)
        Counter(D -> Counter(C))
```

LIFO resolution lets D's response counter C's response, A's protection then
counters B's interaction, and A's win attempt resolves. Any link may itself be
answered if a player has a legal card and sufficient mana.

## Priority-window semantics

1. A legal cast or activation pushes one `StackItem` and resets consecutive
   passes to zero.
2. The acting player retains priority after adding an item.
3. Passing advances priority clockwise to the next living player.
4. If all living players pass while the stack is non-empty, resolve its top
   item, reset passes, and give priority to the active player.
5. A resolving `Counter` marks its unresolved target countered. A countered item
   has no effect when it reaches the top.
6. A resolving `WinAttempt` ends the game for its controller.
7. If all living players pass with an empty stack, close the priority window and
   advance the phase or turn.
8. A response window has no response-depth cap. A configurable action watchdog
   exists only to identify non-terminating or invalid pilots and returns a
   `SimulationError` rather than a game result.

The engine owns legality. Pilots choose only from legal actions exposed in
`PlayerView`; custom pilots that manufacture invalid actions cause a typed
error with the trial and seat attached.

## Pilot contract and information boundary

```rust
pub trait Pilot: Send + Sync {
    fn choose_action(
        &self,
        view: &PlayerView<'_>,
        decision: DecisionPoint,
        rng: &mut dyn RngCore,
    ) -> Action;
}

pub enum DecisionPoint {
    MainAction,
    Priority,
}
```

`PlayerView` contains:

- the pilot's hand and available mana
- all battlefields, graveyards, life/alive state, and public commanders
- active seat, priority seat, turn, and the entire public stack
- legal actions and legal targets
- opponent hand sizes, never opponent card identities

`LinearPilot` spends resources to advance and present its own win. The
`ThreatAwarePilot` holds responses when a win attempt is plausible and targets
the currently resolving threat. Neither pilot contains special-case knowledge
of competitor names.

## Execution plan

Each work package should be independently testable and committed atomically.

### Package 1 — Generalize the arena to contests

**Primary files:** `src/arena/mod.rs`, `model.rs`, `report.rs`, `runner.rs`,
`schedule.rs`, `tests/arena.rs`, `examples/round_robin.rs`

1. Replace fixed two-seat storage with validated dynamic contest/seating types.
2. Generalize result validation, accumulator loops, outcome examples, and
   standings aggregation to the contest seat count.
3. Change trial identity to contest/sample/seating coordinates.
4. Make `MatchSimulator::simulate` return a typed result so invalid custom
   models can fail a run instead of falsifying aggregates.
5. Generalize `GoldfishRaceModel` to select the earliest win among any number of
   players; equal-turn ties use seating order or draw according to `TiePolicy`.
6. Preserve and update the two-player round-robin example and tests.

**Verification:**

- two-player results remain deterministic across worker counts
- a scripted four-seat simulator aggregates one winner and three losses
- invalid winner seats and malformed report vectors return errors
- zero samples and empty schedules remain well-defined

**Suggested commit:** `refactor: generalize arena contests and reports`

### Package 2 — Add four-player pod schedules and seat balancing

**Primary files:** `src/arena/schedule.rs`, `runner.rs`, `report.rs`,
`tests/pods.rs`

1. Add `FixedPod` and `FourPlayerCombinations` with duplicate and seat-count
   validation.
2. Add `SeatingPolicy::{Canonical, Cyclic, AllPermutations}`.
3. Generate all 24 four-seat permutations in stable lexicographic order.
4. Expand each Monte Carlo sample across its seating policy while retaining
   common competitor RNG streams.
5. Report projected and actual game counts plus seat-conditioned records.
6. Make replay regenerate the exact seating and sample streams from `TrialId`.

**Verification:**

- five decks produce exactly five four-player combination contests
- all-permutations produces 24 unique seatings
- each competitor appears six times in each seat per sample
- the same competitor receives the same shuffle across all 24 seatings
- results and replay are identical with one and four Rayon workers
- aggregate totals equal `contests × samples × seatings`

**Suggested commit:** `feat: add balanced four-player pod schedules`

### Package 3 — Extract reusable game initialization

**Primary files:** `src/engine.rs`, new `src/opening.rs`, `src/deck.rs`,
`tests/simulation.rs`, new `tests/opening.rs`

1. Extract shuffle, opening draw, London mulligan, bottoming, and retained-hand
   state into a crate-internal helper.
2. Return hand, remaining library, bottomed cards, opening lands, kept count,
   and mulligan count.
3. Make the existing single-deck engine consume the helper without changing its
   public outcomes or seeded results.
4. Initialize every pod player through its competitor-keyed sample stream.

**Verification:**

- all existing single-deck tests and seeded aggregates remain unchanged
- bottomed cards remain in the library in the documented order
- four players initialize independently and reproducibly

**Suggested commit:** `refactor: share opening-hand initialization`

### Package 4 — Implement the recursive priority engine

**Primary files:** new `src/interactive/mod.rs`, `state.rs`, `cards.rs`,
`priority.rs`, `pilot.rs`; `src/lib.rs`; new `tests/priority.rs`

1. Add player zones, turn resources, typed card profiles, and action legality.
2. Add `StackItemId`, `StackItem`, `Effect`, and LIFO resolution.
3. Implement clockwise priority, consecutive passes, post-resolution priority,
   and empty-stack window closure.
4. Allow `Counter` effects to target any unresolved legal stack item, including
   another `Counter`.
5. Add a non-gameplay watchdog that returns `SimulationError` with a replay ID.
6. Build hidden-information-safe `PlayerView` and legal-action generation.
7. Add deterministic action tracing for selected replay examples.

**Verification:**

- no response: a win attempt resolves
- one response: interaction stops the win
- two responses: protection restores the win
- at least five nested responses resolve correctly in LIFO order
- insufficient cards or mana prevents a response
- all-pass behavior resolves one item at a time and then closes the window
- illegal targets and watchdog exhaustion fail rather than count as draws
- opponent private zones never appear in `PlayerView`

**Suggested commit:** `feat: add recursive multiplayer priority engine`

### Package 5 — Add pilots, metrics, and the four-player vertical example

**Primary files:** `src/interactive/pilot.rs`, `src/arena/report.rs`, new
`examples/commander_pod.rs`, new `tests/commander_pod.rs`, `README.md`

1. Implement `LinearPilot` and `ThreatAwarePilot` over legal action lists.
2. Record win attempts, responses by role, response-chain depth, stopped wins,
   protected wins, responder identity, invalid actions, and turn-to-win.
3. Add representative replay IDs for deepest chain, each winner, and draws.
4. Build a deterministic four-deck example: fast linear combo, protected combo,
   reactive control, and a slower resilient deck.
5. Demonstrate a game containing at least three distinct responders and a
   response chain deeper than two.
6. Document runtime expansion: samples, contests, seatings, and total games.

**Verification:**

- no-interaction four-player toy games agree with multiplayer
  `GoldfishRaceModel`
- response metrics reconcile with the replay trace
- standings sum to one winner plus three losses per decisive pod game
- the release example completes without retaining every trial
- `cargo fmt --all -- --check`
- `cargo test --all-targets`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo doc --no-deps`

**Suggested commit:** `feat: simulate interactive commander pods`

## Final acceptance criteria

- Four decks can run in one pod through all 24 seat permutations.
- More than four registered decks can run every unique four-deck combination.
- An arbitrary number of legal responses can be added until resources run out
  or all players pass.
- A counter can target another counter, producing correct LIFO outcomes.
- At least three different opponents can participate in the same response
  window.
- Seat position and clockwise priority effects are separately reportable.
- Trial replay reproduces seating, opening hands, actions, stack resolution, and
  winner across worker counts.
- Custom pilots cannot observe hidden opponent cards.
- Safety failures abort with typed errors and are excluded from statistics.
- Existing two-player and single-deck workflows still compile and pass.

## Risks and mitigations

- **Combinatorial runtime:** `C(n,4) × samples × 24` grows quickly. Report the
  projected game count before execution and require explicit opt-in above a
  configurable warning threshold.
- **False fidelity:** A stack-like model can look more exact than it is. Name it
  `InteractiveTurnModel`, document supported effects, and keep unsupported
  mechanics explicit.
- **Pilot loops:** Legal-action lists plus a non-gameplay action watchdog prevent
  non-termination. Watchdog hits are errors, not simulated outcomes.
- **Public API churn:** Land the generalized contest types in one atomic commit,
  retain compatibility aliases where simple, and update every example together.
- **Biased seat comparison:** Group trials by sample and execute the complete
  seating set with competitor-keyed streams.
- **Metric memory growth:** Reduce typed counters and histograms incrementally;
  retain only bounded representative traces and replay IDs.

## Resume point

Begin with Package 1. Before editing, run the existing test suite and inspect
the two uncommitted user changes in `README.md` and `src/metrics.rs`; preserve
them unless the user explicitly asks to include them. Do not begin the priority
engine while fixed two-seat assumptions remain in the arena core.
