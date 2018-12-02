extern crate coord_2d;
extern crate direction;
extern crate grid_2d;
extern crate hashbrown;
extern crate image;
extern crate rand;
extern crate rand_xorshift;

mod context;
mod tiled_slice;

use context::*;
use coord_2d::{Coord, Size};
use direction::{CardinalDirection, CardinalDirectionTable};
use grid_2d::coord_system::XThenYIter;
use grid_2d::Grid;
use hashbrown::HashMap;
use image::{DynamicImage, Rgb, RgbImage};
use rand::SeedableRng;
use rand_xorshift::XorShiftRng;
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
    use coord_2d::{Coord, Size};
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
            .or_insert_with(|| PrePattern::new(pattern.offset(), 0));
        info.count += 1;
    }
    let patterns = pre_patterns
        .values()
        .map(|pre_pattern| Pattern::new(pre_pattern.example_coord, pre_pattern.count))
        .collect::<Vec<_>>();

    let pattern_table = PatternTable::new(patterns);
    let stats_per_pattern = pattern_table
        .patterns
        .iter()
        .map(|p| PatternStats::new(p.count))
        .collect::<context::PatternTable<_>>();

    let compatibility_per_pattern = pattern_table
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
        .collect::<context::PatternTable<_>>();
    let output = {
        let global_stats = GlobalStats::new(stats_per_pattern, compatibility_per_pattern);
        let mut context = Context::new(output_size);
        {
            let mut run = context.run(&global_stats, &mut rng);
            loop {
                match run.step() {
                    Step::Complete => break,
                    Step::Incomplete => (),
                }
            }
        }
        Grid::new_fn(context.size(), |coord| {
            let pattern_id = context.get_pattern_id(coord).unwrap();
            pattern_table.colour(pattern_id, &image_grid.grid)
        })
    };
    println!("{:?}", ::std::time::Instant::now() - start_time);
    let output_image = ImageGrid { grid: output };
    output_image.to_image().save("/tmp/a.png").unwrap();
}
