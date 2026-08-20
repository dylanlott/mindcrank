use super::{ArenaError, Competitor, Matchup, MatchupId, validate_competitors};

/// Produces the competitive cases that a simulation method will execute.
pub trait Schedule: Send + Sync {
    fn matchups(&self, competitors: &[Competitor<'_>]) -> Result<Vec<Matchup>, ArenaError>;
}

/// Schedules every unordered pair of competitors once.
///
/// The pair order is based on competitor IDs rather than input order, making
/// matchup IDs stable when the caller reorders the same registry.
#[derive(Clone, Copy, Debug, Default)]
pub struct RoundRobin;

impl Schedule for RoundRobin {
    fn matchups(&self, competitors: &[Competitor<'_>]) -> Result<Vec<Matchup>, ArenaError> {
        validate_competitors(competitors)?;

        let mut indices: Vec<_> = (0..competitors.len()).collect();
        indices.sort_unstable_by(|left, right| competitors[*left].id.cmp(&competitors[*right].id));

        let mut matchups = Vec::new();
        for left in 0..indices.len() {
            for right in (left + 1)..indices.len() {
                matchups.push(Matchup {
                    id: MatchupId(matchups.len() as u64),
                    competitor_indices: [indices[left], indices[right]],
                });
            }
        }

        Ok(matchups)
    }
}
