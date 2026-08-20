use mindcrank::arena::{
    ArenaError, ArenaMonteCarlo, Competitor, GoldfishRaceModel, MatchOutcome, MatchSimulator,
    Matchup, OutcomeReason, RoundRobin, Schedule, TiePolicy, TrialContext,
};
use mindcrank::{Card, Deck, KOfTag, Params};

struct StartingPlayerWins;

impl MatchSimulator for StartingPlayerWins {
    fn simulate(
        &self,
        _competitors: &[Competitor<'_>],
        _matchup: &Matchup,
        context: TrialContext,
    ) -> MatchOutcome {
        MatchOutcome::winner(context.starting_seat, 3, OutcomeReason::TurnOrderTieBreak)
    }
}

fn inert_deck() -> Deck {
    Deck::new(vec![Card::new("Filler"); 20])
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

    let matchups = RoundRobin.matchups(&competitors).unwrap();
    let pairs: Vec<_> = matchups
        .iter()
        .map(|matchup| {
            matchup
                .competitor_indices
                .map(|index| competitors[index].id.as_str())
        })
        .collect();

    assert_eq!(
        pairs,
        vec![
            ["alpha", "bravo"],
            ["alpha", "charlie"],
            ["bravo", "charlie"],
        ]
    );
    assert_eq!(matchups[0].id.0, 0);
    assert_eq!(matchups[2].id.0, 2);
}

#[test]
fn paired_trials_balance_starting_position() {
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
        .run(&competitors, &RoundRobin, &StartingPlayerWins)
        .unwrap();

    assert_eq!(report.matchups.len(), 3);
    for matchup in &report.matchups {
        assert_eq!(matchup.records[0].games, 10);
        assert_eq!(matchup.records[0].wins, 5);
        assert_eq!(matchup.records[1].wins, 5);
        assert_eq!(matchup.on_play[0].games, 5);
        assert_eq!(matchup.on_play[0].wins, 5);
        assert_eq!(matchup.on_draw[0].losses, 5);
    }

    for entry in &report.standings {
        assert_eq!(entry.record.games, 20);
        assert_eq!(entry.record.wins, 10);
        assert_eq!(entry.record.losses, 10);
        assert_eq!(entry.record.draws, 0);
        assert_eq!(entry.record.score_rate(), 0.5);
    }
}

#[test]
fn goldfish_race_awards_the_earlier_win() {
    let fast_deck = Deck::new(vec![Card::new("Threat").with_tag("win"); 20]);
    let slow_deck = inert_deck();
    let win = KOfTag::new("win", 1);
    let mut fast = Params::new(&fast_deck, &win);
    let mut slow = Params::new(&slow_deck, &win);
    fast.max_turns = 3;
    slow.max_turns = 3;
    let competitors = vec![Competitor::new("fast", fast), Competitor::new("slow", slow)];

    let report = ArenaMonteCarlo::new(8)
        .with_seed(7)
        .run(&competitors, &RoundRobin, &GoldfishRaceModel::new())
        .unwrap();
    let matchup = &report.matchups[0];

    assert_eq!(matchup.competitor_ids, ["fast", "slow"]);
    assert_eq!(matchup.records[0].wins, 8);
    assert_eq!(matchup.records[1].losses, 8);
    assert_eq!(matchup.average_turns, Some(0.0));

    let example = matchup.examples.winner[0].unwrap();
    let replay = ArenaMonteCarlo::new(8)
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
        MatchOutcome::winner(0, 0, OutcomeReason::WinCondition)
    );
}

#[test]
fn simultaneous_wins_follow_the_selected_tie_policy() {
    let deck = Deck::new(vec![Card::new("Threat").with_tag("win"); 20]);
    let win = KOfTag::new("win", 1);
    let params = Params::new(&deck, &win);
    let competitors = vec![
        Competitor::new("alpha", params),
        Competitor::new("bravo", params),
    ];

    let draws = ArenaMonteCarlo::new(4)
        .with_seed(11)
        .run(&competitors, &RoundRobin, &GoldfishRaceModel::new())
        .unwrap();
    assert_eq!(draws.matchups[0].records[0].draws, 4);

    let turn_order = GoldfishRaceModel::new().with_tie_policy(TiePolicy::StartingPlayer);
    let resolved = ArenaMonteCarlo::new(4)
        .with_seed(11)
        .run(&competitors, &RoundRobin, &turn_order)
        .unwrap();
    assert_eq!(resolved.matchups[0].records[0].wins, 2);
    assert_eq!(resolved.matchups[0].records[1].wins, 2);
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

    let single = ArenaMonteCarlo::new(2_000)
        .with_seed(99)
        .with_workers(1)
        .run(&competitors, &RoundRobin, &model)
        .unwrap();
    let parallel = ArenaMonteCarlo::new(2_000)
        .with_seed(99)
        .with_workers(4)
        .run(&competitors, &RoundRobin, &model)
        .unwrap();

    assert_eq!(single, parallel);

    let trial_id = single.matchups[0]
        .examples
        .winner
        .iter()
        .flatten()
        .next()
        .copied()
        .or(single.matchups[0].examples.draw)
        .unwrap();
    let first = ArenaMonteCarlo::new(2_000)
        .replay(&competitors, &RoundRobin, &model, single.seed, trial_id)
        .unwrap();
    let second = ArenaMonteCarlo::new(2_000)
        .replay(&competitors, &RoundRobin, &model, single.seed, trial_id)
        .unwrap();
    assert_eq!(first, second);
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

    let error = ArenaMonteCarlo::new(2)
        .run(&competitors, &RoundRobin, &StartingPlayerWins)
        .unwrap_err();

    assert_eq!(error, ArenaError::DuplicateCompetitorId("same".into()));
}

#[test]
fn zero_trials_produces_empty_records_and_standings_entries() {
    let deck = inert_deck();
    let win = KOfTag::new("missing", 1);
    let params = Params::new(&deck, &win);
    let competitors = vec![
        Competitor::new("alpha", params),
        Competitor::new("bravo", params),
    ];

    let report = ArenaMonteCarlo::new(0)
        .with_seed(1)
        .run(&competitors, &RoundRobin, &StartingPlayerWins)
        .unwrap();

    assert_eq!(report.matchups[0].records[0].games, 0);
    assert_eq!(report.matchups[0].average_turns, None);
    assert_eq!(report.matchups[0].win_rate_ci95[0], None);
    assert_eq!(report.standings.len(), 2);
}
