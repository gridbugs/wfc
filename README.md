# Wave Function Collapse

A rust library for generating images which are *similar* to other images.
*Similar* is defined as both:
 - *strict locally similar*: every small (typically 3x3) pattern in the output
   image appears somewhere in the input image.
 - *loose globally similar*: the distribution of small patterns in the output
   image is roughly the same as the distribution of small patterns in the input
   image.

This is based on https://github.com/mxgmn/WaveFunctionCollapse

## Examples

### Simple

This example generates an output image which is similar to the input image.

![Rooms Input](/examples/rooms.png)
-> 
![Rooms Output1](/images/rooms-output1.png)
![Rooms Output2](/images/rooms-output2.png)
![Rooms Output3](/images/rooms-output3.png)

![Bricks Input](/examples/bricks.png)
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

![Flowers Input](/examples/flowers.png)
->
![Flowers Output1](/images/flowers-output1.png)
![Flowers Output2](/images/flowers-output2.png)
![Flowers Output3](/images/flowers-output3.png)
