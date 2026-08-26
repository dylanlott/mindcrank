use super::{ArenaError, Competitor, Contest, ContestId, Matchup, validate_competitors};

/// Produces the competitive cases that a simulation method will execute.
pub trait Schedule: Send + Sync {
    fn contests(&self, competitors: &[Competitor<'_>]) -> Result<Vec<Contest>, ArenaError>;

    /// Compatibility name for two-player schedules.
    fn matchups(&self, competitors: &[Competitor<'_>]) -> Result<Vec<Matchup>, ArenaError> {
        self.contests(competitors)
    }
}

/// Schedules every unordered pair of competitors once.
///
/// The pair order is based on competitor IDs rather than input order, making
/// matchup IDs stable when the caller reorders the same registry.
#[derive(Clone, Copy, Debug, Default)]
pub struct RoundRobin;

impl Schedule for RoundRobin {
    fn contests(&self, competitors: &[Competitor<'_>]) -> Result<Vec<Contest>, ArenaError> {
        validate_competitors(competitors)?;

        let mut indices: Vec<_> = (0..competitors.len()).collect();
        indices.sort_unstable_by(|left, right| competitors[*left].id.cmp(&competitors[*right].id));

        let mut contests = Vec::new();
        for left in 0..indices.len() {
            for right in (left + 1)..indices.len() {
                contests.push(Contest::new(
                    ContestId(contests.len() as u64),
                    vec![indices[left], indices[right]],
                ));
            }
        }

        Ok(contests)
    }
}
