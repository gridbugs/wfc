use coord_2d::{Coord, Size};
use direction::{CardinalDirectionTable, CardinalDirections};
use grid_2d::Grid;
use hashbrown::HashMap;
use pattern::{GlobalStats, PatternId, PatternStats, PatternTable};
use rand::Rng;
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use wave::*;
use wrap::*;

#[derive(Debug)]
struct RemovedPattern {
    coord: Coord,
    pattern_id: PatternId,
}

#[derive(Debug)]
pub struct Contradiction {
    pub coord: Coord,
}

#[derive(PartialEq, Eq, Debug)]
struct CoordEntropy {
    coord: Coord,
    entropy_with_noise: EntropyWithNoise,
}

impl PartialOrd for CoordEntropy {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        other
            .entropy_with_noise
            .partial_cmp(&self.entropy_with_noise)
    }
}

impl Ord for CoordEntropy {
    fn cmp(&self, other: &Self) -> Ordering {
        if self < other {
            return Ordering::Less;
        }
        if self == other {
            return Ordering::Equal;
        }
        return Ordering::Greater;
    }
}

pub enum Step {
    Complete,
    Incomplete,
}

struct CellAtCoordMut<'a> {
    wave_cell: &'a mut WaveCell,
    coord: Coord,
}

enum NextCellChoice<'a> {
    MinEntropyCell(CellAtCoordMut<'a>),
    Complete,
}

struct Propagator {
    remaining_ways_to_become_pattern: Grid<PatternTable<CardinalDirectionTable<u32>>>,
    removed_patterns_to_propagate: Vec<RemovedPattern>,
}

impl<'a> CellAtCoordMut<'a> {
    fn remove_all_patterns_except_one(
        &mut self,
        pattern_id: PatternId,
        global_stats: &GlobalStats,
        propagator: &mut Propagator,
    ) {
        let coord = self.coord;
        self.wave_cell
            .for_each_possible_pattern(|possible_pattern| {
                if possible_pattern.pattern_id() != pattern_id {
                    propagator.add(coord, possible_pattern.pattern_id());
                    possible_pattern.remove(global_stats);
                }
            });
    }
}

const ZERO_CARDINAL_DIRECTION_TABLE: CardinalDirectionTable<u32> =
    CardinalDirectionTable::new_array([0, 0, 0, 0]);

impl Propagator {
    fn add(&mut self, coord: Coord, pattern_id: PatternId) {
        self.removed_patterns_to_propagate
            .push(RemovedPattern {
                coord,
                pattern_id,
            });
        self.remaining_ways_to_become_pattern
            .get_checked_mut(coord)[pattern_id] = ZERO_CARDINAL_DIRECTION_TABLE;
    }
}

struct Observer {
    entropy_priority_queue: BinaryHeap<CoordEntropy>,
    wave: Wave,
}

impl Observer {
    fn choose_next_cell(&mut self) -> NextCellChoice {
        while let Some(coord_entropy) = self.entropy_priority_queue.pop() {
            if self.wave
                .get_checked(coord_entropy.coord)
                .is_undecided()
            {
                let wave_cell = self.wave.get_checked_mut(coord_entropy.coord);
                return NextCellChoice::MinEntropyCell(CellAtCoordMut {
                    coord: coord_entropy.coord,
                    wave_cell,
                });
            }
        }
        NextCellChoice::Complete
    }
}

pub struct Context {
    propagator: Propagator,
    observer: Observer,
    next_entropies: HashMap<Coord, EntropyWithNoise>,
    num_undecided_cells: u32,
}

impl Context {
    pub fn new(output_size: Size) -> Self {
        let remaining_ways_to_become_pattern = Grid::new_default(output_size);
        let removed_patterns_to_propagate = Vec::default();
        let next_entropies = HashMap::default();
        let wave = Wave::new(output_size);
        let entropy_priority_queue = BinaryHeap::default();
        let num_undecided_cells = 0;
        Self {
            propagator: Propagator {
                remaining_ways_to_become_pattern,
                removed_patterns_to_propagate,
            },
            observer: Observer {
                wave,
                entropy_priority_queue,
            },
            next_entropies,
            num_undecided_cells,
        }
    }
    fn init<R: Rng>(&mut self, global_stats: &GlobalStats, rng: &mut R) {
        let initial_ways_to_become_pattern = global_stats
            .compatible_patterns_by_direction()
            .map(|compatible_patterns_per_direction| {
                let mut num_ways_to_become_pattern_from_direction =
                    CardinalDirectionTable::default();
                for direction in CardinalDirections {
                    *num_ways_to_become_pattern_from_direction.get_mut(direction) =
                        compatible_patterns_per_direction
                            .get(direction.opposite())
                            .len() as u32;
                }
                num_ways_to_become_pattern_from_direction
            })
            .collect::<PatternTable<_>>();
        self.propagator
            .remaining_ways_to_become_pattern
            .iter_mut()
            .for_each(|ways_to_become_pattern| {
                *ways_to_become_pattern = initial_ways_to_become_pattern.clone()
            });
        self.propagator.removed_patterns_to_propagate.clear();
        self.next_entropies.clear();
        let entropy_priority_queue = &mut self.observer.entropy_priority_queue;
        entropy_priority_queue.clear();
        self.observer
            .wave
            .enumerate_mut()
            .for_each(|(coord, wave_cell)| {
                wave_cell.init(global_stats, rng);
                let coord_entropy = CoordEntropy {
                    coord,
                    entropy_with_noise: wave_cell.entropy_with_noise(),
                };
                entropy_priority_queue.push(coord_entropy);
            });
        self.num_undecided_cells = self.observer.wave.size().count() as u32;
    }
    pub fn get_pattern_id(&self, coord: Coord) -> Option<PatternId> {
        self.observer
            .wave
            .get_checked(coord)
            .chosen_pattern_id()
    }
    pub fn size(&self) -> Size {
        self.observer.wave.size()
    }
    fn set_pattern(
        &mut self,
        coord: Coord,
        pattern_id: PatternId,
        global_stats: &GlobalStats,
    ) {
        let wave_cell = self.observer.wave.get_checked_mut(coord);
        if wave_cell.is_pattern_compatible(pattern_id)
            || wave_cell.is_undecided_or_all_compatible_patterns_have_zero_probability()
        {
            CellAtCoordMut { wave_cell, coord }.remove_all_patterns_except_one(
                pattern_id,
                global_stats,
                &mut self.propagator,
            );
            self.num_undecided_cells -= 1;
        }
    }
    fn observe<R: Rng>(&mut self, global_stats: &GlobalStats, rng: &mut R) -> Step {
        if self.num_undecided_cells == 0 {
            return Step::Complete;
        }
        let mut cell_at_coord = match self.observer.choose_next_cell() {
            NextCellChoice::Complete => return Step::Complete,
            NextCellChoice::MinEntropyCell(cell_at_coord) => cell_at_coord,
        };
        let pattern_id = cell_at_coord
            .wave_cell
            .choose_pattern_id(global_stats, rng);
        cell_at_coord.remove_all_patterns_except_one(
            pattern_id,
            &global_stats,
            &mut self.propagator,
        );
        self.num_undecided_cells -= 1;
        Step::Incomplete
    }

    fn propagate<W: OutputWrap>(
        &mut self,
        global_stats: &GlobalStats,
    ) -> Result<(), Contradiction> {
        while let Some(removed_pattern) =
            self.propagator.removed_patterns_to_propagate.pop()
        {
            let wave_size = self.observer.wave.size();
            for direction in CardinalDirections {
                let coord_to_update = if let Some(coord_to_update) = W::normalize_coord(
                    removed_pattern.coord + direction.coord(),
                    wave_size,
                ) {
                    coord_to_update
                } else {
                    continue;
                };
                let remaining = self.propagator
                    .remaining_ways_to_become_pattern
                    .get_checked_mut(coord_to_update);
                for &pattern_id in global_stats.compatible_patterns_in_direction(
                    removed_pattern.pattern_id,
                    direction,
                ) {
                    let remaining = &mut remaining[pattern_id];
                    let count = {
                        let count = remaining.get_mut(direction);
                        if *count == 0 {
                            continue;
                        }
                        *count -= 1;
                        *count
                    };
                    if count == 0 {
                        let cell = self.observer.wave.get_checked_mut(coord_to_update);
                        if let Some(pattern) = cell.possible_pattern(pattern_id) {
                            pattern.remove(global_stats);
                        }
                        match cell.state() {
                            WaveCellState::Decided => {
                                self.num_undecided_cells -= 1;
                                self.next_entropies.remove(&coord_to_update);
                            }
                            WaveCellState::Undecided => {
                                let noise = cell.entropy_with_noise();
                                self.next_entropies
                                    .entry(coord_to_update)
                                    .and_modify(|existing_noise| {
                                        if noise < *existing_noise {
                                            *existing_noise = noise;
                                        }
                                    })
                                    .or_insert(noise);
                            }
                            WaveCellState::AllCompatiblePatternsHaveZeroProbability
                            | WaveCellState::NoCompatiblePatterns => {
                                self.next_entropies.remove(&coord_to_update);
                            }
                        }
                        self.propagator.removed_patterns_to_propagate.push(
                            RemovedPattern {
                                coord: coord_to_update,
                                pattern_id,
                            },
                        );
                        *remaining = ZERO_CARDINAL_DIRECTION_TABLE;
                    }
                }
            }
        }
        for (coord, entropy_with_noise) in self.next_entropies.drain() {
            self.observer
                .entropy_priority_queue
                .push(CoordEntropy {
                    coord,
                    entropy_with_noise,
                });
        }
        Ok(())
    }
    fn step<W: OutputWrap, R: Rng>(
        &mut self,
        global_stats: &GlobalStats,
        rng: &mut R,
    ) -> Result<Step, Contradiction> {
        self.propagate::<W>(global_stats)?;
        Ok(self.observe(global_stats, rng))
    }
    pub fn run<'a, W: OutputWrap, R: Rng>(
        &'a mut self,
        global_stats: &'a GlobalStats,
        output_wrap: W,
        rng: &'a mut R,
    ) -> Run<'a, W, R> {
        self.init(global_stats, rng);
        Run {
            context: self,
            global_stats,
            rng,
            output_wrap,
        }
    }
}

pub struct Run<'a, W: OutputWrap, R: 'a + Rng> {
    context: &'a mut Context,
    global_stats: &'a GlobalStats,
    rng: &'a mut R,
    output_wrap: W,
}

impl<'a, W: OutputWrap, R: Rng> Run<'a, W, R> {
    pub fn step(&mut self) -> Result<Step, Contradiction> {
        self.context.step::<W, _>(self.global_stats, self.rng)
    }
    pub fn set_pattern(&mut self, coord: Coord, pattern_id: PatternId) {
        self.context
            .set_pattern(coord, pattern_id, self.global_stats);
    }
}
