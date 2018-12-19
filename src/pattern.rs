use direction::{CardinalDirection, CardinalDirectionTable};
use std::iter;
use std::ops::{Index, IndexMut};
use std::slice;

pub type PatternId = u32;

#[derive(Default, Clone, Debug)]
pub struct PatternTable<T> {
    table: Vec<T>,
}

impl<T> PatternTable<T> {
    pub fn len(&self) -> usize {
        self.table.len()
    }
    pub fn iter(&self) -> slice::Iter<T> {
        self.table.iter()
    }
    pub fn iter_mut(&mut self) -> slice::IterMut<T> {
        self.table.iter_mut()
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

#[derive(Debug)]
pub struct PatternStats {
    count: u32,
    count_log_count: f32,
}

impl PatternStats {
    pub fn new(count: u32) -> Self {
        Self {
            count,
            count_log_count: if count == 0 {
                0.
            } else {
                (count as f32) * (count as f32).log2()
            },
        }
    }
    pub fn count(&self) -> u32 {
        self.count
    }
    pub fn count_log_count(&self) -> f32 {
        self.count_log_count
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
    pub fn sum_pattern_count(&self) -> u32 {
        self.sum_pattern_count
    }
    pub fn sum_pattern_count_log_count(&self) -> f32 {
        self.sum_pattern_count_log_count
    }
    pub fn num_patterns(&self) -> usize {
        self.stats_per_pattern.len()
    }
    pub fn pattern_stats_iter(&self) -> slice::Iter<PatternStats> {
        self.stats_per_pattern.iter()
    }
    pub fn pattern_stats(&self, pattern_id: PatternId) -> &PatternStats {
        &self.stats_per_pattern[pattern_id]
    }
    pub fn compatible_patterns_in_direction(
        &self,
        pattern_id: PatternId,
        direction: CardinalDirection,
    ) -> slice::Iter<PatternId> {
        self.compatibility_per_pattern[pattern_id]
            .get(direction)
            .iter()
    }
    pub fn compatible_patterns_by_direction(
        &self,
    ) -> slice::Iter<CardinalDirectionTable<Vec<PatternId>>> {
        self.compatibility_per_pattern.iter()
    }
}
