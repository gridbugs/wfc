extern crate image;

mod coord {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    pub struct Coord {
        pub x: i32,
        pub y: i32,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Size {
        width: u32,
        height: u32,
    }

    impl Coord {
        pub fn new(x: i32, y: i32) -> Self {
            Self { x, y }
        }
        fn normalize_part(value: i32, size: u32) -> i32 {
            let value = value % size as i32;
            if value < 0 {
                value + size as i32
            } else {
                value
            }
        }
        pub fn normalize(self, size: Size) -> Self {
            Self {
                x: Self::normalize_part(self.x, size.width),
                y: Self::normalize_part(self.y, size.height),
            }
        }
    }

    impl Size {
        pub fn new(width: u32, height: u32) -> Self {
            Self { width, height }
        }
        pub fn width(self) -> u32 {
            self.width
        }
        pub fn height(self) -> u32 {
            self.height
        }
        pub fn count(self) -> usize {
            (self.width * self.height) as usize
        }
    }

    impl ::std::ops::Add for Coord {
        type Output = Coord;
        fn add(self, other: Self) -> Self::Output {
            Coord {
                x: self.x + other.x,
                y: self.y + other.y,
            }
        }
    }

    impl ::std::ops::Sub for Size {
        type Output = Size;
        fn sub(self, other: Self) -> Self::Output {
            if self.width() <= other.width() {
                panic!()
            }
            if self.height() <= other.height() {
                panic!()
            }
            Size::new(self.width() - other.width(), self.height() - other.height())
        }
    }

}

mod grid {
    use coord::{Coord, Size};
    use std::hash::{Hash, Hasher};
    pub struct Grid<T> {
        size: Size,
        cells: Vec<T>,
    }

    fn valid_coord_to_index(coord: Coord, width: u32) -> usize {
        coord.x as usize + coord.y as usize * width as usize
    }

    fn coord_is_valid(coord: Coord, size: Size) -> bool {
        coord.x >= 0
            && coord.y >= 0
            && coord.x < size.width() as i32
            && coord.y < size.height() as i32
    }

    pub type GridIter<'a, T> = ::std::slice::Iter<'a, T>;

    pub struct CoordIter {
        coord: Coord,
        size: Size,
    }

    impl CoordIter {
        pub fn new(size: Size) -> Self {
            Self {
                size,
                coord: Coord { x: 0, y: 0 },
            }
        }
    }

    impl Iterator for CoordIter {
        type Item = Coord;
        fn next(&mut self) -> Option<Self::Item> {
            if self.coord.y == self.size.height() as i32 {
                return None;
            }
            let coord = self.coord;
            self.coord.x += 1;
            if self.coord.x == self.size.width() as i32 {
                self.coord.x = 0;
                self.coord.y += 1;
            }
            Some(coord)
        }
    }

    pub struct Enumerate<'a, T: 'a> {
        grid_iter: GridIter<'a, T>,
        coord_iter: CoordIter,
    }

    impl<'a, T> Iterator for Enumerate<'a, T> {
        type Item = (Coord, &'a T);
        fn next(&mut self) -> Option<Self::Item> {
            self.coord_iter
                .next()
                .and_then(|coord| self.grid_iter.next().map(|value| (coord, value)))
        }
    }

    impl<T> Grid<T> {
        pub fn size(&self) -> Size {
            self.size
        }
        fn get_valid_coord(&self, coord: Coord) -> Option<&T> {
            self.cells
                .get(valid_coord_to_index(coord, self.size.width()))
        }
        fn get_valid_coord_mut(&mut self, coord: Coord) -> Option<&mut T> {
            self.cells
                .get_mut(valid_coord_to_index(coord, self.size.width()))
        }

        pub fn get(&self, coord: Coord) -> Option<&T> {
            if coord_is_valid(coord, self.size) {
                self.get_valid_coord(coord)
            } else {
                None
            }
        }
        pub fn get_mut(&mut self, coord: Coord) -> Option<&mut T> {
            if coord_is_valid(coord, self.size) {
                self.get_valid_coord_mut(coord)
            } else {
                None
            }
        }

        pub fn from_fn<F>(size: Size, f: F) -> Self
        where
            F: Fn(Coord) -> T,
        {
            let count = size.count();
            let mut cells = Vec::with_capacity(count);
            for coord in CoordIter::new(size) {
                cells.push(f(coord));
            }
            assert_eq!(cells.len(), count);
            Self { cells, size }
        }
        fn iter(&self) -> GridIter<T> {
            self.cells.iter()
        }
        fn coord_iter(&self) -> CoordIter {
            CoordIter::new(self.size)
        }
        pub fn enumerate(&self) -> Enumerate<T> {
            Enumerate {
                grid_iter: self.iter(),
                coord_iter: self.coord_iter(),
            }
        }
        pub fn tiled_get(&self, coord: Coord) -> &T {
            let coord = coord.normalize(self.size);
            let width = self.size.width();
            &self.cells[valid_coord_to_index(coord, width)]
        }
        pub fn tiled_slice(&self, top_left: Coord, size: Size) -> TiledGridSlice<T> {
            TiledGridSlice {
                grid: self,
                top_left,
                size,
            }
        }
    }

    #[derive(Clone)]
    pub struct TiledGridSlice<'a, T: 'a> {
        grid: &'a Grid<T>,
        top_left: Coord,
        size: Size,
    }

    pub struct TiledGridSliceIter<'a, T: 'a> {
        grid: &'a Grid<T>,
        coord_iter: CoordIter,
        top_left: Coord,
    }

    impl<'a, T> Iterator for TiledGridSliceIter<'a, T> {
        type Item = &'a T;
        fn next(&mut self) -> Option<Self::Item> {
            self.coord_iter
                .next()
                .map(|coord| self.grid.tiled_get(self.top_left + coord))
        }
    }

    impl<'a, T> TiledGridSlice<'a, T> {
        pub fn top_left(&self) -> Coord {
            self.top_left
        }
        pub fn get(&self, coord: Coord) -> Option<&T> {
            if coord_is_valid(coord, self.size) {
                Some(self.grid.tiled_get(self.top_left + coord))
            } else {
                None
            }
        }
        pub fn iter(&self) -> TiledGridSliceIter<T> {
            TiledGridSliceIter {
                grid: self.grid,
                top_left: self.top_left,
                coord_iter: CoordIter::new(self.size),
            }
        }
    }

    impl<'a, T: Hash> Hash for TiledGridSlice<'a, T> {
        fn hash<H: Hasher>(&self, state: &mut H) {
            for value in self.iter() {
                value.hash(state);
            }
        }
    }

    impl<'a, T: PartialEq> PartialEq for TiledGridSlice<'a, T> {
        fn eq(&self, other: &Self) -> bool {
            self.size == other.size && self.iter().zip(other.iter()).all(|(s, o)| s.eq(o))
        }
    }
    impl<'a, T: Eq> Eq for TiledGridSlice<'a, T> {}

    #[cfg(test)]
    mod test {
        use super::Grid;
        use coord::{Coord, Size};
        use std::collections::HashSet;
        #[test]
        fn tiling() {
            let grid = Grid::from_fn(Size::new(4, 4), |coord| coord);
            let slice = grid.tiled_slice(Coord::new(-1, -1), Size::new(2, 2));
            let value = *slice.get(Coord::new(0, 1)).unwrap();
            assert_eq!(value, Coord::new(3, 0));
        }
        #[test]
        fn tiled_grid_slice_hash() {
            let mut grid = Grid::from_fn(Size::new(4, 4), |_| 0);
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

mod direction {
    use coord::Coord;
    #[derive(Clone, Copy)]
    pub enum Direction {
        North,
        East,
        South,
        West,
    }
    impl Direction {
        pub fn coord(self) -> Coord {
            match self {
                Direction::North => Coord::new(0, -1),
                Direction::East => Coord::new(1, 0),
                Direction::South => Coord::new(0, 1),
                Direction::West => Coord::new(-1, 0),
            }
        }
    }
    #[derive(Default)]
    pub struct DirectionTable<T> {
        values: [T; 4],
    }
    impl<T> DirectionTable<T> {
        pub fn get(&self, direction: Direction) -> &T {
            &self.values[direction as usize]
        }
    }
}

mod pattern {
    use coord::{Coord, Size};
    use direction::Direction;
    use grid::{CoordIter, Grid};
    use image_grid::Colour;
    pub type PatternId = u16;
    pub const MAX_PATTERN_ID: PatternId = ::std::u16::MAX;

    pub fn are_patterns_compatible(
        a: Coord,
        b: Coord,
        b_offset_direction: Direction,
        pattern_size: Size,
        grid: &Grid<Colour>,
    ) -> bool {
        let (overlap_size_to_sub, a_offset, b_offset) = match b_offset_direction {
            Direction::North => (Size::new(0, 1), Coord::new(0, 0), Coord::new(0, 1)),
            Direction::South => (Size::new(0, 1), Coord::new(0, 1), Coord::new(0, 0)),
            Direction::East => (Size::new(1, 0), Coord::new(1, 0), Coord::new(0, 0)),
            Direction::West => (Size::new(1, 0), Coord::new(0, 0), Coord::new(1, 0)),
        };
        let overlap_size = pattern_size - overlap_size_to_sub;
        let a_overlap = a + a_offset;
        let b_overlap = b + b_offset;
        let a_slice = grid.tiled_slice(a_overlap, overlap_size);
        let b_slice = grid.tiled_slice(b_overlap, overlap_size);
        a_slice.iter().zip(b_slice.iter()).all(|(a, b)| a == b)
    }

    #[cfg(test)]
    mod test {
        use super::*;
        use coord::{Coord, Size};
        use direction::Direction;
        use grid::Grid;
        use image_grid::Colour;
        #[test]
        fn compatibile_patterns() {
            let r = Colour { r: 255, g: 0, b: 0 };
            let b = Colour { r: 0, g: 0, b: 255 };
            let array = [[r, b, b], [b, r, b]];
            let grid = Grid::from_fn(Size::new(3, 2), |coord| {
                array[coord.y as usize][coord.x as usize]
            });
            let pattern_size = Size::new(2, 2);
            assert!(are_patterns_compatible(
                Coord::new(0, 0),
                Coord::new(1, 0),
                Direction::East,
                pattern_size,
                &grid,
            ));
            assert!(are_patterns_compatible(
                Coord::new(0, 0),
                Coord::new(1, 0),
                Direction::North,
                pattern_size,
                &grid,
            ));
            assert!(!are_patterns_compatible(
                Coord::new(0, 0),
                Coord::new(1, 0),
                Direction::South,
                pattern_size,
                &grid,
            ));
            assert!(!are_patterns_compatible(
                Coord::new(0, 0),
                Coord::new(1, 0),
                Direction::West,
                pattern_size,
                &grid,
            ));
        }
    }
}

mod image_grid {
    use coord::{Coord, Size};
    use grid::{CoordIter, Grid, TiledGridSlice};
    use image::{DynamicImage, Rgb, RgbImage};

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
    pub struct ImageGrid {
        grid: Grid<Colour>,
    }
    pub struct PatternIter<'a> {
        grid: &'a Grid<Colour>,
        pattern_size: Size,
        coord_iter: CoordIter,
    }
    impl<'a> Iterator for PatternIter<'a> {
        type Item = TiledGridSlice<'a, Colour>;
        fn next(&mut self) -> Option<Self::Item> {
            self.coord_iter
                .next()
                .map(|coord| self.grid.tiled_slice(coord, self.pattern_size))
        }
    }
    impl ImageGrid {
        pub fn from_image(image: &DynamicImage) -> Self {
            let rgb_image = image.to_rgb();
            let size = Size::new(rgb_image.width(), rgb_image.height());
            let grid = Grid::from_fn(size, |Coord { x, y }| {
                Colour::from_rgb(*rgb_image.get_pixel(x as u32, y as u32))
            });
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
                coord_iter: CoordIter::new(self.grid.size()),
            }
        }
    }
}

mod compatibility {
    use coord::Size;
    use direction::{Direction, DirectionTable};
    use grid::Grid;
    use image_grid::Colour;
    use pattern::{self, PatternId};
    struct CompatibilityTable {
        table: Vec<DirectionTable<Vec<PatternId>>>,
    }

    impl CompatibilityTable {
        pub fn compatibile_patterns(
            &self,
            pattern_id: PatternId,
            direction: Direction,
        ) -> ::std::slice::Iter<PatternId> {
            self.table[pattern_id as usize].get(direction).iter()
        }
        pub fn from_grid_tiling(pattern_size: Size, grid: &Grid<Colour>) -> Self {
            unimplemented!()
        }
    }
}

use coord::{Coord, Size};
use std::collections::HashMap;

#[derive(Debug)]
struct PatternInfo {
    example_coord: Coord,
    count: usize,
}

fn main() {
    let image = image::load_from_memory(include_bytes!("rooms.png")).unwrap();
    let image_grid = image_grid::ImageGrid::from_image(&image);
    image_grid.to_image().save("/tmp/a.png").unwrap();
    let pattern_size = Size::new(3, 3);
    let mut pattern_table = HashMap::new();
    for pattern in image_grid.patterns(pattern_size) {
        let info = pattern_table
            .entry(pattern.clone())
            .or_insert_with(|| PatternInfo {
                example_coord: pattern.top_left(),
                count: 0,
            });
        info.count += 1;
    }
    let mut pattern_table = pattern_table.values().collect::<Vec<_>>();
    pattern_table.sort_by_key(|i| i.example_coord);
    println!("{:#?}", pattern_table);
}
