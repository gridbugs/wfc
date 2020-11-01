use animation_helper::WindowPixels;
use coord_2d::{Coord, Size};
use rand::{Rng, SeedableRng};
use rand_xorshift::XorShiftRng;
use std::num::NonZeroU32;
use wfc::*;
use wfc_image::ImagePatterns;

struct Forbid {
    bottom_left_corner_id: PatternId,
    flower_id: PatternId,
    sprout_coord: Coord,
    sprout_id: PatternId,
}

impl ForbidPattern for Forbid {
    fn forbid<W: Wrap, R: Rng>(&mut self, fi: &mut ForbidInterface<W>, rng: &mut R) {
        fi.forbid_all_patterns_except(self.sprout_coord, self.sprout_id, rng)
            .unwrap();
        let output_size = fi.wave_size();
        for i in 0..(output_size.width() as i32) {
            let coord = Coord::new(i, output_size.height() as i32 - 1);
            fi.forbid_all_patterns_except(coord, self.bottom_left_corner_id, rng)
                .unwrap();
        }
        for i in 0..8 {
            for j in 0..(output_size.width() as i32) {
                let coord = Coord::new(j, output_size.height() as i32 - 1 - i);
                fi.forbid_pattern(coord, self.flower_id, rng).unwrap();
            }
        }
    }
}

fn main() {
    let (seed_opt, output_path, animate) = meap::all! {
        opt_opt("INT", 's').name("seed").desc("rng seed"),
        opt_opt::<String, _>("PATH", 'o').name("output").desc("output path"),
        flag('a').name("animate"),
    }
    .with_help_default()
    .parse_env_or_exit();
    let seed = seed_opt.unwrap_or_else(|| rand::thread_rng().gen());
    println!("seed: {}", seed);
    let grid_size = Size::new(48, 48);
    let pixel_size = Size::new(8, 8);
    let mut window_pixels = if animate {
        Some(WindowPixels::new(grid_size, pixel_size))
    } else {
        None
    };
    let mut rng = XorShiftRng::seed_from_u64(seed);
    let image = image::load_from_memory(include_bytes!("flowers.png")).unwrap();
    let pattern_size = NonZeroU32::new(3).unwrap();
    let mut image_patterns =
        ImagePatterns::new(&image, pattern_size, &[Orientation::Original]);
    let start_time = ::std::time::Instant::now();
    let id_grid = image_patterns.id_grid_original_orientation();
    let bottom_left_corner_coord =
        Coord::new(0, image_patterns.grid().size().y() as i32 - 1);
    let bottom_left_corner_id = *id_grid.get_checked(bottom_left_corner_coord);
    let sprout_id = *id_grid.get_checked(Coord::new(7, 21));
    let flower_id = *id_grid.get_checked(Coord::new(4, 1));
    image_patterns
        .pattern_mut(bottom_left_corner_id)
        .clear_count();
    let wave = {
        let global_stats = image_patterns.global_stats();
        let mut wave = Wave::new(grid_size);
        'generate: loop {
            let mut context = Context::new();
            let sprout_coord = Coord::new(
                (rng.gen::<u32>() % grid_size.width()) as i32,
                grid_size.height() as i32 - 2,
            );
            let forbid = Forbid {
                bottom_left_corner_id,
                flower_id,
                sprout_coord,
                sprout_id,
            };
            let mut run = RunBorrow::new_forbid(
                &mut context,
                &mut wave,
                &global_stats,
                forbid,
                &mut rng,
            );
            'inner: loop {
                match run.step(&mut rng) {
                    Ok(observe) => {
                        if let Some(window_pixels) = window_pixels.as_mut() {
                            window_pixels.draw(run.wave_cell_ref_iter(), &image_patterns);
                        }
                        match observe {
                            Observe::Complete => break 'generate,
                            Observe::Incomplete => (),
                        }
                    }
                    Err(PropagateError::Contradiction) => break 'inner,
                }
            }
        }
        let end_time = ::std::time::Instant::now();
        println!("{:?}", end_time - start_time);
        wave
    };
    if let Some(output_path) = output_path.as_ref() {
        image_patterns
            .image_from_wave(&wave)
            .save(output_path)
            .unwrap();
    }
}
