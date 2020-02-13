use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::num::NonZeroU32;
use wfc_image::*;

fn app() -> Result<(), ()> {
    use simon::*;
    let (seed, input_path, output_path, all_orientations, pattern_size, width, height): (
        u64,
        String,
        String,
        bool,
        u32,
        u32,
        u32,
    ) = args_all! {
        opt("s", "seed", "rng seed", "INT")
            .map(|seed| seed.unwrap_or_else(|| rand::thread_rng().gen())),
        opt_required("i", "input", "input path", "PATH"),
        opt_required("o", "output", "output path", "PATH"),
        flag("a", "all-orientations", "all orientations"),
        opt_default::<u32>("p", "pattern-size", "size of patterns in pixels", "INT",  3),
        opt_default::<u32>("x", "width", "width", "INT", 48),
        opt_default::<u32>("y", "height", "height", "INT", 48),
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
    let output_size = Size::new(width, height);
    let mut rng = StdRng::seed_from_u64(seed);
    let start_time = ::std::time::Instant::now();
    let pattern_size =
        NonZeroU32::new(pattern_size).expect("pattern size may not be zero");
    match generate_image_with_rng(
        &input_image,
        pattern_size,
        output_size,
        orientation,
        WrapXY,
        ForbidNothing,
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
