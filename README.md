# 🤖 mindcrank

`mindcrank` is a small Rust library for simulating focused Magic: The Gathering
deck-building scenarios with Monte Carlo simulations.

`mindcrank` is _not_ a full Magic rules engine. Cards carry free-form tags, while
traits define win conditions, mulligan policies, and London-mulligan bottoming
heuristics. This keeps simple setups simple and leaves room for custom tutors,
draw engines, or turn-plan logic.

`mindcrank` is intended to help develop Pareto frontier analysis for MTG decks
in the pursuit of a better toolkit and framework for understanding and brewing
MTG decks.

## Crate Contents

- Composable `WinCondition` implementations (`TwoCardSet`, `KOfTag`, `AnyOf`)
- London mulligans with replaceable keep and bottom policies
- Parallel Monte Carlo runs with a bounded Rayon worker pool
- Reproducible results across worker counts when a seed is supplied
- Explicit wins and misses instead of treating the simulation horizon as a win
- A tag-based card and deck model
- Strict two-axis Pareto comparisons with plot-ready scatter data

## Requirements

Rust 1.97 or newer, on edition 2024. `rust-toolchain.toml` pins this repo to the
stable channel, so `cargo build` and `cargo test` use stable even when your
default toolchain is something else.

## Simulation Skill

`.claude/skills/mindcrank-simulate/` is an agent skill for running a simulation
against a decklist that you paste in. Point a coding agent at this repo and ask it to
read `.claude/skills/mindcrank-simulate/SKILL.md`; Claude Code discovers the
skill automatically. It covers tagging a decklist, choosing a win condition and
mulligan policy, and reading the results, and ships a template harness that
parses Arena and Moxfield decklist export formats.

## Baseline Simulation Example

This is a simple example that models a Thoracle win condition as a baseline comparison.

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

`distribution_draws_to_win` draws a curve around the win percentage. Turn 0 is the
kept opening hand, and the draws and turns figures coincide here because
`draws_per_turn` is 1:

| Turn | Combo in hand |
| ---- | ------------- |
| 0    | 0.58%         |
| 3    | 1.05%         |
| 5    | 1.48%         |
| 10   | 2.89%         |
| 20   | 7.27%         |
| 30   | 13.71%        |
| 50   | 32.84%        |

Two singletons in 99 cards with no way to find them is an intentionally punishing
baseline and a case that functions as a good sanity check for results: 32.84%
by turn 50 sits on the hypergeometric odds of both cards falling in the top 57
of the deck, (57/99)(56/98) = 32.90%, a shade under because mulligans see
fewer cards. The averages cover winning trials only, so read them next to the
win rate rather than alone. Real decks close the gap with tutors and draw
spells — that is what the tag model and custom win conditions exist to express.

## Pareto Deck Comparisons

Compare named decklist mutations under one fixed simulation protocol with
`compare_pareto`. Its two axes are cumulative `P(win by T_fast)` and
`P(win by T_horizon)`. A candidate is omitted from the frontier only when a
different candidate is at least as good on both axes and strictly better on one.
The report retains every candidate and exposes `scatterplot()` data for a UI or
CLI to render; mindcrank itself does not prescribe a renderer.

```rust
use mindcrank::{
    Card, Deck, DeckCandidate, KOfTag, ParetoProtocol, Params, compare_pareto,
};

let glass_cannon = Deck::new(vec![Card::new("Win").with_tag("win"); 20]);
let balanced = Deck::new(vec![Card::new("Win").with_tag("win"); 10]);
let win = KOfTag::new("win", 1);

let protocol = ParetoProtocol::from_params(
    Params::new(&glass_cannon, &win),
    4,  // early-win threshold
    10, // simulation horizon and consistency threshold
    100_000,
    42, // shared seed for every candidate
).unwrap();

let report = compare_pareto(
    &[
        DeckCandidate::new("glass", "Glass cannon", &glass_cannon),
        DeckCandidate::new("balanced", "Balanced", &balanced),
    ],
    protocol,
).unwrap();

for point in report.scatterplot().points {
    println!("{}: ({:.1}%, {:.1}%)", point.label, point.x * 100.0, point.y * 100.0);
}

let csv = report.to_csv(false);
println!("{}", csv);
```

See [`docs/pareto-frontier-spec.md`](docs/pareto-frontier-spec.md) for the
evaluation-protocol contract, result semantics, and planned extensions.

## Arena Simulations

The `arena` module runs deck plans through dynamic contests and a balanced seat rotation.

Its first model, `GoldfishRaceModel`, compares when each deck reaches its existing
`WinCondition` across any number of players - it deliberately doesn't model opposing
interaction. `ArenaMonteCarlo::new(n)` runs `n` random samples per contest, reusing each
deck-specific shuffle across every cyclic seating, and every reported example
can be replayed from the run seed and
its contest/sample/seating `TrialId`.

Run the three-deck round-robin example:

```sh
cargo run --release --example round_robin
```

See [`docs/interactive-simulation-plan.md`](docs/interactive-simulation-plan.md)
for the next slice: a coarse turn model with pilots, threats, protection, and
disruption.

Run the test suite:

```sh
cargo test
```

## Customization

Implement `WinCondition` for richer combo logic, `MulliganPolicy` for a
deck-specific keep strategy, or `BottomHeuristic` for more accurate London
mulligans. Tutor timing and additional draw engines belong in custom policies
or a future turn-plan layer rather than being approximated by the core crate.

## Roadmap

- [x] Calculate Pareto frontiers for a controlled set of decklist variants
- [ ] Create a leaderboard of decklists scored on their Pareto frontiers
