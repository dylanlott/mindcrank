# Next slice: abstract interactive matches

## Goal

Add coarse opponent interaction to competitive simulations without turning
`mindcrank` into a complete Magic rules engine. The arena, round-robin schedule,
Monte Carlo runner, replay IDs, and reports remain unchanged.

## User-visible outcome

A caller can assign each competitor a `Pilot` and run an
`InteractiveTurnModel`. Decks can develop resources, present a win, disrupt an
opponent, protect their own plan, or pass. Reports distinguish clean wins,
protected wins, disrupted wins, and horizon draws.

## Proposed model

```rust
pub trait Pilot: Send + Sync {
    fn choose_action(&self, view: &PlayerView<'_>) -> Action;
}

pub enum Action {
    Develop,
    Draw { cards: usize },
    Tutor { tag: String },
    ThreatenWin,
    Disrupt { target_seat: usize },
    Protect,
    Pass,
}

pub struct InteractiveGameState {
    pub turn: usize,
    pub active_seat: usize,
    pub players: Vec<PlayerState>,
    pub pending_win: Option<PendingWin>,
}
```

Cards continue to use tags. The first vocabulary should recognize `land`,
`draw`, `tutor`, `threat`, `interaction`, and `protection`. A model-specific
card evaluator converts those tags into legal coarse actions.

## Work breakdown

1. Introduce `Pilot`, immutable `PlayerView`, `Action`, and an object-safe
   `CardEvaluator`. Keep policies separate from decks so reports compare a
   deck-and-pilot pairing honestly.
2. Add per-player zones and resources: library, hand, battlefield tags,
   graveyard, available mana, and land plays. Reuse the current London
   mulligan flow by extracting it behind a shared opening-hand helper.
3. Implement a deterministic turn loop with named RNG streams for shuffling,
   pilot choices, and random targeting. Resolve one pending win window before
   advancing the active player.
4. Supply two baseline pilots: `LinearPilot`, which advances its own plan, and
   `ThreatAwarePilot`, which preserves interaction when an opponent is close to
   winning.
5. Extend `MatchOutcome` with typed observations rather than free-form strings:
   actions taken, disruption attempts, protection successes, mana misses, and
   winner archetype. Add reducers without storing every trial.
6. Add a three-deck example demonstrating that an interactive deck can lose a
   goldfish race but improve against a faster linear deck when piloted
   defensively.

## Acceptance criteria

- A scripted interaction card can stop a pending win, and a protection card can
  counter that disruption.
- Hidden opponent zones are absent from `PlayerView`.
- Legal actions cannot create cards, spend unavailable resources, or target a
  missing player.
- Fixed seeds replay the same action sequence across worker counts.
- Mirrored trials preserve deck-specific shuffles and rotate the starting
  player.
- Existing single-deck and `GoldfishRaceModel` results remain unchanged.
- A no-interaction `LinearPilot` scenario agrees with the goldfish race on an
  exact toy deck.

## Explicit non-goals

- Comprehensive card text, priority, layers, or the full stack
- Rules enforcement for arbitrary Magic cards
- Multiplayer politics or learned policies
- Sideboarding and best-of-three matches; these should follow once the action
  model is stable
