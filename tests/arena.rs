use mindcrank::arena::{
    ArenaError, ArenaMonteCarlo, Competitor, Contest, ContestId, ContestOutcome, ContestSimulator,
    GoldfishRaceModel, OutcomeReason, RoundRobin, Schedule, Seating, SimulationError, TiePolicy,
    TrialContext, TrialId,
};
use mindcrank::{Card, Deck, KOfTag, Params};

struct StartingSeatWins;

impl ContestSimulator for StartingSeatWins {
    fn simulate(
        &self,
        _competitors: &[Competitor<'_>],
        _contest: &Contest,
        _context: &TrialContext,
    ) -> Result<ContestOutcome, SimulationError> {
        Ok(ContestOutcome::winner(
            0,
            3,
            OutcomeReason::TurnOrderTieBreak,
        ))
    }
}

#[derive(Clone)]
struct ExplicitSchedule(Vec<Contest>);

impl Schedule for ExplicitSchedule {
    fn contests(&self, _competitors: &[Competitor<'_>]) -> Result<Vec<Contest>, ArenaError> {
        Ok(self.0.clone())
    }
}

fn inert_deck() -> Deck {
    Deck::new(vec![Card::new("Filler"); 20])
}

fn competitors<'a>(count: usize, params: Params<'a>) -> Vec<Competitor<'a>> {
    (0..count)
        .map(|index| Competitor::new(format!("player-{index}"), params))
        .collect()
}

#[test]
fn round_robin_schedules_every_pair_in_id_order() {
    let deck = inert_deck();
    let win = KOfTag::new("missing", 1);
    let params = Params::new(&deck, &win);
    let competitors = vec![
        Competitor::new("charlie", params),
        Competitor::new("alpha", params),
        Competitor::new("bravo", params),
    ];

    let contests = RoundRobin.contests(&competitors).unwrap();
    let pairs: Vec<Vec<_>> = contests
        .iter()
        .map(|contest| {
            contest
                .competitor_indices
                .iter()
                .map(|&index| competitors[index].id.as_str())
                .collect()
        })
        .collect();

    assert_eq!(
        pairs,
        vec![
            vec!["alpha", "bravo"],
            vec!["alpha", "charlie"],
            vec!["bravo", "charlie"],
        ]
    );
    assert_eq!(contests[0].id.0, 0);
    assert_eq!(contests[2].id.0, 2);
}

#[test]
fn cyclic_seatings_balance_starting_position() {
    let deck = inert_deck();
    let win = KOfTag::new("missing", 1);
    let params = Params::new(&deck, &win);
    let competitors = vec![
        Competitor::new("alpha", params),
        Competitor::new("bravo", params),
        Competitor::new("charlie", params),
    ];

    let report = ArenaMonteCarlo::new(10)
        .with_seed(42)
        .with_workers(2)
        .run(&competitors, &RoundRobin, &StartingSeatWins)
        .unwrap();

    assert_eq!(report.contests.len(), 3);
    assert_eq!(report.games, 60);
    for contest in &report.contests {
        assert_eq!(contest.records[0].games, 20);
        assert_eq!(contest.records[0].wins, 10);
        assert_eq!(contest.records[1].wins, 10);
        assert_eq!(contest.records_by_seat[0][0].wins, 10);
        assert_eq!(contest.records_by_seat[0][1].losses, 10);
    }

    for entry in &report.standings {
        assert_eq!(entry.record.games, 40);
        assert_eq!(entry.record.wins, 20);
        assert_eq!(entry.record.losses, 20);
        assert_eq!(entry.record.draws, 0);
        assert_eq!(entry.record.score_rate(), 0.5);
    }
}

#[test]
fn goldfish_race_awards_the_earliest_win() {
    let fast_deck = Deck::new(vec![Card::new("Threat").with_tag("win"); 20]);
    let slow_deck = inert_deck();
    let win = KOfTag::new("win", 1);
    let mut fast = Params::new(&fast_deck, &win);
    let mut slow = Params::new(&slow_deck, &win);
    fast.max_turns = 3;
    slow.max_turns = 3;
    let competitors = vec![Competitor::new("fast", fast), Competitor::new("slow", slow)];

    let report = ArenaMonteCarlo::new(4)
        .with_seed(7)
        .run(&competitors, &RoundRobin, &GoldfishRaceModel::new())
        .unwrap();
    let contest = &report.contests[0];

    assert_eq!(contest.competitor_ids, ["fast", "slow"]);
    assert_eq!(contest.records[0].wins, 8);
    assert_eq!(contest.records[1].losses, 8);
    assert_eq!(contest.average_turns, Some(0.0));

    let example = contest.examples.winner[0].unwrap();
    let replay = ArenaMonteCarlo::new(4)
        .replay(
            &competitors,
            &RoundRobin,
            &GoldfishRaceModel::new(),
            report.seed,
            example,
        )
        .unwrap();
    assert_eq!(
        replay.outcome,
        ContestOutcome::winner(0, 0, OutcomeReason::WinCondition)
    );
}

#[test]
fn multiplayer_goldfish_ties_follow_the_selected_policy() {
    let deck = Deck::new(vec![Card::new("Threat").with_tag("win"); 20]);
    let win = KOfTag::new("win", 1);
    let params = Params::new(&deck, &win);
    let competitors = competitors(4, params);
    let schedule = ExplicitSchedule(vec![Contest::new(ContestId(7), vec![0, 1, 2, 3])]);

    let draws = ArenaMonteCarlo::new(1)
        .with_seed(11)
        .run(&competitors, &schedule, &GoldfishRaceModel::new())
        .unwrap();
    assert!(
        draws.contests[0]
            .records
            .iter()
            .all(|record| record.draws == 4)
    );

    let turn_order = GoldfishRaceModel::new().with_tie_policy(TiePolicy::StartingPlayer);
    let resolved = ArenaMonteCarlo::new(1)
        .with_seed(11)
        .run(&competitors, &schedule, &turn_order)
        .unwrap();
    assert!(
        resolved.contests[0]
            .records
            .iter()
            .all(|record| record.wins == 1 && record.losses == 3)
    );
}

#[test]
fn results_and_replay_are_reproducible_across_worker_counts() {
    let mut frequent_cards = vec![Card::new("Threat").with_tag("win"); 5];
    frequent_cards.extend(vec![Card::new("Filler"); 35]);
    let frequent_deck = Deck::new(frequent_cards);

    let mut rare_cards = vec![Card::new("Threat").with_tag("win"); 2];
    rare_cards.extend(vec![Card::new("Filler"); 38]);
    let rare_deck = Deck::new(rare_cards);

    let win = KOfTag::new("win", 1);
    let mut frequent = Params::new(&frequent_deck, &win);
    let mut rare = Params::new(&rare_deck, &win);
    frequent.max_turns = 5;
    rare.max_turns = 5;
    let competitors = vec![
        Competitor::new("frequent", frequent),
        Competitor::new("rare", rare),
    ];
    let model = GoldfishRaceModel::new();

    let single = ArenaMonteCarlo::new(1_000)
        .with_seed(99)
        .with_workers(1)
        .run(&competitors, &RoundRobin, &model)
        .unwrap();
    let parallel = ArenaMonteCarlo::new(1_000)
        .with_seed(99)
        .with_workers(4)
        .run(&competitors, &RoundRobin, &model)
        .unwrap();

    assert_eq!(single, parallel);

    let trial_id = single.contests[0]
        .examples
        .winner
        .iter()
        .flatten()
        .next()
        .copied()
        .or(single.contests[0].examples.draw)
        .unwrap();
    let first = ArenaMonteCarlo::new(1_000)
        .replay(&competitors, &RoundRobin, &model, single.seed, trial_id)
        .unwrap();
    let second = ArenaMonteCarlo::new(1_000)
        .replay(&competitors, &RoundRobin, &model, single.seed, trial_id)
        .unwrap();
    assert_eq!(first, second);
}

#[test]
fn four_seat_results_map_back_to_canonical_competitors() {
    let deck = inert_deck();
    let win = KOfTag::new("missing", 1);
    let params = Params::new(&deck, &win);
    let competitors = competitors(4, params);
    let schedule = ExplicitSchedule(vec![Contest::new(ContestId(9), vec![0, 1, 2, 3])]);

    let report = ArenaMonteCarlo::new(1)
        .with_seed(17)
        .run(&competitors, &schedule, &StartingSeatWins)
        .unwrap();
    let contest = &report.contests[0];

    assert_eq!(report.games, 4);
    for (contest_slot, record) in contest.records.iter().enumerate() {
        assert_eq!(
            *record,
            mindcrank::arena::Record {
                games: 4,
                wins: 1,
                losses: 3,
                draws: 0,
            }
        );
        assert_eq!(contest.records_by_seat[contest_slot][0].wins, 1);
        assert!(
            contest.records_by_seat[contest_slot][1..]
                .iter()
                .all(|seat_record| seat_record.losses == 1)
        );
    }
}

#[test]
fn simulator_failures_and_invalid_winner_seats_abort_the_run() {
    struct Fails;
    impl ContestSimulator for Fails {
        fn simulate(
            &self,
            _competitors: &[Competitor<'_>],
            _contest: &Contest,
            _context: &TrialContext,
        ) -> Result<ContestOutcome, SimulationError> {
            Err(SimulationError::new("scripted failure"))
        }
    }

    struct InvalidWinner;
    impl ContestSimulator for InvalidWinner {
        fn simulate(
            &self,
            _competitors: &[Competitor<'_>],
            contest: &Contest,
            _context: &TrialContext,
        ) -> Result<ContestOutcome, SimulationError> {
            Ok(ContestOutcome::winner(
                contest.len(),
                1,
                OutcomeReason::ModelDefined("invalid".into()),
            ))
        }
    }

    let deck = inert_deck();
    let win = KOfTag::new("missing", 1);
    let params = Params::new(&deck, &win);
    let competitors = competitors(2, params);
    let trial_id = TrialId {
        contest_id: ContestId(0),
        sample_index: 0,
        seating_index: 0,
    };

    let failure = ArenaMonteCarlo::new(1)
        .with_seed(1)
        .run(&competitors, &RoundRobin, &Fails)
        .unwrap_err();
    assert_eq!(
        failure,
        ArenaError::SimulationFailed {
            trial_id,
            source: SimulationError::new("scripted failure"),
        }
    );

    let invalid = ArenaMonteCarlo::new(1)
        .with_seed(1)
        .run(&competitors, &RoundRobin, &InvalidWinner)
        .unwrap_err();
    assert_eq!(invalid, ArenaError::InvalidWinnerSeat { trial_id, seat: 2 });
}

#[test]
fn malformed_report_vectors_are_rejected() {
    let deck = inert_deck();
    let win = KOfTag::new("missing", 1);
    let params = Params::new(&deck, &win);
    let competitors = competitors(2, params);
    let mut report = ArenaMonteCarlo::new(1)
        .with_seed(1)
        .run(&competitors, &RoundRobin, &StartingSeatWins)
        .unwrap()
        .contests
        .remove(0);

    report.records.pop();
    assert_eq!(
        report.validate(),
        Err(ArenaError::InvalidReportShape {
            contest_id: ContestId(0),
            field: "records".into(),
            expected: 2,
            actual: 1,
        })
    );
}

#[test]
fn invalid_contests_are_rejected_before_simulation() {
    let deck = inert_deck();
    let win = KOfTag::new("missing", 1);
    let params = Params::new(&deck, &win);
    let competitors = competitors(2, params);

    let empty = ExplicitSchedule(vec![Contest::new(ContestId(4), Vec::new())]);
    assert_eq!(
        ArenaMonteCarlo::new(1)
            .run(&competitors, &empty, &StartingSeatWins)
            .unwrap_err(),
        ArenaError::EmptyContest(ContestId(4))
    );

    let repeated = ExplicitSchedule(vec![Contest::new(ContestId(5), vec![0, 0])]);
    assert_eq!(
        ArenaMonteCarlo::new(1)
            .run(&competitors, &repeated, &StartingSeatWins)
            .unwrap_err(),
        ArenaError::DuplicateCompetitorInContest {
            contest_id: ContestId(5),
            index: 0,
        }
    );

    let out_of_bounds = ExplicitSchedule(vec![Contest::new(ContestId(6), vec![0, 2])]);
    assert_eq!(
        ArenaMonteCarlo::new(1)
            .run(&competitors, &out_of_bounds, &StartingSeatWins)
            .unwrap_err(),
        ArenaError::InvalidCompetitorIndex {
            contest_id: ContestId(6),
            index: 2,
        }
    );

    let duplicate_ids = ExplicitSchedule(vec![
        Contest::new(ContestId(7), vec![0]),
        Contest::new(ContestId(7), vec![1]),
    ]);
    assert_eq!(
        ArenaMonteCarlo::new(1)
            .run(&competitors, &duplicate_ids, &StartingSeatWins)
            .unwrap_err(),
        ArenaError::DuplicateContestId(ContestId(7))
    );
}

#[test]
fn seating_requires_a_complete_permutation_of_contest_slots() {
    assert!(Seating::cyclic(0, 0).is_empty());
    assert_eq!(
        Seating::new(vec![0, 0]).validate(ContestId(3), 2),
        Err(ArenaError::InvalidSeating {
            contest_id: ContestId(3),
            reason: "contest slot 0 appears more than once".into(),
        })
    );
}

#[test]
fn duplicate_competitor_ids_are_rejected() {
    let deck = inert_deck();
    let win = KOfTag::new("missing", 1);
    let params = Params::new(&deck, &win);
    let competitors = vec![
        Competitor::new("same", params),
        Competitor::new("same", params),
    ];

    let error = ArenaMonteCarlo::new(1)
        .run(&competitors, &RoundRobin, &StartingSeatWins)
        .unwrap_err();

    assert_eq!(error, ArenaError::DuplicateCompetitorId("same".into()));
}

#[test]
fn zero_samples_and_empty_schedules_are_well_defined() {
    let deck = inert_deck();
    let win = KOfTag::new("missing", 1);
    let params = Params::new(&deck, &win);
    let competitors = competitors(2, params);

    let report = ArenaMonteCarlo::new(0)
        .with_seed(1)
        .run(&competitors, &RoundRobin, &StartingSeatWins)
        .unwrap();
    assert_eq!(report.games, 0);
    assert_eq!(report.contests[0].records[0].games, 0);
    assert_eq!(report.contests[0].average_turns, None);
    assert_eq!(report.contests[0].win_rate_ci95[0], None);
    assert_eq!(report.standings.len(), 2);

    let empty = ExplicitSchedule(Vec::new());
    let report = ArenaMonteCarlo::new(10)
        .with_seed(1)
        .run(&competitors, &empty, &StartingSeatWins)
        .unwrap();
    assert_eq!(report.games, 0);
    assert!(report.contests.is_empty());
    assert!(report.standings.iter().all(|entry| entry.record.games == 0));
}

#[test]
fn replay_rejects_trials_outside_the_run_coordinates() {
    let deck = inert_deck();
    let win = KOfTag::new("missing", 1);
    let params = Params::new(&deck, &win);
    let competitors = competitors(2, params);
    let trial_id = TrialId {
        contest_id: ContestId(0),
        sample_index: 2,
        seating_index: 0,
    };

    assert_eq!(
        ArenaMonteCarlo::new(2)
            .replay(&competitors, &RoundRobin, &StartingSeatWins, 1, trial_id,)
            .unwrap_err(),
        ArenaError::UnknownTrial(trial_id)
    );
}
