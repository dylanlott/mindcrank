use mindcrank::{Card, Deck, KeepIfWinOrDecent, MonteCarloParams, Params, TwoCardSet, monte_carlo};

fn make_deck(num_lands: usize, num_nonlands: usize) -> Deck {
    let mut cards = Vec::with_capacity(num_lands + num_nonlands);

    cards.extend(std::iter::repeat_n(
        Card::new("Land").with_tag("land"),
        num_lands,
    ));
    cards.push(Card::new("Thassa's Oracle").with_tags(["combo:oracle", "nonland"]));
    cards.push(Card::new("Demonic Consultation").with_tags(["combo:consult", "nonland"]));
    cards.extend(std::iter::repeat_n(
        Card::new("Filler").with_tag("nonland"),
        num_nonlands.saturating_sub(2),
    ));

    Deck::new(cards)
}

fn main() {
    let deck = make_deck(37, 62);
    let win = TwoCardSet::new("combo:oracle", "combo:consult");
    let mulligan = KeepIfWinOrDecent::new(&win, 2, 4);

    let mut params = Params::new(&deck, &win).london_mulligan(&mulligan, 3);
    params.max_turns = 50;
    params.draws_per_turn = 1;

    let aggregate = monte_carlo(
        MonteCarloParams::new(params, 1_000_000)
            .with_seed(0x5eed)
            .with_workers(0),
    );

    println!("Trials: {}", aggregate.trials);
    println!("Wins by turn 50: {:.2}%", aggregate.win_rate * 100.0);
    println!(
        "Average draws after opening (wins): {:.2}",
        aggregate.avg_draws_after_opening.unwrap_or_default()
    );
    println!(
        "Opening win rate: {:.4}%",
        aggregate.opening_win_rate * 100.0
    );
    println!("Average opening lands: {:.2}", aggregate.avg_opening_lands);
    println!(
        "Average turns to win (wins): {:.2}",
        aggregate.avg_turns_to_win.unwrap_or_default()
    );
}
