//! Competitive simulation harnesses built on top of the single-deck engine.
//!
//! The arena separates scheduling, match simulation, and sampling. The first
//! supplied model is [`GoldfishRaceModel`], which races independent deck plans
//! without pretending to model interaction between them.

mod model;
mod report;
mod runner;
mod schedule;

use std::fmt;

use crate::Params;

pub use model::{GoldfishRaceModel, MatchSimulator, TiePolicy};
pub use report::{
    ArenaReport, ConfidenceInterval, MatchupReport, OutcomeExamples, Record, StandingsEntry,
};
pub use runner::ArenaMonteCarlo;
pub use schedule::{RoundRobin, Schedule};

/// A stable, user-supplied identity plus the plan used to simulate a deck.
///
/// Keeping the identity separate from the display name makes reports and
/// replay IDs stable when a deck is renamed.
#[derive(Clone)]
pub struct Competitor<'a> {
    pub id: String,
    pub name: String,
    pub params: Params<'a>,
}

impl<'a> Competitor<'a> {
    pub fn new(id: impl Into<String>, params: Params<'a>) -> Self {
        let id = id.into();
        Self {
            name: id.clone(),
            id,
            params,
        }
    }

    pub fn named(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }
}

impl fmt::Debug for Competitor<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Competitor")
            .field("id", &self.id)
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}

/// Identifies one scheduled pairing within a schedule.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct MatchupId(pub u64);

impl fmt::Display for MatchupId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "m{}", self.0)
    }
}

/// A two-player pairing. Indices address the competitor slice given to a run.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Matchup {
    pub id: MatchupId,
    pub competitor_indices: [usize; 2],
}

/// A stable identifier for one trial in one schedule.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TrialId {
    pub matchup_id: MatchupId,
    pub trial_index: usize,
}

impl fmt::Display for TrialId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}:t{}", self.matchup_id, self.trial_index)
    }
}

/// Deterministic input supplied to a match simulator for one trial.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TrialContext {
    pub id: TrialId,
    pub seed: u64,
    /// The first player is seat 0 and the second is seat 1.
    pub starting_seat: usize,
}

impl TrialContext {
    /// Derives a deterministic named RNG stream without consuming another
    /// subsystem's random state.
    pub fn stream_seed(&self, namespace: &str, stream_id: &str) -> u64 {
        let namespace_seed = derive_seed(self.seed, stable_hash(namespace.as_bytes()));
        derive_seed(namespace_seed, stable_hash(stream_id.as_bytes()))
    }

    /// Derives a deterministic RNG stream for a competitor.
    ///
    /// The stream is keyed by competitor ID instead of seat, so mirrored
    /// play/draw trials use the same shuffle for each deck.
    pub fn competitor_seed(&self, competitor_id: &str) -> u64 {
        self.stream_seed("competitor", competitor_id)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MatchResult {
    Winner { seat: usize },
    Draw,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OutcomeReason {
    WinCondition,
    TurnOrderTieBreak,
    SimultaneousWin,
    Horizon,
    /// A reason supplied by a custom model.
    ModelDefined(String),
}

/// The model-independent result of a competitive trial.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MatchOutcome {
    pub result: MatchResult,
    pub turns: usize,
    pub reason: OutcomeReason,
}

impl MatchOutcome {
    pub fn winner(seat: usize, turns: usize, reason: OutcomeReason) -> Self {
        Self {
            result: MatchResult::Winner { seat },
            turns,
            reason,
        }
    }

    pub fn draw(turns: usize, reason: OutcomeReason) -> Self {
        Self {
            result: MatchResult::Draw,
            turns,
            reason,
        }
    }
}

/// Full information needed to reproduce and inspect one trial.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrialRecord {
    pub matchup: Matchup,
    pub context: TrialContext,
    pub outcome: MatchOutcome,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ArenaError {
    DuplicateCompetitorId(String),
    DuplicateMatchupId(MatchupId),
    InvalidCompetitorIndex { matchup_id: MatchupId, index: usize },
    InvalidWinnerSeat { trial_id: TrialId, seat: usize },
    UnknownMatchup(MatchupId),
    TooManyTrials,
    WorkerPool(String),
}

impl fmt::Display for ArenaError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateCompetitorId(id) => {
                write!(formatter, "competitor ID {id:?} appears more than once")
            }
            Self::DuplicateMatchupId(id) => {
                write!(formatter, "matchup ID {id} appears more than once")
            }
            Self::InvalidCompetitorIndex { matchup_id, index } => write!(
                formatter,
                "matchup {matchup_id} references missing competitor index {index}"
            ),
            Self::InvalidWinnerSeat { trial_id, seat } => {
                write!(
                    formatter,
                    "trial {trial_id} returned invalid winner seat {seat}"
                )
            }
            Self::UnknownMatchup(id) => write!(formatter, "unknown matchup {id}"),
            Self::TooManyTrials => write!(formatter, "scheduled trial count overflows usize"),
            Self::WorkerPool(message) => {
                write!(formatter, "failed to build worker pool: {message}")
            }
        }
    }
}

impl std::error::Error for ArenaError {}

pub(crate) fn validate_competitors(competitors: &[Competitor<'_>]) -> Result<(), ArenaError> {
    let mut ids: Vec<_> = competitors
        .iter()
        .map(|competitor| &competitor.id)
        .collect();
    ids.sort_unstable();
    if let Some(duplicate) = ids.windows(2).find(|pair| pair[0] == pair[1]) {
        return Err(ArenaError::DuplicateCompetitorId(duplicate[0].clone()));
    }
    Ok(())
}

pub(crate) fn derive_seed(base: u64, component: u64) -> u64 {
    splitmix64(base ^ splitmix64(component))
}

fn stable_hash(bytes: &[u8]) -> u64 {
    // FNV-1a is used only for stable stream separation, not for security.
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn splitmix64(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9e37_79b9_7f4a_7c15);
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}
