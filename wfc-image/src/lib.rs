extern crate coord_2d;
extern crate grid_2d;
extern crate image;
extern crate wfc;

use coord_2d::{Coord, Size};
use grid_2d::Grid;
use image::{DynamicImage, Rgb, RgbImage};
use wfc::overlapping::{OverlappingPatterns, Pattern};
use wfc::{GlobalStats, PatternId, Wave, WaveCellRef};

pub struct ImagePatterns {
    overlapping_patterns: OverlappingPatterns<Rgb<u8>>,
    empty_colour: Rgb<u8>,
}

impl ImagePatterns {
    pub fn new(image: &DynamicImage, pattern_size: Size) -> Self {
        let rgb_image = image.to_rgb();
        let size = Size::new(rgb_image.width(), rgb_image.height());
        let grid = Grid::new_fn(size, |Coord { x, y }| {
            *rgb_image.get_pixel(x as u32, y as u32)
        });
        let overlapping_patterns = OverlappingPatterns::new(grid, pattern_size);
        Self {
            overlapping_patterns,
            empty_colour: Rgb { data: [0, 0, 0] },
        }
    }

    pub fn set_empty_colour(&mut self, empty_colour: Rgb<u8>) {
        self.empty_colour = empty_colour;
    }

    pub fn to_image(&self) -> DynamicImage {
        let grid = self.overlapping_patterns.grid();
        let size = grid.size();
        let mut rgb_image = RgbImage::new(size.width(), size.height());
        for (Coord { x, y }, colour) in grid.enumerate() {
            rgb_image.put_pixel(x as u32, y as u32, *colour);
        }
        DynamicImage::ImageRgb8(rgb_image)
    }

    pub fn image_from_wave(&self, wave: &Wave) -> DynamicImage {
        let size = wave.grid().size();
        let mut rgb_image = RgbImage::new(size.width(), size.height());
        wave.grid().enumerate().for_each(|(Coord { x, y }, cell)| {
            let colour = match cell.chosen_pattern_id() {
                Ok(pattern_id) => {
                    *self.overlapping_patterns.pattern_top_left_value(pattern_id)
                }
                Err(_) => self.empty_colour,
            };
            rgb_image.put_pixel(x as u32, y as u32, colour);
        });
        DynamicImage::ImageRgb8(rgb_image)
    }

    pub fn weighted_average_colour<'a>(&self, cell: &'a WaveCellRef<'a>) -> Rgb<u8> {
        use wfc::EnumerateCompatiblePatternWeights::*;
        match cell.enumerate_compatible_pattern_weights() {
            MultipleCompatiblePatternsWithoutWeights | NoCompatiblePattern => {
                self.empty_colour
            }
            SingleCompatiblePatternWithoutWeight(pattern_id) => {
                *self.overlapping_patterns.pattern_top_left_value(pattern_id)
            }
            CompatiblePatternsWithWeights(iter) => {
                let (r, g, b) = iter
                    .map(|(pattern_id, weight)| {
                        let &Rgb { data: [r, g, b] } =
                            self.overlapping_patterns.pattern_top_left_value(pattern_id);
                        (r as u32 * weight, g as u32 * weight, b as u32 * weight)
                    })
                    .fold((0, 0, 0), |(acc_r, acc_g, acc_b), (r, g, b)| {
                        (acc_r + r, acc_g + g, acc_b + b)
                    });
                let total_weight = cell.sum_compatible_pattern_weight();
                Rgb {
                    data: [
                        (r / total_weight) as u8,
                        (g / total_weight) as u8,
                        (b / total_weight) as u8,
                    ],
                }
            }
        }
    }

    pub fn grid(&self) -> &Grid<Rgb<u8>> {
        self.overlapping_patterns.grid()
    }

    pub fn id_grid(&self) -> Grid<PatternId> {
        self.overlapping_patterns.id_grid()
    }

    pub fn pattern(&self, pattern_id: PatternId) -> &Pattern {
        self.overlapping_patterns.pattern(pattern_id)
    }

    pub fn pattern_mut(&mut self, pattern_id: PatternId) -> &mut Pattern {
        self.overlapping_patterns.pattern_mut(pattern_id)
    }

    pub fn global_stats(&self) -> GlobalStats {
        self.overlapping_patterns.global_stats()
    }
}
