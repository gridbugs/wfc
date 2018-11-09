extern crate image;

mod coord {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Coord {
        pub x: i32,
        pub y: i32,
    }

    #[derive(Debug, Clone, Copy)]
    pub struct Size {
        pub width: u32,
        pub height: u32,
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
        coord.x >= 0 && coord.y >= 0 && coord.x < size.width as i32
            && coord.y < size.height as i32
    }

    pub type GridIter<'a, T> = ::std::slice::Iter<'a, T>;

    struct CoordIter {
        coord: Coord,
        size: Size,
    }

    impl CoordIter {
        fn new(size: Size) -> Self {
            Self {
                size,
                coord: Coord { x: 0, y: 0 },
            }
        }
    }

    impl Iterator for CoordIter {
        type Item = Coord;
        fn next(&mut self) -> Option<Self::Item> {
            if self.coord.y == self.size.height as i32 {
                return None;
            }
            let coord = self.coord;
            self.coord.x += 1;
            if self.coord.x == self.size.width as i32 {
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
                .get(valid_coord_to_index(coord, self.size.width))
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
        pub fn tiled(&self) -> TiledGrid<T> {
            TiledGrid { grid: self }
        }
    }

    pub struct TiledGrid<'a, T: 'a> {
        grid: &'a Grid<T>,
    }

    impl<'a, T> TiledGrid<'a, T> {
        pub fn get(&self, coord: Coord) -> &T {
            let coord = coord.normalize(self.grid.size);
            let width = self.grid.size.width;
            &self.grid.cells[valid_coord_to_index(coord, width)]
        }
        pub fn slice(&self, top_left: Coord, size: Size) -> TiledGridSlice<T> {
            TiledGridSlice {
                grid: self,
                top_left,
                size,
            }
        }
    }

    pub struct TiledGridSlice<'a, T: 'a> {
        grid: &'a TiledGrid<'a, T>,
        top_left: Coord,
        size: Size,
    }

    impl<'a, T> TiledGridSlice<'a, T> {
        pub fn get(&self, coord: Coord) -> Option<&T> {
            if coord_is_valid(coord, self.size) {
                Some(self.grid.get(self.top_left + coord))
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
            let tiled = grid.tiled();
            let slice = tiled.slice(Coord::new(-1, -1), Size::new(2, 2));
            let value = *slice.get(Coord::new(0, 1)).unwrap();
            assert_eq!(value, Coord::new(3, 0));
        }
    }
}

mod direction {
    #[derive(Clone, Copy)]
    pub enum Direction {
        North,
        East,
        South,
        West,
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
    pub type PatternId = u16;
    pub struct PatternTable<T> {
        data: Vec<T>,
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
}

mod image_grid {
    use coord::{Coord, Size};
    use grid::Grid;
    use image::{DynamicImage, Rgb, RgbImage};

    #[derive(Clone, Copy)]
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
            let size = Size {
                width: rgb_image.width(),
                height: rgb_image.height(),
            };
            let grid = Grid::from_fn(
                size,
                |Coord { x, y }| Colour::from_rgb(*rgb_image.get_pixel(x as u32, y as u32)),
            );
            Self { grid }
        }
        pub fn to_image(&self) -> DynamicImage {
            let size = self.grid.size();
            let mut rgb_image = RgbImage::new(size.width, size.height);
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
