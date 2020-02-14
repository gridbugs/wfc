use crate::{
    retry,
    wrap::{Wrap, WrapXY},
};
use coord_2d::{Coord, Size};
use direction::{CardinalDirection, CardinalDirectionTable, CardinalDirections};
use grid_2d::Grid;
use hashbrown::HashMap;
use rand::Rng;
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::iter;
use std::marker::PhantomData;
use std::num::NonZeroU32;
use std::ops::{Index, IndexMut};
use std::slice;

pub type PatternId = u32;

#[derive(Default, Clone, Debug)]
pub struct PatternTable<T> {
    table: Vec<T>,
}

impl<T> PatternTable<T> {
    pub fn from_vec(table: Vec<T>) -> Self {
        Self { table }
    }
    pub fn len(&self) -> usize {
        self.table.len()
    }
    pub fn drain(&mut self) -> ::std::vec::Drain<T> {
        self.table.drain(..)
    }
    pub fn iter(&self) -> slice::Iter<T> {
        self.table.iter()
    }
    pub fn iter_mut(&mut self) -> slice::IterMut<T> {
        self.table.iter_mut()
    }
    pub fn enumerate(&self) -> impl Iterator<Item = (PatternId, &T)> {
        self.iter()
            .enumerate()
            .map(|(index, item)| (index as PatternId, item))
    }
    pub fn enumerate_mut(&mut self) -> impl Iterator<Item = (PatternId, &mut T)> {
        self.iter_mut()
            .enumerate()
            .map(|(index, item)| (index as PatternId, item))
    }
}
impl<T: Clone> PatternTable<T> {
    fn resize(&mut self, size: usize, value: T) {
        self.table.resize(size, value);
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

pub struct PatternWeight {
    weight: NonZeroU32,
    weight_log_weight: f32,
}

impl PatternWeight {
    pub fn new(weight: NonZeroU32) -> Self {
        Self {
            weight,
            weight_log_weight: (weight.get() as f32) * (weight.get() as f32).log2(),
        }
    }
    pub fn weight(&self) -> u32 {
        self.weight.get()
    }
    pub fn weight_log_weight(&self) -> f32 {
        self.weight_log_weight
    }
}

pub struct GlobalStats {
    pattern_weights: PatternTable<Option<PatternWeight>>,
    compatibility_per_pattern: PatternTable<CardinalDirectionTable<Vec<PatternId>>>,
    num_weighted_patterns: u32,
    sum_pattern_weight: u32,
    sum_pattern_weight_log_weight: f32,
}

struct NumWaysToBecomeEachPatternByDirection<'a> {
    iter: slice::Iter<'a, CardinalDirectionTable<Vec<PatternId>>>,
}

impl<'a> Iterator for NumWaysToBecomeEachPatternByDirection<'a> {
    type Item = CardinalDirectionTable<u32>;
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|compatible_patterns_by_direction| {
            let mut num_ways_to_become_pattern_from_direction =
                CardinalDirectionTable::default();
            for direction in CardinalDirections {
                num_ways_to_become_pattern_from_direction[direction] =
                    compatible_patterns_by_direction
                        .get(direction.opposite())
                        .len() as u32;
            }

            num_ways_to_become_pattern_from_direction
        })
    }
}

pub struct PatternDescription {
    pub weight: Option<NonZeroU32>,
    pub allowed_neighbours: CardinalDirectionTable<Vec<PatternId>>,
}

impl PatternDescription {
    pub fn new(
        weight: Option<NonZeroU32>,
        allowed_neighbours: CardinalDirectionTable<Vec<PatternId>>,
    ) -> Self {
        Self {
            weight,
            allowed_neighbours,
        }
    }
}

struct OptionSliceIter<'a, T> {
    iter: slice::Iter<'a, Option<T>>,
}

impl<'a, T> Iterator for OptionSliceIter<'a, T> {
    type Item = Option<&'a T>;
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|o| o.as_ref())
    }
}

impl GlobalStats {
    pub fn new(mut pattern_descriptions: PatternTable<PatternDescription>) -> Self {
        let pattern_weights = pattern_descriptions
            .iter()
            .map(|desc| desc.weight.map(PatternWeight::new))
            .collect::<PatternTable<_>>();
        let compatibility_per_pattern = pattern_descriptions
            .drain()
            .map(|desc| desc.allowed_neighbours)
            .collect::<PatternTable<_>>();
        let num_weighted_patterns =
            pattern_weights.iter().filter(|p| p.is_some()).count() as u32;
        let sum_pattern_weight = pattern_weights
            .iter()
            .filter_map(|p| p.as_ref().map(|p| p.weight()))
            .sum();
        let sum_pattern_weight_log_weight = pattern_weights
            .iter()
            .filter_map(|p| p.as_ref().map(|p| p.weight_log_weight()))
            .sum();
        Self {
            pattern_weights,
            compatibility_per_pattern,
            num_weighted_patterns,
            sum_pattern_weight,
            sum_pattern_weight_log_weight,
        }
    }
    fn num_weighted_patterns(&self) -> u32 {
        self.num_weighted_patterns
    }
    fn sum_pattern_weight(&self) -> u32 {
        self.sum_pattern_weight
    }
    fn sum_pattern_weight_log_weight(&self) -> f32 {
        self.sum_pattern_weight_log_weight
    }
    fn num_patterns(&self) -> usize {
        self.pattern_weights.len()
    }
    fn pattern_stats(&self, pattern_id: PatternId) -> Option<&PatternWeight> {
        self.pattern_weights[pattern_id].as_ref()
    }
    fn pattern_stats_option_iter(&self) -> OptionSliceIter<PatternWeight> {
        OptionSliceIter {
            iter: self.pattern_weights.iter(),
        }
    }
    fn compatible_patterns_in_direction(
        &self,
        pattern_id: PatternId,
        direction: CardinalDirection,
    ) -> impl Iterator<Item = &PatternId> {
        self.compatibility_per_pattern[pattern_id]
            .get(direction)
            .iter()
    }
    fn compatible_patterns_by_direction(
        &self,
    ) -> slice::Iter<CardinalDirectionTable<Vec<PatternId>>> {
        self.compatibility_per_pattern.iter()
    }
    fn num_ways_to_become_each_pattern_by_direction(
        &self,
    ) -> NumWaysToBecomeEachPatternByDirection {
        NumWaysToBecomeEachPatternByDirection {
            iter: self.compatible_patterns_by_direction(),
        }
    }
}

#[derive(Default, Debug, Clone)]
struct WaveCellStats {
    num_weighted_compatible_patterns: u32,
    // n0 + n1 + n2 + ...
    sum_compatible_pattern_weight: u32,
    // n0*log(n0) + n1*log(n1) + n2*log(n2) + ...
    sum_compatible_pattern_weight_log_weight: f32,
}

impl WaveCellStats {
    fn remove_compatible_pattern(&mut self, pattern_stats: &PatternWeight) {
        assert!(self.num_weighted_compatible_patterns >= 1);
        assert!(self.sum_compatible_pattern_weight >= pattern_stats.weight());

        self.num_weighted_compatible_patterns -= 1;
        self.sum_compatible_pattern_weight -= pattern_stats.weight();
        self.sum_compatible_pattern_weight_log_weight -=
            pattern_stats.weight_log_weight();
    }
    fn entropy(&self) -> f32 {
        assert!(self.sum_compatible_pattern_weight > 0);

        // log(n0+n1+n2+...) - (n0*log(n0) + n1*log(n1) + n2*log(n2) + ...) / (n0+n1+n2+...)
        let sum_compatible_pattern_weight = self.sum_compatible_pattern_weight as f32;
        sum_compatible_pattern_weight.log2()
            - (self.sum_compatible_pattern_weight_log_weight
                / sum_compatible_pattern_weight)
    }
}

#[derive(Default, Clone, Debug)]
struct NumWaysToBecomePattern {
    direction_table: CardinalDirectionTable<u32>,
}

struct DecrementedToZero;

impl NumWaysToBecomePattern {
    const ZERO_CARDINAL_DIRECTION_TABLE: CardinalDirectionTable<u32> =
        CardinalDirectionTable::new_array([0, 0, 0, 0]);
    fn new(direction_table: CardinalDirectionTable<u32>) -> Self {
        if direction_table.iter().any(|&count| count == 0) {
            Self {
                direction_table: Self::ZERO_CARDINAL_DIRECTION_TABLE,
            }
        } else {
            Self { direction_table }
        }
    }
    fn is_zero(&self) -> bool {
        // if any element is 0, all elements must be 0, so it's sufficient to
        // test a single element
        assert!(
            *self.direction_table.get(CardinalDirection::North) != 0
                || self.direction_table == Self::ZERO_CARDINAL_DIRECTION_TABLE
        );
        *self.direction_table.get(CardinalDirection::North) == 0
    }
    fn clear_all_directions(&mut self) {
        self.direction_table = Self::ZERO_CARDINAL_DIRECTION_TABLE;
    }
    fn try_decrement(
        &mut self,
        direction: CardinalDirection,
    ) -> Option<DecrementedToZero> {
        {
            let count = self.direction_table.get_mut(direction);
            if *count == 0 {
                return None;
            }
            if *count != 1 {
                *count -= 1;
                return None;
            }
        }
        self.clear_all_directions();
        Some(DecrementedToZero)
    }
}

#[derive(Default, Debug, Clone)]
pub struct WaveCell {
    // random value to break entropy ties
    noise: u32,
    num_compatible_patterns: u32,
    stats: WaveCellStats,
    // Keep track of the number of ways each neighbour could be assigned a pattern to allow this
    // cell to be each pattern. This doubles as a way of keeping track of which patterns are
    // compatible with this cell.
    num_ways_to_become_each_pattern: PatternTable<NumWaysToBecomePattern>,
}

enum DecrementNumWaysToBecomePattern {
    NoPatternRemoved,
    RemovedNonWeightedPattern,
    RemovedFinalCompatiblePattern,
    RemovedFinalWeightedCompatiblePattern,
    Finalized,
    RemovedWeightedPatternMultipleCandidatesRemain,
}

#[derive(PartialEq, Debug, Clone, Copy)]
struct EntropyWithNoise {
    entropy: f32,
    noise: u32,
    // Record this field of WaveCellStats at the time of creating this entry.  This value will be
    // different from the cell's value when retrieved from the heap if and only if the cell's
    // entropy has changed. Storing this saves us from having to re-compute the cell's entropy to
    // compare it to the stored entropy.
    num_weighted_compatible_patterns: u32,
}

impl Eq for EntropyWithNoise {}

impl PartialOrd for EntropyWithNoise {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match self.entropy.partial_cmp(&other.entropy) {
            Some(Ordering::Equal) => self.noise.partial_cmp(&other.noise),
            other_ordering => other_ordering,
        }
    }
}

#[derive(Debug)]
pub enum ChosenPatternIdError {
    NoCompatiblePatterns,
    MultipleCompatiblePatterns,
}

impl WaveCell {
    pub fn chosen_pattern_id(&self) -> Result<PatternId, ChosenPatternIdError> {
        if self.num_compatible_patterns == 1 {
            let pattern_id = self
                .num_ways_to_become_each_pattern
                .enumerate()
                .filter_map(|(pattern_id, num_ways_to_become_pattern)| {
                    if num_ways_to_become_pattern.is_zero() {
                        None
                    } else {
                        Some(pattern_id)
                    }
                })
                .next()
                .expect("Missing pattern");
            Ok(pattern_id)
        } else if self.num_compatible_patterns == 0 {
            Err(ChosenPatternIdError::NoCompatiblePatterns)
        } else {
            Err(ChosenPatternIdError::MultipleCompatiblePatterns)
        }
    }
    fn weighted_compatible_stats_enumerate<'a>(
        &'a self,
        global_stats: &'a GlobalStats,
    ) -> impl Iterator<Item = (PatternId, &'a PatternWeight)> {
        self.num_ways_to_become_each_pattern
            .iter()
            .zip(global_stats.pattern_stats_option_iter())
            .enumerate()
            .filter_map(
                |(pattern_id_usize, (num_ways_to_become_pattern, pattern_stats))| {
                    if num_ways_to_become_pattern.is_zero() {
                        None
                    } else {
                        pattern_stats.map(|pattern_stats| {
                            (pattern_id_usize as PatternId, pattern_stats)
                        })
                    }
                },
            )
    }
    fn sum_compatible_pattern_weight(&self, global_stats: &GlobalStats) -> u32 {
        self.num_ways_to_become_each_pattern
            .iter()
            .zip(global_stats.pattern_stats_option_iter())
            .filter_map(|(num_ways_to_become_pattern, pattern_stats)| {
                if num_ways_to_become_pattern.is_zero() {
                    None
                } else {
                    pattern_stats.map(|pattern_stats| pattern_stats.weight())
                }
            })
            .sum()
    }
    fn decrement_num_ways_to_become_pattern(
        &mut self,
        pattern_id: PatternId,
        direction: CardinalDirection,
        global_stats: &GlobalStats,
    ) -> DecrementNumWaysToBecomePattern {
        use self::DecrementNumWaysToBecomePattern as D;
        match self.num_ways_to_become_each_pattern[pattern_id].try_decrement(direction) {
            Some(DecrementedToZero) => {
                assert!(self.num_compatible_patterns >= 1);
                self.num_compatible_patterns -= 1;
                if let Some(pattern_stats) = global_stats.pattern_stats(pattern_id) {
                    self.stats.remove_compatible_pattern(pattern_stats);
                    match self.stats.num_weighted_compatible_patterns {
                        0 => {
                            if self.num_compatible_patterns == 0 {
                                D::RemovedFinalCompatiblePattern
                            } else {
                                D::RemovedFinalWeightedCompatiblePattern
                            }
                        }
                        _ => {
                            assert!(self.num_compatible_patterns != 0);
                            if self.num_compatible_patterns == 1 {
                                assert!(self.stats.num_weighted_compatible_patterns == 1);
                                D::Finalized
                            } else {
                                D::RemovedWeightedPatternMultipleCandidatesRemain
                            }
                        }
                    }
                } else {
                    D::RemovedNonWeightedPattern
                }
            }
            None => D::NoPatternRemoved,
        }
    }
    fn entropy_with_noise(&self) -> EntropyWithNoise {
        let entropy = self.stats.entropy();
        let noise = self.noise;
        let num_weighted_compatible_patterns =
            self.stats.num_weighted_compatible_patterns;
        EntropyWithNoise {
            entropy,
            noise,
            num_weighted_compatible_patterns,
        }
    }
    fn choose_pattern_id<R: Rng>(
        &self,
        global_stats: &GlobalStats,
        rng: &mut R,
    ) -> PatternId {
        assert!(self.stats.num_weighted_compatible_patterns >= 1);
        assert!(self.stats.sum_compatible_pattern_weight >= 1);
        assert_eq!(
            self.sum_compatible_pattern_weight(global_stats),
            self.stats.sum_compatible_pattern_weight
        );

        let mut remaining = rng.gen_range(0, self.stats.sum_compatible_pattern_weight);
        for (pattern_id, pattern_stats) in
            self.weighted_compatible_stats_enumerate(global_stats)
        {
            if remaining >= pattern_stats.weight() {
                remaining -= pattern_stats.weight();
            } else {
                assert!(global_stats.pattern_stats(pattern_id).is_some());
                return pattern_id;
            }
        }
        unreachable!("The weight is positive and based on global_stats");
    }
    fn init<R: Rng>(&mut self, global_stats: &GlobalStats, rng: &mut R) {
        self.noise = rng.gen();
        self.num_compatible_patterns = global_stats.num_patterns() as u32;
        self.stats.num_weighted_compatible_patterns =
            global_stats.num_weighted_patterns();
        self.stats.sum_compatible_pattern_weight = global_stats.sum_pattern_weight();
        self.stats.sum_compatible_pattern_weight_log_weight =
            global_stats.sum_pattern_weight_log_weight();
        self.num_ways_to_become_each_pattern
            .resize(global_stats.num_patterns(), Default::default());
        self.num_ways_to_become_each_pattern
            .iter_mut()
            .zip(global_stats.num_ways_to_become_each_pattern_by_direction())
            .for_each(|(dst, src)| *dst = NumWaysToBecomePattern::new(src));
    }
}

#[derive(Clone)]
pub struct Wave {
    grid: Grid<WaveCell>,
}

impl Wave {
    pub fn new(size: Size) -> Self {
        Self {
            grid: Grid::new_default(size),
        }
    }
    fn init<R: Rng>(&mut self, global_stats: &GlobalStats, rng: &mut R) {
        self.grid
            .iter_mut()
            .for_each(|cell| cell.init(global_stats, rng));
    }
    pub fn grid(&self) -> &Grid<WaveCell> {
        &self.grid
    }
}

#[derive(Debug, Clone)]
struct RemovedPattern {
    coord: Coord,
    pattern_id: PatternId,
}

#[derive(Default, Clone)]
struct Propagator {
    removed_patterns_to_propagate: Vec<RemovedPattern>,
}

struct Contradiction;

impl Propagator {
    fn clear(&mut self) {
        self.removed_patterns_to_propagate.clear();
    }
    fn propagate<W: Wrap>(
        &mut self,
        wave: &mut Wave,
        global_stats: &GlobalStats,
        entropy_changes_by_coord: &mut HashMap<Coord, EntropyWithNoise>,
        num_cells_with_more_than_one_weighted_compatible_pattern: &mut u32,
    ) -> Result<(), Contradiction> {
        entropy_changes_by_coord.clear();
        let wave_size = wave.grid.size();
        while let Some(removed_pattern) = self.removed_patterns_to_propagate.pop() {
            for direction in CardinalDirections {
                let coord_to_update = if let Some(coord_to_update) = W::normalize_coord(
                    removed_pattern.coord + direction.coord(),
                    wave_size,
                ) {
                    coord_to_update
                } else {
                    continue;
                };
                let cell = wave.grid.get_checked_mut(coord_to_update);
                for &pattern_id in global_stats.compatible_patterns_in_direction(
                    removed_pattern.pattern_id,
                    direction,
                ) {
                    use self::DecrementNumWaysToBecomePattern as D;
                    match cell.decrement_num_ways_to_become_pattern(
                        pattern_id,
                        direction,
                        global_stats,
                    ) {
                        D::NoPatternRemoved => continue,
                        D::RemovedNonWeightedPattern => (),
                        D::RemovedWeightedPatternMultipleCandidatesRemain => {
                            let entropy = cell.entropy_with_noise();
                            entropy_changes_by_coord
                                .entry(coord_to_update)
                                .and_modify(|existing_entropy| {
                                    if entropy < *existing_entropy {
                                        *existing_entropy = entropy;
                                    }
                                })
                                .or_insert(entropy);
                        }
                        D::Finalized => {
                            *num_cells_with_more_than_one_weighted_compatible_pattern -=
                                1;
                            entropy_changes_by_coord.remove(&coord_to_update);
                        }
                        D::RemovedFinalCompatiblePattern => {
                            return Err(Contradiction);
                        }
                        D::RemovedFinalWeightedCompatiblePattern => {
                            entropy_changes_by_coord.remove(&coord_to_update);
                        }
                    }
                    self.removed_patterns_to_propagate.push(RemovedPattern {
                        coord: coord_to_update,
                        pattern_id,
                    });
                }
            }
        }
        Ok(())
    }
}

#[derive(PartialEq, Eq, Debug, Clone)]
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

#[derive(Default, Clone)]
struct Observer {
    entropy_priority_queue: BinaryHeap<CoordEntropy>,
}

#[derive(Debug)]
struct CellAtCoordMut<'a> {
    wave_cell: &'a mut WaveCell,
    coord: Coord,
}

impl<'a> CellAtCoordMut<'a> {
    fn remove_all_patterns_except_one(
        &mut self,
        pattern_id_to_keep: PatternId,
        global_stats: &GlobalStats,
        propagator: &mut Propagator,
    ) {
        for (pattern_id, num_ways_to_become_pattern) in self
            .wave_cell
            .num_ways_to_become_each_pattern
            .enumerate_mut()
        {
            if pattern_id != pattern_id_to_keep {
                if !num_ways_to_become_pattern.is_zero() {
                    num_ways_to_become_pattern.clear_all_directions();
                    assert!(self.wave_cell.num_compatible_patterns >= 1);
                    self.wave_cell.num_compatible_patterns -= 1;
                    if let Some(pattern_stats) = global_stats.pattern_stats(pattern_id) {
                        self.wave_cell
                            .stats
                            .remove_compatible_pattern(pattern_stats);
                    }
                    propagator
                        .removed_patterns_to_propagate
                        .push(RemovedPattern {
                            coord: self.coord,
                            pattern_id,
                        });
                }
            }
        }
    }
}

#[derive(Debug)]
enum ChooseNextCell<'a> {
    MinEntropyCell(CellAtCoordMut<'a>),
    NoCellsWithMultipleWeightedPatterns,
}

impl Observer {
    fn clear(&mut self) {
        self.entropy_priority_queue.clear();
    }
    fn choose_next_cell<'a>(&mut self, wave: &'a mut Wave) -> ChooseNextCell<'a> {
        while let Some(coord_entropy) = self.entropy_priority_queue.pop() {
            let index = wave
                .grid
                .index_of_coord(coord_entropy.coord)
                .expect("Coord out of bounds");
            let wave_cell = wave.grid.get_index_checked(index);
            if wave_cell.stats.num_weighted_compatible_patterns
                == coord_entropy
                    .entropy_with_noise
                    .num_weighted_compatible_patterns
                && wave_cell.num_compatible_patterns > 1
            {
                return ChooseNextCell::MinEntropyCell(CellAtCoordMut {
                    wave_cell: wave.grid.get_index_checked_mut(index),
                    coord: coord_entropy.coord,
                });
            }
        }
        ChooseNextCell::NoCellsWithMultipleWeightedPatterns
    }
}

#[derive(Default, Clone)]
pub struct Context {
    propagator: Propagator,
    entropy_changes_by_coord: HashMap<Coord, EntropyWithNoise>,
    observer: Observer,
    num_cells_with_more_than_one_weighted_compatible_pattern: u32,
}

#[derive(Debug)]
pub enum Observe {
    Incomplete,
    Complete,
}

#[derive(Debug)]
pub enum PropagateError {
    Contradiction,
}

struct WaveCellHandle<'a> {
    cell_at_coord_mut: CellAtCoordMut<'a>,
    propagator: &'a mut Propagator,
    global_stats: &'a GlobalStats,
}

impl<'a> WaveCellHandle<'a> {
    fn new(
        wave: &'a mut Wave,
        coord: Coord,
        propagator: &'a mut Propagator,
        global_stats: &'a GlobalStats,
    ) -> Self {
        let cell_at_coord_mut = CellAtCoordMut {
            wave_cell: wave.grid.get_checked_mut(coord),
            coord,
        };
        Self {
            cell_at_coord_mut,
            propagator,
            global_stats,
        }
    }
    fn forbid_all_patterns_except(&mut self, pattern_id: PatternId) {
        self.cell_at_coord_mut.remove_all_patterns_except_one(
            pattern_id,
            self.global_stats,
            &mut self.propagator,
        );
    }
    fn forbid_pattern(&mut self, pattern_id: PatternId) {
        if self
            .cell_at_coord_mut
            .wave_cell
            .num_ways_to_become_each_pattern[pattern_id]
            .is_zero()
        {
            return;
        }
        self.cell_at_coord_mut
            .wave_cell
            .num_ways_to_become_each_pattern[pattern_id]
            .clear_all_directions();
        self.cell_at_coord_mut.wave_cell.num_compatible_patterns -= 1;
        if let Some(pattern_stats) = self.global_stats.pattern_stats(pattern_id) {
            self.cell_at_coord_mut
                .wave_cell
                .stats
                .remove_compatible_pattern(pattern_stats);
        }
        self.propagator
            .removed_patterns_to_propagate
            .push(RemovedPattern {
                coord: self.cell_at_coord_mut.coord,
                pattern_id,
            });
    }
}

impl Context {
    pub fn new() -> Self {
        Default::default()
    }
    fn init(&mut self, wave: &Wave, global_stats: &GlobalStats) {
        self.propagator.clear();
        self.observer.clear();
        self.entropy_changes_by_coord.clear();
        if global_stats.num_weighted_patterns() > 1 {
            self.num_cells_with_more_than_one_weighted_compatible_pattern =
                wave.grid.size().count() as u32;
            wave.grid.enumerate().for_each(|(coord, cell)| {
                self.observer.entropy_priority_queue.push(CoordEntropy {
                    coord,
                    entropy_with_noise: cell.entropy_with_noise(),
                });
            });
        } else {
            self.num_cells_with_more_than_one_weighted_compatible_pattern = 0;
        }
    }
    fn propagate<W: Wrap>(
        &mut self,
        wave: &mut Wave,
        global_stats: &GlobalStats,
    ) -> Result<(), PropagateError> {
        self.propagator
            .propagate::<W>(
                wave,
                global_stats,
                &mut self.entropy_changes_by_coord,
                &mut self.num_cells_with_more_than_one_weighted_compatible_pattern,
            )
            .map_err(|_: Contradiction| PropagateError::Contradiction)?;
        for (coord, entropy_with_noise) in self.entropy_changes_by_coord.drain() {
            self.observer.entropy_priority_queue.push(CoordEntropy {
                coord,
                entropy_with_noise,
            });
        }
        Ok(())
    }
    fn observe<R: Rng>(
        &mut self,
        wave: &mut Wave,
        global_stats: &GlobalStats,
        rng: &mut R,
    ) -> Observe {
        if self.num_cells_with_more_than_one_weighted_compatible_pattern == 0 {
            return Observe::Complete;
        }
        let mut cell_at_coord = match self.observer.choose_next_cell(wave) {
            ChooseNextCell::NoCellsWithMultipleWeightedPatterns => {
                return Observe::Complete;
            }
            ChooseNextCell::MinEntropyCell(cell_at_coord) => cell_at_coord,
        };
        let pattern_id = cell_at_coord.wave_cell.choose_pattern_id(global_stats, rng);
        cell_at_coord.remove_all_patterns_except_one(
            pattern_id,
            &global_stats,
            &mut self.propagator,
        );
        self.num_cells_with_more_than_one_weighted_compatible_pattern -= 1;
        Observe::Incomplete
    }
}

pub trait ForbidPattern {
    fn forbid<W: Wrap, R: Rng>(&mut self, fi: &mut ForbidInterface<W>, rng: &mut R);
}

#[derive(Clone)]
pub struct ForbidNothing;
impl ForbidPattern for ForbidNothing {
    fn forbid<W: Wrap, R: Rng>(&mut self, _fi: &mut ForbidInterface<W>, _rng: &mut R) {}
}

pub struct ForbidRef<'a, F: ForbidPattern>(&'a mut F);
impl<'a, F: ForbidPattern> ForbidPattern for ForbidRef<'a, F> {
    fn forbid<W: Wrap, R: Rng>(&mut self, fi: &mut ForbidInterface<W>, rng: &mut R) {
        self.0.forbid(fi, rng);
    }
}

/// Represents a running instance of wfc which borrows its resources, making it
/// possible to re-use memory across multiple runs.
pub struct RunBorrow<'a, W: Wrap = WrapXY, F: ForbidPattern = ForbidNothing> {
    core: RunBorrowCore<'a, W>,
    forbid: F,
}

impl<'a> RunBorrow<'a> {
    pub fn new<R: Rng>(
        context: &'a mut Context,
        wave: &'a mut Wave,
        global_stats: &'a GlobalStats,
        rng: &mut R,
    ) -> Self {
        Self::new_wrap_forbid(context, wave, global_stats, WrapXY, ForbidNothing, rng)
    }
}

impl<'a, W: Wrap> RunBorrow<'a, W> {
    pub fn new_wrap<R: Rng>(
        context: &'a mut Context,
        wave: &'a mut Wave,
        global_stats: &'a GlobalStats,
        wrap: W,
        rng: &mut R,
    ) -> Self {
        Self::new_wrap_forbid(context, wave, global_stats, wrap, ForbidNothing, rng)
    }
}

impl<'a, F: ForbidPattern> RunBorrow<'a, WrapXY, F> {
    pub fn new_forbid<R: Rng>(
        context: &'a mut Context,
        wave: &'a mut Wave,
        global_stats: &'a GlobalStats,
        forbid: F,
        rng: &mut R,
    ) -> Self {
        Self::new_wrap_forbid(context, wave, global_stats, WrapXY, forbid, rng)
    }
}

impl<'a, W: Wrap, F: ForbidPattern> RunBorrow<'a, W, F> {
    pub fn new_wrap_forbid<R: Rng>(
        context: &'a mut Context,
        wave: &'a mut Wave,
        global_stats: &'a GlobalStats,
        wrap: W,
        mut forbid: F,
        rng: &mut R,
    ) -> Self {
        let mut core = RunBorrowCore::new(context, wave, global_stats, wrap, rng);
        forbid.forbid(&mut ForbidInterface(&mut core), rng);
        Self { core, forbid }
    }
}

struct RunBorrowCore<'a, W: Wrap = WrapXY> {
    context: &'a mut Context,
    wave: &'a mut Wave,
    global_stats: &'a GlobalStats,
    output_wrap: PhantomData<W>,
}

pub struct WaveCellRef<'a> {
    wave_cell: &'a WaveCell,
    global_stats: &'a GlobalStats,
}

pub enum WaveCellRefWeight {
    Weight(u32),
    SingleNonWeightedPattern,
}

pub struct MultipleWeightedPatternsEnumerateWeights<'a> {
    iter: iter::Enumerate<
        iter::Zip<
            slice::Iter<'a, NumWaysToBecomePattern>,
            OptionSliceIter<'a, PatternWeight>,
        >,
    >,
}

impl<'a> Iterator for MultipleWeightedPatternsEnumerateWeights<'a> {
    type Item = (PatternId, u32);
    fn next(&mut self) -> Option<Self::Item> {
        while let Some((pattern_id_usize, (num_ways_to_become_pattern, pattern_stats))) =
            self.iter.next()
        {
            if num_ways_to_become_pattern.is_zero() {
                continue;
            } else {
                let pattern_id = pattern_id_usize as PatternId;
                let weight =
                    match pattern_stats.map(|pattern_stats| pattern_stats.weight()) {
                        Some(weight) => weight,
                        None => 0,
                    };
                return Some((pattern_id, weight));
            }
        }
        None
    }
}

pub enum NoPatternsWithWeights {
    SingleCompatiblePattern(PatternId),
}

pub enum EnumerateCompatiblePatternWeights<'a> {
    CompatiblePatternsWithWeights(MultipleWeightedPatternsEnumerateWeights<'a>),
    SingleCompatiblePatternWithoutWeight(PatternId),
    NoCompatiblePattern,
    MultipleCompatiblePatternsWithoutWeights,
}

impl<'a> WaveCellRef<'a> {
    pub fn sum_compatible_pattern_weight(&self) -> u32 {
        self.wave_cell.stats.sum_compatible_pattern_weight
    }
    pub fn enumerate_compatible_pattern_weights(
        &self,
    ) -> EnumerateCompatiblePatternWeights {
        if self.wave_cell.num_compatible_patterns == 0 {
            return EnumerateCompatiblePatternWeights::NoCompatiblePattern;
        }
        if self.wave_cell.stats.num_weighted_compatible_patterns == 0 {
            if self.wave_cell.num_compatible_patterns == 1 {
                return EnumerateCompatiblePatternWeights::SingleCompatiblePatternWithoutWeight(
                    self.wave_cell.chosen_pattern_id().unwrap());
            } else {
                return EnumerateCompatiblePatternWeights::MultipleCompatiblePatternsWithoutWeights;
            }
        }
        let iter = self
            .wave_cell
            .num_ways_to_become_each_pattern
            .iter()
            .zip(self.global_stats.pattern_stats_option_iter())
            .enumerate();
        EnumerateCompatiblePatternWeights::CompatiblePatternsWithWeights(
            MultipleWeightedPatternsEnumerateWeights { iter },
        )
    }
}

impl<'a, W: Wrap, F: ForbidPattern> RunBorrow<'a, W, F> {
    pub fn reset<R: Rng>(&mut self, rng: &mut R) {
        self.core.reset(rng);
        self.forbid
            .forbid(&mut ForbidInterface(&mut self.core), rng);
    }

    pub fn step<R: Rng>(&mut self, rng: &mut R) -> Result<Observe, PropagateError> {
        let result = self.core.step(rng);
        if result.is_err() {
            self.reset(rng);
        }
        result
    }

    pub fn collapse<R: Rng>(&mut self, rng: &mut R) -> Result<(), PropagateError> {
        let result = self.core.collapse(rng);
        if result.is_err() {
            self.reset(rng);
        }
        result
    }

    pub fn wave_cell_ref(&self, coord: Coord) -> WaveCellRef {
        self.core.wave_cell_ref(coord)
    }

    pub fn wave_cell_ref_iter(&self) -> impl Iterator<Item = WaveCellRef> {
        self.core.wave_cell_ref_iter()
    }

    pub fn wave_cell_ref_enumerate(&self) -> impl Iterator<Item = (Coord, WaveCellRef)> {
        self.core.wave_cell_ref_enumerate()
    }

    pub fn collapse_retrying<R, RB>(&mut self, mut retry: RB, rng: &mut R) -> RB::Return
    where
        R: Rng,
        RB: retry::RetryBorrow,
    {
        retry.retry(self, rng)
    }
}

impl<'a, W: Wrap> RunBorrowCore<'a, W> {
    fn new<R: Rng>(
        context: &'a mut Context,
        wave: &'a mut Wave,
        global_stats: &'a GlobalStats,
        output_wrap: W,
        rng: &mut R,
    ) -> Self {
        let _ = output_wrap;
        wave.init(global_stats, rng);
        context.init(wave, global_stats);
        Self {
            context,
            wave,
            global_stats,
            output_wrap: PhantomData,
        }
    }

    fn reset<R: Rng>(&mut self, rng: &mut R) {
        self.wave.init(self.global_stats, rng);
        self.context.init(&self.wave, self.global_stats);
    }

    fn propagate(&mut self) -> Result<(), PropagateError> {
        self.context.propagate::<W>(self.wave, self.global_stats)
    }

    fn observe<R: Rng>(&mut self, rng: &mut R) -> Observe {
        self.context.observe(self.wave, self.global_stats, rng)
    }

    fn step<R: Rng>(&mut self, rng: &mut R) -> Result<Observe, PropagateError> {
        match self.observe(rng) {
            Observe::Complete => Ok(Observe::Complete),
            Observe::Incomplete => {
                self.propagate()?;
                Ok(Observe::Incomplete)
            }
        }
    }

    fn wave_cell_handle(&mut self, coord: Coord) -> WaveCellHandle {
        WaveCellHandle::new(
            self.wave,
            coord,
            &mut self.context.propagator,
            self.global_stats,
        )
    }

    fn forbid_all_patterns_except(
        &mut self,
        coord: Coord,
        pattern_id: PatternId,
    ) -> Result<(), PropagateError> {
        self.wave_cell_handle(coord)
            .forbid_all_patterns_except(pattern_id);
        self.propagate()
    }

    fn forbid_pattern(
        &mut self,
        coord: Coord,
        pattern_id: PatternId,
    ) -> Result<(), PropagateError> {
        self.wave_cell_handle(coord).forbid_pattern(pattern_id);
        self.propagate()
    }

    fn collapse<R: Rng>(&mut self, rng: &mut R) -> Result<(), PropagateError> {
        loop {
            match self.observe(rng) {
                Observe::Complete => return Ok(()),
                Observe::Incomplete => {
                    self.propagate()?;
                }
            }
        }
    }

    fn wave_cell_ref(&self, coord: Coord) -> WaveCellRef {
        let wave_cell = self.wave.grid.get_checked(coord);
        WaveCellRef {
            wave_cell,
            global_stats: self.global_stats,
        }
    }

    fn wave_cell_ref_iter(&self) -> impl Iterator<Item = WaveCellRef> {
        self.wave.grid.iter().map(move |wave_cell| WaveCellRef {
            wave_cell,
            global_stats: self.global_stats,
        })
    }

    fn wave_cell_ref_enumerate(&self) -> impl Iterator<Item = (Coord, WaveCellRef)> {
        self.wave.grid.enumerate().map(move |(coord, wave_cell)| {
            let wave_cell_ref = WaveCellRef {
                wave_cell,
                global_stats: self.global_stats,
            };
            (coord, wave_cell_ref)
        })
    }
}

pub struct ForbidInterface<'a, 'b, W: Wrap>(&'a mut RunBorrowCore<'b, W>);

impl<'a, 'b, W: Wrap> ForbidInterface<'a, 'b, W> {
    pub fn wave_size(&self) -> Size {
        self.0.wave.grid.size()
    }

    pub fn forbid_all_patterns_except<R: Rng>(
        &mut self,
        coord: Coord,
        pattern_id: PatternId,
        rng: &mut R,
    ) -> Result<(), PropagateError> {
        let result = self.0.forbid_all_patterns_except(coord, pattern_id);
        if result.is_err() {
            self.0.reset(rng);
        }
        result
    }

    pub fn forbid_pattern<R: Rng>(
        &mut self,
        coord: Coord,
        pattern_id: PatternId,
        rng: &mut R,
    ) -> Result<(), PropagateError> {
        let result = self.0.forbid_pattern(coord, pattern_id);
        if result.is_err() {
            self.0.reset(rng);
        }
        result
    }
}

#[derive(Clone)]
/// Represents a running instance of wfc which allocates and owns its resources
pub struct RunOwn<'a, W: Wrap = WrapXY, F: ForbidPattern = ForbidNothing> {
    context: Context,
    wave: Wave,
    global_stats: &'a GlobalStats,
    output_wrap: PhantomData<W>,
    forbid: F,
}

pub enum OwnedObserve<'a, W: Wrap> {
    Complete(Wave),
    Incomplete(RunOwn<'a, W>),
}

pub enum OwnedPropagateError<'a, W: Wrap> {
    Contradiction(RunOwn<'a, W>),
}

impl<'a> RunOwn<'a> {
    pub fn new<R: Rng>(
        output_size: Size,
        global_stats: &'a GlobalStats,
        rng: &mut R,
    ) -> Self {
        Self::new_wrap_forbid(output_size, global_stats, WrapXY, ForbidNothing, rng)
    }
}

impl<'a, W: Wrap> RunOwn<'a, W> {
    pub fn new_wrap<R: Rng>(
        output_size: Size,
        global_stats: &'a GlobalStats,
        wrap: W,
        rng: &mut R,
    ) -> Self {
        Self::new_wrap_forbid(output_size, global_stats, wrap, ForbidNothing, rng)
    }
}

impl<'a, F: ForbidPattern> RunOwn<'a, WrapXY, F>
where
    F: Clone + Sync + Send,
{
    pub fn new_forbid<R: Rng>(
        output_size: Size,
        global_stats: &'a GlobalStats,
        forbid: F,
        rng: &mut R,
    ) -> Self {
        Self::new_wrap_forbid(output_size, global_stats, WrapXY, forbid, rng)
    }
}

impl<'a, W: Wrap, F: ForbidPattern> RunOwn<'a, W, F>
where
    F: Clone + Sync + Send,
{
    pub fn new_wrap_forbid<R: Rng>(
        output_size: Size,
        global_stats: &'a GlobalStats,
        wrap: W,
        forbid: F,
        rng: &mut R,
    ) -> Self {
        let _ = wrap;
        let wave = Wave::new(output_size);
        let context = Context::new();
        let mut s = Self {
            context,
            wave,
            global_stats,
            output_wrap: PhantomData,
            forbid,
        };
        s.borrow_mut().reset(rng);
        s
    }
}

impl<'a, W: Wrap, F: ForbidPattern> RunOwn<'a, W, F>
where
    F: Clone + Sync + Send,
{
    fn borrow_mut(&mut self) -> RunBorrow<W, ForbidRef<F>> {
        let core = RunBorrowCore {
            context: &mut self.context,
            wave: &mut self.wave,
            global_stats: self.global_stats,
            output_wrap: self.output_wrap,
        };
        RunBorrow {
            core,
            forbid: ForbidRef(&mut self.forbid),
        }
    }

    pub fn step<R: Rng>(&mut self, rng: &mut R) -> Result<Observe, PropagateError> {
        self.borrow_mut().step(rng)
    }

    pub fn collapse<R: Rng>(&mut self, rng: &mut R) -> Result<(), PropagateError> {
        self.borrow_mut().collapse(rng)
    }

    pub fn wave_cell_ref(&self, coord: Coord) -> WaveCellRef {
        let wave_cell = self.wave.grid.get_checked(coord);
        WaveCellRef {
            wave_cell,
            global_stats: self.global_stats,
        }
    }

    pub fn wave_cell_ref_iter(&self) -> impl Iterator<Item = WaveCellRef> {
        self.wave.grid.iter().map(move |wave_cell| WaveCellRef {
            wave_cell,
            global_stats: self.global_stats,
        })
    }

    pub fn wave_cell_ref_enumerate(&self) -> impl Iterator<Item = (Coord, WaveCellRef)> {
        self.wave.grid.enumerate().map(move |(coord, wave_cell)| {
            let wave_cell_ref = WaveCellRef {
                wave_cell,
                global_stats: self.global_stats,
            };
            (coord, wave_cell_ref)
        })
    }

    pub fn into_wave(self) -> Wave {
        self.wave
    }

    pub fn collapse_retrying<R, RO>(self, mut retry: RO, rng: &mut R) -> RO::Return
    where
        R: Rng + Send + Sync + Clone,
        RO: retry::RetryOwn,
    {
        retry.retry(self, rng)
    }
}
