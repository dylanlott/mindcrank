# mindcrank API reference

Everything the crate exports, and how to extend it. Read `SKILL.md` first for the
workflow; this file is for when the template's four sections are not enough.

## Model

`Card` is a name, an optional type, and a set of free-form tags. Nothing else.
All simulation logic reads tags.

```rust
Card::new("Thassa's Oracle")
    .with_type("Creature")                       // optional, unused by the engine
    .with_tags(["combo:oracle", "creature"])
```

`Deck` wraps `Vec<Card>` and is immutable during a run: each trial shuffles a
copy, so the same `Deck` is shared across all threads.

```rust
Deck::new(cards);
deck.cards() -> &[Card];
deck.len();
count_tag(deck.cards(), "land") -> usize;   // free function, works on any &[Card]
```

## One trial

```rust
pub struct Params<'a> {
    pub deck: &'a Deck,
    pub win: &'a dyn WinCondition,
    pub hand_size: usize,            // default 7
    pub max_turns: usize,            // default 50 — the horizon, in draw steps
    pub draws_per_turn: usize,       // default 1
    pub use_london_mulligan: bool,   // default false
    pub max_mulligans: usize,        // clamped to hand_size
    pub mulligan: Option<&'a dyn MulliganPolicy>,
    pub bottom_heuristic: Option<&'a dyn BottomHeuristic>,
    pub seed: Option<u64>,
}
```

Builder methods return `Self`; the plain fields are public, so set them directly:

```rust
let mut params = Params::new(&deck, &win)
    .london_mulligan(&policy, 3)     // enables London + sets max_mulligans
    .bottom_with(&heuristic)         // optional; DefaultBottomHeuristic otherwise
    .with_seed(42);
params.max_turns = 8;
params.draws_per_turn = 1;
```

`run_once(&params) -> TrialOutcome` plays a single game. Useful for debugging a
custom policy; use `monte_carlo` for real numbers.

### Turn sequencing

1. Shuffle a fresh copy of the deck, draw `hand_size`.
2. If London is on, ask the policy to keep. On a mulligan, **reshuffle the whole
   deck** and draw a fresh `hand_size`. Repeat up to `max_mulligans`, then keep
   whatever is in hand.
3. On the keep, bottom exactly `mulligans_taken` cards chosen by the bottom
   heuristic; they go to the bottom of the library and are effectively gone.
4. Check the win condition against the opening hand — a hit here is
   `turns_to_win: Some(0)` and `opening_win: true`.
5. For turns `1..=max_turns`: draw `draws_per_turn`, add to hand, check the win
   condition. Stop early if the library empties.

Cards never leave the hand. The hand is cumulative, so `max_turns` draws means
`kept + max_turns * draws_per_turn` cards seen.

## Monte Carlo

```rust
let aggregate = monte_carlo(
    MonteCarloParams::new(params, 200_000)
        .with_seed(0x5eed)   // falls back to params.seed, then to a random seed
        .with_workers(0),    // 0 = Rayon's global pool; N = a bounded pool of N
);
```

Each trial's RNG is seeded by `splitmix64(master_seed + index)`, so results are
identical across worker counts and machines for a given seed. Two runs that
differ only in `workers` must produce equal `Aggregate` values — the test suite
asserts this.

Trial counts: 100k is enough for a headline figure (±0.3pp at p≈0.5), 1M when
resolving two variants that sit within a point of each other. Sampling error is
`sqrt(p(1-p)/n)`; the template prints the 95% interval.

## Metrics

```rust
pub struct TrialOutcome {
    pub won: bool,
    pub draws_after_opening: usize,  // for a miss: draws taken before the horizon
    pub opening_win: bool,
    pub opening_lands: usize,        // lands in the provisional hand, pre-bottoming
    pub kept: usize,                 // cards retained after bottoming
    pub turns_to_win: Option<usize>, // None on a miss — never conflated with a slow win
}

pub struct Aggregate {
    pub trials: usize,
    pub wins: usize,
    pub misses: usize,
    pub win_rate: f64,
    pub avg_draws_after_opening: Option<f64>,  // winning trials only
    pub opening_win_rate: f64,
    pub avg_opening_lands: f64,                // all trials
    pub avg_turns_to_win: Option<f64>,         // winning trials only
    pub distribution_draws_to_win: BTreeMap<usize, usize>,  // draws -> winning trials
}
```

The `Option` averages are `None` when there were no wins, rather than `0.0`.
Reaching the horizon is a miss, never a win — the crate does not treat "ran out
of turns" as success.

`distribution_draws_to_win` is keyed by **draws**, not turns. With
`draws_per_turn: 1` they coincide; otherwise turn N is key `N * draws_per_turn`.
Summing keys `0..=N` gives the cumulative curve the template prints.

## Built-in win conditions

```rust
TwoCardSet::new("combo:a", "combo:b")   // >=1 of each tag
KOfTag::new("artifact", 3)              // >=k cards with the tag
AnyOf::new(vec![Box::new(x), Box::new(y)])  // any child satisfied
```

`AnyOf` also has `push(impl WinCondition + 'static)`.

## Custom win conditions

Implement two methods. `satisfied` is the logic; `card_priority` tells the
default bottom heuristic what to protect during mulligans (higher = keep, 0 =
expendable, and the built-ins use 100 for relevant pieces).

```rust
use mindcrank::{Card, WinCondition, count_tag};

/// Needs the payoff, plus either enabler, plus two mana rocks.
struct RockCombo;

impl WinCondition for RockCombo {
    fn satisfied(&self, hand: &[Card]) -> bool {
        let payoff = count_tag(hand, "combo:payoff") > 0;
        let enabler = count_tag(hand, "combo:enabler:a") > 0
            || count_tag(hand, "combo:enabler:b") > 0;
        payoff && enabler && count_tag(hand, "rock") >= 2
    }

    fn card_priority(&self, card: &Card) -> i32 {
        if card.has_tag("combo:payoff") {
            100
        } else if card.has_tag("combo:enabler:a") || card.has_tag("combo:enabler:b") {
            90
        } else if card.has_tag("rock") {
            60
        } else {
            0
        }
    }
}
```

`WinCondition` requires `Send + Sync`, since trials run in parallel. Keep
implementations pure — no interior mutability, no counters.

Put custom types in the example file above `main`. Because `win_condition()` in
the template returns `impl WinCondition`, swapping the body is all that is
needed. Return `Box<dyn WinCondition>` if you need to pick a condition at
runtime.

## Custom mulligan policies

```rust
pub trait MulliganPolicy: Send + Sync {
    fn keep(&self, opening_hand: &[Card]) -> bool;
}
```

Built-ins: `KeepIfLandsBetween::new(min, max)`, `KeepIfWinOrDecent::new(&win,
min_lands, max_lands)`, and `KeepIf::new(closure)` for one-offs:

```rust
let policy = KeepIf::new(|hand: &[Card]| {
    let lands = count_tag(hand, "land");
    (2..=5).contains(&lands) && count_tag(hand, "combo:payoff") > 0
});
```

The policy cannot see how many mulligans have already been taken, so it cannot
loosen as the hand shrinks — a real limitation of the current engine. It is also
called on every attempt, and the last hand is force-kept once `max_mulligans` is
reached, so a policy that never accepts still terminates.

## Custom bottom heuristics

```rust
pub trait BottomHeuristic: Send + Sync {
    fn cards_to_bottom(&self, hand: &[Card], count: usize, win: &dyn WinCondition) -> Vec<usize>;
}
```

Return hand indices to bottom. `DefaultBottomHeuristic` ranks by
`win.card_priority(card)` against a floor of 80 for lands and 70 for
`tutor`/`draw`, then bottoms the lowest. Note the consequence: a `card_priority`
below 80 will not protect a card ahead of a land.

The engine repairs bad output — out-of-range, duplicate, or too-few indices are
completed from the default heuristic — so a custom heuristic can never change the
hand size. Overriding `card_priority` on the win condition is usually enough;
write a heuristic only when the choice depends on the hand as a whole (for
example, bottoming the 5th land only when a combo piece is present).

## Invariants worth preserving

If you change the engine, these are the properties the test suite pins:

- A fixed seed gives identical `Aggregate` values at any worker count.
- Kept hand size is exactly `hand_size - mulligans_taken`, including the
  0-mulligan case.
- `Deck::draw_n` is non-destructive; the shared `Deck` is never mutated.
- A miss has `turns_to_win: None` and is counted in `misses`.

Validate any engine change against exact hypergeometric math: one copy in a
99-card deck, kept at 7 with `max_turns = 10`, must be found 17/99 = 17.17% of
the time.
