use coord_2d::{Coord, Size};
use direction::{CardinalDirection, CardinalDirectionTable, CardinalDirections};
use grid_2d::coord_system::XThenYIter;
use grid_2d::Grid;
use hashbrown::HashMap;
use orientation::{self, Orientation, OrientationTable};
use std::hash::Hash;
use std::num::NonZeroU32;
use tiled_slice::TiledGridSlice;
use wfc::{GlobalStats, PatternDescription, PatternId, PatternTable};

fn are_patterns_compatible<T: PartialEq>(
    a: &TiledGridSlice<T>,
    b: &TiledGridSlice<T>,
    b_offset_direction: CardinalDirection,
) -> bool {
    let size = a.size();
    assert!(size == b.size());
    let axis = b_offset_direction.axis();
    let compare_size = size.with_axis(axis, |d| d - 1);
    let (a_offset, b_offset) = match b_offset_direction {
        CardinalDirection::North => (Coord::new(0, 0), Coord::new(0, 1)),
        CardinalDirection::South => (Coord::new(0, 1), Coord::new(0, 0)),
        CardinalDirection::East => (Coord::new(1, 0), Coord::new(0, 0)),
        CardinalDirection::West => (Coord::new(0, 0), Coord::new(1, 0)),
    };
    let coords = || XThenYIter::new(compare_size);
    let a_iter = coords().map(|c| a.get_checked(c + a_offset));
    let b_iter = coords().map(|c| b.get_checked(c + b_offset));
    a_iter.zip(b_iter).all(|(a, b)| a == b)
}

#[derive(Debug)]
pub struct Pattern {
    coords: Vec<Coord>,
    count: u32,
    orientation: Orientation,
}

impl Pattern {
    fn new(orientation: Orientation) -> Self {
        Self {
            coords: Vec::new(),
            count: 0,
            orientation,
        }
    }
    fn tiled_grid_slice<'a, T>(
        &self,
        grid: &'a Grid<T>,
        size: Size,
    ) -> TiledGridSlice<'a, T> {
        TiledGridSlice::new(grid, self.coord(), size, self.orientation)
    }
    pub fn coord(&self) -> Coord {
        self.coords[0]
    }
    pub fn clear_count(&mut self) {
        self.count = 0;
    }
}

pub struct OverlappingPatterns<T: Eq + Clone + Hash> {
    pattern_table: PatternTable<Pattern>,
    pattern_size: Size,
    grid: Grid<T>,
}

impl<T: Eq + Clone + Hash> OverlappingPatterns<T> {
    pub fn new(grid: Grid<T>, pattern_size: Size, orientations: &[Orientation]) -> Self {
        let pattern_table = {
            let mut pattern_map = HashMap::new();
            for &orientation in orientations.iter() {
                for coord in XThenYIter::new(grid.size()) {
                    let pattern_slice =
                        TiledGridSlice::new(&grid, coord, pattern_size, orientation);
                    let pattern = pattern_map
                        .entry(pattern_slice.clone())
                        .or_insert_with(|| Pattern::new(orientation));
                    pattern.coords.push(pattern_slice.offset());
                    pattern.count += 1;
                }
            }
            let mut patterns = pattern_map
                .drain()
                .map(|(_, pattern)| pattern)
                .collect::<Vec<_>>();
            patterns.sort_by_key(|pattern| pattern.coord());
            PatternTable::from_vec(patterns)
        };
        Self {
            pattern_table,
            pattern_size,
            grid,
        }
    }
    pub fn new_all_orientations(grid: Grid<T>, pattern_size: Size) -> Self {
        Self::new(grid, pattern_size, &orientation::ALL)
    }
    pub fn new_original_orientation(grid: Grid<T>, pattern_size: Size) -> Self {
        Self::new(grid, pattern_size, &[Orientation::Original])
    }
    pub fn grid(&self) -> &Grid<T> {
        &self.grid
    }
    pub fn pattern(&self, pattern_id: PatternId) -> &Pattern {
        &self.pattern_table[pattern_id]
    }
    pub fn pattern_mut(&mut self, pattern_id: PatternId) -> &mut Pattern {
        &mut self.pattern_table[pattern_id]
    }
    pub fn pattern_top_left_value(&self, pattern_id: PatternId) -> &T {
        let pattern = self.pattern(pattern_id);
        let tiled_grid_slice = pattern.tiled_grid_slice(&self.grid, self.pattern_size);
        tiled_grid_slice.get_checked(Coord::new(0, 0))
    }
    pub fn id_grid(&self) -> Grid<OrientationTable<PatternId>> {
        let empty: OrientationTable<PatternId> = OrientationTable::new();
        let mut maybe_id_grid = Grid::new_clone(self.grid.size(), empty);
        self.pattern_table
            .iter()
            .enumerate()
            .for_each(|(pattern_id_usize, pattern)| {
                pattern.coords.iter().for_each(|&coord| {
                    maybe_id_grid
                        .get_checked_mut(coord)
                        .insert(pattern.orientation, pattern_id_usize as PatternId);
                });
            });
        Grid::new_fn(self.grid.size(), |coord| {
            maybe_id_grid.get_checked(coord).clone()
        })
    }
    pub fn id_grid_original_orientation(&self) -> Grid<PatternId> {
        let id_grid = self.id_grid();
        Grid::new_fn(id_grid.size(), |coord| {
            id_grid
                .get_checked(coord)
                .get(Orientation::Original)
                .expect("Missing original orientation")
                .clone()
        })
    }
    fn compatible_patterns<'b>(
        &'b self,
        pattern: &'b Pattern,
        direction: CardinalDirection,
    ) -> impl 'b + Iterator<Item = PatternId> {
        let tiled_grid_slice = pattern.tiled_grid_slice(&self.grid, self.pattern_size);
        self.pattern_table
            .enumerate()
            .filter(move |(_id, other)| {
                let other_tiled_grid_slice =
                    other.tiled_grid_slice(&self.grid, self.pattern_size);
                are_patterns_compatible(
                    &tiled_grid_slice,
                    &other_tiled_grid_slice,
                    direction,
                )
            })
            .map(|(id, _other)| id)
    }
    pub fn pattern_descriptions(&self) -> PatternTable<PatternDescription> {
        self.pattern_table
            .iter()
            .map(|pattern| {
                let weight = NonZeroU32::new(pattern.count);
                let mut allowed_neighbours = CardinalDirectionTable::default();
                for direction in CardinalDirections {
                    allowed_neighbours[direction] = self
                        .compatible_patterns(pattern, direction)
                        .collect::<Vec<_>>();
                }
                PatternDescription::new(weight, allowed_neighbours)
            })
            .collect::<PatternTable<_>>()
    }
    pub fn global_stats(&self) -> GlobalStats {
        GlobalStats::new(self.pattern_descriptions())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use coord_2d::{Coord, Size};
    use direction::CardinalDirection;
    use grid_2d::Grid;
    #[test]
    fn compatibile_patterns() {
        let r = 0;
        let b = 1;
        let array = [[r, b, b], [b, r, b]];
        let grid = Grid::new_fn(Size::new(3, 2), |coord| {
            array[coord.y as usize][coord.x as usize]
        });
        let pattern_size = Size::new(2, 2);
        assert!(are_patterns_compatible(
            Coord::new(0, 0),
            Coord::new(1, 0),
            CardinalDirection::East,
            pattern_size,
            &grid,
        ));
        assert!(are_patterns_compatible(
            Coord::new(0, 0),
            Coord::new(1, 0),
            CardinalDirection::North,
            pattern_size,
            &grid,
        ));
        assert!(!are_patterns_compatible(
            Coord::new(0, 0),
            Coord::new(1, 0),
            CardinalDirection::South,
            pattern_size,
            &grid,
        ));
        assert!(!are_patterns_compatible(
            Coord::new(0, 0),
            Coord::new(1, 0),
            CardinalDirection::West,
            pattern_size,
            &grid,
        ));
    }
}
