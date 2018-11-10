extern crate image;

mod coord {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Coord {
        pub x: i32,
        pub y: i32,
    }

    #[derive(Debug, Clone, Copy)]
    pub struct Size {
        width: u32,
        height: u32,
    }

    impl Coord {
        pub fn new(x: i32, y: i32) -> Self {
            Self { x, y }
        }
        fn normalize_part(value: i32, size: u32) -> i32 {
            let value = (value % size as i32);
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
        pub fn get(&self, coord: Coord) -> Option<&T> {
            if coord_is_valid(coord, self.size) {
                self.get_valid_coord(coord)
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

    pub struct TiledGridSlice<'a, T: 'a> {
        grid: &'a Grid<T>,
        top_left: Coord,
        size: Size,
    }

    impl<'a, T> TiledGridSlice<'a, T> {
        pub fn get(&self, coord: Coord) -> Option<&T> {
            if coord_is_valid(coord, self.size) {
                Some(self.grid.tiled_get(self.top_left + coord))
            } else {
                None
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::Grid;
        use coord::{Coord, Size};
        #[test]
        fn tiling() {
            let grid = Grid::from_fn(Size::new(4, 4), |coord| coord);
            let slice = grid.tiled_slice(Coord::new(-1, -1), Size::new(2, 2));
            let value = *slice.get(Coord::new(0, 1)).unwrap();
            assert_eq!(value, Coord::new(3, 0));
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
    pub struct PatternTable<T> {
        data: Vec<T>,
    }
    pub struct Pattern {
        top_left: Coord,
        size: Size,
    }
    impl<T: Default + Clone> PatternTable<T> {
        pub fn new(size: usize) -> Self {
            let mut data = Vec::with_capacity(size);
            data.resize(size, Default::default());
            Self { data }
        }
    }
    impl<T> PatternTable<T> {
        pub fn get(&self, pattern_id: PatternId) -> Option<&T> {
            self.data.get(pattern_id as usize)
        }
    }

    pub fn pattern_coords(grid_size: Size, pattern_size: Size) -> PatternTable<Coord> {
        PatternTable {
            data: CoordIter::new(grid_size).collect(),
        }
    }

    pub fn are_patterns_compatible(
        a: Coord,
        b: Coord,
        b_offset_direction: Direction,
        pattern_size: Size,
        grid: Grid<Colour>,
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
        true
    }
}

mod image_grid {
    use coord::{Coord, Size};
    use grid::Grid;
    use image::{DynamicImage, Rgb, RgbImage};

    #[derive(Clone, Copy, PartialEq, Eq)]
    pub struct Colour {
        r: u8,
        g: u8,
        b: u8,
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
    }
}

mod compatibility {
    use direction::{Direction, DirectionTable};
    use grid::Grid;
    use image_grid::Colour;
    use pattern::{PatternId, PatternTable};
    struct CompatibilityTable {
        table: PatternTable<DirectionTable<Vec<PatternId>>>,
    }

    impl CompatibilityTable {
        pub fn compatibile_patterns(
            &self,
            pattern_id: PatternId,
            direction: Direction,
        ) -> Option<::std::slice::Iter<PatternId>> {
            self.table
                .get(pattern_id)
                .map(|direction_table| direction_table.get(direction).iter())
        }
        pub fn from_grid_tiling(grid: &Grid<Colour>) -> Self {
            unimplemented!()
        }
    }
}

fn main() {
    let image = image::load_from_memory(include_bytes!("rooms.png")).unwrap();
    let image_grid = image_grid::ImageGrid::from_image(&image);
    image_grid.to_image().save("/tmp/a.png").unwrap();
}
