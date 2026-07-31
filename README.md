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

Run the test suite:

```sh
cargo test
```

## Extending it

Implement `WinCondition` for richer combo logic, `MulliganPolicy` for a
deck-specific keep strategy, or `BottomHeuristic` for more accurate London
mulligans. Tutor timing and additional draw engines belong in custom policies
or a future turn-plan layer rather than being approximated by the core crate.
