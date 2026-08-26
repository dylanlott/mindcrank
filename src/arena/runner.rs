use std::collections::{BTreeMap, BTreeSet, btree_map::Entry};

use rayon::prelude::*;

use super::report::{ContestAccumulator, build_standings};
use super::{
    ArenaError, ArenaReport, Competitor, Contest, ContestId, ContestResult, ContestSimulator,
    Schedule, Seating, TrialContext, TrialId, TrialRecord, derive_seed, validate_competitors,
};

/// Parallel Monte Carlo execution for an arena schedule.
///
/// Each sample is replayed through every cyclic seating for its contest. The
/// sample seed is shared across those seatings, so each competitor receives the
/// same random stream while its position changes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ArenaMonteCarlo {
    pub samples_per_contest: usize,
    pub seed: Option<u64>,
    /// Zero uses Rayon's global worker pool.
    pub workers: usize,
}

impl ArenaMonteCarlo {
    pub fn new(samples_per_contest: usize) -> Self {
        Self {
            samples_per_contest,
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
        simulator: &dyn ContestSimulator,
    ) -> Result<ArenaReport, ArenaError> {
        validate_competitors(competitors)?;
        let contests = schedule.contests(competitors)?;
        validate_contests(competitors, &contests)?;

        let (work, total_games) = build_work(&contests, self.samples_per_contest)?;
        let master_seed = self.seed.unwrap_or_else(rand::random);

        let simulate = || {
            (0..total_games)
                .into_par_iter()
                .fold(
                    || Ok(BTreeMap::<ContestId, ContestAccumulator>::new()),
                    |accumulators, job_index| -> Result<_, ArenaError> {
                        let mut accumulators = accumulators?;
                        let work_index = work.partition_point(|item| item.end <= job_index);
                        let item = &work[work_index];
                        let local_index = job_index - item.start;
                        let sample_index = local_index / item.seating_count;
                        let seating_index = local_index % item.seating_count;
                        let contest = &contests[item.contest_index];
                        let trial = execute_trial(
                            competitors,
                            simulator,
                            contest,
                            master_seed,
                            TrialId {
                                contest_id: contest.id,
                                sample_index,
                                seating_index,
                            },
                        )?;
                        accumulators
                            .entry(contest.id)
                            .or_insert_with(|| ContestAccumulator::new(contest.id, contest.len()))
                            .record(&trial)?;
                        Ok(accumulators)
                    },
                )
                .reduce(
                    || Ok(BTreeMap::<ContestId, ContestAccumulator>::new()),
                    |left, right| -> Result<_, ArenaError> {
                        let mut left = left?;
                        for (contest_id, accumulator) in right? {
                            match left.entry(contest_id) {
                                Entry::Occupied(mut entry) => {
                                    entry.get_mut().merge(accumulator)?;
                                }
                                Entry::Vacant(entry) => {
                                    entry.insert(accumulator);
                                }
                            }
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

        let reports = contests
            .iter()
            .map(|contest| {
                accumulators
                    .remove(&contest.id)
                    .unwrap_or_else(|| ContestAccumulator::new(contest.id, contest.len()))
                    .into_report(contest, competitors)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let standings = build_standings(competitors, &reports);

        Ok(ArenaReport {
            seed: master_seed,
            samples_per_contest: self.samples_per_contest,
            games: total_games,
            contests: reports,
            standings,
        })
    }

    /// Replays a trial ID using an explicit resolved master seed.
    pub fn replay(
        self,
        competitors: &[Competitor<'_>],
        schedule: &dyn Schedule,
        simulator: &dyn ContestSimulator,
        master_seed: u64,
        trial_id: TrialId,
    ) -> Result<TrialRecord, ArenaError> {
        validate_competitors(competitors)?;
        let contests = schedule.contests(competitors)?;
        validate_contests(competitors, &contests)?;
        let contest = contests
            .iter()
            .find(|contest| contest.id == trial_id.contest_id)
            .ok_or(ArenaError::UnknownContest(trial_id.contest_id))?;

        if trial_id.sample_index >= self.samples_per_contest
            || trial_id.seating_index >= contest.len()
        {
            return Err(ArenaError::UnknownTrial(trial_id));
        }

        execute_trial(competitors, simulator, contest, master_seed, trial_id)
    }
}

#[derive(Clone, Copy, Debug)]
struct ContestWork {
    contest_index: usize,
    start: usize,
    end: usize,
    seating_count: usize,
}

fn build_work(
    contests: &[Contest],
    samples_per_contest: usize,
) -> Result<(Vec<ContestWork>, usize), ArenaError> {
    let mut work = Vec::with_capacity(contests.len());
    let mut next = 0_usize;
    for (contest_index, contest) in contests.iter().enumerate() {
        let games = samples_per_contest
            .checked_mul(contest.len())
            .ok_or(ArenaError::TooManyGames)?;
        let end = next.checked_add(games).ok_or(ArenaError::TooManyGames)?;
        work.push(ContestWork {
            contest_index,
            start: next,
            end,
            seating_count: contest.len(),
        });
        next = end;
    }
    Ok((work, next))
}

fn execute_trial(
    competitors: &[Competitor<'_>],
    simulator: &dyn ContestSimulator,
    contest: &Contest,
    master_seed: u64,
    id: TrialId,
) -> Result<TrialRecord, ArenaError> {
    let contest_seed = derive_seed(master_seed, contest.id.0);
    let sample_seed = derive_seed(contest_seed, id.sample_index as u64);
    let seating = Seating::cyclic(contest.len(), id.seating_index);
    seating.validate(contest.id, contest.len())?;
    let context = TrialContext {
        id,
        sample_seed,
        seating,
    };
    let outcome = simulator
        .simulate(competitors, contest, &context)
        .map_err(|source| ArenaError::SimulationFailed {
            trial_id: id,
            source,
        })?;

    if let ContestResult::Winner { seat } = outcome.result
        && seat >= contest.len()
    {
        return Err(ArenaError::InvalidWinnerSeat { trial_id: id, seat });
    }

    Ok(TrialRecord {
        contest: contest.clone(),
        context,
        outcome,
    })
}

fn validate_contests(
    competitors: &[Competitor<'_>],
    contests: &[Contest],
) -> Result<(), ArenaError> {
    let mut ids = BTreeSet::new();
    for contest in contests {
        if !ids.insert(contest.id) {
            return Err(ArenaError::DuplicateContestId(contest.id));
        }
        if contest.is_empty() {
            return Err(ArenaError::EmptyContest(contest.id));
        }

        let mut indices = BTreeSet::new();
        for &index in &contest.competitor_indices {
            if index >= competitors.len() {
                return Err(ArenaError::InvalidCompetitorIndex {
                    contest_id: contest.id,
                    index,
                });
            }
            if !indices.insert(index) {
                return Err(ArenaError::DuplicateCompetitorInContest {
                    contest_id: contest.id,
                    index,
                });
            }
        }
    }
    Ok(())
}
