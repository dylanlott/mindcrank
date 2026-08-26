use std::collections::BTreeMap;

use super::{ArenaError, Competitor, Contest, ContestId, ContestResult, TrialId, TrialRecord};

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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OutcomeExamples {
    /// One replayable winning trial per canonical competitor, when observed.
    pub winner: Vec<Option<TrialId>>,
    pub draw: Option<TrialId>,
}

impl OutcomeExamples {
    fn new(competitor_count: usize) -> Self {
        Self {
            winner: vec![None; competitor_count],
            draw: None,
        }
    }
}

/// Aggregated results for one contest.
#[derive(Clone, Debug, PartialEq)]
pub struct ContestReport {
    pub contest_id: ContestId,
    /// Canonical competitor order, independent of seating.
    pub competitor_ids: Vec<String>,
    /// Overall records in the same order as `competitor_ids`.
    pub records: Vec<Record>,
    /// `records_by_seat[contest slot][game seat]`.
    pub records_by_seat: Vec<Vec<Record>>,
    pub average_turns: Option<f64>,
    pub win_rate_ci95: Vec<Option<ConfidenceInterval>>,
    pub examples: OutcomeExamples,
}

impl ContestReport {
    /// Validates all seat-aligned vectors before a report is consumed.
    pub fn validate(&self) -> Result<(), ArenaError> {
        let expected = self.competitor_ids.len();
        validate_len(self.contest_id, "records", expected, self.records.len())?;
        validate_len(
            self.contest_id,
            "records_by_seat",
            expected,
            self.records_by_seat.len(),
        )?;
        for (contest_slot, records) in self.records_by_seat.iter().enumerate() {
            validate_len(
                self.contest_id,
                &format!("records_by_seat[{contest_slot}]"),
                expected,
                records.len(),
            )?;
        }
        validate_len(
            self.contest_id,
            "win_rate_ci95",
            expected,
            self.win_rate_ci95.len(),
        )?;
        validate_len(
            self.contest_id,
            "examples.winner",
            expected,
            self.examples.winner.len(),
        )
    }
}

/// Compatibility name for two-player callers. Storage is now dynamic.
pub type MatchupReport = ContestReport;

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
    pub samples_per_contest: usize,
    /// Total simulated games across every contest and cyclic seating.
    pub games: usize,
    pub contests: Vec<ContestReport>,
    /// Sorted by score rate, then win rate, then competitor ID.
    pub standings: Vec<StandingsEntry>,
}

impl ArenaReport {
    /// Compatibility accessor for callers that still describe contests as matchups.
    pub fn matchups(&self) -> &[MatchupReport] {
        &self.contests
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ContestAccumulator {
    contest_id: ContestId,
    records: Vec<Record>,
    records_by_seat: Vec<Vec<Record>>,
    turns: u128,
    games: usize,
    examples: OutcomeExamples,
}

impl ContestAccumulator {
    pub(crate) fn new(contest_id: ContestId, seat_count: usize) -> Self {
        Self {
            contest_id,
            records: vec![Record::default(); seat_count],
            records_by_seat: vec![vec![Record::default(); seat_count]; seat_count],
            turns: 0,
            games: 0,
            examples: OutcomeExamples::new(seat_count),
        }
    }

    pub(crate) fn record(&mut self, trial: &TrialRecord) -> Result<(), ArenaError> {
        let seat_count = self.records.len();
        trial
            .context
            .seating
            .validate(self.contest_id, seat_count)?;
        self.games += 1;
        self.turns += trial.outcome.turns as u128;

        match trial.outcome.result {
            ContestResult::Winner { seat: winner_seat } => {
                let winner_slot = trial.context.seating.contest_slots[winner_seat];
                for (seat, &contest_slot) in trial.context.seating.contest_slots.iter().enumerate()
                {
                    if seat == winner_seat {
                        self.records[contest_slot].record_win();
                        self.records_by_seat[contest_slot][seat].record_win();
                    } else {
                        self.records[contest_slot].record_loss();
                        self.records_by_seat[contest_slot][seat].record_loss();
                    }
                }
                keep_first(&mut self.examples.winner[winner_slot], trial.context.id);
            }
            ContestResult::Draw => {
                for (seat, &contest_slot) in trial.context.seating.contest_slots.iter().enumerate()
                {
                    self.records[contest_slot].record_draw();
                    self.records_by_seat[contest_slot][seat].record_draw();
                }
                keep_first(&mut self.examples.draw, trial.context.id);
            }
        }
        Ok(())
    }

    pub(crate) fn merge(&mut self, other: Self) -> Result<(), ArenaError> {
        let expected = self.records.len();
        validate_len(
            self.contest_id,
            "accumulator.records",
            expected,
            other.records.len(),
        )?;
        validate_len(
            self.contest_id,
            "accumulator.records_by_seat",
            expected,
            other.records_by_seat.len(),
        )?;

        for contest_slot in 0..expected {
            validate_len(
                self.contest_id,
                &format!("accumulator.records_by_seat[{contest_slot}]"),
                expected,
                other.records_by_seat[contest_slot].len(),
            )?;
            self.records[contest_slot].merge(other.records[contest_slot]);
            for seat in 0..expected {
                self.records_by_seat[contest_slot][seat]
                    .merge(other.records_by_seat[contest_slot][seat]);
            }
            keep_option(
                &mut self.examples.winner[contest_slot],
                other.examples.winner[contest_slot],
            );
        }
        keep_option(&mut self.examples.draw, other.examples.draw);
        self.turns += other.turns;
        self.games += other.games;
        Ok(())
    }

    pub(crate) fn into_report(
        self,
        contest: &Contest,
        competitors: &[Competitor<'_>],
    ) -> Result<ContestReport, ArenaError> {
        let competitor_ids = contest
            .competitor_indices
            .iter()
            .map(|&index| competitors[index].id.clone())
            .collect();
        let win_rate_ci95 = self.records.iter().map(Record::win_rate_ci95).collect();

        let report = ContestReport {
            contest_id: contest.id,
            competitor_ids,
            records: self.records,
            records_by_seat: self.records_by_seat,
            average_turns: (self.games > 0).then_some(self.turns as f64 / self.games as f64),
            win_rate_ci95,
            examples: self.examples,
        };
        report.validate()?;
        Ok(report)
    }
}

pub(crate) fn build_standings(
    competitors: &[Competitor<'_>],
    contests: &[ContestReport],
) -> Vec<StandingsEntry> {
    let mut records: BTreeMap<&str, Record> = competitors
        .iter()
        .map(|competitor| (competitor.id.as_str(), Record::default()))
        .collect();

    for contest in contests {
        for (competitor_id, record) in contest.competitor_ids.iter().zip(&contest.records) {
            records.entry(competitor_id).or_default().merge(*record);
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

fn validate_len(
    contest_id: ContestId,
    field: &str,
    expected: usize,
    actual: usize,
) -> Result<(), ArenaError> {
    if actual == expected {
        Ok(())
    } else {
        Err(ArenaError::InvalidReportShape {
            contest_id,
            field: field.to_owned(),
            expected,
            actual,
        })
    }
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
