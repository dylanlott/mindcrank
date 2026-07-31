//! Monte Carlo simulation harness for a single decklist.
//!
//! Copied from `.claude/skills/mindcrank-simulate/templates/simulate.rs`.
//! Edit the four numbered sections below, then run:
//!
//! ```sh
//! cargo run --release --example sim_scratch
//! ```

use mindcrank::{
    Aggregate, Card, Deck, KeepIfWinOrDecent, MonteCarloParams, Params, TwoCardSet, WinCondition,
    count_tag, monte_carlo,
};

// ===========================================================================
// 1. DECKLIST
// ===========================================================================
// One card per line, each starting with a count: `4 Lightning Bolt`.
// `4x Name`, `1 Name (SET) 123`, and `#`/`//` comments are also accepted.
// Lines that do not start with a digit are treated as section headers:
// a header containing "sideboard" or "maybeboard" skips everything until the
// next header, and a header containing "commander" moves those cards to the
// command zone (excluded from the shuffled library).
const DECKLIST: &str = "\
Deck
37 Island
1 Thassa's Oracle
1 Demonic Consultation
60 Filler
";

// ===========================================================================
// 2. TAGS
// ===========================================================================
// `("tag", &["Card A", "Card B"])`. A card may appear in several groups.
// Names are matched case-insensitively, ignoring apostrophe style.
// Untagged cards are inert filler — that is fine and expected.
//
// Reserved tags the engine itself reads:
//   "land"  -> the `avg_opening_lands` metric, land-based mulligan policies,
//              and bottoming priority. Tag every land.
//   "tutor" / "draw" -> bottoming priority only (kept over filler).
const TAG_GROUPS: &[(&str, &[&str])] = &[
    ("land", &["Island"]),
    ("combo:oracle", &["Thassa's Oracle"]),
    ("combo:consult", &["Demonic Consultation"]),
];

// ===========================================================================
// 3. SIMULATION SETUP
// ===========================================================================
const TRIALS: usize = 200_000;
const SEED: u64 = 0x5eed;
const MAX_TURNS: usize = 10;
const DRAWS_PER_TURN: usize = 1;
const HAND_SIZE: usize = 7;
const MAX_MULLIGANS: usize = 2;
/// Inclusive land window a non-winning opening hand must land in to be kept.
const KEEP_LANDS: (usize, usize) = (2, 5);

fn win_condition() -> impl WinCondition {
    TwoCardSet::new("combo:oracle", "combo:consult")
}

// ===========================================================================
// 4. MAIN
// ===========================================================================
fn main() {
    let (main_deck, command_zone) = parse_decklist(DECKLIST);
    let deck = build_deck(&main_deck);

    report_deck(&deck, &command_zone);

    let win = win_condition();
    let mulligan = KeepIfWinOrDecent::new(&win, KEEP_LANDS.0, KEEP_LANDS.1);

    let mut params = Params::new(&deck, &win).london_mulligan(&mulligan, MAX_MULLIGANS);
    params.hand_size = HAND_SIZE;
    params.max_turns = MAX_TURNS;
    params.draws_per_turn = DRAWS_PER_TURN;

    let aggregate = monte_carlo(
        MonteCarloParams::new(params, TRIALS)
            .with_seed(SEED)
            .with_workers(0),
    );

    report_results(&aggregate);
}

// ===========================================================================
// Harness — no need to edit below this line.
// ===========================================================================

/// Lowercases and folds typographic apostrophes so hand-typed card names match
/// exported decklists.
fn normalize(name: &str) -> String {
    name.trim()
        .chars()
        .filter(|c| !matches!(c, '\'' | '\u{2019}'))
        .flat_map(char::to_lowercase)
        .collect()
}

/// Parsed decklist lines, as `(count, card name)`.
type Entries = Vec<(usize, String)>;

/// Returns `(main deck, command zone)`.
fn parse_decklist(raw: &str) -> (Entries, Entries) {
    enum Section {
        Main,
        Commander,
        Skip,
    }

    let mut section = Section::Main;
    let mut main = Vec::new();
    let mut commander = Vec::new();

    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("//") {
            continue;
        }

        if !line.starts_with(|c: char| c.is_ascii_digit()) {
            let header = line.to_lowercase();
            section = if header.contains("sideboard") || header.contains("maybeboard") {
                Section::Skip
            } else if header.contains("commander") {
                Section::Commander
            } else {
                Section::Main
            };
            continue;
        }

        let (count, name) = split_entry(line);
        match section {
            Section::Main => main.push((count, name)),
            Section::Commander => commander.push((count, name)),
            Section::Skip => {}
        }
    }

    (main, commander)
}

/// Splits `4x Lightning Bolt (M10) 146` into `(4, "Lightning Bolt")`.
fn split_entry(line: &str) -> (usize, String) {
    let digits: String = line.chars().take_while(char::is_ascii_digit).collect();
    let count = digits.parse().unwrap_or(1);
    let mut rest = line[digits.len()..].trim_start();
    rest = rest.strip_prefix('x').unwrap_or(rest).trim_start();

    // Trailing `(SET) 123` / `(SET)` printing information.
    let name = match rest.rfind('(') {
        Some(open) => rest[..open].trim_end(),
        None => rest.trim_end(),
    };

    (count, name.to_string())
}

fn build_deck(entries: &[(usize, String)]) -> Deck {
    let mut cards = Vec::new();

    for (count, name) in entries {
        let key = normalize(name);
        let mut card = Card::new(name.clone());
        for (tag, names) in TAG_GROUPS {
            if names.iter().any(|listed| normalize(listed) == key) {
                card = card.with_tag(*tag);
            }
        }
        cards.extend(std::iter::repeat_n(card, *count));
    }

    Deck::new(cards)
}

/// Prints deck composition and flags tag entries that matched no card, which is
/// almost always a misspelled card name rather than an intentional omission.
fn report_deck(deck: &Deck, command_zone: &[(usize, String)]) {
    println!("Library: {} cards", deck.len());

    if !command_zone.is_empty() {
        let names: Vec<_> = command_zone.iter().map(|(_, name)| name.as_str()).collect();
        println!("Command zone (excluded from library): {}", names.join(", "));
    }

    let present: Vec<String> = deck
        .cards()
        .iter()
        .map(|card| normalize(&card.name))
        .collect();
    let mut unmatched = Vec::new();

    for (tag, names) in TAG_GROUPS {
        println!("  {tag}: {}", count_tag(deck.cards(), tag));
        for listed in *names {
            let key = normalize(listed);
            if !present.contains(&key) {
                unmatched.push(format!("{listed} (tag \"{tag}\")"));
            }
        }
    }

    if !unmatched.is_empty() {
        println!("\nWARNING: tagged names not found in the decklist:");
        for entry in &unmatched {
            println!("  - {entry}");
        }
        println!("Check spelling, or move the card to the command zone section.");
    }

    println!();
}

fn report_results(aggregate: &Aggregate) {
    let rate = aggregate.win_rate;
    // 95% Monte Carlo confidence interval on the win rate itself. It says
    // nothing about whether the model matches real games.
    let margin = if aggregate.trials > 0 {
        1.96 * (rate * (1.0 - rate) / aggregate.trials as f64).sqrt()
    } else {
        0.0
    };

    println!("Trials: {}", aggregate.trials);
    println!(
        "Assembled by turn {MAX_TURNS}: {:.2}% (+/- {:.2}pp, 95% CI)",
        rate * 100.0,
        margin * 100.0
    );
    println!("  wins: {}  misses: {}", aggregate.wins, aggregate.misses);
    println!(
        "In the opening hand: {:.2}%",
        aggregate.opening_win_rate * 100.0
    );
    println!("Average opening lands: {:.2}", aggregate.avg_opening_lands);

    match aggregate.avg_turns_to_win {
        Some(turns) => println!("Average turns to assemble (wins only): {turns:.2}"),
        None => println!("Average turns to assemble: n/a (no wins)"),
    }
    match aggregate.avg_draws_after_opening {
        Some(draws) => println!("Average draws after opening (wins only): {draws:.2}"),
        None => println!("Average draws after opening: n/a (no wins)"),
    }

    println!("\nCumulative assembly rate by turn:");
    let mut cumulative = 0usize;
    for turn in 0..=MAX_TURNS {
        // With DRAWS_PER_TURN draws each turn, a win on turn N has taken
        // N * DRAWS_PER_TURN draws.
        cumulative += aggregate
            .distribution_draws_to_win
            .get(&(turn * DRAWS_PER_TURN))
            .copied()
            .unwrap_or(0);
        println!(
            "  turn {turn:>2}: {:>6.2}%",
            cumulative as f64 / aggregate.trials as f64 * 100.0
        );
    }
}
