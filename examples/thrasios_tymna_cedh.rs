//! A tournament-backed Thrasios/Tymna cEDH simulation.
//!
//! Run with:
//!
//! ```sh
//! cargo run --release --example thrasios_tymna_cedh
//! ```
//!
//! This is deliberately an *assembly/access* model, not a Magic rules engine.
//! A hit means the cards seen contain a natural two-card line, or (in the
//! tutor-aware scenarios) enough distinct tutors to cover its missing pieces.
//! It does not pay mana, resolve triggers, sequence permanents, fight through
//! interaction, or verify that a top-deck tutor has had another draw step.

use mindcrank::arena::{ArenaMonteCarlo, Competitor, GoldfishRaceModel, RoundRobin};
use mindcrank::{
    Aggregate, AnyOf, BottomHeuristic, Card, Deck, DeckCandidate, KOfTag, MonteCarloParams,
    MulliganPolicy, Params, ParetoProtocol, Piece, Route, TutorAwareWin, WinCondition,
    compare_pareto, count_tag, monte_carlo, run_once,
};

const DECKLIST: &str = include_str!("artifacts/thrasios_tymna_zenith_2026.txt");

const TRIALS: usize = 200_000;
const PARETO_TRIALS: usize = 100_000;
const ARENA_SAMPLES: usize = 25_000;
const SEED: u64 = 0x7a7a_2026;
const HAND_SIZE: usize = 7;
const MAX_MULLIGANS: usize = 3;
const HORIZON_TURN: usize = 7;
const FAST_TURN: usize = 3;

/// Card roles read by the model. Untagged cards remain meaningful deck slots,
/// but are inert to the questions asked here.
const TAG_GROUPS: &[(&str, &[&str])] = &[
    (
        "land",
        &[
            "Ancient Tomb",
            "Bayou",
            "Bloodstained Mire",
            "Boseiju, Who Endures",
            "Breeding Pool",
            "City of Brass",
            "Command Tower",
            "Emergence Zone",
            "Exotic Orchard",
            "Flooded Strand",
            "Gaea's Cradle",
            "Gemstone Caverns",
            "Mana Confluence",
            "Marsh Flats",
            "Misty Rainforest",
            "Otawara, Soaring City",
            "Polluted Delta",
            "Savannah",
            "Scalding Tarn",
            "Scrubland",
            "Shifting Woodland",
            "Talon Gates of Madara",
            "Tropical Island",
            "Tundra",
            "Underground Sea",
            "Verdant Catacombs",
            "Windswept Heath",
        ],
    ),
    (
        "mana:accelerant",
        &[
            "Birds of Paradise",
            "Deathrite Shaman",
            "Delighted Halfling",
            "Noble Hierarch",
            "Chrome Mox",
            "Lotus Petal",
            "Mox Amber",
            "Mox Diamond",
            "Mana Vault",
            "Sol Ring",
            "Arcane Signet",
        ],
    ),
    (
        "mana:source",
        &[
            "Ancient Tomb",
            "Bayou",
            "Bloodstained Mire",
            "Boseiju, Who Endures",
            "Breeding Pool",
            "City of Brass",
            "Command Tower",
            "Emergence Zone",
            "Exotic Orchard",
            "Flooded Strand",
            "Gaea's Cradle",
            "Gemstone Caverns",
            "Mana Confluence",
            "Marsh Flats",
            "Misty Rainforest",
            "Otawara, Soaring City",
            "Polluted Delta",
            "Savannah",
            "Scalding Tarn",
            "Scrubland",
            "Shifting Woodland",
            "Talon Gates of Madara",
            "Tropical Island",
            "Tundra",
            "Underground Sea",
            "Verdant Catacombs",
            "Windswept Heath",
            "Birds of Paradise",
            "Deathrite Shaman",
            "Delighted Halfling",
            "Noble Hierarch",
            "Chrome Mox",
            "Lotus Petal",
            "Mox Amber",
            "Mox Diamond",
            "Mana Vault",
            "Sol Ring",
            "Arcane Signet",
        ],
    ),
    (
        "draw",
        &[
            "Esper Sentinel",
            "Faerie Mastermind",
            "Wan Shi Tong, Librarian",
            "The One Ring",
            "Mystic Remora",
            "Rhystic Study",
        ],
    ),
    (
        "protection",
        &[
            "Grand Abolisher",
            "Voice of Victory",
            "Ranger-Captain of Eos",
            "Pact of Negation",
            "Flusterstorm",
            "Mental Misstep",
            "Silence",
            "Swan Song",
            "Veil of Summer",
            "Fierce Guardianship",
            "Flare of Denial",
            "Force of Negation",
            "Mindbreak Trap",
            "Force of Will",
        ],
    ),
    (
        "interaction",
        &[
            "Drannith Magistrate",
            "Orcish Bowmasters",
            "Opposition Agent",
            "Pact of Negation",
            "Flusterstorm",
            "Mental Misstep",
            "Silence",
            "Swan Song",
            "Veil of Summer",
            "Abrupt Decay",
            "Snap",
            "Fierce Guardianship",
            "Flare of Denial",
            "Force of Negation",
            "Deadly Rollick",
            "Mindbreak Trap",
            "Force of Will",
        ],
    ),
    ("combo:oracle", &["Thassa's Oracle"]),
    ("combo:consult", &["Demonic Consultation", "Tainted Pact"]),
    ("combo:kinnan", &["Kinnan, Bonder Prodigy"]),
    ("combo:basalt", &["Basalt Monolith"]),
    ("combo:druid", &["Devoted Druid"]),
    ("combo:swift", &["Swift Reconfiguration"]),
    (
        "tutor:any",
        &["Demonic Tutor", "Vampiric Tutor", "Wishclaw Talisman"],
    ),
    (
        "tutor:creature",
        &[
            "Demonic Tutor",
            "Vampiric Tutor",
            "Wishclaw Talisman",
            "Finale of Devastation",
            "Nature's Rhythm",
            "Eldritch Evolution",
            "Chord of Calling",
            "Survival of the Fittest",
        ],
    ),
    (
        "tutor:artifact",
        &[
            "Demonic Tutor",
            "Vampiric Tutor",
            "Wishclaw Talisman",
            "Enlightened Tutor",
        ],
    ),
    (
        "tutor:enchantment",
        &[
            "Demonic Tutor",
            "Vampiric Tutor",
            "Wishclaw Talisman",
            "Enlightened Tutor",
        ],
    ),
    (
        "tutor:instant",
        &["Demonic Tutor", "Vampiric Tutor", "Wishclaw Talisman"],
    ),
    (
        "tutor",
        &[
            "Demonic Tutor",
            "Vampiric Tutor",
            "Wishclaw Talisman",
            "Finale of Devastation",
            "Nature's Rhythm",
            "Eldritch Evolution",
            "Chord of Calling",
            "Survival of the Fittest",
            "Enlightened Tutor",
        ],
    ),
];

fn routes() -> Vec<Route> {
    vec![
        Route::new(
            "Thassa's Oracle + consultation effect",
            [
                Piece::new("combo:oracle", "tutor:creature"),
                Piece::new("combo:consult", "tutor:instant"),
            ],
        ),
        Route::new(
            "Kinnan + Basalt Monolith (Thrasios outlet in command zone)",
            [
                Piece::new("combo:kinnan", "tutor:creature"),
                Piece::new("combo:basalt", "tutor:artifact"),
            ],
        ),
        Route::new(
            "Devoted Druid + Swift Reconfiguration (Thrasios outlet in command zone)",
            [
                Piece::new("combo:druid", "tutor:creature"),
                Piece::new("combo:swift", "tutor:enchantment"),
            ],
        ),
    ]
}

/// A cEDH-flavored opening policy: require a land, at least two approximate
/// mana sources, and either card advantage or progress toward a combo.
struct CedhMulligan<'a> {
    access: &'a TutorAwareWin,
}

impl MulliganPolicy for CedhMulligan<'_> {
    fn keep(&self, hand: &[Card]) -> bool {
        let lands = count_tag(hand, "land");
        let mana = count_tag(hand, "mana:source");
        let has_plan = count_tag(hand, "draw") > 0
            || count_tag(hand, "tutor") > 0
            || self
                .access
                .routes()
                .iter()
                .flat_map(|route| &route.pieces)
                .any(|piece| count_tag(hand, &piece.role) > 0);

        self.access.satisfied(hand) || ((1..=4).contains(&lands) && mana >= 2 && has_plan)
    }
}

/// A hand-aware London bottomer. It protects combo pieces and tutors, keeps the
/// first two lands, then values acceleration, draw, protection, interaction,
/// and inert cards in that order.
struct CedhBottomer;

impl BottomHeuristic for CedhBottomer {
    fn cards_to_bottom(&self, hand: &[Card], count: usize, win: &dyn WinCondition) -> Vec<usize> {
        let mut land_ordinal = 0usize;
        let mut scored = hand
            .iter()
            .enumerate()
            .map(|(index, card)| {
                let score = if card.has_tag("land") {
                    land_ordinal += 1;
                    if land_ordinal <= 2 { 90 } else { 45 }
                } else if card.has_tag("mana:accelerant") {
                    88
                } else if card.has_tag("draw") {
                    84
                } else if card.has_tag("protection") {
                    78
                } else if card.has_tag("interaction") {
                    60
                } else {
                    10
                };

                (index, score.max(win.card_priority(card)))
            })
            .collect::<Vec<_>>();

        scored.sort_by_key(|&(index, score)| (score, index));
        scored
            .into_iter()
            .take(count.min(hand.len()))
            .map(|(index, _)| index)
            .collect()
    }
}

#[derive(Clone, Debug)]
struct Entry {
    count: usize,
    name: String,
    card_type: String,
}

fn main() {
    let (mainboard, commanders) = parse_decklist(DECKLIST);
    let deck = build_deck(&mainboard);

    report_deck(&deck, &commanders);
    assert_eq!(deck.len(), 98, "Commander library must contain 98 cards");
    assert_eq!(commanders.len(), 2, "partner pair must contain two cards");

    let access = TutorAwareWin::new(routes());
    let natural = natural_combo_condition(access.routes());
    let mulligan = CedhMulligan { access: &access };
    let bottomer = CedhBottomer;

    println!("Modeled natural routes:");
    for route in access.routes() {
        println!("  - {}", route.name);
    }
    println!();

    let debug_params = params(&deck, &access, &mulligan, &bottomer, 1).with_seed(SEED);
    println!("Reproducible single trial: {:?}\n", run_once(&debug_params));

    let scenarios: [(&str, &dyn WinCondition, usize); 3] = [
        ("natural pieces only", &natural, 1),
        ("tutor-aware access", &access, 1),
        ("tutor-aware + sustained draw", &access, 2),
    ];

    let results = scenarios
        .iter()
        .map(|(label, win, draws)| {
            let aggregate = monte_carlo(
                MonteCarloParams::new(params(&deck, *win, &mulligan, &bottomer, *draws), TRIALS)
                    .with_seed(SEED)
                    .with_workers(0),
            );
            (*label, *draws, aggregate)
        })
        .collect::<Vec<_>>();

    report_scenarios(&results);

    let mana = KOfTag::new("mana:source", 3);
    let mana_result = monte_carlo(
        MonteCarloParams::new(params(&deck, &mana, &mulligan, &bottomer, 1), TRIALS)
            .with_seed(SEED)
            .with_workers(0),
    );
    println!(
        "\nMana checkpoint: three tagged mana-source cards seen by turn 2: {:.2}%",
        mana_result.win_rate_by_turn(2) * 100.0
    );

    report_pareto(&deck, &access, &mulligan, &bottomer);
    report_arena(&deck, &access, &mulligan, &bottomer);
    report_limitations();
}

fn natural_combo_condition(routes: &[Route]) -> AnyOf {
    AnyOf::new(
        routes
            .iter()
            .cloned()
            .map(|route| Box::new(route) as Box<dyn WinCondition>)
            .collect(),
    )
}

fn params<'a>(
    deck: &'a Deck,
    win: &'a dyn WinCondition,
    mulligan: &'a dyn MulliganPolicy,
    bottomer: &'a dyn BottomHeuristic,
    draws_per_turn: usize,
) -> Params<'a> {
    let mut params = Params::new(deck, win)
        .london_mulligan(mulligan, MAX_MULLIGANS)
        .bottom_with(bottomer);
    params.hand_size = HAND_SIZE;
    params.max_turns = HORIZON_TURN;
    params.draws_per_turn = draws_per_turn;
    params
}

fn report_scenarios(results: &[(&str, usize, Aggregate)]) {
    println!("Scenario results ({TRIALS} trials each, fixed seed {SEED:#x}):");
    println!(
        "  {:<31} {:>5} {:>8} {:>8} {:>8} {:>8}",
        "scenario", "draws", "opening", "turn 3", "turn 5", "turn 7"
    );
    for (label, draws, aggregate) in results {
        println!(
            "  {label:<31} {draws:>5} {:>7.2}% {:>7.2}% {:>7.2}% {:>7.2}%",
            aggregate.opening_win_rate * 100.0,
            aggregate.win_rate_by_turn(3) * 100.0,
            aggregate.win_rate_by_turn(5) * 100.0,
            aggregate.win_rate_by_turn(7) * 100.0,
        );
    }

    println!("\nTutor-aware cumulative access curve:");
    let aggregate = &results[1].2;
    for turn in 0..=HORIZON_TURN {
        println!(
            "  turn {turn}: {:>6.2}% ({:>6} trials)",
            aggregate.win_rate_by_turn(turn) * 100.0,
            aggregate.wins_by_turn(turn),
        );
    }
    println!(
        "  avg kept hand: {:.2}; avg opening lands: {:.2}; avg turn on hits: {:.2}",
        aggregate.avg_kept_hand_size,
        aggregate.avg_opening_lands,
        aggregate.avg_turns_to_win.unwrap_or_default(),
    );
}

fn report_pareto(
    stock: &Deck,
    access: &TutorAwareWin,
    mulligan: &dyn MulliganPolicy,
    bottomer: &dyn BottomHeuristic,
) {
    let no_tutors = replace_tagged(stock, "tutor", "Inert tutor replacement");
    let tutor_dense = add_universal_tutor_proxies(stock, 2);
    let protocol = ParetoProtocol::from_params(
        params(stock, access, mulligan, bottomer, 1),
        FAST_TURN,
        HORIZON_TURN,
        PARETO_TRIALS,
        SEED,
    )
    .expect("valid Pareto protocol")
    .with_workers(0);

    let report = compare_pareto(
        &[
            DeckCandidate::new("stock", "Stock tournament list", stock),
            DeckCandidate::new("no-tutors", "Tutors replaced with inert cards", &no_tutors),
            DeckCandidate::new(
                "tutor-dense",
                "Two interaction slots become tutors",
                &tutor_dense,
            ),
        ],
        protocol,
    )
    .expect("valid Pareto candidates");

    println!("\nPareto sensitivity (same protocol; decklist is the only variable):");
    for point in report.scatterplot().points {
        println!(
            "  {:<39} turn {} {:>6.2}% | turn {} {:>6.2}% | frontier: {}",
            point.label,
            FAST_TURN,
            point.x * 100.0,
            HORIZON_TURN,
            point.y * 100.0,
            point.is_frontier,
        );
    }
    println!("\nPlot-ready Pareto CSV:\n{}", report.to_csv(false));
}

fn replace_tagged(deck: &Deck, tag: &str, replacement: &str) -> Deck {
    Deck::new(
        deck.cards()
            .iter()
            .map(|card| {
                if card.has_tag(tag) {
                    Card::new(replacement).with_type("Replacement")
                } else {
                    card.clone()
                }
            })
            .collect(),
    )
}

fn add_universal_tutor_proxies(deck: &Deck, count: usize) -> Deck {
    let mut replaced = 0usize;
    Deck::new(
        deck.cards()
            .iter()
            .map(|card| {
                if replaced < count && card.has_tag("interaction") && !card.has_tag("protection") {
                    replaced += 1;
                    Card::new(format!("Universal Tutor Proxy {replaced}"))
                        .with_type("Sorcery")
                        .with_tags([
                            "tutor",
                            "tutor:any",
                            "tutor:creature",
                            "tutor:artifact",
                            "tutor:enchantment",
                            "tutor:instant",
                        ])
                } else {
                    card.clone()
                }
            })
            .collect(),
    )
}

fn report_arena(
    stock: &Deck,
    access: &TutorAwareWin,
    mulligan: &dyn MulliganPolicy,
    bottomer: &dyn BottomHeuristic,
) {
    let no_tutors = replace_tagged(stock, "tutor", "Inert tutor replacement");
    let tutor_dense = add_universal_tutor_proxies(stock, 2);
    let competitors = [
        Competitor::new("stock", params(stock, access, mulligan, bottomer, 1))
            .named("Stock tournament list"),
        Competitor::new(
            "no-tutors",
            params(&no_tutors, access, mulligan, bottomer, 1),
        )
        .named("Tutors replaced"),
        Competitor::new(
            "tutor-dense",
            params(&tutor_dense, access, mulligan, bottomer, 1),
        )
        .named("Two extra tutors"),
    ];
    let report = ArenaMonteCarlo::new(ARENA_SAMPLES)
        .with_seed(SEED)
        .with_workers(0)
        .run(&competitors, &RoundRobin, &GoldfishRaceModel::new())
        .expect("valid arena setup");

    println!(
        "Arena goldfish round robin ({ARENA_SAMPLES} samples per matchup; {} seat-balanced games):",
        report.games
    );
    for entry in report.standings {
        println!(
            "  {:<23} {:>6.2}% score ({:>6} W / {:>6} L / {:>6} D)",
            entry.name,
            entry.record.score_rate() * 100.0,
            entry.record.wins,
            entry.record.losses,
            entry.record.draws,
        );
    }
    println!();
}

fn report_limitations() {
    println!("Model boundaries:");
    println!("  - Results are combo assembly/access rates, not game win rates.");
    println!("  - Tutors are deterministic substitutes and do not pay mana or consume a turn.");
    println!("  - The two-draw scenario is a ceiling with an engine online from turn one.");
    println!("  - 'mana source' ignores colors, summoning sickness, and conditional costs.");
    println!("  - Opponents, stack interaction, board state, and commander casting are omitted.");
    println!("  - The arena is an independent goldfish race, not a multiplayer game model.");
}

fn normalize(name: &str) -> String {
    name.trim()
        .chars()
        .filter(|character| !matches!(character, '\'' | '\u{2019}'))
        .flat_map(char::to_lowercase)
        .collect()
}

fn parse_decklist(raw: &str) -> (Vec<Entry>, Vec<Entry>) {
    enum Destination {
        Main,
        Commander,
        Skip,
    }

    let mut destination = Destination::Main;
    let mut card_type = "Unknown".to_string();
    let mut mainboard = Vec::new();
    let mut commanders = Vec::new();

    for raw_line in raw.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("//") {
            continue;
        }

        if !line.starts_with(|character: char| character.is_ascii_digit()) {
            card_type = line.to_string();
            let header = line.to_lowercase();
            destination = if header.contains("commander") {
                Destination::Commander
            } else if header.contains("sideboard") || header.contains("maybeboard") {
                Destination::Skip
            } else {
                Destination::Main
            };
            continue;
        }

        let digit_count = line
            .chars()
            .take_while(|character| character.is_ascii_digit())
            .count();
        let count = line[..digit_count].parse().unwrap_or(1);
        let name = line[digit_count..]
            .trim_start()
            .strip_prefix('x')
            .unwrap_or(line[digit_count..].trim_start())
            .trim_start()
            .to_string();
        let entry = Entry {
            count,
            name,
            card_type: card_type.clone(),
        };

        match destination {
            Destination::Main => mainboard.push(entry),
            Destination::Commander => commanders.push(entry),
            Destination::Skip => {}
        }
    }

    (mainboard, commanders)
}

fn build_deck(entries: &[Entry]) -> Deck {
    let mut cards = Vec::new();
    for entry in entries {
        let key = normalize(&entry.name);
        let mut card = Card::new(&entry.name).with_type(&entry.card_type);
        for (tag, names) in TAG_GROUPS {
            if names.iter().any(|name| normalize(name) == key) {
                card = card.with_tag(*tag);
            }
        }
        cards.extend(std::iter::repeat_n(card, entry.count));
    }
    Deck::new(cards)
}

fn report_deck(deck: &Deck, commanders: &[Entry]) {
    println!("Thrasios/Tymna cEDH — Facundo Cabrera, Zenith cEDH 7/3 (1st)");
    println!("Source artifact: examples/artifacts/thrasios_tymna_zenith_2026.txt");
    println!("Library: {} cards", deck.len());
    println!(
        "Command zone: {}",
        commanders
            .iter()
            .map(|entry| entry.name.as_str())
            .collect::<Vec<_>>()
            .join(" / ")
    );

    for tag in [
        "land",
        "mana:accelerant",
        "mana:source",
        "draw",
        "tutor",
        "protection",
        "interaction",
    ] {
        println!("  {tag:<18} {}", count_tag(deck.cards(), tag));
    }

    let present = deck
        .cards()
        .iter()
        .map(|card| normalize(&card.name))
        .collect::<Vec<_>>();
    let unmatched = TAG_GROUPS
        .iter()
        .flat_map(|(tag, names)| names.iter().map(move |name| (*tag, *name)))
        .filter(|(_, name)| !present.contains(&normalize(name)))
        .collect::<Vec<_>>();
    assert!(
        unmatched.is_empty(),
        "tag names missing from deck: {unmatched:?}"
    );
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tagged(name: &str, tags: &[&str]) -> Card {
        Card::new(name).with_tags(tags.iter().copied())
    }

    #[test]
    fn artifact_has_98_card_library_and_two_commanders() {
        let (mainboard, commanders) = parse_decklist(DECKLIST);
        assert_eq!(mainboard.iter().map(|entry| entry.count).sum::<usize>(), 98);
        assert_eq!(commanders.iter().map(|entry| entry.count).sum::<usize>(), 2);
        assert_eq!(build_deck(&mainboard).len(), 98);
    }

    #[test]
    fn tutor_matching_does_not_double_count_one_tutor() {
        let access = TutorAwareWin::new(routes());
        let one_tutor = vec![tagged(
            "Demonic Tutor",
            &["tutor", "tutor:any", "tutor:creature", "tutor:instant"],
        )];
        assert!(!access.satisfied(&one_tutor));

        let two_tutors = vec![
            tagged("Demonic Tutor", &["tutor", "tutor:any"]),
            tagged("Wishclaw Talisman", &["tutor", "tutor:any"]),
        ];
        assert!(access.satisfied(&two_tutors));
    }

    #[test]
    fn typed_tutor_only_covers_eligible_piece() {
        let access = TutorAwareWin::new(routes());
        let oracle_and_creature_tutor = vec![
            tagged("Thassa's Oracle", &["combo:oracle"]),
            tagged("Finale of Devastation", &["tutor", "tutor:creature"]),
        ];
        assert!(!access.satisfied(&oracle_and_creature_tutor));

        let consult_and_creature_tutor = vec![
            tagged("Demonic Consultation", &["combo:consult"]),
            tagged("Finale of Devastation", &["tutor", "tutor:creature"]),
        ];
        assert!(access.satisfied(&consult_and_creature_tutor));
    }

    #[test]
    fn every_tagged_name_exists_in_artifact() {
        let (mainboard, _) = parse_decklist(DECKLIST);
        let present = mainboard
            .iter()
            .map(|entry| normalize(&entry.name))
            .collect::<Vec<_>>();
        for (tag, names) in TAG_GROUPS {
            for name in *names {
                assert!(
                    present.contains(&normalize(name)),
                    "{name} in tag {tag} is absent"
                );
            }
        }
    }
}
