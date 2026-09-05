use std::collections::HashSet;

use crate::{
    Aggregate, BottomHeuristic, Deck, MonteCarloParams, MulliganPolicy, Params, WinCondition,
    monte_carlo,
};

/// A named decklist evaluated in a Pareto comparison.
#[derive(Clone, Debug)]
pub struct DeckCandidate<'a> {
    /// Stable caller-defined identity. It must be unique within one comparison.
    pub id: String,
    /// Human-readable label for reports and plot consumers.
    pub name: String,
    pub deck: &'a Deck,
}

impl<'a> DeckCandidate<'a> {
    pub fn new(id: impl Into<String>, name: impl Into<String>, deck: &'a Deck) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            deck,
        }
    }
}

/// The fixed single-deck simulation configuration for every frontier candidate.
///
/// Candidates vary only by decklist. Keeping all other assumptions here makes
/// the resulting frontier meaningful within this one evaluation protocol.
#[derive(Clone, Copy)]
pub struct ParetoProtocol<'a> {
    pub win: &'a dyn WinCondition,
    pub hand_size: usize,
    pub draws_per_turn: usize,
    pub use_london_mulligan: bool,
    pub max_mulligans: usize,
    pub mulligan: Option<&'a dyn MulliganPolicy>,
    pub bottom_heuristic: Option<&'a dyn BottomHeuristic>,
    /// The early-win threshold used for the x axis.
    pub fast_turn: usize,
    /// The simulation horizon and long-horizon threshold used for the y axis.
    pub horizon_turn: usize,
    pub trials: usize,
    /// Required shared seed. Every candidate receives this same master seed.
    pub seed: u64,
    /// Zero uses Rayon's global worker pool.
    pub workers: usize,
}

impl<'a> ParetoProtocol<'a> {
    /// Builds a protocol from existing single-deck parameters.
    ///
    /// The provided `Params::max_turns` is intentionally replaced by
    /// `horizon_turn`: a Pareto run has one explicit horizon for every deck.
    pub fn from_params(
        params: Params<'a>,
        fast_turn: usize,
        horizon_turn: usize,
        trials: usize,
        seed: u64,
    ) -> Result<Self, ParetoError> {
        let protocol = Self {
            win: params.win,
            hand_size: params.hand_size,
            draws_per_turn: params.draws_per_turn,
            use_london_mulligan: params.use_london_mulligan,
            max_mulligans: params.max_mulligans,
            mulligan: params.mulligan,
            bottom_heuristic: params.bottom_heuristic,
            fast_turn,
            horizon_turn,
            trials,
            seed,
            workers: 0,
        };
        protocol.validate()?;
        Ok(protocol)
    }

    pub fn with_workers(mut self, workers: usize) -> Self {
        self.workers = workers;
        self
    }

    fn validate(&self) -> Result<(), ParetoError> {
        if self.trials == 0 {
            return Err(ParetoError::ZeroTrials);
        }
        if self.fast_turn > self.horizon_turn {
            return Err(ParetoError::FastTurnExceedsHorizon {
                fast_turn: self.fast_turn,
                horizon_turn: self.horizon_turn,
            });
        }
        Ok(())
    }

    fn params_for(&self, deck: &'a Deck) -> Params<'a> {
        Params {
            deck,
            win: self.win,
            hand_size: self.hand_size,
            max_turns: self.horizon_turn,
            draws_per_turn: self.draws_per_turn,
            use_london_mulligan: self.use_london_mulligan,
            max_mulligans: self.max_mulligans,
            mulligan: self.mulligan,
            bottom_heuristic: self.bottom_heuristic,
            seed: Some(self.seed),
        }
    }

    fn metadata(&self) -> ParetoRunMetadata {
        ParetoRunMetadata {
            fast_turn: self.fast_turn,
            horizon_turn: self.horizon_turn,
            trials: self.trials,
            seed: self.seed,
            workers: self.workers,
            hand_size: self.hand_size,
            draws_per_turn: self.draws_per_turn,
            use_london_mulligan: self.use_london_mulligan,
            max_mulligans: self.max_mulligans,
            has_mulligan_policy: self.mulligan.is_some(),
            has_bottom_heuristic: self.bottom_heuristic.is_some(),
        }
    }
}

/// Copyable details of the protocol that make a report reproducible and
/// displayable without attempting to serialize caller-supplied trait objects.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ParetoRunMetadata {
    pub fast_turn: usize,
    pub horizon_turn: usize,
    pub trials: usize,
    pub seed: u64,
    pub workers: usize,
    pub hand_size: usize,
    pub draws_per_turn: usize,
    pub use_london_mulligan: bool,
    pub max_mulligans: usize,
    pub has_mulligan_policy: bool,
    pub has_bottom_heuristic: bool,
}

/// One candidate's metrics and frontier membership.
#[derive(Clone, Debug, PartialEq)]
pub struct ParetoCandidateResult {
    pub id: String,
    pub name: String,
    pub aggregate: Aggregate,
    pub early_wins: usize,
    pub early_win_rate: f64,
    pub horizon_wins: usize,
    pub horizon_win_rate: f64,
    pub is_frontier: bool,
}

/// Complete result of comparing deck candidates under one fixed protocol.
#[derive(Clone, Debug, PartialEq)]
pub struct ParetoReport {
    pub protocol: ParetoRunMetadata,
    /// One result per input candidate, retained in input order.
    pub candidates: Vec<ParetoCandidateResult>,
    /// Indices into `candidates` for all nondominated points, in input order.
    pub frontier_indices: Vec<usize>,
}

impl ParetoReport {
    /// Produces semantic scatterplot data for a speed-versus-consistency view.
    /// No rendering policy is imposed by the library.
    pub fn scatterplot(&self) -> ParetoScatterplot {
        ParetoScatterplot {
            x_axis: ParetoAxis {
                label: format!("P(win by turn {})", self.protocol.fast_turn),
                turn: self.protocol.fast_turn,
            },
            y_axis: ParetoAxis {
                label: format!("P(win by turn {})", self.protocol.horizon_turn),
                turn: self.protocol.horizon_turn,
            },
            points: self
                .candidates
                .iter()
                .map(|candidate| ParetoPoint {
                    candidate_id: candidate.id.clone(),
                    label: candidate.name.clone(),
                    x: candidate.early_win_rate,
                    y: candidate.horizon_win_rate,
                    is_frontier: candidate.is_frontier,
                    tooltip: ParetoTooltip {
                        early_win_rate: candidate.early_win_rate,
                        horizon_win_rate: candidate.horizon_win_rate,
                        opening_win_rate: candidate.aggregate.opening_win_rate,
                        avg_kept_hand_size: candidate.aggregate.avg_kept_hand_size,
                        trials: candidate.aggregate.trials,
                    },
                })
                .collect(),
        }
    }

    /// Returns a CSV representation for plotting or importing into spreadsheet tools.
    ///
    /// When `frontier_only` is `true`, only frontier candidates are emitted.
    /// Every row contains one candidate with candidate identity, protocol thresholds,
    /// x/y values, frontier membership, and key run statistics.
    pub fn to_csv(&self, frontier_only: bool) -> String {
        let mut csv = String::from(
            "candidate_id,label,fast_turn,horizon_turn,early_win_rate,horizon_win_rate,is_frontier,early_wins,horizon_wins,trials,opening_win_rate,avg_kept_hand_size\n",
        );

        for candidate in self
            .candidates
            .iter()
            .filter(|candidate| !frontier_only || candidate.is_frontier)
        {
            let candidate_id = quote_csv(&candidate.id);
            let label = quote_csv(&candidate.name);
            use std::fmt::Write as _;
            writeln!(
                &mut csv,
                "{candidate_id},{label},{},{},{:.12},{:.12},{},{},{},{},{:.12},{:.12}",
                self.protocol.fast_turn,
                self.protocol.horizon_turn,
                candidate.early_win_rate,
                candidate.horizon_win_rate,
                candidate.is_frontier,
                candidate.early_wins,
                candidate.horizon_wins,
                candidate.aggregate.trials,
                candidate.aggregate.opening_win_rate,
                candidate.aggregate.avg_kept_hand_size,
            )
            .expect("writing to String cannot fail");
        }

        csv
    }
}

/// Metadata for one scatterplot axis.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParetoAxis {
    pub label: String,
    pub turn: usize,
}

/// Semantic scatterplot data for all candidates, including dominated points.
#[derive(Clone, Debug, PartialEq)]
pub struct ParetoScatterplot {
    pub x_axis: ParetoAxis,
    pub y_axis: ParetoAxis,
    pub points: Vec<ParetoPoint>,
}

/// A scatterplot point with all fields necessary for basic labeling and tooltips.
#[derive(Clone, Debug, PartialEq)]
pub struct ParetoPoint {
    pub candidate_id: String,
    pub label: String,
    pub x: f64,
    pub y: f64,
    pub is_frontier: bool,
    pub tooltip: ParetoTooltip,
}

/// Values intended for an individual scatterplot point's tooltip.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParetoTooltip {
    pub early_win_rate: f64,
    pub horizon_win_rate: f64,
    pub opening_win_rate: f64,
    pub avg_kept_hand_size: f64,
    pub trials: usize,
}

/// Validation failures that prevent a Pareto comparison from starting.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ParetoError {
    NoCandidates,
    DuplicateId {
        id: String,
    },
    ZeroTrials,
    FastTurnExceedsHorizon {
        fast_turn: usize,
        horizon_turn: usize,
    },
}

impl std::fmt::Display for ParetoError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoCandidates => formatter.write_str("Pareto comparison requires a candidate"),
            Self::DuplicateId { id } => write!(formatter, "duplicate Pareto candidate ID: {id}"),
            Self::ZeroTrials => {
                formatter.write_str("Pareto comparison requires at least one trial")
            }
            Self::FastTurnExceedsHorizon {
                fast_turn,
                horizon_turn,
            } => write!(
                formatter,
                "fast turn {fast_turn} exceeds horizon turn {horizon_turn}"
            ),
        }
    }
}

impl std::error::Error for ParetoError {}

/// Evaluates deck variants under one protocol and returns every nondominated
/// speed-versus-consistency point.
pub fn compare_pareto<'a>(
    candidates: &[DeckCandidate<'a>],
    protocol: ParetoProtocol<'a>,
) -> Result<ParetoReport, ParetoError> {
    protocol.validate()?;
    validate_candidates(candidates)?;

    let mut results = candidates
        .iter()
        .map(|candidate| {
            let aggregate = monte_carlo(
                MonteCarloParams::new(protocol.params_for(candidate.deck), protocol.trials)
                    .with_seed(protocol.seed)
                    .with_workers(protocol.workers),
            );
            let early_wins = aggregate.wins_by_turn(protocol.fast_turn);
            let horizon_wins = aggregate.wins_by_turn(protocol.horizon_turn);

            ParetoCandidateResult {
                id: candidate.id.clone(),
                name: candidate.name.clone(),
                early_win_rate: early_wins as f64 / aggregate.trials as f64,
                horizon_win_rate: horizon_wins as f64 / aggregate.trials as f64,
                aggregate,
                early_wins,
                horizon_wins,
                is_frontier: false,
            }
        })
        .collect::<Vec<_>>();

    for index in 0..results.len() {
        results[index].is_frontier = !results
            .iter()
            .enumerate()
            .any(|(other_index, other)| other_index != index && dominates(other, &results[index]));
    }

    let frontier_indices = results
        .iter()
        .enumerate()
        .filter_map(|(index, candidate)| candidate.is_frontier.then_some(index))
        .collect();

    Ok(ParetoReport {
        protocol: protocol.metadata(),
        candidates: results,
        frontier_indices,
    })
}

fn validate_candidates(candidates: &[DeckCandidate<'_>]) -> Result<(), ParetoError> {
    if candidates.is_empty() {
        return Err(ParetoError::NoCandidates);
    }

    let mut ids = HashSet::with_capacity(candidates.len());
    for candidate in candidates {
        if !ids.insert(candidate.id.as_str()) {
            return Err(ParetoError::DuplicateId {
                id: candidate.id.clone(),
            });
        }
    }
    Ok(())
}

fn dominates(left: &ParetoCandidateResult, right: &ParetoCandidateResult) -> bool {
    left.early_wins >= right.early_wins
        && left.horizon_wins >= right.horizon_wins
        && (left.early_wins > right.early_wins || left.horizon_wins > right.horizon_wins)
}

fn quote_csv(value: &str) -> String {
    format!("\"{}\"", value.replace('\"', "\"\""))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn result(early_wins: usize, horizon_wins: usize) -> ParetoCandidateResult {
        ParetoCandidateResult {
            id: format!("{early_wins}-{horizon_wins}"),
            name: "test".into(),
            aggregate: Aggregate::default(),
            early_wins,
            early_win_rate: 0.0,
            horizon_wins,
            horizon_win_rate: 0.0,
            is_frontier: false,
        }
    }

    #[test]
    fn strict_dominance_requires_one_better_axis() {
        let dominating = result(4, 8);
        assert!(dominates(&dominating, &result(4, 7)));
        assert!(dominates(&dominating, &result(3, 8)));
        assert!(!dominates(&dominating, &result(5, 7)));
        assert!(!dominates(&dominating, &result(4, 8)));
    }
}
