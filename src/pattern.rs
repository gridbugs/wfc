use direction::{CardinalDirection, CardinalDirectionTable, CardinalDirections};
use std::iter;
use std::num::NonZeroU32;
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
    pub fn resize(&mut self, size: usize, value: T) {
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

pub struct PatternStats {
    weight: NonZeroU32,
    weight_log_weight: f32,
}

impl PatternStats {
    pub fn from_u32_weight(weight: u32) -> Option<Self> {
        NonZeroU32::new(weight).map(|weight| Self {
            weight,
            weight_log_weight: (weight.get() as f32) * (weight.get() as f32).log2(),
        })
    }
    pub fn weight(&self) -> u32 {
        self.weight.get()
    }
    pub fn weight_log_weight(&self) -> f32 {
        self.weight_log_weight
    }
}

pub struct GlobalStats {
    stats_per_pattern: PatternTable<Option<PatternStats>>,
    compatibility_per_pattern: PatternTable<CardinalDirectionTable<Vec<PatternId>>>,
    num_weighted_patterns: u32,
    sum_pattern_weight: u32,
    sum_pattern_weight_log_weight: f32,
}

pub struct NumWaysToBecomeEachPatternByDirection<'a> {
    iter: slice::Iter<'a, CardinalDirectionTable<Vec<PatternId>>>,
}

impl<'a> Iterator for NumWaysToBecomeEachPatternByDirection<'a> {
    type Item = CardinalDirectionTable<u32>;
    fn next(&mut self) -> Option<Self::Item> {
        self.iter
            .next()
            .map(|compatible_patterns_by_direction| {
                let mut num_ways_to_become_pattern_from_direction =
                    CardinalDirectionTable::default();
                for direction in CardinalDirections {
                    *num_ways_to_become_pattern_from_direction.get_mut(direction) =
                        compatible_patterns_by_direction
                            .get(direction.opposite())
                            .len() as u32;
                }

                num_ways_to_become_pattern_from_direction
            })
    }
}

impl GlobalStats {
    pub fn new(
        stats_per_pattern: PatternTable<Option<PatternStats>>,
        compatibility_per_pattern: PatternTable<CardinalDirectionTable<Vec<PatternId>>>,
    ) -> Self {
        let num_weighted_patterns = stats_per_pattern
            .iter()
            .filter(|p| p.is_some())
            .count() as u32;
        let sum_pattern_weight = stats_per_pattern
            .iter()
            .filter_map(|p| p.as_ref().map(|p| p.weight()))
            .sum();
        let sum_pattern_weight_log_weight = stats_per_pattern
            .iter()
            .filter_map(|p| p.as_ref().map(|p| p.weight_log_weight()))
            .sum();
        Self {
            stats_per_pattern,
            compatibility_per_pattern,
            num_weighted_patterns,
            sum_pattern_weight,
            sum_pattern_weight_log_weight,
        }
    }
    pub fn num_weighted_patterns(&self) -> u32 {
        self.num_weighted_patterns
    }
    pub fn sum_pattern_weight(&self) -> u32 {
        self.sum_pattern_weight
    }
    pub fn sum_pattern_weight_log_weight(&self) -> f32 {
        self.sum_pattern_weight_log_weight
    }
    pub fn num_patterns(&self) -> usize {
        self.stats_per_pattern.len()
    }
    pub fn pattern_stats(&self, pattern_id: PatternId) -> Option<&PatternStats> {
        self.stats_per_pattern[pattern_id].as_ref()
    }
    pub fn pattern_stats_option_iter(
        &self,
    ) -> impl Iterator<Item = Option<&PatternStats>> {
        self.stats_per_pattern.iter().map(|o| o.as_ref())
    }
    pub fn compatible_patterns_in_direction(
        &self,
        pattern_id: PatternId,
        direction: CardinalDirection,
    ) -> impl Iterator<Item = &PatternId> {
        self.compatibility_per_pattern[pattern_id]
            .get(direction)
            .iter()
    }
    pub fn compatible_patterns_by_direction(
        &self,
    ) -> slice::Iter<CardinalDirectionTable<Vec<PatternId>>> {
        self.compatibility_per_pattern.iter()
    }
    pub fn num_ways_to_become_each_pattern_by_direction(
        &self,
    ) -> NumWaysToBecomeEachPatternByDirection {
        NumWaysToBecomeEachPatternByDirection {
            iter: self.compatible_patterns_by_direction(),
        }
    }
}
