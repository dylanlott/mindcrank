use crate::run_once;

use super::{Competitor, Contest, ContestOutcome, OutcomeReason, SimulationError, TrialContext};

/// Resolves one scheduled contest. Scheduling and repetition are handled by the
/// arena runner rather than the model.
pub trait ContestSimulator: Send + Sync {
    fn simulate(
        &self,
        competitors: &[Competitor<'_>],
        contest: &Contest,
        context: &TrialContext,
    ) -> Result<ContestOutcome, SimulationError>;
}

/// Defines how equal-turn wins are resolved by [`GoldfishRaceModel`].
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum TiePolicy {
    /// Equal-turn wins are reported as a draw. This is the conservative default
    /// because the single-deck engine does not model within-turn timing.
    #[default]
    Draw,
    /// The player whose turn comes first wins an equal-turn race.
    StartingPlayer,
}

/// Races independent single-deck simulations and awards the contest to the
/// deck that reaches its win condition first.
///
/// This model intentionally has no interaction, shared zones, mana, stack, or
/// combat. It is useful as a competitive baseline, not as a full game model.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct GoldfishRaceModel {
    pub tie_policy: TiePolicy,
}

impl GoldfishRaceModel {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_tie_policy(mut self, tie_policy: TiePolicy) -> Self {
        self.tie_policy = tie_policy;
        self
    }
}

impl ContestSimulator for GoldfishRaceModel {
    fn simulate(
        &self,
        competitors: &[Competitor<'_>],
        contest: &Contest,
        context: &TrialContext,
    ) -> Result<ContestOutcome, SimulationError> {
        let mut earliest_turn = None;
        let mut earliest_seats = Vec::new();
        let mut horizon = 0;

        for (seat, &contest_slot) in context.seating.contest_slots.iter().enumerate() {
            let competitor = &competitors[contest.competitor_indices[contest_slot]];
            let mut params = competitor.params;
            params.seed = Some(context.competitor_seed(&competitor.id));
            horizon = horizon.max(params.max_turns);

            if let Some(turn) = run_once(&params).turns_to_win {
                match earliest_turn {
                    None => {
                        earliest_turn = Some(turn);
                        earliest_seats.push(seat);
                    }
                    Some(earliest) if turn < earliest => {
                        earliest_turn = Some(turn);
                        earliest_seats.clear();
                        earliest_seats.push(seat);
                    }
                    Some(earliest) if turn == earliest => earliest_seats.push(seat),
                    Some(_) => {}
                }
            }
        }

        let outcome = match (earliest_turn, earliest_seats.as_slice()) {
            (None, _) => ContestOutcome::draw(horizon, OutcomeReason::Horizon),
            (Some(turn), [seat]) => {
                ContestOutcome::winner(*seat, turn, OutcomeReason::WinCondition)
            }
            (Some(turn), seats) => match self.tie_policy {
                TiePolicy::Draw => ContestOutcome::draw(turn, OutcomeReason::SimultaneousWin),
                TiePolicy::StartingPlayer => {
                    ContestOutcome::winner(seats[0], turn, OutcomeReason::TurnOrderTieBreak)
                }
            },
        };
        Ok(outcome)
    }
}
