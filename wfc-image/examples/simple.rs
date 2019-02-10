extern crate image;
extern crate rand;
extern crate simon;
extern crate wfc_image;

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use wfc_image::*;

fn app() -> Result<(), ()> {
    use simon::*;
    let (seed, input_path, output_path, all_orientations): (u64, String, String, bool) =
        args_all! {
            opt("s", "seed", "rng seed", "INT")
                .map(|seed| seed.unwrap_or_else(|| rand::thread_rng().gen())),
            opt_required("i", "input", "input path", "PATH"),
            opt_required("o", "output", "output path", "PATH"),
            flag("a", "all-orientations", "all orientations"),
        }
        .with_help_default()
        .parse_env_default_or_exit();
    println!("seed: {}", seed);
    let orientation: &[Orientation] = if all_orientations {
        &orientation::ALL
    } else {
        &[Orientation::Original]
    };
    let input_image = image::open(input_path).unwrap();
    let pattern_size = 3;
    let output_size = Size::new(48, 48);
    let mut rng = StdRng::seed_from_u64(seed);
    let start_time = ::std::time::Instant::now();
    match generate_image_with_rng(
        &input_image,
        pattern_size,
        output_size,
        orientation,
        wrap::WrapXY,
        retry::NumTimes(10),
        &mut rng,
    ) {
        Err(_) => {
            eprintln!("Too many contradictions");
            Err(())
        }
        Ok(output_image) => {
            let end_time = ::std::time::Instant::now();
            println!("{:?}", end_time - start_time);
            output_image.save(output_path).unwrap();
            Ok(())
        }
    }
}

fn main() {
    ::std::process::exit(match app() {
        Ok(()) => 0,
        Err(()) => 1,
    })
}
