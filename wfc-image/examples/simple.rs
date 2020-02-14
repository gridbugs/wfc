use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::num::NonZeroU32;
use wfc_image::*;

fn app() -> Result<(), ()> {
    use simon::*;
    let (
        seed,
        input_path,
        output_path,
        all_orientations,
        pattern_size,
        width,
        height,
        parallel,
    ): (u64, String, String, bool, u32, u32, u32, bool) = args_all! {
        opt("s", "seed", "rng seed", "INT")
            .map(|seed| seed.unwrap_or_else(|| rand::thread_rng().gen())),
        opt("i", "input", "input path", "PATH").required(),
        opt("o", "output", "output path", "PATH").required(),
        flag("a", "all-orientations", "all orientations"),
        opt::<u32>("p", "pattern-size", "size of patterns in pixels", "INT").with_default(3),
        opt::<u32>("x", "width", "width", "INT").with_default(48),
        opt::<u32>("y", "height", "height", "INT").with_default(48),
        flag("", "parallel", "run multiple attempts in parallel"),
    }
    .with_help_default()
    .parse_env_or_exit();
    println!("seed: {}", seed);
    let orientation: &[Orientation] = if all_orientations {
        &orientation::ALL
    } else {
        &[Orientation::Original]
    };
    let input_image = image::open(input_path).unwrap();
    let output_size = Size::new(width, height);
    let mut rng = StdRng::seed_from_u64(seed);
    let start_time = ::std::time::Instant::now();
    let pattern_size =
        NonZeroU32::new(pattern_size).expect("pattern size may not be zero");
    let result = if parallel {
        generate_image_with_rng(
            &input_image,
            pattern_size,
            output_size,
            orientation,
            WrapXY,
            ForbidNothing,
            retry::ParNumTimes(10),
            &mut rng,
        )
    } else {
        generate_image_with_rng(
            &input_image,
            pattern_size,
            output_size,
            orientation,
            WrapXY,
            ForbidNothing,
            retry::NumTimes(10),
            &mut rng,
        )
    };
    match result {
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
