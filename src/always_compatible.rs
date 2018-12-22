use context::{Context, Observe, PropagateError, Run, Wave, WaveCellHandle};
use coord_2d::Coord;
use pattern::GlobalStats;
use rand::Rng;
use wrap::OutputWrap;

/// An interface to wfc which statically prevents manually setting incompatible patterns.  When
/// interracting directly with a `Run`, it's possible to manually choose the patterns in adjacent
/// cells to be incompatible with one another, without calling `propagate` in between.  This
/// interface prevents such usage by restricting manual updates to an individual cell, and forcing
/// propagation by means of types, before a different cell can be manually updated.

pub struct Ready<'a, W: OutputWrap> {
    run: Run<'a, W>,
}

struct Observed<'a, W: OutputWrap> {
    run: Run<'a, W>,
}

pub struct Manual<'a, W: OutputWrap> {
    run: Run<'a, W>,
    coord: Coord,
}

fn propagate<'a, W: OutputWrap>(
    mut run: Run<'a, W>,
) -> Result<Ready<'a, W>, PropagateError> {
    run.propagate().map(|()| Ready { run })
}

pub enum Step<'a, W: OutputWrap> {
    Incomplete(Ready<'a, W>),
    Complete,
}

impl<'a, W: OutputWrap> Ready<'a, W> {
    pub fn new<R: Rng>(
        context: &'a mut Context,
        wave: &'a mut Wave,
        global_stats: &'a GlobalStats,
        output_wrap: W,
        rng: &mut R,
    ) -> Self {
        let run = Run::new(context, wave, global_stats, output_wrap, rng);
        Self { run }
    }

    pub fn into_run(self) -> Run<'a, W> {
        self.run
    }

    pub fn manual(self, coord: Coord) -> Manual<'a, W> {
        Manual {
            run: self.run,
            coord,
        }
    }

    pub fn step<R: Rng>(mut self, rng: &mut R) -> Result<Step<'a, W>, PropagateError> {
        match self.run.observe(rng) {
            Observe::Complete => Ok(Step::Complete),
            Observe::Incomplete => propagate(self.run).map(Step::Incomplete),
        }
    }

    pub fn with_wave_cell_handle<F: FnMut(WaveCellHandle)>(
        self,
        coord: Coord,
        mut f: F,
    ) -> Result<Ready<'a, W>, PropagateError> {
        let mut manual = self.manual(coord);
        f(manual.wave_cell_handle());
        manual.propagate()
    }
}

impl<'a, W: OutputWrap> Manual<'a, W> {
    pub fn wave_cell_handle(&mut self) -> WaveCellHandle {
        self.run.wave_cell_handle(self.coord)
    }

    pub fn propagate(self) -> Result<Ready<'a, W>, PropagateError> {
        propagate(self.run)
    }
}
