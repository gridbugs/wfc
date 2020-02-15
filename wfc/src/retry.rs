use crate::{
    wfc::{ForbidPattern, PropagateError, RunBorrow, RunOwn, Wave},
    wrap::Wrap,
};
use rand::Rng;

pub trait RetryOwn: private::Sealed {
    type Return;
    fn retry<'a, W, F, R>(&mut self, run: RunOwn<'a, W, F>, rng: &mut R) -> Self::Return
    where
        W: Wrap + Clone + Sync + Send,
        F: ForbidPattern + Clone + Sync + Send,
        R: Rng;
}

#[derive(Debug, Clone, Copy)]
pub struct Forever;

impl RetryOwn for Forever {
    type Return = Wave;
    fn retry<'a, W, F, R>(
        &mut self,
        mut run: RunOwn<'a, W, F>,
        rng: &mut R,
    ) -> Self::Return
    where
        W: Wrap + Clone + Sync + Send,
        F: ForbidPattern + Clone + Sync + Send,
        R: Rng,
    {
        loop {
            match run.collapse(rng) {
                Ok(()) => (),
                Err(PropagateError::Contradiction) => continue,
            }
            return run.into_wave();
        }
    }
}

/// Retry method which retries a specified number of times, possibly in parallel, where the first
/// attempt to complete without contradiction will be taken. A symptom of the parallelism is that
/// running this with an rng with a known seed may still produce inconsistent results due to
/// non-deterministic timing between threads. This retry method is not suitable for use cases where
/// reproducability is important. It outperforms `NumTimes` in cases where the first attempt leads
/// to contradiction.
#[cfg(feature = "parallel")]
#[derive(Debug, Clone, Copy)]
pub struct ParNumTimes(pub usize);

#[cfg(feature = "parallel")]
impl RetryOwn for ParNumTimes {
    type Return = Result<Wave, PropagateError>;
    fn retry<'a, W, F, R>(&mut self, run: RunOwn<'a, W, F>, rng: &mut R) -> Self::Return
    where
        W: Wrap + Clone + Sync + Send,
        F: ForbidPattern + Clone + Sync + Send,
        R: Rng,
    {
        use rand::SeedableRng;
        use rand_xorshift::XorShiftRng;
        use rayon::prelude::*;
        // Each thread runs with a different rng so they can produce different results.  The
        // `RetryOwn` trait doesn't provide a way to produce new rngs of type `R` besides `clone`,
        // which won't help since we want each rng to be different.  Instead, each thread runs with
        // a `XorShiftRng` seeded with a random number taken from the original rng. `XorShiftRng`
        // is chosen because it is fast, and a cryptographically secure rng (which it is not) is
        // not required for this purpose. It does mean that the rng used by this runner can't be
        // chosen by the caller, but the only way to allow this is to change the `RetryOwn`
        // interface which doesn't seem worth it.
        let rngs = (0..self.0)
            .map(|_| XorShiftRng::seed_from_u64(rng.gen()))
            .collect::<Vec<_>>();
        rngs.into_par_iter()
            .filter_map(|mut rng| {
                let mut runner = run.clone();
                let collapse_result = runner.collapse(&mut rng);
                collapse_result.map(|_| runner.into_wave()).ok()
            })
            .find_any(|_| true)
            .ok_or(PropagateError::Contradiction)
    }
}

/// Retry method which retries a specified number of times, sequentially where the first attempt to
/// complete without contradiction will be taken.
#[derive(Debug, Clone, Copy)]
pub struct NumTimes(pub usize);

impl RetryOwn for NumTimes {
    type Return = Result<Wave, PropagateError>;
    fn retry<'a, W, F, R>(
        &mut self,
        mut run: RunOwn<'a, W, F>,
        rng: &mut R,
    ) -> Self::Return
    where
        W: Wrap + Clone + Sync + Send,
        F: ForbidPattern + Clone + Sync + Send,
        R: Rng,
    {
        loop {
            match run.collapse(rng) {
                Ok(()) => return Ok(run.into_wave()),
                Err(e) => {
                    if self.0 == 0 {
                        return Err(e);
                    } else {
                        self.0 -= 1;
                    }
                }
            }
        }
    }
}

pub trait RetryBorrow: private::Sealed {
    type Return;
    fn retry<'a, W, F, R>(
        &mut self,
        run: &mut RunBorrow<'a, W, F>,
        rng: &mut R,
    ) -> Self::Return
    where
        W: Wrap,
        F: ForbidPattern,
        R: Rng;
}

impl RetryBorrow for Forever {
    type Return = ();
    fn retry<'a, W, F, R>(
        &mut self,
        run: &mut RunBorrow<'a, W, F>,
        rng: &mut R,
    ) -> Self::Return
    where
        W: Wrap,
        F: ForbidPattern,
        R: Rng,
    {
        loop {
            match run.collapse(rng) {
                Ok(()) => break,
                Err(PropagateError::Contradiction) => continue,
            }
        }
    }
}

impl RetryBorrow for NumTimes {
    type Return = Result<(), PropagateError>;
    fn retry<'a, W, F, R>(
        &mut self,
        run: &mut RunBorrow<'a, W, F>,
        rng: &mut R,
    ) -> Self::Return
    where
        W: Wrap,
        F: ForbidPattern,
        R: Rng,
    {
        loop {
            match run.collapse(rng) {
                Ok(()) => return Ok(()),
                Err(e) => {
                    if self.0 == 0 {
                        return Err(e);
                    } else {
                        self.0 -= 1;
                    }
                }
            }
        }
    }
}

mod private {
    use super::*;

    pub trait Sealed {}

    impl Sealed for Forever {}
    impl Sealed for NumTimes {}

    #[cfg(feature = "parallel")]
    impl Sealed for ParNumTimes {}
}
