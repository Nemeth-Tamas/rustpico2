//! Fixed-capacity cooperative job scheduler.
//!
//! Jobs run in the main thread, never inside an ISR.
//! One-shot and repeating deadlines use TIMER0 microseconds.
//! No allocator, heap, std or external crates.

pub const MAX_JOBS: usize = 8;

const MAX_DELAY_MS: u32 = 600_000;
const MIN_ONESHOT_MS: u32 = 10;
const MIN_REPEAT_MS: u32 = 1_000;

#[derive(Clone, Copy)]
pub struct Job {
    pub active: bool,
    pub id: u32,
    pub deadline_us: u64,
    pub period_us: u64,
}

impl Job {
    const EMPTY: Self = Self {
        active: false,
        id: 0,
        deadline_us: 0,
        period_us: 0,
    };
}

pub struct FiredJob {
    pub id: u32,
    pub repeating: bool,
}

pub enum ScheduleError {
    InvalidDelay,
    Full,
}

pub struct Scheduler {
    jobs: [Job; MAX_JOBS],
    next_id: u32,
}

impl Scheduler {
    pub const fn new() -> Self {
        Self {
            jobs: [Job::EMPTY; MAX_JOBS],
            next_id: 1,
        }
    }

    /// Add a one-shot or repeating job.
    ///
    /// One-shot delay: 10..600000 ms.
    /// Repeating period: 1000..600000 ms.
    pub fn schedule(
        &mut self,
        delay_ms: u32,
        repeating: bool,
        now_us: u64,
    ) -> Result<u32, ScheduleError> {
        let minimum = if repeating {
            MIN_REPEAT_MS
        } else {
            MIN_ONESHOT_MS
        };

        if !(minimum..=MAX_DELAY_MS).contains(&delay_ms) {
            return Err(ScheduleError::InvalidDelay);
        }

        let Some(slot) = self.jobs.iter_mut().find(|job| !job.active) else {
            return Err(ScheduleError::Full);
        };

        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);

        if self.next_id == 0 {
            self.next_id = 1;
        }

        let delay_us = u64::from(delay_ms) * 1_000;

        *slot = Job {
            active: true,
            id,
            deadline_us: now_us.saturating_add(delay_us),
            period_us: if repeating { delay_us } else { 0 },
        };

        Ok(id)
    }

    /// Cancel a job by ID.
    pub fn cancel(&mut self, id: u32) -> bool {
        for job in &mut self.jobs {
            if job.active && job.id == id {
                job.active = false;
                return true;
            }
        }

        false
    }

    /// Iterate over active jobs.
    pub fn active(&self) -> impl Iterator<Item = &Job> {
        self.jobs.iter().filter(|job| job.active)
    }

    /// Return the earliest active deadline, if any.
    pub fn next_deadline(&self) -> Option<u64> {
        self.active().map(|job| job.deadline_us).min()
    }

    /// Take one due event and update its task slot.
    ///
    /// Repeating jobs advance from the current time.
    /// Missed intervals are intentionally skipped rather
    /// than accumulating a burst of catch-up executions.
    pub fn take_due(&mut self, now_us: u64) -> Option<FiredJob> {
        for job in &mut self.jobs {
            if !job.active || now_us < job.deadline_us {
                continue;
            }

            let fired = FiredJob {
                id: job.id,
                repeating: job.period_us != 0,
            };

            if job.period_us == 0 {
                job.active = false;
            } else {
                job.deadline_us = now_us.saturating_add(job.period_us);
            }

            return Some(fired);
        }

        None
    }
}
