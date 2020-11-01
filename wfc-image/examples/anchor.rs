use meap::Parser;
use rand::{Rng, SeedableRng};
use rand_xorshift::XorShiftRng;
use std::collections::HashSet;
use std::num::NonZeroU32;
use wfc::retry::*;
use wfc::*;
use wfc_image::*;

struct Args {
    output_size: Size,
    pattern_size: u32,
    seed: u64,
    input_image: image::DynamicImage,
    output_path: String,
    orientations: &'static [orientation::Orientation],
    retries: usize,
    allow_corner: bool,
}

impl Args {
    fn parser() -> impl Parser<Item = Self> {
        meap::let_map! {
            let {
                width = opt_opt("INT", 'x').name("width").desc("output width").with_default(48);
                height = opt_opt("INT", 'y').name("height").desc("output height").with_default(48);
                pattern_size = opt_opt("INT", 'p').name("pattern-size").desc("pattern size").with_default(3);
                seed_opt = opt_opt("INT", 's').name("seed").desc("rng seed");
                input_path = opt_req::<String, _>("PATH", 'i').name("input-path").desc("input path");
                output_path = opt_req::<String, _>("PATH", 'o').name("output-path").desc("output path");
                all_orientations = flag('a').name("all-orientations").desc("include all orientations");
                retries = opt_opt("INT", 'r').name("retries").desc("number of retries").with_default(10);
                allow_corner = flag('c').name("allow-corner").desc("allow bottom right corner");
            } in {
                Self {
                    output_size: Size::new(width, height),
                    pattern_size,
                    seed: seed_opt.unwrap_or_else(|| rand::thread_rng().gen()),
                    input_image: image::open(input_path).unwrap(),
                    output_path,
                    orientations: if all_orientations {
                        &orientation::ALL
                    } else {
                        &[Orientation::Original]
                    },
                    retries,
                    allow_corner,
                }
            }
        }
    }
}

struct Forbid {
    pattern_ids: HashSet<PatternId>,
    offset: i32,
}

impl ForbidPattern for Forbid {
    fn forbid<W: Wrap, R: Rng>(&mut self, fi: &mut ForbidInterface<W>, rng: &mut R) {
        let output_size = fi.wave_size();
        (0..(output_size.width() as i32))
            .map(|x| Coord::new(x, output_size.height() as i32 - self.offset as i32))
            .chain(
                (0..(output_size.width() as i32)).map(|y| {
                    Coord::new(output_size.width() as i32 - self.offset as i32, y)
                }),
            )
            .for_each(|coord| {
                self.pattern_ids.iter().for_each(|&pattern_id| {
                    fi.forbid_all_patterns_except(coord, pattern_id, rng)
                        .unwrap();
                });
            });
    }
}

fn app(args: Args) -> Result<(), ()> {
    println!("{}", args.seed);
    let mut rng = XorShiftRng::seed_from_u64(args.seed);
    let mut image_patterns = ImagePatterns::new(
        &args.input_image,
        NonZeroU32::new(args.pattern_size).expect("pattern size may not be zero"),
        args.orientations,
    );
    let input_size = image_patterns.grid().size();
    let bottom_right_offset = args.pattern_size - (args.pattern_size / 2);
    let id_grid = image_patterns.id_grid();
    let bottom_right_coord = Coord::new(
        input_size.width() as i32 - bottom_right_offset as i32,
        input_size.height() as i32 - bottom_right_offset as i32,
    );
    let bottom_right_ids = id_grid
        .get_checked(bottom_right_coord)
        .iter()
        .cloned()
        .collect::<HashSet<_>>();
    if !args.allow_corner {
        bottom_right_ids.iter().for_each(|&pattern_id| {
            image_patterns.pattern_mut(pattern_id).clear_count();
        });
    }
    let global_stats = image_patterns.global_stats();
    let mut wave = Wave::new(args.output_size);
    let mut context = Context::new();
    let result = {
        let forbid = Forbid {
            pattern_ids: bottom_right_ids,
            offset: bottom_right_offset as i32,
        };
        let mut run = RunBorrow::new_forbid(
            &mut context,
            &mut wave,
            &global_stats,
            forbid,
            &mut rng,
        );
        run.collapse_retrying(NumTimes(args.retries), &mut rng)
    };
    match result {
        Err(_) => {
            eprintln!("Too many contradictions!");
            Err(())
        }
        Ok(()) => {
            image_patterns
                .image_from_wave(&wave)
                .save(args.output_path)
                .unwrap();
            Ok(())
        }
    }
}

fn main() {
    let args = Args::parser().with_help_default().parse_env_or_exit();
    ::std::process::exit(match app(args) {
        Ok(()) => 0,
        Err(()) => 1,
    })
}
