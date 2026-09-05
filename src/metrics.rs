use std::collections::BTreeMap;

/// The result of one simulated game.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrialOutcome {
    pub won: bool,
    /// Draws made after the kept opening hand. For a miss, this is the number
    /// of draws made before the simulation horizon ended.
    pub draws_after_opening: usize,
    /// Whether or not the opening hand had a win in it
    pub opening_win: bool,
    /// Lands in the provisional hand that was kept, before bottoming.
    pub opening_lands: usize,
    /// Cards retained after London-mulligan bottoming.
    pub kept: usize,
    /// `None` when the win condition was not met within the horizon.
    pub turns_to_win: Option<usize>,
}

/// Summary statistics over a set of trials.
#[derive(Clone, Debug, PartialEq)]
pub struct Aggregate {
    pub trials: usize,
    pub wins: usize,
    pub misses: usize,
    pub win_rate: f64,
    /// Average among winning trials only.
    pub avg_draws_after_opening: Option<f64>,
    pub opening_win_rate: f64,
    pub avg_opening_lands: f64,
    /// Average cards retained after London-mulligan bottoming, across all trials.
    pub avg_kept_hand_size: f64,
    /// Average among winning trials only.
    pub avg_turns_to_win: Option<f64>,
    /// Winning trials grouped by draws required. Misses are counted separately.
    pub distribution_draws_to_win: BTreeMap<usize, usize>,
    /// Winning trials grouped by the turn on which they won. Misses are counted separately.
    pub distribution_turns_to_win: BTreeMap<usize, usize>,
}

impl Default for Aggregate {
    fn default() -> Self {
        Self {
            trials: 0,
            wins: 0,
            misses: 0,
            win_rate: 0.0,
            avg_draws_after_opening: None,
            opening_win_rate: 0.0,
            avg_opening_lands: 0.0,
            avg_kept_hand_size: 0.0,
            avg_turns_to_win: None,
            distribution_draws_to_win: BTreeMap::new(),
            distribution_turns_to_win: BTreeMap::new(),
        }
    }
}

impl Aggregate {
    pub fn from_outcomes(outcomes: &[TrialOutcome]) -> Self {
        if outcomes.is_empty() {
            return Self::default();
        }

        let mut wins = 0;
        let mut opening_wins = 0;
        let mut sum_winning_draws = 0;
        let mut sum_winning_turns = 0;
        let mut sum_opening_lands = 0;
        let mut sum_kept = 0;
        let mut distribution = BTreeMap::new();
        let mut turn_distribution = BTreeMap::new();

        for outcome in outcomes {
            sum_opening_lands += outcome.opening_lands;
            sum_kept += outcome.kept;
            if outcome.opening_win {
                opening_wins += 1;
            }
            if outcome.won {
                wins += 1;
                sum_winning_draws += outcome.draws_after_opening;
                sum_winning_turns += outcome.turns_to_win.unwrap_or_default();
                *distribution.entry(outcome.draws_after_opening).or_insert(0) += 1;
                if let Some(turn) = outcome.turns_to_win {
                    *turn_distribution.entry(turn).or_insert(0) += 1;
                }
            }
        }

        let trials = outcomes.len();
        let winning_denominator = (wins > 0).then_some(wins as f64);

        Self {
            trials,
            wins,
            misses: trials - wins,
            win_rate: wins as f64 / trials as f64,
            avg_draws_after_opening: winning_denominator
                .map(|denominator| sum_winning_draws as f64 / denominator),
            opening_win_rate: opening_wins as f64 / trials as f64,
            avg_opening_lands: sum_opening_lands as f64 / trials as f64,
            avg_kept_hand_size: sum_kept as f64 / trials as f64,
            avg_turns_to_win: winning_denominator
                .map(|denominator| sum_winning_turns as f64 / denominator),
            distribution_draws_to_win: distribution,
            distribution_turns_to_win: turn_distribution,
        }
    }

    /// Number of trials that won on or before `turn`.
    pub fn wins_by_turn(&self, turn: usize) -> usize {
        self.distribution_turns_to_win
            .range(..=turn)
            .map(|(_, wins)| wins)
            .sum()
    }

    /// Probability of a win on or before `turn` over all trials.
    pub fn win_rate_by_turn(&self, turn: usize) -> f64 {
        if self.trials == 0 {
            0.0
        } else {
            self.wins_by_turn(turn) as f64 / self.trials as f64
        }
    }
}
