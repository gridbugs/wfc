use coord_2d::{Coord, Size};
use direction::{CardinalDirectionTable, CardinalDirections};
use grid_2d::Grid;
use hashbrown::HashMap;
use rand::Rng;
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::iter;
use std::ops::{Index, IndexMut};
use std::slice;

pub trait OutputWrap {
    fn normalize_coord(coord: Coord, size: Size) -> Option<Coord>;
}

pub struct WrapNone;
pub struct WrapX;
pub struct WrapY;
pub struct WrapXY;

impl OutputWrap for WrapNone {
    fn normalize_coord(coord: Coord, size: Size) -> Option<Coord> {
        if coord.is_valid(size) {
            Some(coord)
        } else {
            None
        }
    }
}

fn value_is_valid(value: i32, size: u32) -> bool {
    value >= 0 && (value as u32) < size
}

fn normalize_value(value: i32, size: u32) -> i32 {
    let value = value % size as i32;
    if value < 0 {
        value + size as i32
    } else {
        value
    }
}

impl OutputWrap for WrapX {
    fn normalize_coord(coord: Coord, size: Size) -> Option<Coord> {
        if value_is_valid(coord.y, size.y()) {
            let x = normalize_value(coord.x, size.x());
            Some(Coord::new(x, coord.y))
        } else {
            None
        }
    }
}

impl OutputWrap for WrapXY {
    fn normalize_coord(coord: Coord, size: Size) -> Option<Coord> {
        Some(coord.normalize(size))
    }
}

impl OutputWrap for WrapY {
    fn normalize_coord(coord: Coord, size: Size) -> Option<Coord> {
        if value_is_valid(coord.x, size.x()) {
            let y = normalize_value(coord.y, size.y());
            Some(Coord::new(coord.x, y))
        } else {
            None
        }
    }
}

pub type PatternId = u32;

#[derive(Default, Clone)]
pub struct PatternTable<T> {
    table: Vec<T>,
}

impl<T> PatternTable<T> {
    fn len(&self) -> usize {
        self.table.len()
    }
    fn iter(&self) -> slice::Iter<T> {
        self.table.iter()
    }
}

impl<T> iter::FromIterator<T> for PatternTable<T> {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = T>,
    {
        Self {
            table: Vec::from_iter(iter),
        }
    }
}

impl<T> Index<PatternId> for PatternTable<T> {
    type Output = T;
    fn index(&self, index: PatternId) -> &Self::Output {
        self.table.index(index as usize)
    }
}

impl<T> IndexMut<PatternId> for PatternTable<T> {
    fn index_mut(&mut self, index: PatternId) -> &mut Self::Output {
        self.table.index_mut(index as usize)
    }
}

pub struct PatternStats {
    count: u32,
    count_log_count: f32,
}

impl PatternStats {
    pub fn new(count: u32) -> Self {
        Self {
            count,
            count_log_count: (count as f32) * (count as f32).log2(),
        }
    }
}

pub struct GlobalStats {
    stats_per_pattern: PatternTable<PatternStats>,
    compatibility_per_pattern: PatternTable<CardinalDirectionTable<Vec<PatternId>>>,
    sum_pattern_count: u32,
    sum_pattern_count_log_count: f32,
}

impl GlobalStats {
    pub fn new(
        stats_per_pattern: PatternTable<PatternStats>,
        compatibility_per_pattern: PatternTable<CardinalDirectionTable<Vec<PatternId>>>,
    ) -> Self {
        let sum_pattern_count = stats_per_pattern.iter().map(|p| p.count).sum();
        let sum_pattern_count_log_count = stats_per_pattern
            .iter()
            .map(|p| p.count_log_count)
            .sum();
        Self {
            stats_per_pattern,
            compatibility_per_pattern,
            sum_pattern_count,
            sum_pattern_count_log_count,
        }
    }
    fn num_patterns(&self) -> usize {
        self.stats_per_pattern.len()
    }
    fn stats_iter(&self) -> slice::Iter<PatternStats> {
        self.stats_per_pattern.iter()
    }
}

#[derive(PartialEq, Debug, Clone, Copy)]
struct EntropyWithNoise {
    entropy: f32,
    noise: u32,
}

impl Eq for EntropyWithNoise {}

impl PartialOrd for EntropyWithNoise {
    fn partial_cmp(&self, other: &Self) -> Option<::std::cmp::Ordering> {
        match self.entropy.partial_cmp(&other.entropy) {
            Some(Ordering::Equal) => self.noise.partial_cmp(&other.noise),
            other_ordering => other_ordering,
        }
    }
}

#[derive(Debug)]
struct RemovedPattern {
    coord: Coord,
    pattern_id: PatternId,
}

#[derive(Debug, Default)]
struct WaveCellMetadata {
    num_possible_patterns: u32,
    // n0 + n1 + n2 + ...
    sum_possible_pattern_count: u32,
    // n0*log(n0) + n1*log(n1) + n2*log(n2) + ...
    sum_possible_pattern_count_log_count: f32,
}

impl WaveCellMetadata {
    fn remove_possible_pattern(&mut self, pattern_stats: &PatternStats) {
        assert!(
            pattern_stats.count < self.sum_possible_pattern_count,
            "Should never remove the last pattern of a cell"
        );
        self.num_possible_patterns -= 1;
        self.sum_possible_pattern_count -= pattern_stats.count;
        self.sum_possible_pattern_count_log_count -= pattern_stats.count_log_count;
    }
    fn entropy(&self) -> f32 {
        // log(n0+n1+n2+...) - (n0*log(n0) + n1*log(n1) + n2*log(n2) + ...) / (n0+n1+n2+...)
        let sum_possible_pattern_count = self.sum_possible_pattern_count as f32;
        sum_possible_pattern_count.log2()
            - (self.sum_possible_pattern_count_log_count / sum_possible_pattern_count)
    }
}

#[derive(Debug, Default)]
pub struct WaveCell {
    possible_pattern_ids: Vec<bool>,
    metadata: WaveCellMetadata,
    noise: u32,
}

impl WaveCell {
    fn init<R: Rng>(&mut self, global_stats: &GlobalStats, rng: &mut R) {
        self.noise = rng.gen();
        self.metadata.num_possible_patterns = global_stats.num_patterns() as u32;
        self.metadata.sum_possible_pattern_count = global_stats.sum_pattern_count;
        self.metadata.sum_possible_pattern_count_log_count =
            global_stats.sum_pattern_count_log_count;
        self.possible_pattern_ids = (0..self.metadata.num_possible_patterns)
            .map(|_| true)
            .collect();
    }
    fn remove_possible_pattern(
        &mut self,
        pattern_id: PatternId,
        global_stats: &GlobalStats,
    ) {
        assert!(self.metadata.num_possible_patterns > 1);
        let possible_pattern_id = &mut self.possible_pattern_ids[pattern_id as usize];
        if !*possible_pattern_id {
            return;
        }
        *possible_pattern_id = false;
        self.metadata
            .remove_possible_pattern(&global_stats.stats_per_pattern[pattern_id]);
    }
    fn chosen_pattern_id(&self) -> Option<PatternId> {
        self.possible_pattern_ids
            .iter()
            .position(Clone::clone)
            .map(|index| index as PatternId)
    }
    fn choose_pattern_id<R: Rng>(
        &self,
        global_stats: &GlobalStats,
        rng: &mut R,
    ) -> PatternId {
        assert!(self.metadata.num_possible_patterns > 1);
        let mut remaining = rng.gen_range(0, self.metadata.sum_possible_pattern_count);
        for (pattern_id, pattern) in self.possible_pattern_ids
            .iter()
            .zip(global_stats.stats_iter().enumerate())
            .filter_map(|(&is_possible, pattern)| {
                if is_possible {
                    Some(pattern)
                } else {
                    None
                }
            }) {
            if pattern.count < remaining {
                remaining -= pattern.count;
            } else {
                return pattern_id as PatternId;
            }
        }
        unreachable!("possible patterns inconsistent with pattern table");
    }
    fn is_decided(&self) -> bool {
        self.metadata.num_possible_patterns == 1
    }
    fn entropy_with_noise(&self) -> EntropyWithNoise {
        assert!(self.metadata.num_possible_patterns > 1);
        let entropy = self.metadata.entropy();
        let noise = self.noise;
        EntropyWithNoise { entropy, noise }
    }
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

enum NextCellChoice<'a> {
    MinEntropyCell {
        wave_cell: &'a mut WaveCell,
        coord: Coord,
    },
    Complete,
}

struct Propagator {
    remaining_ways_to_become_pattern: Grid<PatternTable<CardinalDirectionTable<u32>>>,
    removed_patterns_to_propagate: Vec<RemovedPattern>,
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
    wave: Grid<WaveCell>,
}

impl Observer {
    fn choose_next_cell(&mut self) -> NextCellChoice {
        while let Some(coord_entropy) = self.entropy_priority_queue.pop() {
            if !self.wave
                .get(coord_entropy.coord)
                .unwrap()
                .is_decided()
            {
                let wave_cell = self.wave.get_mut(coord_entropy.coord).unwrap();
                return NextCellChoice::MinEntropyCell {
                    coord: coord_entropy.coord,
                    wave_cell,
                };
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
        let wave = Grid::new_default(output_size);
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
            .compatibility_per_pattern
            .iter()
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
        if wave_cell.metadata.num_possible_patterns > 1
            && wave_cell.possible_pattern_ids[pattern_id as usize]
        {
            Self::remove_remaining_possibile_patterns(
                coord,
                wave_cell,
                &mut self.propagator,
                pattern_id,
                global_stats,
            );
            self.num_undecided_cells -= 1;
        }
    }
    fn remove_remaining_possibile_patterns(
        coord: Coord,
        wave_cell: &mut WaveCell,
        propagator: &mut Propagator,
        chosen_pattern_id: PatternId,
        global_stats: &GlobalStats,
    ) {
        for ((pattern_id, is_possible), pattern_stats) in wave_cell
            .possible_pattern_ids
            .iter_mut()
            .enumerate()
            .zip(global_stats.stats_iter())
        {
            if pattern_id as PatternId != chosen_pattern_id {
                if *is_possible {
                    *is_possible = false;
                    wave_cell
                        .metadata
                        .remove_possible_pattern(pattern_stats);
                    propagator.add(coord, pattern_id as PatternId);
                }
            }
        }
    }
    fn observe<R: Rng>(&mut self, global_stats: &GlobalStats, rng: &mut R) -> Step {
        if self.num_undecided_cells == 0 {
            return Step::Complete;
        }
        let (wave_cell, coord) = match self.observer.choose_next_cell() {
            NextCellChoice::Complete => return Step::Complete,
            NextCellChoice::MinEntropyCell { wave_cell, coord } => (wave_cell, coord),
        };
        let chosen_pattern_id = wave_cell.choose_pattern_id(global_stats, rng);
        Self::remove_remaining_possibile_patterns(
            coord,
            wave_cell,
            &mut self.propagator,
            chosen_pattern_id,
            global_stats,
        );
        self.num_undecided_cells -= 1;
        Step::Incomplete
    }

    fn propagate<W: OutputWrap>(&mut self, global_stats: &GlobalStats) {
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
                for &pattern_id in global_stats.compatibility_per_pattern
                    [removed_pattern.pattern_id]
                    .get(direction)
                    .iter()
                {
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
                        cell.remove_possible_pattern(pattern_id, global_stats);

                        if cell.is_decided() {
                            self.num_undecided_cells -= 1;
                            self.next_entropies.remove(&coord_to_update);
                        } else {
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
    }
    fn step<W: OutputWrap, R: Rng>(
        &mut self,
        global_stats: &GlobalStats,
        rng: &mut R,
    ) -> Step {
        self.propagate::<W>(global_stats);
        self.observe(global_stats, rng)
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
    pub fn step(&mut self) -> Step {
        self.context.step::<W, _>(self.global_stats, self.rng)
    }
    pub fn set_pattern(&mut self, coord: Coord, pattern_id: PatternId) {
        self.context
            .set_pattern(coord, pattern_id, self.global_stats);
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn wraps() {
        assert_eq! {
            WrapNone::normalize_coord(Coord::new(2, 3), Size::new(4, 5)),
            Some(Coord::new(2, 3))
        };
        assert_eq! {
            WrapNone::normalize_coord(Coord::new(4, 3), Size::new(4, 5)),
            None,
        };
        assert_eq! {
            WrapX::normalize_coord(Coord::new(4, 3), Size::new(4, 5)),
            Some(Coord::new(0, 3)),
        };
        assert_eq! {
            WrapY::normalize_coord(Coord::new(4, 3), Size::new(4, 5)),
            None,
        };
        assert_eq! {
            WrapY::normalize_coord(Coord::new(2, 6), Size::new(4, 5)),
            Some(Coord::new(2, 1)),
        };
        assert_eq! {
            WrapXY::normalize_coord(Coord::new(2, 6), Size::new(4, 5)),
            Some(Coord::new(2, 1)),
        };
    }
}
