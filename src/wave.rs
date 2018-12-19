use coord_2d::{Coord, Size};
use grid_2d::Grid;
use pattern::{GlobalStats, PatternId, PatternStats, PatternTable};
use rand::Rng;
use std::cmp::Ordering;

pub enum WaveCellState {
    Undecided,
    Decided,
    NoCompatiblePatterns,
    AllCompatiblePatternsHaveZeroProbability,
}

#[derive(PartialEq, Debug, Clone, Copy)]
pub struct EntropyWithNoise {
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
        self.num_possible_patterns -= 1;
        self.sum_possible_pattern_count -= pattern_stats.count();
        self.sum_possible_pattern_count_log_count -= pattern_stats.count_log_count();
    }
    fn entropy(&self) -> f32 {
        // log(n0+n1+n2+...) - (n0*log(n0) + n1*log(n1) + n2*log(n2) + ...) / (n0+n1+n2+...)
        let sum_possible_pattern_count = self.sum_possible_pattern_count as f32;
        sum_possible_pattern_count.log2()
            - (self.sum_possible_pattern_count_log_count / sum_possible_pattern_count)
    }
    fn is_undecided(&self) -> bool {
        self.num_possible_patterns > 1 && self.sum_possible_pattern_count != 0
    }
    fn is_undecided_or_all_compatible_patterns_have_zero_probability(&self) -> bool {
        self.num_possible_patterns > 1
    }
    fn state(&self) -> WaveCellState {
        if self.num_possible_patterns == 0 {
            WaveCellState::NoCompatiblePatterns
        } else if self.sum_possible_pattern_count == 0 {
            WaveCellState::AllCompatiblePatternsHaveZeroProbability
        } else if self.num_possible_patterns == 1 {
            WaveCellState::Decided
        } else {
            WaveCellState::Undecided
        }
    }
}

#[derive(Debug, Default)]
pub struct WaveCell {
    possible_pattern_ids: PatternTable<bool>,
    metadata: WaveCellMetadata,
    noise: u32,
}

pub struct PossiblePattern<'a> {
    is_possible: &'a mut bool,
    pattern_id: PatternId,
    metadata: &'a mut WaveCellMetadata,
}

impl<'a> PossiblePattern<'a> {
    pub fn remove(self, global_stats: &GlobalStats) {
        *self.is_possible = false;
        self.metadata
            .remove_possible_pattern(global_stats.pattern_stats(self.pattern_id));
    }
    pub fn pattern_id(&self) -> PatternId {
        self.pattern_id
    }
}

impl WaveCell {
    pub fn init<R: Rng>(&mut self, global_stats: &GlobalStats, rng: &mut R) {
        self.noise = rng.gen();
        self.metadata.num_possible_patterns = global_stats.num_patterns() as u32;
        self.metadata.sum_possible_pattern_count = global_stats.sum_pattern_count();
        self.metadata.sum_possible_pattern_count_log_count =
            global_stats.sum_pattern_count_log_count();
        self.possible_pattern_ids = (0..self.metadata.num_possible_patterns)
            .map(|_| true)
            .collect();
    }
    pub fn is_pattern_compatible(&self, pattern_id: PatternId) -> bool {
        self.possible_pattern_ids[pattern_id]
    }
    pub fn possible_pattern(&mut self, pattern_id: PatternId) -> Option<PossiblePattern> {
        let is_possible = &mut self.possible_pattern_ids[pattern_id];
        if *is_possible {
            Some(PossiblePattern {
                is_possible,
                pattern_id,
                metadata: &mut self.metadata,
            })
        } else {
            None
        }
    }
    pub fn for_each_possible_pattern<F>(&mut self, mut f: F)
    where
        F: FnMut(PossiblePattern),
    {
        for (pattern_id, is_possible) in self.possible_pattern_ids.iter_mut().enumerate()
        {
            if *is_possible {
                f(PossiblePattern {
                    is_possible,
                    pattern_id: pattern_id as PatternId,
                    metadata: &mut self.metadata,
                });
            }
        }
    }
    pub fn chosen_pattern_id(&self) -> Option<PatternId> {
        self.possible_pattern_ids
            .iter()
            .position(Clone::clone)
            .map(|index| index as PatternId)
    }
    pub fn choose_pattern_id<R: Rng>(
        &self,
        global_stats: &GlobalStats,
        rng: &mut R,
    ) -> PatternId {
        assert!(self.metadata.is_undecided(), "Cell is not undecided");
        let mut remaining = rng.gen_range(0, self.metadata.sum_possible_pattern_count);
        for (pattern_id, pattern_stats) in self.possible_pattern_ids
            .iter()
            .zip(global_stats.pattern_stats_iter().enumerate())
            .filter_map(|(&is_possible, pattern_stats)| {
                if is_possible {
                    Some(pattern_stats)
                } else {
                    None
                }
            }) {
            if pattern_stats.count() == 0 {
                continue;
            } else if remaining > pattern_stats.count() {
                remaining -= pattern_stats.count();
            } else {
                return pattern_id as PatternId;
            }
        }
        unreachable!("If the cell is undecided, the loop will choose a pattern id");
    }
    pub fn entropy_with_noise(&self) -> EntropyWithNoise {
        if self.metadata.num_possible_patterns <= 1 {
            panic!("{:#?}", self);
        }
        let entropy = self.metadata.entropy();
        let noise = self.noise;
        EntropyWithNoise { entropy, noise }
    }
    pub fn state(&self) -> WaveCellState {
        self.metadata.state()
    }
    pub fn is_undecided_or_all_compatible_patterns_have_zero_probability(&self) -> bool {
        self.metadata
            .is_undecided_or_all_compatible_patterns_have_zero_probability()
    }
    pub fn is_undecided(&self) -> bool {
        self.metadata.is_undecided()
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
    pub fn get_checked(&self, coord: Coord) -> &WaveCell {
        self.grid.get_checked(coord)
    }
    pub fn get_checked_mut(&mut self, coord: Coord) -> &mut WaveCell {
        self.grid.get_checked_mut(coord)
    }
    pub fn size(&self) -> Size {
        self.grid.size()
    }
    pub fn enumerate_mut(&mut self) -> impl Iterator<Item = (Coord, &mut WaveCell)> {
        self.grid.enumerate_mut()
    }
}
