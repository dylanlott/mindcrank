use std::collections::BTreeMap;

use super::{Competitor, MatchResult, Matchup, MatchupId, TrialId, TrialRecord};

/// Wins, losses, and draws from one competitor's perspective.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Record {
    pub games: usize,
    pub wins: usize,
    pub losses: usize,
    pub draws: usize,
}

impl Record {
    pub fn win_rate(&self) -> f64 {
        rate(self.wins, self.games)
    }

    /// A draw contributes half a point.
    pub fn score_rate(&self) -> f64 {
        if self.games == 0 {
            0.0
        } else {
            (self.wins as f64 + self.draws as f64 * 0.5) / self.games as f64
        }
    }

    pub fn win_rate_ci95(&self) -> Option<ConfidenceInterval> {
        wilson_interval(self.wins, self.games)
    }

    fn record_win(&mut self) {
        self.games += 1;
        self.wins += 1;
    }

    fn record_loss(&mut self) {
        self.games += 1;
        self.losses += 1;
    }

    fn record_draw(&mut self) {
        self.games += 1;
        self.draws += 1;
    }

    fn merge(&mut self, other: Self) {
        self.games += other.games;
        self.wins += other.wins;
        self.losses += other.losses;
        self.draws += other.draws;
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ConfidenceInterval {
    pub low: f64,
    pub high: f64,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct OutcomeExamples {
    /// One replayable winning trial for each competitor, when observed.
    pub winner: [Option<TrialId>; 2],
    pub draw: Option<TrialId>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MatchupReport {
    pub matchup_id: MatchupId,
    pub competitor_ids: [String; 2],
    /// Overall records in the same order as `competitor_ids`.
    pub records: [Record; 2],
    /// Records restricted to games where the corresponding competitor started.
    pub on_play: [Record; 2],
    /// Records restricted to games where the corresponding competitor did not start.
    pub on_draw: [Record; 2],
    pub average_turns: Option<f64>,
    pub win_rate_ci95: [Option<ConfidenceInterval>; 2],
    pub examples: OutcomeExamples,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StandingsEntry {
    pub competitor_id: String,
    pub name: String,
    pub record: Record,
    pub win_rate_ci95: Option<ConfidenceInterval>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ArenaReport {
    /// The resolved seed, including when the run was created without one.
    pub seed: u64,
    pub trials_per_matchup: usize,
    pub matchups: Vec<MatchupReport>,
    /// Sorted by score rate, then win rate, then competitor ID.
    pub standings: Vec<StandingsEntry>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct MatchupAccumulator {
    records: [Record; 2],
    on_play: [Record; 2],
    on_draw: [Record; 2],
    turns: u128,
    trials: usize,
    examples: OutcomeExamples,
}

impl MatchupAccumulator {
    pub(crate) fn record(&mut self, trial: &TrialRecord) {
        self.trials += 1;
        self.turns += trial.outcome.turns as u128;

        let starter = trial.context.starting_seat;
        let follower = 1 - starter;
        match trial.outcome.result {
            MatchResult::Winner { seat } => {
                let loser = 1 - seat;
                self.records[seat].record_win();
                self.records[loser].record_loss();

                if seat == starter {
                    self.on_play[seat].record_win();
                    self.on_draw[loser].record_loss();
                } else {
                    self.on_draw[seat].record_win();
                    self.on_play[loser].record_loss();
                }
                keep_first(&mut self.examples.winner[seat], trial.context.id);
            }
            MatchResult::Draw => {
                for record in &mut self.records {
                    record.record_draw();
                }
                self.on_play[starter].record_draw();
                self.on_draw[follower].record_draw();
                keep_first(&mut self.examples.draw, trial.context.id);
            }
        }
    }

    pub(crate) fn merge(&mut self, other: Self) {
        for seat in 0..2 {
            self.records[seat].merge(other.records[seat]);
            self.on_play[seat].merge(other.on_play[seat]);
            self.on_draw[seat].merge(other.on_draw[seat]);
            keep_option(&mut self.examples.winner[seat], other.examples.winner[seat]);
        }
        keep_option(&mut self.examples.draw, other.examples.draw);
        self.turns += other.turns;
        self.trials += other.trials;
    }

    pub(crate) fn into_report(
        self,
        matchup: &Matchup,
        competitors: &[Competitor<'_>],
    ) -> MatchupReport {
        let competitor_ids = matchup
            .competitor_indices
            .map(|index| competitors[index].id.clone());
        let win_rate_ci95 = self.records.map(|record| record.win_rate_ci95());

        MatchupReport {
            matchup_id: matchup.id,
            competitor_ids,
            records: self.records,
            on_play: self.on_play,
            on_draw: self.on_draw,
            average_turns: (self.trials > 0).then_some(self.turns as f64 / self.trials as f64),
            win_rate_ci95,
            examples: self.examples,
        }
    }
}

pub(crate) fn build_standings(
    competitors: &[Competitor<'_>],
    matchups: &[MatchupReport],
) -> Vec<StandingsEntry> {
    let mut records: BTreeMap<&str, Record> = competitors
        .iter()
        .map(|competitor| (competitor.id.as_str(), Record::default()))
        .collect();

    for matchup in matchups {
        for seat in 0..2 {
            records
                .entry(&matchup.competitor_ids[seat])
                .or_default()
                .merge(matchup.records[seat]);
        }
    }

    let mut standings: Vec<_> = competitors
        .iter()
        .map(|competitor| {
            let record = records[competitor.id.as_str()];
            StandingsEntry {
                competitor_id: competitor.id.clone(),
                name: competitor.name.clone(),
                win_rate_ci95: record.win_rate_ci95(),
                record,
            }
        })
        .collect();

    standings.sort_by(|left, right| {
        right
            .record
            .score_rate()
            .total_cmp(&left.record.score_rate())
            .then_with(|| right.record.win_rate().total_cmp(&left.record.win_rate()))
            .then_with(|| left.competitor_id.cmp(&right.competitor_id))
    });
    standings
}

fn keep_first(slot: &mut Option<TrialId>, trial_id: TrialId) {
    if slot.is_none_or(|current| trial_id < current) {
        *slot = Some(trial_id);
    }
}

fn keep_option(slot: &mut Option<TrialId>, candidate: Option<TrialId>) {
    if let Some(candidate) = candidate {
        keep_first(slot, candidate);
    }
}

fn rate(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}

fn wilson_interval(successes: usize, trials: usize) -> Option<ConfidenceInterval> {
    if trials == 0 {
        return None;
    }

    let n = trials as f64;
    let p = successes as f64 / n;
    let z = 1.959_963_984_540_054_f64;
    let z_squared = z * z;
    let denominator = 1.0 + z_squared / n;
    let center = (p + z_squared / (2.0 * n)) / denominator;
    let margin = z * ((p * (1.0 - p) / n + z_squared / (4.0 * n * n)).sqrt()) / denominator;

    Some(ConfidenceInterval {
        low: (center - margin).max(0.0),
        high: (center + margin).min(1.0),
    })
}
