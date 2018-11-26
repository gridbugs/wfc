extern crate coord_2d;
extern crate direction;
extern crate grid_2d;
extern crate hashbrown;
extern crate image;
extern crate rand;
extern crate rand_xorshift;

mod tiled_slice {
    use coord_2d::*;
    use grid_2d::coord_system::{CoordSystem, XThenY, XThenYIter};
    use grid_2d::*;
    use std::hash::{Hash, Hasher};

    pub fn new<'a, T, S: CoordSystem + Clone>(
        grid: &'a Grid<T, S>,
        top_left: Coord,
        size: Size,
    ) -> TiledGridSlice<'a, T, S> {
        TiledGridSlice {
            grid,
            top_left,
            size,
        }
    }

    #[derive(Clone)]
    pub struct TiledGridSlice<'a, T: 'a, S: 'a + CoordSystem + Clone = XThenY> {
        grid: &'a Grid<T, S>,
        top_left: Coord,
        size: Size,
    }

    pub struct TiledGridSliceIter<'a, T: 'a, S: 'a + CoordSystem> {
        grid: &'a Grid<T, S>,
        coord_iter: XThenYIter,
        top_left: Coord,
    }

    impl<'a, T, S> Iterator for TiledGridSliceIter<'a, T, S>
    where
        S: CoordSystem + Clone,
    {
        type Item = &'a T;
        fn next(&mut self) -> Option<Self::Item> {
            self.coord_iter
                .next()
                .map(|coord| self.grid.tiled_get(self.top_left + coord))
        }
    }

    impl<'a, T, S> TiledGridSlice<'a, T, S>
    where
        S: CoordSystem + Clone,
    {
        pub fn top_left(&self) -> Coord {
            self.top_left
        }
        pub fn get(&self, coord: Coord) -> Option<&T> {
            if coord.is_valid(self.size) {
                Some(self.grid.tiled_get(self.top_left + coord))
            } else {
                None
            }
        }
        pub fn iter(&self) -> TiledGridSliceIter<T, S> {
            TiledGridSliceIter {
                grid: self.grid,
                top_left: self.top_left,
                coord_iter: XThenYIter::from(self.size),
            }
        }
    }

    impl<'a, T: Hash, S> Hash for TiledGridSlice<'a, T, S>
    where
        S: CoordSystem + Clone,
    {
        fn hash<H: Hasher>(&self, state: &mut H) {
            for value in self.iter() {
                value.hash(state);
            }
        }
    }

    impl<'a, T: PartialEq, S> PartialEq for TiledGridSlice<'a, T, S>
    where
        S: CoordSystem + Clone,
    {
        fn eq(&self, other: &Self) -> bool {
            self.size == other.size && self.iter().zip(other.iter()).all(|(s, o)| s.eq(o))
        }
    }
    impl<'a, T: Eq, S> Eq for TiledGridSlice<'a, T, S>
    where
        S: CoordSystem + Clone,
    {
    }

    #[cfg(test)]
    mod test {
        use super::*;
        use coord_2d::{Coord, Size};
        use grid_2d::coord_system::{XThenY, XThenYIter};
        use std::collections::HashSet;
        #[test]
        fn tiling() {
            let grid = Grid::new_fn(XThenY::new(Size::new(4, 4)), |coord| coord);
            let slice = grid.tiled_slice(Coord::new(-1, -1), Size::new(2, 2));
            let value = *slice.get(Coord::new(0, 1)).unwrap();
            assert_eq!(value, Coord::new(3, 0));
        }
        #[test]
        fn tiled_grid_slice_hash() {
            let mut grid = Grid::new_fn(XThenY::new(Size::new(4, 4)), |_| 0);
            *grid.get_mut(Coord::new(3, 3)).unwrap() = 1;
            let size = Size::new(2, 2);
            let a = grid.tiled_slice(Coord::new(0, 0), size);
            let b = grid.tiled_slice(Coord::new(2, 2), size);
            let c = grid.tiled_slice(Coord::new(0, 2), size);
            let mut set = HashSet::new();
            set.insert(a);
            set.insert(b);
            set.insert(c);
            assert_eq!(set.len(), 2);
        }
    }
}

use coord_2d::{Coord, Size};
use direction::{CardinalDirection, CardinalDirectionTable, CardinalDirections};
use grid_2d::coord_system::XThenYIter;
use grid_2d::Grid;
use hashbrown::HashMap;
use image::{DynamicImage, Rgb, RgbImage};
use rand::{Rng, SeedableRng};
use rand_xorshift::XorShiftRng;
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use tiled_slice::*;

pub fn are_patterns_compatible(
    a: Coord,
    b: Coord,
    b_offset_direction: CardinalDirection,
    pattern_size: Size,
    grid: &Grid<Colour>,
) -> bool {
    let (overlap_size_to_sub, a_offset, b_offset) = match b_offset_direction {
        CardinalDirection::North => (Size::new(0, 1), Coord::new(0, 0), Coord::new(0, 1)),
        CardinalDirection::South => (Size::new(0, 1), Coord::new(0, 1), Coord::new(0, 0)),
        CardinalDirection::East => (Size::new(1, 0), Coord::new(1, 0), Coord::new(0, 0)),
        CardinalDirection::West => (Size::new(1, 0), Coord::new(0, 0), Coord::new(1, 0)),
    };
    let overlap_size = pattern_size - overlap_size_to_sub;
    let a_overlap = a + a_offset;
    let b_overlap = b + b_offset;
    let a_slice = tiled_slice::new(grid, a_overlap, overlap_size);
    let b_slice = tiled_slice::new(grid, b_overlap, overlap_size);
    a_slice
        .iter()
        .zip(b_slice.iter())
        .all(|(a, b)| a == b)
}

#[cfg(test)]
mod pattern_test {
    use super::*;
    use coord::{Coord, Size};
    use direction::CardinalDirection;
    use grid_2d::Grid;
    #[test]
    fn compatibile_patterns() {
        let r = Colour {
            r: 255,
            g: 0,
            b: 0,
        };
        let b = Colour {
            r: 0,
            g: 0,
            b: 255,
        };
        let array = [[r, b, b], [b, r, b]];
        let grid = Grid::new_fn(XThenY::new(Size::new(3, 2)), |coord| {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Colour {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}
impl Colour {
    fn from_rgb(Rgb { data: [r, g, b] }: Rgb<u8>) -> Self {
        Self { r, g, b }
    }
    fn to_rgb(self) -> Rgb<u8> {
        Rgb {
            data: [self.r, self.g, self.b],
        }
    }
}
pub struct PatternIter<'a> {
    grid: &'a Grid<Colour>,
    pattern_size: Size,
    coord_iter: XThenYIter,
}
impl<'a> Iterator for PatternIter<'a> {
    type Item = TiledGridSlice<'a, Colour>;
    fn next(&mut self) -> Option<Self::Item> {
        self.coord_iter
            .next()
            .map(|coord| tiled_slice::new(self.grid, coord, self.pattern_size))
    }
}
pub struct ImageGrid {
    pub grid: Grid<Colour>,
}
impl ImageGrid {
    pub fn from_image(image: &DynamicImage) -> Self {
        let rgb_image = image.to_rgb();
        let size = Size::new(rgb_image.width(), rgb_image.height());
        let grid = Grid::new_fn(
            size,
            |Coord { x, y }| Colour::from_rgb(*rgb_image.get_pixel(x as u32, y as u32)),
        );
        Self { grid }
    }
    pub fn to_image(&self) -> DynamicImage {
        let size = self.grid.size();
        let mut rgb_image = RgbImage::new(size.width(), size.height());
        for (Coord { x, y }, colour) in self.grid.enumerate() {
            rgb_image.put_pixel(x as u32, y as u32, colour.to_rgb());
        }
        DynamicImage::ImageRgb8(rgb_image)
    }
    pub fn patterns(&self, pattern_size: Size) -> PatternIter {
        PatternIter {
            grid: &self.grid,
            pattern_size,
            coord_iter: XThenYIter::from(self.grid.size()),
        }
    }
}

struct PrePattern {
    example_coord: Coord,
    count: u32,
}

impl PrePattern {
    fn new(example_coord: Coord, count: u32) -> Self {
        Self {
            example_coord,
            count,
        }
    }
}

#[derive(Debug)]
struct Pattern {
    example_coord: Coord,
    count: u32,
    count_log_count: f32,
}

impl Pattern {
    fn new(example_coord: Coord, count: u32) -> Self {
        let count_log_count = (count as f32) * (count as f32).log2();
        Self {
            example_coord,
            count,
            count_log_count,
        }
    }
}

#[derive(Debug)]
struct PatternTable {
    patterns: Vec<Pattern>,
    sum_pattern_count: u32,
    sum_pattern_count_log_count: f32,
}

impl PatternTable {
    fn new(mut patterns: Vec<Pattern>) -> Self {
        patterns.sort_by_key(|i| i.example_coord);
        let sum_pattern_count = patterns.iter().map(|p| p.count).sum();
        let sum_pattern_count_log_count =
            patterns.iter().map(|p| p.count_log_count).sum();
        Self {
            patterns,
            sum_pattern_count,
            sum_pattern_count_log_count,
        }
    }
    fn colour(&self, pattern_id: PatternId, input_grid: &Grid<Colour>) -> Colour {
        input_grid
            .get(self.patterns[pattern_id as usize].example_coord)
            .cloned()
            .unwrap()
    }
}

#[derive(PartialEq, Debug)]
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
struct CellMetadata {
    num_possible_patterns: u32,
    // n0 + n1 + n2 + ...
    sum_possible_pattern_count: u32,
    // n0*log(n0) + n1*log(n1) + n2*log(n2) + ...
    sum_possible_pattern_count_log_count: f32,
}

impl CellMetadata {
    fn remove_possible_pattern(&mut self, pattern: &Pattern) {
        assert!(
            pattern.count < self.sum_possible_pattern_count,
            "Should never remove the last pattern of a cell"
        );
        self.num_possible_patterns -= 1;
        self.sum_possible_pattern_count -= pattern.count;
        self.sum_possible_pattern_count_log_count -= pattern.count_log_count;
    }
    fn entropy(&self) -> f32 {
        // log(n0+n1+n2+...) - (n0*log(n0) + n1*log(n1) + n2*log(n2) + ...) / (n0+n1+n2+...)
        let sum_possible_pattern_count = self.sum_possible_pattern_count as f32;
        sum_possible_pattern_count.log2()
            - (self.sum_possible_pattern_count_log_count / sum_possible_pattern_count)
    }
}

#[derive(Debug)]
pub struct Cell {
    possible_pattern_ids: Vec<bool>,
    metadata: CellMetadata,
    noise: u32,
}

impl Cell {
    fn new<R: Rng>(pattern_table: &PatternTable, rng: &mut R) -> Self {
        let possible_pattern_ids = pattern_table.patterns.iter().map(|_| true).collect();
        let num_possible_patterns = pattern_table.patterns.len() as u32;
        let sum_possible_pattern_count = pattern_table.sum_pattern_count;
        let sum_possible_pattern_count_log_count =
            pattern_table.sum_pattern_count_log_count;
        Self {
            possible_pattern_ids,
            noise: rng.gen(),
            metadata: CellMetadata {
                num_possible_patterns,
                sum_possible_pattern_count,
                sum_possible_pattern_count_log_count,
            },
        }
    }
    fn remove_possible_pattern(
        &mut self,
        pattern_id: PatternId,
        pattern_table: &PatternTable,
    ) {
        assert!(self.metadata.num_possible_patterns > 1);
        let possible_pattern_id = &mut self.possible_pattern_ids[pattern_id as usize];
        if !*possible_pattern_id {
            return;
        }
        *possible_pattern_id = false;
        self.metadata
            .remove_possible_pattern(&pattern_table.patterns[pattern_id as usize]);
    }
    fn chosen_pattern_id(&self) -> PatternId {
        self.possible_pattern_ids
            .iter()
            .position(Clone::clone)
            .unwrap() as PatternId
    }
    fn choose_pattern_id<R: Rng>(
        &self,
        pattern_table: &PatternTable,
        rng: &mut R,
    ) -> PatternId {
        assert!(self.metadata.num_possible_patterns > 1);
        let mut remaining = rng.gen_range(0, self.metadata.sum_possible_pattern_count);
        for (pattern_id, pattern) in self.possible_pattern_ids
            .iter()
            .zip(pattern_table.patterns.iter().enumerate())
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

#[derive(Debug)]
struct RemovedPattern {
    coord: Coord,
    pattern_id: PatternId,
}

#[derive(Debug)]
struct Propagator {
    remaining_ways_to_become_pattern: Grid<Vec<CardinalDirectionTable<u32>>>,
    removed_patterns_to_propagate: Vec<RemovedPattern>,
    next_entropies: HashMap<Coord, EntropyWithNoise>,
}

impl Propagator {
    fn new(
        size: Size,
        compatibility: &Vec<CardinalDirectionTable<Vec<PatternId>>>,
    ) -> Self {
        let initial_ways_to_become_pattern = compatibility
            .iter()
            .map(|compatible_patterns_per_direction| {
                let mut num_ways_to_become_pattern_from_direction =
                    CardinalDirectionTable::default();
                for direction in direction::CardinalDirections {
                    *num_ways_to_become_pattern_from_direction.get_mut(direction) =
                        compatible_patterns_per_direction
                            .get(direction.opposite())
                            .len() as u32;
                }
                num_ways_to_become_pattern_from_direction
            })
            .collect::<Vec<_>>();
        let remaining_ways_to_become_pattern =
            Grid::new_fn(size, |_| initial_ways_to_become_pattern.clone());

        Self {
            remaining_ways_to_become_pattern,
            removed_patterns_to_propagate: Vec::new(),
            next_entropies: HashMap::default(),
        }
    }

    fn add(&mut self, coord: Coord, pattern_id: PatternId) {
        self.removed_patterns_to_propagate
            .push(RemovedPattern {
                coord,
                pattern_id,
            });
        self.remaining_ways_to_become_pattern
            .tiled_get_mut(coord)[pattern_id as usize]
            .iter_mut()
            .for_each(|c| *c = 0);
    }

    fn propagate(
        &mut self,
        compatibility: &Vec<CardinalDirectionTable<Vec<PatternId>>>,
        pattern_table: &PatternTable,
        output_grid: &mut Grid<Cell>,
        entropy_priority_queue: &mut BinaryHeap<CoordEntropy>,
        num_undecided_cells: &mut u32,
    ) {
        while let Some(removed_pattern) = self.removed_patterns_to_propagate.pop() {
            for direction in CardinalDirections {
                let coord_to_update = removed_pattern.coord + direction.coord();
                let remaining = self.remaining_ways_to_become_pattern
                    .tiled_get_mut(coord_to_update);
                for &pattern_id in compatibility[removed_pattern.pattern_id as usize]
                    .get(direction)
                    .iter()
                {
                    let remaining = &mut remaining[pattern_id as usize];
                    let count = {
                        let count = remaining.get_mut(direction);
                        if *count == 0 {
                            continue;
                        }
                        *count -= 1;
                        *count
                    };
                    if count == 0 {
                        let size = output_grid.size();
                        let cell = output_grid.tiled_get_mut(coord_to_update);
                        cell.remove_possible_pattern(pattern_id, pattern_table);

                        if cell.is_decided() {
                            *num_undecided_cells -= 1;
                            self.next_entropies.remove(&coord_to_update);
                        } else {
                            self.next_entropies.insert(
                                coord_to_update.normalize(size),
                                cell.entropy_with_noise(),
                            );
                        }

                        self.removed_patterns_to_propagate
                            .push(RemovedPattern {
                                coord: coord_to_update,
                                pattern_id,
                            });
                        remaining.iter_mut().for_each(|c| *c = 0);
                    }
                }
            }
        }
        for (coord, entropy_with_noise) in self.next_entropies.drain() {
            entropy_priority_queue.push(CoordEntropy {
                coord,
                entropy_with_noise,
            });
        }
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

struct Observer {
    grid: Grid<Cell>,
    entropy_priority_queue: BinaryHeap<CoordEntropy>,
    num_undecided_cells: u32,
}

enum NextCellChoice<'a> {
    MinEntropyCell { cell: &'a mut Cell, coord: Coord },
    Complete,
}

enum Observation {
    Complete,
    Incomplete,
}

type PatternId = u32;

impl Observer {
    fn new<R: Rng>(size: Size, pattern_table: &PatternTable, rng: &mut R) -> Self {
        let grid = Grid::new_fn(size, |_| Cell::new(&pattern_table, rng));
        let entropy_priority_queue = grid.enumerate()
            .map(|(coord, cell)| CoordEntropy {
                coord,
                entropy_with_noise: cell.entropy_with_noise(),
            })
            .collect();
        let num_undecided_cells = size.count() as u32;
        Self {
            grid,
            entropy_priority_queue,
            num_undecided_cells,
        }
    }

    fn choose_next_cell(&mut self) -> NextCellChoice {
        while let Some(coord_entropy) = self.entropy_priority_queue.pop() {
            if !self.grid
                .get(coord_entropy.coord)
                .unwrap()
                .is_decided()
            {
                let cell = self.grid.get_mut(coord_entropy.coord).unwrap();
                return NextCellChoice::MinEntropyCell {
                    coord: coord_entropy.coord,
                    cell,
                };
            }
        }
        NextCellChoice::Complete
    }

    fn observe<R: Rng>(
        &mut self,
        pattern_table: &PatternTable,
        propagator: &mut Propagator,
        rng: &mut R,
    ) -> Observation {
        if self.num_undecided_cells == 0 {
            return Observation::Complete;
        }
        {
            let (cell, coord) = match self.choose_next_cell() {
                NextCellChoice::Complete => return Observation::Complete,
                NextCellChoice::MinEntropyCell { cell, coord } => (cell, coord),
            };
            let chosen_pattern_id = cell.choose_pattern_id(pattern_table, rng);

            for ((pattern_id, is_possible), pattern) in cell.possible_pattern_ids
                .iter_mut()
                .enumerate()
                .zip(pattern_table.patterns.iter())
            {
                if pattern_id as PatternId != chosen_pattern_id {
                    if *is_possible {
                        *is_possible = false;
                        cell.metadata.remove_possible_pattern(pattern);
                        propagator.add(coord, pattern_id as PatternId);
                    }
                }
            }
        }
        self.num_undecided_cells -= 1;
        Observation::Incomplete
    }

    fn output(
        &self,
        size: Size,
        pattern_table: &PatternTable,
        input_grid: &Grid<Colour>,
    ) -> Grid<Colour> {
        Grid::new_fn(size, |coord| {
            let cell = self.grid.get(coord).unwrap();
            let pattern_id = cell.chosen_pattern_id();
            let colour = pattern_table.colour(pattern_id, input_grid);
            colour
        })
    }
}

fn fixed_rng() -> XorShiftRng {
    XorShiftRng::from_seed([0; 16])
}

fn main() {
    let mut rng = XorShiftRng::from_rng(rand::thread_rng()).unwrap();
    let image = image::load_from_memory(include_bytes!("rooms.png")).unwrap();
    let image_grid = ImageGrid::from_image(&image);
    let pattern_size = Size::new(3, 3);
    let output_size = Size::new(48, 48);
    let start_time = ::std::time::Instant::now();
    let mut pre_patterns = HashMap::new();
    for pattern in image_grid.patterns(pattern_size) {
        let info = pre_patterns
            .entry(pattern.clone())
            .or_insert_with(|| PrePattern::new(pattern.top_left(), 0));
        info.count += 1;
    }
    let patterns = pre_patterns
        .values()
        .map(|pre_pattern| Pattern::new(pre_pattern.example_coord, pre_pattern.count))
        .collect::<Vec<_>>();

    let pattern_table = PatternTable::new(patterns);

    let compatibility_table = pattern_table
        .patterns
        .iter()
        .map(|a_info| {
            let mut direction_table = CardinalDirectionTable::default();
            for direction in direction::CardinalDirections {
                *direction_table.get_mut(direction) = pattern_table
                    .patterns
                    .iter()
                    .enumerate()
                    .filter(|(_id, b_info)| {
                        are_patterns_compatible(
                            a_info.example_coord,
                            b_info.example_coord,
                            direction,
                            pattern_size,
                            &image_grid.grid,
                        )
                    })
                    .map(|(id, _info)| id as PatternId)
                    .collect::<Vec<_>>();
            }
            direction_table
        })
        .collect::<Vec<_>>();

    let mut propagator = Propagator::new(output_size, &compatibility_table);
    let mut observer = Observer::new(output_size, &pattern_table, &mut rng);

    loop {
        let observation = observer.observe(&pattern_table, &mut propagator, &mut rng);
        match observation {
            Observation::Complete => {
                break;
            }
            Observation::Incomplete => propagator.propagate(
                &compatibility_table,
                &pattern_table,
                &mut observer.grid,
                &mut observer.entropy_priority_queue,
                &mut observer.num_undecided_cells,
            ),
        }
    }

    println!("{:?}", ::std::time::Instant::now() - start_time);
    let output = observer.output(output_size, &pattern_table, &image_grid.grid);

    let output_image = ImageGrid { grid: output };

    output_image.to_image().save("/tmp/a.png").unwrap();
}
