extern crate coord_2d;
extern crate grid_2d;
extern crate image;
extern crate rand;
extern crate rand_xorshift;
#[macro_use]
extern crate simon;
extern crate wfc;

use coord_2d::{Coord, Size};
use grid_2d::Grid;
use image::{DynamicImage, Rgb, RgbImage};
use rand::{Rng, SeedableRng};
use rand_xorshift::XorShiftRng;
use wfc::overlapping::OverlappingPatterns;
use wfc::wrap::*;
use wfc::*;

fn image_to_grid(image: &DynamicImage) -> Grid<Rgb<u8>> {
    let rgb_image = image.to_rgb();
    let size = Size::new(rgb_image.width(), rgb_image.height());
    Grid::new_fn(size, |Coord { x, y }| {
        *rgb_image.get_pixel(x as u32, y as u32)
    })
}

fn grid_to_image(grid: &Grid<Rgb<u8>>) -> DynamicImage {
    let size = grid.size();
    let mut rgb_image = RgbImage::new(size.width(), size.height());
    for (Coord { x, y }, colour) in grid.enumerate() {
        rgb_image.put_pixel(x as u32, y as u32, *colour);
    }
    DynamicImage::ImageRgb8(rgb_image)
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
    use simon::*;
    let (seed, input_path, output_path): (u128, String, String) = args_all! {
        opt("s", "seed", "rng seed", "INT")
            .map(|seed| seed.unwrap_or_else(|| rand::thread_rng().gen())),
        opt_required("i", "input", "input path", "PATH"),
        opt_required("o", "output", "output path", "PATH"),
    }.with_help_default()
        .parse_env_default_or_exit();
    println!("seed: {}", seed);
    let image = image::open(input_path).unwrap();
    let mut rng = rng_from_integer_seed(seed);
    let image_grid = image_to_grid(&image);
    let pattern_size = Size::new(3, 3);
    let output_size = Size::new(48, 48);
    let start_time = ::std::time::Instant::now();
    let mut overlapping_patterns = OverlappingPatterns::new(&image_grid, pattern_size);
    let output = {
        let global_stats = overlapping_patterns.global_stats();
        let mut wave = Wave::new(output_size);
        'generate: loop {
            let mut context = Context::new();
            let mut run = Run::new(
                &mut context,
                &mut wave,
                &global_stats,
                WrapXY,
                &mut rng,
            );
            match run.collapse(&mut rng) {
                Ok(()) => break,
                Err(PropagateError::Contradiction) => continue,
            }
        }
        let end_time = ::std::time::Instant::now();
        println!("{:?}", end_time - start_time);
        Grid::new_fn(output_size, |coord| {
            if let Ok(pattern_id) = wave.get_checked(coord).chosen_pattern_id() {
                image_grid
                    .get_checked(overlapping_patterns.pattern(pattern_id).coord())
                    .clone()
            } else {
                Rgb {
                    data: [255, 0, 0],
                }
            }
        })
    };
    grid_to_image(&output).save(output_path).unwrap();
}
