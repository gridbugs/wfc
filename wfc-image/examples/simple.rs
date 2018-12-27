extern crate coord_2d;
extern crate image;
extern crate rand;
extern crate rand_xorshift;
extern crate simon;
extern crate wfc;
extern crate wfc_image;

use coord_2d::Size;
use rand::{Rng, SeedableRng};
use rand_xorshift::XorShiftRng;
use wfc::wrap::*;
use wfc::*;
use wfc_image::ImagePatterns;

fn rng_from_integer_seed(seed: u128) -> XorShiftRng {
    let mut seed_array = [0; 16];
    seed_array.iter_mut().enumerate().for_each(|(i, part)| {
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
    }
    .with_help_default()
    .parse_env_default_or_exit();
    println!("seed: {}", seed);
    let image = image::open(input_path).unwrap();
    let mut rng = rng_from_integer_seed(seed);
    let pattern_size = Size::new(3, 3);
    let image_patterns = ImagePatterns::new(&image, pattern_size);
    let output_size = Size::new(48, 48);
    let start_time = ::std::time::Instant::now();
    let wave = {
        let global_stats = image_patterns.global_stats();
        let mut wave = Wave::new(output_size);
        'generate: loop {
            let mut context = Context::new();
            let mut run =
                Run::new(&mut context, &mut wave, &global_stats, WrapXY, &mut rng);
            match run.collapse(&mut rng) {
                Ok(()) => break,
                Err(PropagateError::Contradiction) => continue,
            }
        }
        let end_time = ::std::time::Instant::now();
        println!("{:?}", end_time - start_time);
        wave
    };
    image_patterns
        .image_from_wave(&wave)
        .save(output_path)
        .unwrap();
}
