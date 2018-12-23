extern crate coord_2d;
extern crate direction;
extern crate grid_2d;
extern crate hashbrown;
extern crate image;
extern crate rand;
extern crate rand_xorshift;
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
    let seed: u128 = rand::thread_rng().gen();
    println!("{}", seed);
    let mut rng = rng_from_integer_seed(seed);
    let image = image::load_from_memory(include_bytes!("flowers.png")).unwrap();
    let image_grid = image_to_grid(&image);
    let pattern_size = Size::new(3, 3);
    let output_size = Size::new(48, 48);
    let start_time = ::std::time::Instant::now();
    let mut overlapping_patterns = OverlappingPatterns::new(&image_grid, pattern_size);
    let id_grid = overlapping_patterns.id_grid();
    let bottom_left_corner_coord = Coord::new(0, image_grid.size().y() as i32 - 1);
    let bottom_left_corner_id = *id_grid.get_checked(bottom_left_corner_coord);
    let sprout_id = *id_grid.get_checked(Coord::new(7, 21));
    let flower_id = *id_grid.get_checked(Coord::new(4, 1));

    overlapping_patterns
        .pattern_mut(bottom_left_corner_id)
        .clear_count();

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

            let sprout_coord = Coord::new(
                (rng.gen::<u32>() % output_size.width()) as i32,
                output_size.height() as i32 - 2,
            );

            run.forbid_all_patterns_except(sprout_coord, sprout_id)
                .unwrap();

            for i in 0..(output_size.width() as i32) {
                let coord = Coord::new(i, output_size.height() as i32 - 1);
                run.forbid_all_patterns_except(coord, bottom_left_corner_id)
                    .unwrap();
            }

            for i in 0..8 {
                for j in 0..(output_size.width() as i32) {
                    let coord = Coord::new(j, output_size.height() as i32 - 1 - i);
                    run.forbid_pattern(coord, flower_id).unwrap();
                }
            }

            'inner: loop {
                match run.step(&mut rng) {
                    Ok(Observe::Complete) => break 'generate,
                    Ok(Observe::Incomplete) => (),
                    Err(PropagateError::Contradiction) => break 'inner,
                }
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

    grid_to_image(&output).save("/tmp/a.png").unwrap();
}
