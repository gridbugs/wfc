extern crate coord_2d;
extern crate direction;
extern crate grid_2d;
extern crate hashbrown;
extern crate image;
extern crate rand;
extern crate rand_xorshift;

mod context;
mod pattern;
mod tiled_slice;
mod wrap;

use context::*;
use coord_2d::{Coord, Size};
use direction::{CardinalDirection, CardinalDirectionTable};
use grid_2d::coord_system::XThenYIter;
use grid_2d::Grid;
use hashbrown::HashMap;
use image::{DynamicImage, Rgb, RgbImage};
use pattern::{GlobalStats, PatternId, PatternStats, PatternTable};
use rand::{Rng, SeedableRng};
use rand_xorshift::XorShiftRng;
use tiled_slice::*;
use wrap::*;

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
    coords: Vec<Coord>,
    count: u32,
}

impl PrePattern {
    fn new() -> Self {
        Self {
            coords: Vec::new(),
            count: 0,
        }
    }
}

#[derive(Debug)]
struct Pattern {
    coords: Vec<Coord>,
    count: u32,
    count_log_count: f32,
}

impl Pattern {
    fn new(coords: Vec<Coord>, count: u32) -> Self {
        let count_log_count = (count as f32) * (count as f32).log2();
        Self {
            coords,
            count,
            count_log_count,
        }
    }
}

#[derive(Debug)]
struct PatternTable_ {
    patterns: Vec<Pattern>,
    sum_pattern_count: u32,
    sum_pattern_count_log_count: f32,
}

impl PatternTable_ {
    fn new(mut patterns: Vec<Pattern>) -> Self {
        patterns.sort_by_key(|i| i.coords[0]);
        let sum_pattern_count = patterns.iter().map(|p| p.count).sum();
        let sum_pattern_count_log_count =
            patterns.iter().map(|p| p.count_log_count).sum();
        Self {
            patterns,
            sum_pattern_count,
            sum_pattern_count_log_count,
        }
    }
    fn set_count(&mut self, pattern_id: PatternId, count: u32) {
        let (count_diff, count_log_count_diff) = {
            let pattern = &mut self.patterns[pattern_id as usize];
            let count_diff = pattern.count - count;
            let count_log_count = if count == 0 {
                0.
            } else {
                (count as f32) * (count as f32).log2()
            };
            let count_log_count_diff = pattern.count_log_count - count_log_count;
            pattern.count = count;
            pattern.count_log_count = count_log_count;
            (count_diff, count_log_count_diff)
        };
        self.sum_pattern_count -= count_diff;
        self.sum_pattern_count_log_count -= count_log_count_diff;
    }
    fn colour(&self, pattern_id: PatternId, input_grid: &Grid<Colour>) -> Colour {
        input_grid
            .get(self.patterns[pattern_id as usize].coords[0])
            .cloned()
            .unwrap()
    }
    fn id_grid(&self, size: Size) -> Grid<PatternId> {
        let mut maybe_id_grid = Grid::new_clone(size, None);
        self.patterns
            .iter()
            .enumerate()
            .for_each(|(pattern_id, pattern)| {
                pattern.coords.iter().for_each(|&coord| {
                    *maybe_id_grid.get_checked_mut(coord) = Some(pattern_id as PatternId);
                });
            });
        Grid::new_fn(size, |coord| {
            maybe_id_grid.get_checked(coord).unwrap().clone()
        })
    }
}

fn rng_from_integer_seed(seed: u128) -> XorShiftRng {
    let mut seed_array = [0; 16];
    seed_array
        .iter_mut()
        .enumerate()
        .for_each(|(i, part)| {
            *part = (seed >> (i * 8)) as u8 & 0xff;
        });
    XorShiftRng::from_seed(seed_array)
}

fn main() {
    let seed: u128 = rand::thread_rng().gen();
    //let seed = 16786364572527804998395607799673680153;
    println!("{}", seed);
    //let mut rng = XorShiftRng::from_rng(rand::thread_rng()).unwrap();
    let mut rng = rng_from_integer_seed(seed);
    let image = image::load_from_memory(include_bytes!("flowers.png")).unwrap();
    let image_grid = ImageGrid::from_image(&image);
    let pattern_size = Size::new(3, 3);
    let output_size = Size::new(48, 48);
    let start_time = ::std::time::Instant::now();
    let mut pre_patterns = HashMap::new();
    for pattern in image_grid.patterns(pattern_size) {
        let info = pre_patterns
            .entry(pattern.clone())
            .or_insert_with(|| PrePattern::new());
        info.coords.push(pattern.offset());
        info.count += 1;
    }
    let patterns = pre_patterns
        .drain()
        .map(|(_, pre_pattern)| Pattern::new(pre_pattern.coords, pre_pattern.count))
        .collect::<Vec<_>>();

    let mut pattern_table = PatternTable_::new(patterns);
    let id_grid = pattern_table.id_grid(image_grid.grid.size());
    let bottom_left_corner_coord = Coord::new(0, image_grid.grid.size().y() as i32 - 1);
    let bottom_left_corner_id = *id_grid.get_checked(bottom_left_corner_coord);
    let sprout_id = *id_grid.get_checked(Coord::new(7, 21));
    pattern_table.set_count(bottom_left_corner_id, 0);
    let stats_per_pattern = pattern_table
        .patterns
        .iter()
        .map(|p| PatternStats::from_u32_weight(p.count))
        .collect::<PatternTable<_>>();

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
                            a_info.coords[0],
                            b_info.coords[0],
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
        .collect::<PatternTable<_>>();
    let output = {
        let global_stats = GlobalStats::new(stats_per_pattern, compatibility_per_pattern);
        let mut wave = Wave::new(output_size);
        let mut context = Context::new();
        {
            let mut run = context.run::<WrapXY, _>(&mut wave, &global_stats, &mut rng);
            let sprout_coord = Coord::new(
                (rng.gen::<u32>() % output_size.width()) as i32,
                output_size.height() as i32 - 2,
            );

            run.set_pattern(sprout_coord, sprout_id);

            for i in 0..(output_size.width() as i32) {
                let coord = Coord::new(i, output_size.height() as i32 - 1);
                run.set_pattern(coord, bottom_left_corner_id);
            }
            /*
            for _ in 0..61 {
                run.step(&mut rng);
            }*/
            //run._debug();
            //
            'steps: loop {
                match run.step(&mut rng) {
                    Progress::Complete => break,
                    Progress::Incomplete => (),
                }
            }
        }

        Grid::new_fn(output_size, |coord| {
            if let Some(pattern_id) =
                wave.get_checked(coord).first_compatible_pattern_id()
            {
                pattern_table.colour(pattern_id, &image_grid.grid)
            } else {
                Colour {
                    r: 255,
                    g: 0,
                    b: 0,
                }
            }
        })
    };

    let output_image = ImageGrid { grid: output };
    output_image.to_image().save("/tmp/a.png").unwrap();
}
