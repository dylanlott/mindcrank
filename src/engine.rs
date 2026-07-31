use std::collections::HashSet;

use rand::SeedableRng;
use rand::rngs::StdRng;
use rayon::prelude::*;

use crate::{
    Aggregate, BottomHeuristic, Card, Deck, DefaultBottomHeuristic, MulliganPolicy, TrialOutcome,
    WinCondition, count_tag,
};

/// Controls one game trial.
#[derive(Clone, Copy)]
pub struct Params<'a> {
    pub deck: &'a Deck,
    pub win: &'a dyn WinCondition,
    pub hand_size: usize,
    pub max_turns: usize,
    pub draws_per_turn: usize,
    pub use_london_mulligan: bool,
    pub max_mulligans: usize,
    pub mulligan: Option<&'a dyn MulliganPolicy>,
    pub bottom_heuristic: Option<&'a dyn BottomHeuristic>,
    pub seed: Option<u64>,
}

impl<'a> Params<'a> {
    pub fn new(deck: &'a Deck, win: &'a dyn WinCondition) -> Self {
        Self {
            deck,
            win,
            hand_size: 7,
            max_turns: 50,
            draws_per_turn: 1,
            use_london_mulligan: false,
            max_mulligans: 0,
            mulligan: None,
            bottom_heuristic: None,
            seed: None,
        }
    }

    pub fn london_mulligan(mut self, policy: &'a dyn MulliganPolicy, max_mulligans: usize) -> Self {
        self.use_london_mulligan = true;
        self.max_mulligans = max_mulligans;
        self.mulligan = Some(policy);
        self
    }

    pub fn bottom_with(mut self, heuristic: &'a dyn BottomHeuristic) -> Self {
        self.bottom_heuristic = Some(heuristic);
        self
    }

    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = Some(seed);
        self
    }
}

/// Runs one game using `params.seed`, or a random seed when none is supplied.
pub fn run_once(params: &Params<'_>) -> TrialOutcome {
    let seed = params.seed.unwrap_or_else(rand::random);
    run_once_with_seed(params, seed)
}

fn run_once_with_seed(params: &Params<'_>, seed: u64) -> TrialOutcome {
    let mut rng = StdRng::seed_from_u64(seed);
    let hand_size = params.hand_size.max(1);
    let draws_per_turn = params.draws_per_turn.max(1);
    let default_bottomer = DefaultBottomHeuristic;
    let bottomer = params.bottom_heuristic.unwrap_or(&default_bottomer);
    let max_mulligans = params.max_mulligans.min(hand_size);
    let mut mulligans = 0;

    loop {
        // A London mulligan redraw is a fresh shuffle of the original deck.
        let mut library = params.deck.shuffle(&mut rng);
        let opening = library.draw_n_mut(hand_size);
        let opening_lands = count_tag(&opening, "land");

        if !params.use_london_mulligan {
            return play_out(
                opening,
                library,
                draws_per_turn,
                params.max_turns,
                params.win,
                opening_lands,
            );
        }

        let keep = params.mulligan.is_none_or(|policy| policy.keep(&opening));

        if keep || mulligans >= max_mulligans {
            let to_bottom = mulligans.min(opening.len());
            let requested = bottomer.cards_to_bottom(&opening, to_bottom, params.win);
            let indices = normalized_bottom_indices(&opening, to_bottom, requested, params.win);
            let (kept, bottomed) = partition_hand(opening, &indices);
            library.put_on_bottom(bottomed);

            return play_out(
                kept,
                library,
                draws_per_turn,
                params.max_turns,
                params.win,
                opening_lands,
            );
        }

        mulligans += 1;
    }
}

fn normalized_bottom_indices(
    hand: &[Card],
    count: usize,
    requested: Vec<usize>,
    win: &dyn WinCondition,
) -> Vec<usize> {
    let mut seen = HashSet::with_capacity(count);
    let mut indices = Vec::with_capacity(count);

    for index in requested {
        if index < hand.len() && seen.insert(index) {
            indices.push(index);
            if indices.len() == count {
                return indices;
            }
        }
    }

    // A custom heuristic that returns too few or invalid indices is completed
    // with the default policy instead of producing the wrong hand size.
    for index in DefaultBottomHeuristic.cards_to_bottom(hand, hand.len(), win) {
        if seen.insert(index) {
            indices.push(index);
            if indices.len() == count {
                break;
            }
        }
    }

    indices
}

fn partition_hand(hand: Vec<Card>, bottom_indices: &[usize]) -> (Vec<Card>, Vec<Card>) {
    let bottom_indices: HashSet<_> = bottom_indices.iter().copied().collect();
    let mut kept = Vec::with_capacity(hand.len().saturating_sub(bottom_indices.len()));
    let mut bottomed = Vec::with_capacity(bottom_indices.len());

    for (index, card) in hand.into_iter().enumerate() {
        if bottom_indices.contains(&index) {
            bottomed.push(card);
        } else {
            kept.push(card);
        }
    }

    (kept, bottomed)
}

fn play_out(
    mut hand: Vec<Card>,
    mut library: Deck,
    draws_per_turn: usize,
    max_turns: usize,
    win: &dyn WinCondition,
    opening_lands: usize,
) -> TrialOutcome {
    let kept = hand.len();
    if win.satisfied(&hand) {
        return TrialOutcome {
            won: true,
            draws_after_opening: 0,
            opening_win: true,
            opening_lands,
            kept,
            turns_to_win: Some(0),
        };
    }

    let mut draws = 0;
    for turn in 1..=max_turns {
        if library.is_empty() {
            break;
        }

        let drawn = library.draw_n_mut(draws_per_turn);
        draws += drawn.len();
        hand.extend(drawn);

        if win.satisfied(&hand) {
            return TrialOutcome {
                won: true,
                draws_after_opening: draws,
                opening_win: false,
                opening_lands,
                kept,
                turns_to_win: Some(turn),
            };
        }
    }

    TrialOutcome {
        won: false,
        draws_after_opening: draws,
        opening_win: false,
        opening_lands,
        kept,
        turns_to_win: None,
    }
}

/// Controls a parallel Monte Carlo run.
#[derive(Clone, Copy)]
pub struct MonteCarloParams<'a> {
    pub params: Params<'a>,
    pub trials: usize,
    pub seed: Option<u64>,
    /// Zero uses Rayon's global pool.
    pub workers: usize,
}

impl<'a> MonteCarloParams<'a> {
    pub fn new(params: Params<'a>, trials: usize) -> Self {
        Self {
            params,
            trials,
            seed: None,
            workers: 0,
        }
    }

    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = Some(seed);
        self
    }

    pub fn with_workers(mut self, workers: usize) -> Self {
        self.workers = workers;
        self
    }
}

/// Runs independent trials in parallel and reduces them into an aggregate.
///
/// A fixed seed is reproducible even when the worker count changes.
pub fn monte_carlo(params: MonteCarloParams<'_>) -> Aggregate {
    if params.trials == 0 {
        return Aggregate::default();
    }

    let master_seed = params
        .seed
        .or(params.params.seed)
        .unwrap_or_else(rand::random);

    let simulate = || {
        (0..params.trials)
            .into_par_iter()
            .map(|index| {
                let seed = splitmix64(master_seed.wrapping_add(index as u64));
                run_once_with_seed(&params.params, seed)
            })
            .collect::<Vec<_>>()
    };

    let outcomes = if params.workers == 0 {
        simulate()
    } else {
        rayon::ThreadPoolBuilder::new()
            .num_threads(params.workers)
            .build()
            .expect("failed to build Monte Carlo worker pool")
            .install(simulate)
    };

    Aggregate::from_outcomes(&outcomes)
}

fn splitmix64(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9e37_79b9_7f4a_7c15);
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}
