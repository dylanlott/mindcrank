//! Small, composable building blocks for Magic: The Gathering deck simulations.
//!
//! The crate deliberately models cards as tagged values instead of trying to
//! implement the full rules of Magic. Win conditions, mulligan policies, and
//! bottoming heuristics are traits, so a simulation can become as detailed as
//! its question requires.

mod card;
mod deck;
mod engine;
mod frontier;
mod metrics;
mod mulligan;
mod win_condition;

pub mod arena;

pub use card::Card;
pub use deck::{Deck, count_tag};
pub use engine::{MonteCarloParams, Params, monte_carlo, run_once};
pub use frontier::{
    DeckCandidate, ParetoAxis, ParetoCandidateResult, ParetoError, ParetoPoint, ParetoProtocol,
    ParetoReport, ParetoRunMetadata, ParetoScatterplot, ParetoTooltip, compare_pareto,
};
pub use metrics::{Aggregate, TrialOutcome};
pub use mulligan::{
    BottomHeuristic, DefaultBottomHeuristic, KeepIf, KeepIfLandsBetween, KeepIfWinOrDecent,
    MulliganPolicy,
};
pub use win_condition::{AnyOf, KOfTag, TwoCardSet, WinCondition};
