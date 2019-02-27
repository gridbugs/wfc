extern crate image;
extern crate wfc_image;

use std::num::NonZeroU32;
use wfc_image::*;

fn main() {
    let args = ::std::env::args().collect::<Vec<_>>();
    if args.len() != 3 {
        println!("usage: {} INPUT_PATH OUTPUT_PATH", args[0]);
        ::std::process::exit(1);
    }
    let input_path = &args[1];
    let output_path = &args[2];
    let input_image = image::open(input_path).unwrap();
    let output_size = Size::new(48, 48);
    let pattern_size = NonZeroU32::new(3).unwrap();
    let output_image = wfc_image::generate_image(
        &input_image,
        pattern_size,
        output_size,
        &[Orientation::Original],
        WrapXY,
        ForbidNothing,
        retry::NumTimes(10),
    )
    .expect("Too many contradictions");
    output_image.save(output_path).expect("Failed to save");
}
