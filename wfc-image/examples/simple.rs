extern crate image;
extern crate rand;
extern crate rand_xorshift;
extern crate simon;
extern crate wfc_image;

use rand::{Rng, SeedableRng};
use rand_xorshift::XorShiftRng;
use wfc_image::*;

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
    let input_image = image::open(input_path).unwrap();
    let pattern_size = PatternSize(Size::new(3, 3));
    let output_size = OutputSize(Size::new(48, 48));
    let mut rng = rng_from_integer_seed(seed);
    let start_time = ::std::time::Instant::now();
    let output_image = generate_image_with_rng(
        &input_image,
        pattern_size,
        output_size,
        wrap::WrapXY,
        retry::Forever,
        &mut rng,
    );
    let end_time = ::std::time::Instant::now();
    println!("{:?}", end_time - start_time);
    output_image.save(output_path).unwrap();
}
