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

pub use model::ContestSimulator as MatchSimulator;
pub use model::{ContestSimulator, GoldfishRaceModel, TiePolicy};
pub use report::{
    ArenaReport, ConfidenceInterval, ContestReport, MatchupReport, OutcomeExamples, Record,
    StandingsEntry,
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

/// Identifies one scheduled contest within a schedule.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ContestId(pub u64);

impl fmt::Display for ContestId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "c{}", self.0)
    }
}

/// A scheduled group of competitors in canonical, seating-independent order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Contest {
    pub id: ContestId,
    /// Indices into the competitor slice supplied to an arena run.
    pub competitor_indices: Vec<usize>,
}

impl Contest {
    pub fn new(id: ContestId, competitor_indices: impl Into<Vec<usize>>) -> Self {
        Self {
            id,
            competitor_indices: competitor_indices.into(),
        }
    }

    pub fn len(&self) -> usize {
        self.competitor_indices.len()
    }

    pub fn is_empty(&self) -> bool {
        self.competitor_indices.is_empty()
    }
}

/// Maps each game seat to a slot in [`Contest::competitor_indices`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Seating {
    pub contest_slots: Vec<usize>,
}

impl Seating {
    pub fn new(contest_slots: impl Into<Vec<usize>>) -> Self {
        Self {
            contest_slots: contest_slots.into(),
        }
    }

    /// Creates one cyclic rotation. Seat zero acts first.
    pub fn cyclic(seat_count: usize, rotation: usize) -> Self {
        if seat_count == 0 {
            return Self::new(Vec::new());
        }
        Self::new(
            (0..seat_count)
                .map(|seat| (seat + rotation) % seat_count)
                .collect::<Vec<_>>(),
        )
    }

    pub fn len(&self) -> usize {
        self.contest_slots.len()
    }

    pub fn is_empty(&self) -> bool {
        self.contest_slots.is_empty()
    }

    /// Returns the game seat occupied by a canonical contest slot.
    pub fn seat_for_contest_slot(&self, contest_slot: usize) -> Option<usize> {
        self.contest_slots
            .iter()
            .position(|slot| *slot == contest_slot)
    }

    pub fn validate(&self, contest_id: ContestId, seat_count: usize) -> Result<(), ArenaError> {
        if self.len() != seat_count {
            return Err(ArenaError::InvalidSeating {
                contest_id,
                reason: format!(
                    "expected {seat_count} seats, found {}",
                    self.contest_slots.len()
                ),
            });
        }

        let mut seen = vec![false; seat_count];
        for &slot in &self.contest_slots {
            if slot >= seat_count {
                return Err(ArenaError::InvalidSeating {
                    contest_id,
                    reason: format!("contest slot {slot} is outside 0..{seat_count}"),
                });
            }
            if std::mem::replace(&mut seen[slot], true) {
                return Err(ArenaError::InvalidSeating {
                    contest_id,
                    reason: format!("contest slot {slot} appears more than once"),
                });
            }
        }
        Ok(())
    }
}

/// A stable identifier for one trial in one schedule.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TrialId {
    pub contest_id: ContestId,
    pub sample_index: usize,
    pub seating_index: usize,
}

impl fmt::Display for TrialId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}:s{}:p{}",
            self.contest_id, self.sample_index, self.seating_index
        )
    }
}

/// Deterministic input supplied to a match simulator for one trial.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrialContext {
    pub id: TrialId,
    pub sample_seed: u64,
    /// Seat order for this game. Seat zero acts first.
    pub seating: Seating,
}

impl TrialContext {
    /// Derives a deterministic named RNG stream without consuming another
    /// subsystem's random state.
    pub fn stream_seed(&self, namespace: &str, stream_id: &str) -> u64 {
        let namespace_seed = derive_seed(self.sample_seed, stable_hash(namespace.as_bytes()));
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
pub enum ContestResult {
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
pub struct ContestOutcome {
    pub result: ContestResult,
    pub turns: usize,
    pub reason: OutcomeReason,
}

impl ContestOutcome {
    pub fn winner(seat: usize, turns: usize, reason: OutcomeReason) -> Self {
        Self {
            result: ContestResult::Winner { seat },
            turns,
            reason,
        }
    }

    pub fn draw(turns: usize, reason: OutcomeReason) -> Self {
        Self {
            result: ContestResult::Draw,
            turns,
            reason,
        }
    }
}

/// Full information needed to reproduce and inspect one trial.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrialRecord {
    pub contest: Contest,
    pub context: TrialContext,
    pub outcome: ContestOutcome,
}

/// A typed failure returned by a contest model.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SimulationError {
    pub message: String,
}

impl SimulationError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for SimulationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for SimulationError {}

impl From<String> for SimulationError {
    fn from(message: String) -> Self {
        Self::new(message)
    }
}

impl From<&str> for SimulationError {
    fn from(message: &str) -> Self {
        Self::new(message)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ArenaError {
    DuplicateCompetitorId(String),
    DuplicateContestId(ContestId),
    EmptyContest(ContestId),
    DuplicateCompetitorInContest {
        contest_id: ContestId,
        index: usize,
    },
    InvalidCompetitorIndex {
        contest_id: ContestId,
        index: usize,
    },
    InvalidSeating {
        contest_id: ContestId,
        reason: String,
    },
    InvalidWinnerSeat {
        trial_id: TrialId,
        seat: usize,
    },
    InvalidReportShape {
        contest_id: ContestId,
        field: String,
        expected: usize,
        actual: usize,
    },
    UnknownContest(ContestId),
    UnknownTrial(TrialId),
    SimulationFailed {
        trial_id: TrialId,
        source: SimulationError,
    },
    TooManyGames,
    WorkerPool(String),
}

impl fmt::Display for ArenaError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateCompetitorId(id) => {
                write!(formatter, "competitor ID {id:?} appears more than once")
            }
            Self::DuplicateContestId(id) => {
                write!(formatter, "contest ID {id} appears more than once")
            }
            Self::EmptyContest(id) => write!(formatter, "contest {id} has no competitors"),
            Self::DuplicateCompetitorInContest { contest_id, index } => write!(
                formatter,
                "contest {contest_id} references competitor index {index} more than once"
            ),
            Self::InvalidCompetitorIndex { contest_id, index } => write!(
                formatter,
                "contest {contest_id} references missing competitor index {index}"
            ),
            Self::InvalidSeating { contest_id, reason } => {
                write!(
                    formatter,
                    "contest {contest_id} has invalid seating: {reason}"
                )
            }
            Self::InvalidWinnerSeat { trial_id, seat } => {
                write!(
                    formatter,
                    "trial {trial_id} returned invalid winner seat {seat}"
                )
            }
            Self::InvalidReportShape {
                contest_id,
                field,
                expected,
                actual,
            } => write!(
                formatter,
                "contest {contest_id} report field {field} expected length {expected}, found {actual}"
            ),
            Self::UnknownContest(id) => write!(formatter, "unknown contest {id}"),
            Self::UnknownTrial(id) => write!(formatter, "unknown trial {id}"),
            Self::SimulationFailed { trial_id, source } => {
                write!(formatter, "trial {trial_id} failed: {source}")
            }
            Self::TooManyGames => write!(formatter, "scheduled game count overflows usize"),
            Self::WorkerPool(message) => {
                write!(formatter, "failed to build worker pool: {message}")
            }
        }
    }
}

impl std::error::Error for ArenaError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::SimulationFailed { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// Compatibility name for two-player callers. Storage is now dynamic.
pub type MatchupId = ContestId;
/// Compatibility name for two-player callers. Storage is now dynamic.
pub type Matchup = Contest;
/// Compatibility name for two-player callers.
pub type MatchResult = ContestResult;
/// Compatibility name for two-player callers.
pub type MatchOutcome = ContestOutcome;

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
