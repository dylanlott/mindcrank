---
slug: commander-pod-simulations-with-recursive-response-priority
title: Commander pod simulations with recursive response priority
status: open
created: 2026-08-20
updated: 2026-08-20
---

# Thread: Commander pod simulations with recursive response priority

## Goal

Generalize mindcrank's competitive arena to fair four-player Commander pods and
add a coarse interactive turn model whose priority windows allow arbitrarily
deep response chains bounded by cards and mana rather than a fixed protection
limit.

## Context

The current baseline is commit `e5d2a28` (`feat: add competitive simulation
arena`). It supports deterministic two-player round robins, a goldfish race
model, paired play/draw trials, incremental reports, and replay IDs.

The next design originally assumed a single win/disrupt/protect exchange. That
has been superseded. Interaction and protection will both create typed stack
effects, counters may target other counters, and a priority window continues
until every living player passes. Four-player pods use all 24 seat permutations
per Monte Carlo sample so starting position and clockwise priority order are
balanced.

The executable plan is in `docs/interactive-simulation-plan.md`. It is divided
into five atomic packages:

1. Generalize fixed two-player matchups and reports into dynamic contests.
2. Add four-player pod schedules and full seat-permutation sampling.
3. Extract shared opening-hand and London-mulligan initialization.
4. Implement the recursive stack/priority engine and hidden-information views.
5. Add baseline pilots, response metrics, and a four-player vertical example.

Important locked decisions:

- no gameplay response-depth cap
- all 24 seat permutations by default for fixed four-player pods
- common competitor RNG streams across permutations
- illegal actions and watchdog exhaustion abort with typed errors
- incremental aggregation; do not store every trial
- retain the current two-player and single-deck workflows

The working tree also contains pre-existing, uncommitted user edits to the
README title and a comment in `src/metrics.rs`. Preserve them unless explicitly
directed otherwise.

## References

- `docs/interactive-simulation-plan.md` — authoritative execution plan
- `src/arena/mod.rs` — current fixed two-player public types
- `src/arena/runner.rs` — paired Monte Carlo seeding and replay
- `src/arena/report.rs` — fixed-array aggregation to generalize
- `src/arena/model.rs` — two-player goldfish model
- `src/arena/schedule.rs` — current round-robin schedule
- `tests/arena.rs` — compatibility and determinism baseline
- commit `e5d2a28` — competitive arena baseline

## Next Steps

1. Read `docs/interactive-simulation-plan.md` completely.
2. Run `git status`, `cargo test --all-targets`, and preserve unrelated changes.
3. Execute Package 1 only: generalize arena primitives, reports, errors, and the
   goldfish model to dynamic contests.
4. Keep Package 1 atomic and green before starting pod schedules.
5. Verify formatting, tests, Clippy, rustdoc, and worker-count reproducibility.

Resume this context with:

```text
$gsd-thread commander-pod-simulations-with-recursive-response-priority
```
