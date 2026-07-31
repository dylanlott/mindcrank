use std::sync::atomic::{AtomicUsize, Ordering};

use approx::assert_abs_diff_eq;
use mindcrank::{
    AnyOf, BottomHeuristic, Card, Deck, KOfTag, KeepIfLandsBetween, MonteCarloParams,
    MulliganPolicy, Params, TwoCardSet, WinCondition, monte_carlo, run_once,
};

#[test]
fn win_conditions_compose() {
    let hand = vec![
        Card::new("Oracle").with_tag("combo:oracle"),
        Card::new("Consult").with_tag("combo:consult"),
    ];
    let combo = TwoCardSet::new("combo:oracle", "combo:consult");
    assert!(combo.satisfied(&hand));

    let any = AnyOf::new(vec![Box::new(KOfTag::new("creature", 3)), Box::new(combo)]);
    assert!(any.satisfied(&hand));
}

#[test]
fn deck_draw_is_non_destructive() {
    let deck = Deck::new(vec![Card::new("A"), Card::new("B"), Card::new("C")]);
    let (drawn, rest) = deck.draw_n(2);

    assert_eq!(drawn.len(), 2);
    assert_eq!(rest.len(), 1);
    assert_eq!(deck.len(), 3);
}

struct MullOnce(AtomicUsize);

impl MulliganPolicy for MullOnce {
    fn keep(&self, _opening_hand: &[Card]) -> bool {
        self.0.fetch_add(1, Ordering::SeqCst) > 0
    }
}

struct BottomTagged(&'static str);

impl BottomHeuristic for BottomTagged {
    fn cards_to_bottom(&self, hand: &[Card], count: usize, _win: &dyn WinCondition) -> Vec<usize> {
        hand.iter()
            .enumerate()
            .filter(|(_, card)| card.has_tag(self.0))
            .take(count)
            .map(|(index, _)| index)
            .collect()
    }
}

#[test]
fn london_bottoms_cards_into_the_library() {
    let deck = Deck::new(vec![Card::new("Win").with_tag("win"), Card::new("Filler")]);
    let win = KOfTag::new("win", 1);
    let policy = MullOnce(AtomicUsize::new(0));
    let bottomer = BottomTagged("win");
    let mut params = Params::new(&deck, &win)
        .london_mulligan(&policy, 1)
        .bottom_with(&bottomer)
        .with_seed(7);
    params.hand_size = 2;
    params.max_turns = 1;

    let outcome = run_once(&params);
    assert!(outcome.won);
    assert!(!outcome.opening_win);
    assert_eq!(outcome.kept, 1);
    assert_eq!(outcome.draws_after_opening, 1);
    assert_eq!(outcome.turns_to_win, Some(1));
}

#[test]
fn keeping_at_zero_mulligans_bottoms_nothing() {
    let deck = Deck::new(vec![Card::new("Filler"); 20]);
    let win = KOfTag::new("missing", 1);
    let keep_everything = KeepIfLandsBetween::new(0, 7);
    let mut params = Params::new(&deck, &win)
        .london_mulligan(&keep_everything, 3)
        .with_seed(4);
    params.max_turns = 0;

    assert_eq!(run_once(&params).kept, 7);
}

#[test]
fn misses_are_not_reported_as_slow_wins() {
    let deck = Deck::new(vec![Card::new("Filler"); 20]);
    let win = KOfTag::new("missing", 1);
    let mut params = Params::new(&deck, &win).with_seed(1);
    params.max_turns = 2;

    let outcome = run_once(&params);
    assert!(!outcome.won);
    assert_eq!(outcome.turns_to_win, None);

    let aggregate = monte_carlo(MonteCarloParams::new(params, 10).with_seed(123));
    assert_eq!(aggregate.wins, 0);
    assert_eq!(aggregate.misses, 10);
    assert_eq!(aggregate.avg_turns_to_win, None);
}

#[test]
fn monte_carlo_is_reproducible_across_worker_counts() {
    let mut cards = vec![Card::new("Win").with_tag("win"); 4];
    cards.extend(vec![Card::new("Filler"); 56]);
    let deck = Deck::new(cards);
    let win = KOfTag::new("win", 1);
    let mut params = Params::new(&deck, &win);
    params.max_turns = 5;

    let single = monte_carlo(
        MonteCarloParams::new(params, 5_000)
            .with_seed(42)
            .with_workers(1),
    );
    let parallel = monte_carlo(
        MonteCarloParams::new(params, 5_000)
            .with_seed(42)
            .with_workers(4),
    );

    assert_eq!(single, parallel);
    assert_abs_diff_eq!(single.win_rate, single.wins as f64 / single.trials as f64);
}
