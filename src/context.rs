use coord_2d::{Coord, Size};
use direction::{CardinalDirection, CardinalDirectionTable, CardinalDirections};
use grid_2d::Grid;
use hashbrown::{HashMap, HashSet};
use pattern::{GlobalStats, PatternId, PatternStats, PatternTable};
use rand::Rng;
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::marker::PhantomData;
use wrap::OutputWrap;

#[derive(Default, Debug)]
struct WaveCellStats {
    num_weighted_compatible_patterns: u32,
    // n0 + n1 + n2 + ...
    sum_compatible_pattern_weight: u32,
    // n0*log(n0) + n1*log(n1) + n2*log(n2) + ...
    sum_compatible_pattern_weight_log_weight: f32,
}

impl WaveCellStats {
    fn remove_compatible_pattern(&mut self, pattern_stats: &PatternStats) {
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

#[derive(Default, Debug)]
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
pub struct EntropyWithNoise {
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

impl WaveCell {
    pub fn first_compatible_pattern_id(&self) -> Option<PatternId> {
        if self.stats.num_weighted_compatible_patterns > 1 {
            return None;
        }
        self.num_ways_to_become_each_pattern
            .enumerate()
            .filter_map(|(pattern_id, num_ways_to_become_pattern)| {
                if num_ways_to_become_pattern.is_zero() {
                    None
                } else {
                    Some(pattern_id)
                }
            })
            .next()
    }
    fn compatible_pattern_ids(&self) -> Vec<PatternId> {
        self.num_ways_to_become_each_pattern
            .iter()
            .enumerate()
            .filter_map(|(pattern_id_usize, num_ways_to_become_pattern)| {
                if num_ways_to_become_pattern.is_zero() {
                    None
                } else {
                    Some(pattern_id_usize as PatternId)
                }
            })
            .collect()
    }
    fn weighted_compatible_stats_enumerate<'a>(
        &'a self,
        global_stats: &'a GlobalStats,
    ) -> impl Iterator<Item = (PatternId, &'a PatternStats)> {
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
                        non_zero => {
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
            if remaining > pattern_stats.weight() {
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

pub struct Wave {
    grid: Grid<WaveCell>,
}

impl Wave {
    pub fn new(size: Size) -> Self {
        Self {
            grid: Grid::new_default(size),
        }
    }
    pub fn size(&self) -> Size {
        self.grid.size()
    }
    fn init<R: Rng>(&mut self, global_stats: &GlobalStats, rng: &mut R) {
        self.grid
            .iter_mut()
            .for_each(|cell| cell.init(global_stats, rng));
    }
    pub fn get_checked(&self, coord: Coord) -> &WaveCell {
        self.grid.get_checked(coord)
    }
}

#[derive(Debug)]
struct RemovedPattern {
    coord: Coord,
    pattern_id: PatternId,
}

#[derive(Default)]
struct Propagator {
    removed_patterns_to_propagate: Vec<RemovedPattern>,
}

struct Contradiction;

impl Propagator {
    fn clear(&mut self) {
        self.removed_patterns_to_propagate.clear();
    }
    fn propagate<W: OutputWrap>(
        &mut self,
        wave: &mut Wave,
        global_stats: &GlobalStats,
        entropy_changes_by_coord: &mut HashMap<Coord, EntropyWithNoise>,
        num_cells_with_more_than_one_weighted_compatible_pattern: &mut u32,
        soft_contradicting_coords: &mut HashSet<Coord>,
    ) -> Result<(), Contradiction> {
        entropy_changes_by_coord.clear();
        let wave_size = wave.size();
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
                            // no way to recover from this type of contradiction
                            return Err(Contradiction);
                        }
                        D::RemovedFinalWeightedCompatiblePattern => {
                            // it's posible to recover from this by manually setting the pattern
                            entropy_changes_by_coord.remove(&coord_to_update);
                            soft_contradicting_coords.insert(coord_to_update);
                        }
                    }
                    self.removed_patterns_to_propagate
                        .push(RemovedPattern {
                            coord: coord_to_update,
                            pattern_id,
                        });
                }
            }
        }
        Ok(())
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

#[derive(Default)]
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
        for (pattern_id, num_ways_to_become_pattern) in self.wave_cell
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
            let index = wave.grid
                .index_of_coord(coord_entropy.coord)
                .expect("Coord out of bounds");
            if wave.grid[index]
                .stats
                .num_weighted_compatible_patterns
                == coord_entropy
                    .entropy_with_noise
                    .num_weighted_compatible_patterns
                && wave.grid[index].num_compatible_patterns > 1
            {
                return ChooseNextCell::MinEntropyCell(CellAtCoordMut {
                    wave_cell: &mut wave.grid[index],
                    coord: coord_entropy.coord,
                });
            }
        }
        ChooseNextCell::NoCellsWithMultipleWeightedPatterns
    }
}

#[derive(Default)]
pub struct Context {
    propagator: Propagator,
    entropy_changes_by_coord: HashMap<Coord, EntropyWithNoise>,
    observer: Observer,
    num_cells_with_more_than_one_weighted_compatible_pattern: u32,
    soft_contradicting_coords: HashSet<Coord>,
}

pub enum Progress {
    Incomplete,
    Complete,
}

#[derive(Debug)]
pub enum ChoosePatternError {
    IncompatiblePattern,
}

#[derive(Debug)]
pub enum ForbidPattern {
    AlreadyForbidden,
    Done,
}

#[derive(Debug)]
pub enum ForbidPatternError {
    WouldCauseContradiction,
}

#[derive(Debug)]
pub enum StepError {
    Contradiction,
}

impl Context {
    pub fn new() -> Self {
        Default::default()
    }
    fn init(&mut self, wave: &Wave, global_stats: &GlobalStats) {
        self.propagator.clear();
        self.observer.clear();
        self.entropy_changes_by_coord.clear();
        self.soft_contradicting_coords.clear();
        if global_stats.num_weighted_patterns() > 1 {
            self.num_cells_with_more_than_one_weighted_compatible_pattern =
                wave.size().count() as u32;
            wave.grid.enumerate().for_each(|(coord, cell)| {
                self.observer
                    .entropy_priority_queue
                    .push(CoordEntropy {
                        coord,
                        entropy_with_noise: cell.entropy_with_noise(),
                    });
            });
        } else {
            self.num_cells_with_more_than_one_weighted_compatible_pattern = 0;
        }
    }
    fn propagate<W: OutputWrap>(
        &mut self,
        wave: &mut Wave,
        global_stats: &GlobalStats,
    ) -> Result<(), Contradiction> {
        self.propagator.propagate::<W>(
            wave,
            global_stats,
            &mut self.entropy_changes_by_coord,
            &mut self.num_cells_with_more_than_one_weighted_compatible_pattern,
            &mut self.soft_contradicting_coords,
        )?;
        for (coord, entropy_with_noise) in self.entropy_changes_by_coord.drain() {
            self.observer
                .entropy_priority_queue
                .push(CoordEntropy {
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
    ) -> Progress {
        if self.num_cells_with_more_than_one_weighted_compatible_pattern == 0 {
            return Progress::Complete;
        }
        let mut cell_at_coord = match self.observer.choose_next_cell(wave) {
            ChooseNextCell::NoCellsWithMultipleWeightedPatterns => {
                return Progress::Complete
            }
            ChooseNextCell::MinEntropyCell(cell_at_coord) => cell_at_coord,
        };
        let pattern_id = cell_at_coord
            .wave_cell
            .choose_pattern_id(global_stats, rng);
        cell_at_coord.remove_all_patterns_except_one(
            pattern_id,
            &global_stats,
            &mut self.propagator,
        );
        self.num_cells_with_more_than_one_weighted_compatible_pattern -= 1;
        Progress::Incomplete
    }
    fn step<W: OutputWrap, R: Rng>(
        &mut self,
        wave: &mut Wave,
        global_stats: &GlobalStats,
        rng: &mut R,
    ) -> Result<Progress, StepError> {
        self.propagate::<W>(wave, global_stats)
            .map_err(|_: Contradiction| StepError::Contradiction)?;
        Ok(self.observe(wave, global_stats, rng))
    }
    fn choose_pattern(
        &mut self,
        coord: Coord,
        wave: &mut Wave,
        pattern_id: PatternId,
        global_stats: &GlobalStats,
    ) -> Result<(), ChoosePatternError> {
        let wave_cell = wave.grid.get_checked_mut(coord);
        if wave_cell.num_ways_to_become_each_pattern[pattern_id].is_zero() {
            return Err(ChoosePatternError::IncompatiblePattern);
        }
        CellAtCoordMut { wave_cell, coord }.remove_all_patterns_except_one(
            pattern_id,
            global_stats,
            &mut self.propagator,
        );
        Ok(())
    }
    fn forbid_pattern(
        &mut self,
        coord: Coord,
        wave: &mut Wave,
        pattern_id: PatternId,
        global_stats: &GlobalStats,
    ) -> Result<ForbidPattern, ForbidPatternError> {
        let wave_cell = wave.grid.get_checked_mut(coord);
        if wave_cell.num_ways_to_become_each_pattern[pattern_id].is_zero() {
            return Ok(ForbidPattern::AlreadyForbidden);
        }
        if wave_cell.num_compatible_patterns == 1 {
            return Err(ForbidPatternError::WouldCauseContradiction);
        }
        wave_cell.num_ways_to_become_each_pattern[pattern_id].clear_all_directions();
        wave_cell.num_compatible_patterns -= 1;
        if let Some(pattern_stats) = global_stats.pattern_stats(pattern_id) {
            wave_cell
                .stats
                .remove_compatible_pattern(pattern_stats);
        }
        self.propagator
            .removed_patterns_to_propagate
            .push(RemovedPattern {
                coord,
                pattern_id,
            });
        Ok(ForbidPattern::Done)
    }
    pub fn run<'a, W: OutputWrap, R: Rng>(
        &'a mut self,
        wave: &'a mut Wave,
        global_stats: &'a GlobalStats,
        rng: &mut R,
    ) -> Run<W> {
        Run::new(self, wave, global_stats, rng)
    }
}

pub struct Run<'a, W: OutputWrap> {
    context: &'a mut Context,
    wave: &'a mut Wave,
    global_stats: &'a GlobalStats,
    output_wrap: PhantomData<W>,
}

impl<'a, W: OutputWrap> Run<'a, W> {
    pub fn step<R: Rng>(&mut self, rng: &mut R) -> Result<Progress, StepError> {
        self.context
            .step::<W, _>(self.wave, self.global_stats, rng)
    }
    pub fn choose_pattern(
        &mut self,
        coord: Coord,
        pattern_id: PatternId,
    ) -> Result<(), ChoosePatternError> {
        self.context
            .choose_pattern(coord, &mut self.wave, pattern_id, self.global_stats)
    }
    pub fn forbid_pattern(
        &mut self,
        coord: Coord,
        pattern_id: PatternId,
    ) -> Result<ForbidPattern, ForbidPatternError> {
        self.context
            .forbid_pattern(coord, &mut self.wave, pattern_id, self.global_stats)
    }

    pub fn new<R: Rng>(
        context: &'a mut Context,
        wave: &'a mut Wave,
        global_stats: &'a GlobalStats,
        rng: &mut R,
    ) -> Self {
        wave.init(global_stats, rng);
        context.init(wave, global_stats);
        Self {
            context,
            wave,
            global_stats,
            output_wrap: PhantomData,
        }
    }
}
