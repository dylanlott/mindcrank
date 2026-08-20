use std::collections::BTreeMap;

use rayon::prelude::*;

use super::report::{MatchupAccumulator, build_standings};
use super::{
    ArenaError, ArenaReport, Competitor, MatchResult, MatchSimulator, Matchup, MatchupId, Schedule,
    TrialContext, TrialId, TrialRecord, derive_seed, validate_competitors,
};

/// Parallel Monte Carlo execution for an arena schedule.
///
/// Trials are paired: trial `2n` puts seat 0 on the play and trial `2n + 1`
/// puts seat 1 on the play, while both use the same underlying sample seed.
/// This provides balanced play/draw results and common random numbers.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ArenaMonteCarlo {
    pub trials_per_matchup: usize,
    pub seed: Option<u64>,
    /// Zero uses Rayon's global worker pool.
    pub workers: usize,
}

impl ArenaMonteCarlo {
    pub fn new(trials_per_matchup: usize) -> Self {
        Self {
            trials_per_matchup,
            seed: None,
            workers: 0,
        }
    }

    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = Some(seed);
        self
    }

    pub fn with_workers(mut self, workers: usize) -> Self {
        self.workers = workers;
        self
    }

    pub fn run(
        self,
        competitors: &[Competitor<'_>],
        schedule: &dyn Schedule,
        simulator: &dyn MatchSimulator,
    ) -> Result<ArenaReport, ArenaError> {
        validate_competitors(competitors)?;
        let matchups = schedule.matchups(competitors)?;
        validate_matchups(competitors, &matchups)?;

        let total_trials = matchups
            .len()
            .checked_mul(self.trials_per_matchup)
            .ok_or(ArenaError::TooManyTrials)?;
        let master_seed = self.seed.unwrap_or_else(rand::random);

        let simulate = || {
            (0..total_trials)
                .into_par_iter()
                .fold(
                    || Ok(BTreeMap::<MatchupId, MatchupAccumulator>::new()),
                    |accumulators, job_index| {
                        let mut accumulators = accumulators?;
                        let matchup_index = job_index / self.trials_per_matchup;
                        let trial_index = job_index % self.trials_per_matchup;
                        let matchup = &matchups[matchup_index];
                        let trial = execute_trial(
                            competitors,
                            simulator,
                            matchup,
                            master_seed,
                            trial_index,
                        )?;
                        accumulators.entry(matchup.id).or_default().record(&trial);
                        Ok(accumulators)
                    },
                )
                .reduce(
                    || Ok(BTreeMap::<MatchupId, MatchupAccumulator>::new()),
                    |left, right| {
                        let mut left = left?;
                        for (matchup_id, accumulator) in right? {
                            left.entry(matchup_id).or_default().merge(accumulator);
                        }
                        Ok(left)
                    },
                )
        };

        let mut accumulators = if self.workers == 0 {
            simulate()?
        } else {
            rayon::ThreadPoolBuilder::new()
                .num_threads(self.workers)
                .build()
                .map_err(|error| ArenaError::WorkerPool(error.to_string()))?
                .install(simulate)?
        };

        let reports: Vec<_> = matchups
            .iter()
            .map(|matchup| {
                accumulators
                    .remove(&matchup.id)
                    .unwrap_or_default()
                    .into_report(matchup, competitors)
            })
            .collect();
        let standings = build_standings(competitors, &reports);

        Ok(ArenaReport {
            seed: master_seed,
            trials_per_matchup: self.trials_per_matchup,
            matchups: reports,
            standings,
        })
    }

    /// Replays a trial ID using an explicit resolved master seed.
    pub fn replay(
        self,
        competitors: &[Competitor<'_>],
        schedule: &dyn Schedule,
        simulator: &dyn MatchSimulator,
        master_seed: u64,
        trial_id: TrialId,
    ) -> Result<TrialRecord, ArenaError> {
        validate_competitors(competitors)?;
        let matchups = schedule.matchups(competitors)?;
        validate_matchups(competitors, &matchups)?;
        let matchup = matchups
            .iter()
            .find(|matchup| matchup.id == trial_id.matchup_id)
            .ok_or(ArenaError::UnknownMatchup(trial_id.matchup_id))?;

        execute_trial(
            competitors,
            simulator,
            matchup,
            master_seed,
            trial_id.trial_index,
        )
    }
}

fn execute_trial(
    competitors: &[Competitor<'_>],
    simulator: &dyn MatchSimulator,
    matchup: &Matchup,
    master_seed: u64,
    trial_index: usize,
) -> Result<TrialRecord, ArenaError> {
    let id = TrialId {
        matchup_id: matchup.id,
        trial_index,
    };
    let matchup_seed = derive_seed(master_seed, matchup.id.0);
    let sample_seed = derive_seed(matchup_seed, (trial_index / 2) as u64);
    let context = TrialContext {
        id,
        seed: sample_seed,
        starting_seat: trial_index % 2,
    };
    let outcome = simulator.simulate(competitors, matchup, context);

    if let MatchResult::Winner { seat } = outcome.result
        && seat >= matchup.competitor_indices.len()
    {
        return Err(ArenaError::InvalidWinnerSeat { trial_id: id, seat });
    }

    Ok(TrialRecord {
        matchup: matchup.clone(),
        context,
        outcome,
    })
}

fn validate_matchups(
    competitors: &[Competitor<'_>],
    matchups: &[Matchup],
) -> Result<(), ArenaError> {
    let mut ids = std::collections::BTreeSet::new();
    for matchup in matchups {
        if !ids.insert(matchup.id) {
            return Err(ArenaError::DuplicateMatchupId(matchup.id));
        }
        for index in matchup.competitor_indices {
            if index >= competitors.len() {
                return Err(ArenaError::InvalidCompetitorIndex {
                    matchup_id: matchup.id,
                    index,
                });
            }
        }
    }
    Ok(())
}
