use mindcrank::arena::{ArenaMonteCarlo, Competitor, GoldfishRaceModel, RoundRobin};
use mindcrank::{Card, Deck, KOfTag, Params};

fn deck(threats: usize) -> Deck {
    let mut cards = vec![Card::new("Threat").with_tag("win"); threats];
    cards.extend(vec![Card::new("Filler"); 60 - threats]);
    Deck::new(cards)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let fast_deck = deck(8);
    let medium_deck = deck(5);
    let slow_deck = deck(2);
    let win = KOfTag::new("win", 1);

    let mut fast = Params::new(&fast_deck, &win);
    let mut medium = Params::new(&medium_deck, &win);
    let mut slow = Params::new(&slow_deck, &win);
    fast.max_turns = 10;
    medium.max_turns = 10;
    slow.max_turns = 10;

    let competitors = vec![
        Competitor::new("fast", fast).named("Eight threats"),
        Competitor::new("medium", medium).named("Five threats"),
        Competitor::new("slow", slow).named("Two threats"),
    ];
    let report = ArenaMonteCarlo::new(100_000).with_seed(0x5eed).run(
        &competitors,
        &RoundRobin,
        &GoldfishRaceModel::new(),
    )?;

    println!("seed: {}", report.seed);
    println!(
        "{} samples per contest; {} total games",
        report.samples_per_contest, report.games
    );
    for entry in report.standings {
        println!(
            "{:<16} {:>6.2}% score ({:>6} W / {:>6} L / {:>6} D)",
            entry.name,
            entry.record.score_rate() * 100.0,
            entry.record.wins,
            entry.record.losses,
            entry.record.draws,
        );
    }

    Ok(())
}
