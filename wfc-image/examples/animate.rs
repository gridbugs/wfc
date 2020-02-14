use coord_2d::Coord;
use pixel_grid::{Window, WindowSpec};
use rand::{Rng, SeedableRng};
use rand_xorshift::XorShiftRng;
use std::num::NonZeroU32;
use std::thread;
use std::time::Duration;
use wfc::wrap::*;
use wfc::*;
use wfc_image::{ImagePatterns, Size};

struct Forbid {
    wrapped_top_left_corner_id: Option<PatternId>,
    bottom_left_corner_id: Option<PatternId>,
    pattern_size: u32,
}

impl ForbidPattern for Forbid {
    fn forbid<W: Wrap, R: Rng>(&mut self, fi: &mut ForbidInterface<W>, rng: &mut R) {
        let output_size = fi.wave_size();
        if let Some(bottom_left_corner_id) = self.bottom_left_corner_id {
            for i in 0..(output_size.width() as i32) {
                let coord = Coord::new(i, output_size.height() as i32 - 1);
                fi.forbid_all_patterns_except(coord, bottom_left_corner_id, rng)
                    .unwrap();
            }
        }
        if let Some(wrapped_top_left_corner_id) = self.wrapped_top_left_corner_id {
            for i in 0..(output_size.width() as i32) {
                let coord = Coord::new(
                    i,
                    output_size.height() as i32 - self.pattern_size as i32 + 1,
                );
                fi.forbid_all_patterns_except(coord, wrapped_top_left_corner_id, rng)
                    .unwrap();
            }
        }
    }
}

fn main() {
    use simon::*;
    let (
        seed,
        input_path,
        forever,
        anchor_top,
        anchor_bottom,
        width,
        height,
        delay,
        pattern_size,
        all_orientations,
    ) = args_all! {
        opt("s", "seed", "rng seed", "INT")
            .map(|seed| seed.unwrap_or_else(|| rand::thread_rng().gen())),
        opt::<String>("i", "input", "input path", "PATH").required(),
        flag("f", "forever", "repeat forever"),
        flag("t", "anchor-top", "anchor top"),
        flag("b", "anchor-bottom", "anchor bottom"),
        opt::<u32>("x", "width", "width", "INT").with_default(48),
        opt::<u32>("y", "height", "height", "INT").with_default(48),
        opt::<u64>("d", "delay", "delay between steps", "MS"),
        opt::<u32>("p", "pattern-size", "size of patterns in pixels", "INT").with_default(3),
        flag("a", "all-orientations", "all orientations"),
    }
    .with_help_default()
    .parse_env_or_exit();
    if (anchor_top || anchor_bottom) && all_orientations {
        eprintln!("Can't anchor with all orientations");
        ::std::process::exit(1);
    }
    println!("seed: {}", seed);
    let orientation: &[Orientation] = if all_orientations {
        &orientation::ALL
    } else {
        &[Orientation::Original]
    };
    let image = image::open(input_path).unwrap();
    let output_size = Size::new(width, height);
    let mut image_patterns = ImagePatterns::new(
        &image,
        NonZeroU32::new(pattern_size).expect("pattern size may not be zero"),
        orientation,
    );
    let input_size = image_patterns.grid().size();
    let id_grid = image_patterns.id_grid_original_orientation();
    let bottom_left_corner_id = if anchor_bottom {
        let coord = Coord::new(0, input_size.y() as i32 - 1);
        let pattern_id = *id_grid.get_checked(coord);
        image_patterns.pattern_mut(pattern_id).clear_count();
        Some(pattern_id)
    } else {
        None
    };
    let wrapped_top_left_corner_id = if anchor_top {
        let coord = Coord::new(0, input_size.y() as i32 - pattern_size as i32 + 1);
        let pattern_id = *id_grid.get_checked(coord);
        image_patterns.pattern_mut(pattern_id).clear_count();
        Some(pattern_id)
    } else {
        None
    };
    let mut rng = XorShiftRng::seed_from_u64(seed);
    let window_spec = WindowSpec {
        title: "animate".to_string(),
        grid_size: output_size,
        cell_size: Size::new(8, 8),
    };
    let mut window = Window::new(window_spec);
    let global_stats = image_patterns.global_stats();
    let mut wave = Wave::new(output_size);
    let mut context = Context::new();
    let delay = delay.map(Duration::from_millis);
    'generate: loop {
        let forbid = Forbid {
            bottom_left_corner_id,
            wrapped_top_left_corner_id,
            pattern_size,
        };
        let mut run = RunBorrow::new_forbid(
            &mut context,
            &mut wave,
            &global_stats,
            forbid,
            &mut rng,
        );
        'inner: loop {
            window.with_pixel_grid(|mut pixel_grid| {
                run.wave_cell_ref_iter()
                    .zip(pixel_grid.iter_mut())
                    .for_each(|(cell, mut pixel)| {
                        let colour = image_patterns.weighted_average_colour(&cell);
                        pixel.set_colour_array_rgba_u8(colour.0);
                    });
            });
            window.draw();
            if let Some(delay) = delay {
                thread::sleep(delay);
            }
            if window.is_closed() {
                return;
            }
            match run.step(&mut rng) {
                Ok(observe) => match observe {
                    Observe::Complete => {
                        if forever {
                            continue 'generate;
                        } else {
                            break 'generate;
                        }
                    }
                    Observe::Incomplete => (),
                },
                Err(PropagateError::Contradiction) => break 'inner,
            }
        }
    }
}
