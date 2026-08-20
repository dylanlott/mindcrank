use crate::run_once;

use super::{Competitor, MatchOutcome, Matchup, OutcomeReason, TrialContext};

/// Resolves one scheduled match. Scheduling and repetition are handled by the
/// arena runner rather than the model.
pub trait MatchSimulator: Send + Sync {
    fn simulate(
        &self,
        competitors: &[Competitor<'_>],
        matchup: &Matchup,
        context: TrialContext,
    ) -> MatchOutcome;
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

/// Races two independent single-deck simulations and awards the match to the
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

impl MatchSimulator for GoldfishRaceModel {
    fn simulate(
        &self,
        competitors: &[Competitor<'_>],
        matchup: &Matchup,
        context: TrialContext,
    ) -> MatchOutcome {
        let plans = matchup.competitor_indices.map(|index| &competitors[index]);
        let outcomes = plans.map(|competitor| {
            let mut params = competitor.params;
            params.seed = Some(context.competitor_seed(&competitor.id));
            run_once(&params)
        });

        match (outcomes[0].turns_to_win, outcomes[1].turns_to_win) {
            (Some(left), Some(right)) if left < right => {
                MatchOutcome::winner(0, left, OutcomeReason::WinCondition)
            }
            (Some(left), Some(right)) if right < left => {
                MatchOutcome::winner(1, right, OutcomeReason::WinCondition)
            }
            (Some(turn), Some(_)) => match self.tie_policy {
                TiePolicy::Draw => MatchOutcome::draw(turn, OutcomeReason::SimultaneousWin),
                TiePolicy::StartingPlayer => MatchOutcome::winner(
                    context.starting_seat,
                    turn,
                    OutcomeReason::TurnOrderTieBreak,
                ),
            },
            (Some(turn), None) => MatchOutcome::winner(0, turn, OutcomeReason::WinCondition),
            (None, Some(turn)) => MatchOutcome::winner(1, turn, OutcomeReason::WinCondition),
            (None, None) => {
                let horizon = plans
                    .iter()
                    .map(|competitor| competitor.params.max_turns)
                    .max()
                    .unwrap_or_default();
                MatchOutcome::draw(horizon, OutcomeReason::Horizon)
            }
        }
    }
}
