---
name: mindcrank-simulate
description: Run Monte Carlo consistency simulations on a Magic: The Gathering decklist using the mindcrank Rust crate. Use when asked how often a deck assembles a combo, finds a card, or hits its land drops — win rate, consistency, "what are the odds", turn-by-turn assembly curves, or comparing two versions of a decklist.
---

# Simulating a decklist with mindcrank

mindcrank answers one shape of question: **how often does a shuffled decklist put
the cards you need into your hand, and by when?** It draws opening hands, takes
London mulligans, then draws one card per turn until a tag-based win condition is
satisfied or the horizon runs out, across many trials in parallel.

It is not a rules engine. There is no mana, no stack, no opponent, no tutors
actually resolving, no card leaving your hand. "Won" means *assembled in hand*.
Say so when reporting results — a 40% assembly rate is not a 40% win rate.

## Before writing any code

Resolve three things. Infer what you reasonably can from the decklist and state
your assumptions; ask the user only when a wrong guess would invalidate the run.

1. **What counts as assembled?** The specific cards or card categories. "Thassa's
   Oracle + Demonic Consultation", "any 3 artifacts", "1 of 4 wincons plus a
   ritual". This becomes the `WinCondition`.
2. **By what turn?** A Commander combo deck cares about turns 3-6; a 60-card
   aggro deck cares about turns 1-4. Sets `MAX_TURNS`.
3. **Format.** 60-card with a sideboard, or 99 + commander? Commanders belong in
   the command zone, not the library — see Pitfalls.

## Workflow

**1. Sanity-check the repo.** `cargo test` from the repo root. All tests must
pass before you trust any number you produce.

**2. Copy the template.**

```sh
cp .claude/skills/mindcrank-simulate/templates/simulate.rs examples/sim_scratch.rs
```

`examples/sim_scratch.rs` is gitignored. Rename it (and the `--example` argument)
to keep a simulation around.

**3. Fill in the four numbered sections.** Paste the decklist verbatim into
`DECKLIST`, write the tag groups, set the horizon, write the win condition. See
Tagging and Win conditions below.

**4. Run it.**

```sh
cargo run --release --example sim_scratch
```

`--release` is not optional; debug builds are roughly 20x slower. 200k trials
takes a second or two. Use 1M when comparing two close variants.

**5. Check the deck report before reading results.** The harness prints library
size and a count per tag, and warns about tagged card names it could not find in
the decklist. A `WARNING` there means a typo — fix it and rerun. A library size
that isn't 60 or 99 means the decklist or the command zone is wrong.

## Tagging

Tags are free-form. Only tag what the win condition or a mulligan policy reads;
everything else is inert filler, which is correct and expected.

```rust
const TAG_GROUPS: &[(&str, &[&str])] = &[
    ("land", &["Island", "Polluted Delta", "Underground Sea"]),
    ("combo:oracle", &["Thassa's Oracle"]),
    ("combo:consult", &["Demonic Consultation", "Tainted Pact"]),
    ("draw", &["Brainstorm", "Ponder"]),
];
```

Names match case-insensitively and ignore apostrophe style. One card can carry
several tags. Group *by role*, not by card — `combo:consult` above covers both
cards that fill that slot, so the win condition stays a simple pair.

Three tag names are read by the engine itself:

| Tag | Effect |
| --- | --- |
| `land` | Drives the `avg_opening_lands` metric, `KeepIfLandsBetween`, `KeepIfWinOrDecent`. **Always tag every land** — an untagged manabase silently makes every mulligan policy nonsense. |
| `tutor`, `draw` | Bottoming priority only: kept over filler during London mulligans. |

## Win conditions

| Question | Use |
| --- | --- |
| Two specific pieces | `TwoCardSet::new("combo:a", "combo:b")` |
| Any k cards with a tag | `KOfTag::new("artifact", 3)` |
| Several independent routes | `AnyOf::new(vec![Box::new(a), Box::new(b)])` |
| Anything else | Implement `WinCondition` — see `reference.md` |

Land drops are a `KOfTag`: `KOfTag::new("land", 4)` with `MAX_TURNS = 3` asks how
often you have four lands in hand by turn 3.

Anything conditional — "piece A *and* either B or C", "2 mana rocks plus a
payoff", counting across categories — needs a hand-written `WinCondition`. It is
a single method over `&[Card]`; `reference.md` has a worked example.

## Mulligan policy

The template uses `KeepIfWinOrDecent`, which keeps an already-assembled hand or
one inside a land window. Set `KEEP_LANDS` to what a real player would ship:
`(2, 5)` for a 99-card deck with ~37 lands, `(2, 4)` for a leaner 60-card deck.
`MAX_MULLIGANS = 2` is a reasonable default; 3 for a deck that hard-mulligans
for a combo.

Mulligan settings move results by percentage points, so state them alongside any
number you report. For a deck-vs-deck comparison, hold them constant.

## Reading the output

- **Assembled by turn N** — the headline. The `95% CI` is Monte Carlo sampling
  error on that percentage *only*; it says nothing about whether the model
  matches real games. If two variants' intervals overlap, you have not shown a
  difference — raise `TRIALS`.
- **In the opening hand** — assembled before drawing. Note this exceeds the raw
  hypergeometric odds, because mulliganing re-rolls the opener.
- **Average turns to assemble** — winning trials only, so it is conditional. A
  deck that assembles 20% of the time can still show a low average. Read it
  next to the headline, never alone.
- **Cumulative assembly rate by turn** — the useful curve. Turn 0 is the opening
  hand. Report a few rows of this rather than a single number.

Same `SEED` plus same `TRIALS` is bit-for-bit reproducible regardless of worker
count, so a diff between two runs is a real difference, not shuffle noise.

## Pitfalls

- **Commanders.** Put them under a `Commander` header in `DECKLIST`; the harness
  keeps them out of the library and reports them. If a combo piece *is* the
  commander, it is always available — drop that piece from the win condition and
  state the assumption (it presumes the commander is cast and survives).
- **Sideboards.** A header containing "sideboard" or "maybeboard" is skipped.
  Verify the library size in the deck report.
- **The model draws every turn, including turn 1.** No play/draw distinction. So
  "turn N" means "after N draw steps" — on the play, real turn N sees one card
  fewer.
- **`draws_per_turn` is a blunt instrument.** Setting it to 2 to represent a draw
  engine assumes that engine is online from turn 1, which flatters the deck.
  Prefer `1` and note cantrips as an unmodeled upside.
- **Tutors do not tutor.** A tutor is just a card; the `tutor` tag only affects
  bottoming. To model one, either count it as a copy of what it fetches in the
  win condition, or write a `WinCondition` that treats "tutor + target still in
  library" as satisfied.
- **Untagged combo piece.** A win rate of exactly 0% almost always means a tag
  never landed on a card. Check the deck report's per-tag counts.

## Going further

`reference.md` in this directory covers the full API surface: every field on
`Params` and `MonteCarloParams`, every metric on `Aggregate`, and how to
implement custom `WinCondition`, `MulliganPolicy`, and `BottomHeuristic`
values. Read it when the template's four sections are not enough.
