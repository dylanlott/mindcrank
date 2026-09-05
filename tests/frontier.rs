use std::collections::BTreeMap;

use mindcrank::{
    Aggregate, Card, Deck, DeckCandidate, KOfTag, MonteCarloParams, Params, ParetoError,
    ParetoProtocol, TrialOutcome, WinCondition, compare_pareto, monte_carlo,
};

fn winning_deck() -> Deck {
    Deck::new(vec![Card::new("Win").with_tag("win"); 20])
}

fn inert_deck() -> Deck {
    Deck::new(vec![Card::new("Filler"); 20])
}

fn protocol<'a>(deck: &'a Deck, win: &'a dyn WinCondition) -> ParetoProtocol<'a> {
    ParetoProtocol::from_params(Params::new(deck, win), 3, 10, 100, 42).unwrap()
}

#[test]
fn aggregate_tracks_exact_turn_rates_and_kept_hand_size() {
    let aggregate = Aggregate::from_outcomes(&[
        TrialOutcome {
            won: true,
            draws_after_opening: 0,
            opening_win: true,
            opening_lands: 2,
            kept: 7,
            turns_to_win: Some(0),
        },
        TrialOutcome {
            won: true,
            draws_after_opening: 2,
            opening_win: false,
            opening_lands: 3,
            kept: 6,
            turns_to_win: Some(2),
        },
        TrialOutcome {
            won: false,
            draws_after_opening: 3,
            opening_win: false,
            opening_lands: 4,
            kept: 5,
            turns_to_win: None,
        },
    ]);

    assert_eq!(aggregate.distribution_turns_to_win.get(&0), Some(&1));
    assert_eq!(aggregate.distribution_turns_to_win.get(&2), Some(&1));
    assert_eq!(aggregate.wins_by_turn(0), 1);
    assert_eq!(aggregate.wins_by_turn(1), 1);
    assert_eq!(aggregate.wins_by_turn(2), 2);
    assert_eq!(aggregate.win_rate_by_turn(2), 2.0 / 3.0);
    assert_eq!(aggregate.avg_kept_hand_size, 6.0);
}

#[test]
fn strict_frontier_keeps_ties_and_discards_dominated_candidates() {
    let winner = winning_deck();
    let matching_winner = winning_deck();
    let inert = inert_deck();
    let win = KOfTag::new("win", 1);
    let report = compare_pareto(
        &[
            DeckCandidate::new("winner", "Winner", &winner),
            DeckCandidate::new("matching", "Matching winner", &matching_winner),
            DeckCandidate::new("inert", "Inert", &inert),
        ],
        protocol(&winner, &win),
    )
    .unwrap();

    assert_eq!(report.frontier_indices, [0, 1]);
    assert!(report.candidates[0].is_frontier);
    assert!(report.candidates[1].is_frontier);
    assert!(!report.candidates[2].is_frontier);
    assert_eq!(report.candidates[0].early_wins, 100);
    assert_eq!(report.candidates[0].horizon_wins, 100);
    assert_eq!(report.candidates[2].early_wins, 0);
    assert_eq!(report.candidates[2].horizon_wins, 0);
}

#[test]
fn result_is_independent_of_candidate_order_and_worker_count() {
    let mut frequent_cards = vec![Card::new("Win").with_tag("win"); 4];
    frequent_cards.extend(vec![Card::new("Filler"); 56]);
    let frequent = Deck::new(frequent_cards);

    let mut rare_cards = vec![Card::new("Win").with_tag("win"); 2];
    rare_cards.extend(vec![Card::new("Filler"); 58]);
    let rare = Deck::new(rare_cards);
    let win = KOfTag::new("win", 1);
    let base = ParetoProtocol::from_params(Params::new(&frequent, &win), 2, 5, 2_000, 99).unwrap();

    let single = compare_pareto(
        &[
            DeckCandidate::new("frequent", "Frequent", &frequent),
            DeckCandidate::new("rare", "Rare", &rare),
        ],
        base.with_workers(1),
    )
    .unwrap();
    let parallel = compare_pareto(
        &[
            DeckCandidate::new("frequent", "Frequent", &frequent),
            DeckCandidate::new("rare", "Rare", &rare),
        ],
        base.with_workers(4),
    )
    .unwrap();
    let reversed = compare_pareto(
        &[
            DeckCandidate::new("rare", "Rare", &rare),
            DeckCandidate::new("frequent", "Frequent", &frequent),
        ],
        base,
    )
    .unwrap();

    let metrics = |report: &mindcrank::ParetoReport| {
        report
            .candidates
            .iter()
            .map(|candidate| {
                (
                    candidate.id.clone(),
                    (
                        candidate.aggregate.clone(),
                        candidate.early_wins,
                        candidate.horizon_wins,
                        candidate.is_frontier,
                    ),
                )
            })
            .collect::<BTreeMap<_, _>>()
    };

    assert_eq!(metrics(&single), metrics(&parallel));
    assert_eq!(metrics(&single), metrics(&reversed));
}

struct WinAtHandSize(usize);

impl WinCondition for WinAtHandSize {
    fn satisfied(&self, hand: &[Card]) -> bool {
        hand.len() >= self.0
    }
}

#[test]
fn thresholds_include_opening_wins_and_exclude_later_wins_from_early_rate() {
    let deck = inert_deck();
    let win = WinAtHandSize(3);
    let mut params = Params::new(&deck, &win);
    params.hand_size = 1;
    params.draws_per_turn = 1;
    let protocol = ParetoProtocol::from_params(params, 1, 2, 50, 7).unwrap();

    let report =
        compare_pareto(&[DeckCandidate::new("delayed", "Delayed", &deck)], protocol).unwrap();
    let result = &report.candidates[0];

    assert_eq!(result.early_win_rate, 0.0);
    assert_eq!(result.horizon_win_rate, 1.0);
    assert_eq!(result.aggregate.wins_by_turn(2), 50);
}

#[test]
fn validation_errors_are_reported_before_simulation() {
    let deck = winning_deck();
    let win = KOfTag::new("win", 1);
    let valid = protocol(&deck, &win);

    assert_eq!(compare_pareto(&[], valid), Err(ParetoError::NoCandidates));
    assert_eq!(
        compare_pareto(
            &[
                DeckCandidate::new("same", "One", &deck),
                DeckCandidate::new("same", "Two", &deck),
            ],
            valid,
        ),
        Err(ParetoError::DuplicateId { id: "same".into() })
    );
    assert!(matches!(
        ParetoProtocol::from_params(Params::new(&deck, &win), 3, 10, 0, 1),
        Err(ParetoError::ZeroTrials)
    ));
    assert!(matches!(
        ParetoProtocol::from_params(Params::new(&deck, &win), 11, 10, 1, 1),
        Err(ParetoError::FastTurnExceedsHorizon {
            fast_turn: 11,
            horizon_turn: 10,
        })
    ));
}

#[test]
fn scatterplot_exposes_axis_and_tooltip_values() {
    let winner = winning_deck();
    let inert = inert_deck();
    let win = KOfTag::new("win", 1);
    let report = compare_pareto(
        &[
            DeckCandidate::new("winner", "Winner", &winner),
            DeckCandidate::new("inert", "Inert", &inert),
        ],
        protocol(&winner, &win),
    )
    .unwrap();
    let plot = report.scatterplot();

    assert_eq!(plot.x_axis.label, "P(win by turn 3)");
    assert_eq!(plot.y_axis.label, "P(win by turn 10)");
    assert_eq!(plot.points.len(), 2);
    assert_eq!(plot.points[0].x, report.candidates[0].early_win_rate);
    assert_eq!(plot.points[0].y, report.candidates[0].horizon_win_rate);
    assert_eq!(
        plot.points[0].tooltip.avg_kept_hand_size,
        report.candidates[0].aggregate.avg_kept_hand_size
    );
    assert_eq!(
        plot.points[0].tooltip.trials,
        report.candidates[0].aggregate.trials
    );
}

#[test]
fn report_exports_csv_rows_for_plotting() {
    let winner = winning_deck();
    let inert = inert_deck();
    let win = KOfTag::new("win", 1);
    let report = compare_pareto(
        &[
            DeckCandidate::new("winner", "Winner", &winner),
            DeckCandidate::new("inert", "Inert", &inert),
        ],
        protocol(&winner, &win),
    )
    .unwrap();

    let csv = report.to_csv(false);
    let rows = csv.lines().collect::<Vec<_>>();
    assert_eq!(rows.len(), 3);
    assert_eq!(
        rows[0],
        "candidate_id,label,fast_turn,horizon_turn,early_win_rate,horizon_win_rate,is_frontier,early_wins,horizon_wins,trials,opening_win_rate,avg_kept_hand_size"
    );
    assert!(rows[1].starts_with("\"winner\",\"Winner\",3,10"));
    assert_eq!(rows[1].split(',').count(), 12);
    assert_eq!(rows[2].split(',').count(), 12);

    let frontier_csv = report.to_csv(true);
    assert_eq!(frontier_csv.lines().count(), 2);
}

#[test]
fn zero_trial_aggregate_has_zero_cumulative_rate() {
    let aggregate = monte_carlo(MonteCarloParams::new(
        Params::new(&inert_deck(), &KOfTag::new("win", 1)),
        0,
    ));

    assert_eq!(aggregate.wins_by_turn(10), 0);
    assert_eq!(aggregate.win_rate_by_turn(10), 0.0);
}
