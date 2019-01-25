extern crate image;
extern crate wfc_image;

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
    let output_size = OutputSize(Size::new(48, 48));
    let pattern_size = PatternSize(Size::new(3, 3));
    let output_image = wfc_image::generate_image(
        &input_image,
        pattern_size,
        output_size,
        &[Orientation::Original],
        wrap::WrapXY,
        retry::Forever,
    );
    output_image.save(output_path).unwrap();
}
