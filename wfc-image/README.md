# Wave Function Collapse for Image Files

A helper for [wfc](https://github.com/stevebob/wfc/tree/master/wfc) to simplify generating
images based on image files, using the [image](https://crates.io/crates/image) crate.

## Examples

### Simple

This example generates an output image which is similar to the input image.

![Rooms Input](/images/rooms.png)
->
![Rooms Output1](/images/rooms-output1.png)
![Rooms Output2](/images/rooms-output2.png)
![Rooms Output3](/images/rooms-output3.png)

![Bricks Input](/images/bricks.png)
->
![Bricks Output1](/images/bricks-output1.png)
![Bricks Output2](/images/bricks-output2.png)
![Bricks Output3](/images/bricks-output3.png)


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
![Flowers Output3](/images/flowers-output3.png)

#### Animation

Pass the flag `--animate` to view a realtime animation of the image being generated:

![Flowers Animation](/images/flowers-animation.gif)
