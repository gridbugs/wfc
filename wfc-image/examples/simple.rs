use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::num::NonZeroU32;
use wfc_image::*;

fn app() -> Result<(), ()> {
    let (
        seed_opt,
        input_path,
        output_path,
        all_orientations,
        pattern_size,
        width,
        height,
        parallel,
    ) = meap::all! {
        opt_opt("INT", 's').name("seed").desc("rng seed"),
        opt_req::<String, _>("PATH", 'i').name("input").desc("input path"),
        opt_req::<String, _>("PATH", 'o').name("output").desc("output path"),
        flag('a').name("all-orientations").desc("all orientations"),
        opt_opt::<u32, _>("INT", 'p').name("pattern-size").desc("size of patterns in pixels").with_default(3),
        opt_opt::<u32, _>("INT", 'x').name("width").desc("width").with_default(48),
        opt_opt::<u32, _>("INT", 'y').name("height").desc("height").with_default(48),
        flag("parallel").desc("run multiple attempts in parallel"),
    }
    .with_help_default()
    .parse_env_or_exit();
    let seed = seed_opt.unwrap_or_else(|| rand::thread_rng().gen());
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
        #[cfg(feature = "parallel")]
        {
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
        }
        #[cfg(not(feature = "parallel"))]
        panic!("Recompile with `--features=parallel` to enable parallel retry")
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
