# mindcrank

`mindcrank` is a small Rust library for simulating focused Magic: The Gathering
deck-building scenarios with Monte Carlo simulation.

It intentionally does not implement the full Magic rules engine. Cards carry
free-form tags, while traits define win conditions, mulligan policies, and
London-mulligan bottoming heuristics. This keeps simple setups simple and
leaves room for custom tutors, draw engines, or turn-plan logic.

`mindcrank` is intended to help develop Pareto frontier analysis for MTG decks
in the pursuit of a better toolkit and framework for understanding and brewing
MTG decks.

## Included

- Composable `WinCondition` implementations (`TwoCardSet`, `KOfTag`, `AnyOf`)
- London mulligans with replaceable keep and bottom policies
- Parallel Monte Carlo runs with a bounded Rayon worker pool
- Reproducible results across worker counts when a seed is supplied
- Explicit wins and misses instead of treating the simulation horizon as a win
- A tag-based card and deck model

## Mindcrank simulation skill

`.claude/skills/mindcrank-simulate/` is an agent skill for running a simulation
against a decklist you paste in. Point a coding agent at this repo and ask it to
read `.claude/skills/mindcrank-simulate/SKILL.md`; Claude Code discovers the
skill automatically. It covers tagging a decklist, choosing a win condition and
mulligan policy, and reading the results, and ships a template harness that
parses Arena and Moxfield exports.

## Example

```rust
use mindcrank::{
    Card, Deck, KeepIfWinOrDecent, MonteCarloParams, Params, TwoCardSet,
    monte_carlo,
};

let mut cards = vec![Card::new("Land").with_tag("land"); 37];
cards.push(Card::new("Thassa's Oracle").with_tag("combo:oracle"));
cards.push(Card::new("Demonic Consultation").with_tag("combo:consult"));
cards.extend(vec![Card::new("Filler"); 60]);

let deck = Deck::new(cards);
let win = TwoCardSet::new("combo:oracle", "combo:consult");
let mulligan = KeepIfWinOrDecent::new(&win, 2, 4);
let params = Params::new(&deck, &win).london_mulligan(&mulligan, 3);

let result = monte_carlo(
    MonteCarloParams::new(params, 100_000)
        .with_seed(42)
        .with_workers(0),
);

println!("win rate: {:.2}%", result.win_rate * 100.0);
```

Run the complete example:

```sh
cargo run --release --example two_card_combo
```

That deck is 99 cards — 37 lands, one Thassa's Oracle, one Demonic Consultation,
and 60 inert filler cards — kept on two to four lands with up to three London
mulligans, over one million trials on a fixed seed:

```
Trials: 1000000
Wins by turn 50: 32.84%
Average draws after opening (wins): 31.62
Opening win rate: 0.5770%
Average opening lands: 2.83
Average turns to win (wins): 31.62
```

`distribution_draws_to_win` holds the shape behind that headline. Turn 0 is the
kept opening hand, and the draws and turns figures coincide here only because
`draws_per_turn` is 1:

| Turn | Both pieces in hand |
| ---- | ------------------- |
| 0    | 0.58%               |
| 3    | 1.05%               |
| 5    | 1.48%               |
| 10   | 2.89%               |
| 20   | 7.27%               |
| 30   | 13.71%              |
| 50   | 32.84%              |

Two singletons in 99 cards with no way to find them is a deliberately punishing
baseline, and it doubles as the engine's correctness check: 32.84% by turn 50
sits on the hypergeometric odds of both cards falling in the top 57 of the deck,
(57/99)(56/98) = 32.90%, a shade under because mulligans see fewer cards. The
averages cover winning trials only, so read them next to the win rate rather
than alone. Real decks close the gap with tutors and draw spells — that is what
the tag model and custom win conditions exist to express.

Run the test suite:

```sh
cargo test
```

## Extending it

Implement `WinCondition` for richer combo logic, `MulliganPolicy` for a
deck-specific keep strategy, or `BottomHeuristic` for more accurate London
mulligans. Tutor timing and additional draw engines belong in custom policies
or a future turn-plan layer rather than being approximated by the core crate.
