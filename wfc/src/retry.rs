use crate::{
    wfc::{ForbidPattern, PropagateError, RunBorrow, RunOwn, Wave},
    wrap::Wrap,
};
use rand::Rng;

pub trait RetryOwn: private::Sealed {
    type Return;
    fn retry<'a, W, F, R>(&mut self, run: RunOwn<'a, W, F>, rng: &mut R) -> Self::Return
    where
        W: Wrap,
        F: ForbidPattern,
        R: Rng + Sync + Send + Clone;
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
        W: Wrap,
        F: ForbidPattern,
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

use rayon::prelude::*;
#[derive(Debug, Clone, Copy)]
pub struct ParNumTimes(pub usize);

impl RetryOwn for ParNumTimes {
    type Return = Result<Wave, PropagateError>;
    fn retry<'a, W, F, R>(&mut self, run: RunOwn<'a, W, F>, rng: &mut R) -> Self::Return
    where
        W: Wrap,
        F: ForbidPattern,
        R: Rng + Sync + Send + Clone,
    {
        (0..self.0)
            .into_par_iter()
            .map(|_| {
                let mut c = run.clone();
                let i = c.collapse(&mut rng.clone());
                (i.is_ok(), c.into_wave())
            })
            .find_any(|i| i.0)
            .map(|i| i.1)
            .ok_or(PropagateError::Contradiction)
    }
}

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
        W: Wrap,
        F: ForbidPattern,
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
    impl Sealed for ParNumTimes {}
}
