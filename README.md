# Wave Function Collapse

A rust library for generating images which are *similar* to other images.
*Similar* is defined as both:
 - *strict locally similar*: every small (typically 3x3) pattern in the output
   image appears somewhere in the input image.
 - *loose globally similar*: the distribution of small patterns in the output
   image is roughly the same as the distribution of small patterns in the input
   image.

This is based on https://github.com/mxgmn/WaveFunctionCollapse

## Example

Running the `flowers` example generates similar images to this:

![Flowers Input](/images/flowers-input.png)

Here are some sample generated images:

![Flowers Output1](/images/flowers-output1.png)

![Flowers Output2](/images/flowers-output2.png)

![Flowers Output3](/images/flowers-output3.png)
