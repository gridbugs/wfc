# Wave Function Collapse for Image Files

[![Version](https://img.shields.io/crates/v/wfc_image.svg)](https://crates.io/crates/wfc_image)
[![Documentation](https://docs.rs/wfc_image/badge.svg)](https://docs.rs/wfc_image)

A helper for [wfc](https://github.com/stevebob/wfc/tree/master/wfc) to simplify generating
images based on image files, using the [image](https://crates.io/crates/image) crate.

## Examples

Most of the sample images are taken from [mxgmn/WaveFunctionCollapse](https://github.com/mxgmn/WaveFunctionCollapse).

### Simple

This example generates an output image which is similar to the input image.

![Rooms Input](/images/rooms.png)
->
![Rooms Output1](/images/rooms-output1.png)
![Rooms Output2](/images/rooms-output2.png)

![Bricks Input](/images/bricks.png)
->
![Bricks Output1](/images/bricks-output1.png)
![Bricks Output2](/images/bricks-output2.png)

### Flowers

It's also possible to manually restrict the output to encode specific
properties. In this example:
 - The bottom row of patterns is set to be ground.
 - A sprout pattern is placed in a random position along the bottom of the
   output.
 - Ground patterns are forbidden from being automatically chosen.
 - The flower pattern is forbidden to appear in the bottom few rows of output,
   to enforce a minimum height of flowers.

![Flowers Input](/images/flowers.png)
->
![Flowers Output1](/images/flowers-output1.png)
![Flowers Output2](/images/flowers-output2.png)

Pass the flag `--animate` to view a realtime animation of the image being generated:

![Flowers Animation](/images/flowers-animate.gif)

### Animate

This is a general tool for displaying in realtime, the generation of an image
from a specified image file.

![Link Input](/images/link.png)
->
![Link Animation](/images/link-animate.gif)

![Sewers Input](/images/sewers.png)
->
![Sewers Animation](/images/sewers-animate.gif)

![Cat Input](/images/cat.png)
->
![cat Animation](/images/cat-animate.gif)

```bash
cargo run --release --example=animate -- -i examples/cat.png -x100 -y60 -p3 --forever
cargo run --release --example=animate -- -i examples/flowers.png -x100 -y60 -p3 --all-orientations --forever
```

### Anchor

Takes the tile in the bottom-right corner and forces it to appear along the entire
bottom and right sides of the output, wrapping to the top and left sides. This is
an easy way to prevent the output from wrapping. Optionally, the bottom-right corner
tile (which is often blank) can be removed from the choices of tiles for other cells.

The point of this example is to easily experiment with generating levels for roguelikes.

By default the bottom-right corner tile is removed from the possibilities for other cells:

```bash
cargo run --release --example=anchor -- -a -i examples/cat.png -o /tmp/a.png -p3 -x100 -y100
```

![cat maze](/images/cat-maze.png)

You can allow the bottom-right corner tile to appear in other tiles with the `--allow-corner` flag.

```bash
cargo run --release --example=anchor -- -a -i examples/cat.png -o /tmp/a.png -p3 -x100 -y100 --allow-corner
```

![cat maze sparse](/images/cat-maze-sparse.png)
