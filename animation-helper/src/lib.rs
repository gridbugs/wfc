use coord_2d::Size;
use wfc::WaveCellRef;
use wfc_image::ImagePatterns;

pub struct WindowPixels {
    _window: winit::window::Window,
    pixels: pixels::Pixels,
}

impl WindowPixels {
    pub fn new(grid_size: Size, pixel_size: Size) -> Self {
        let event_loop = winit::event_loop::EventLoop::new();
        let size = winit::dpi::LogicalSize::new(
            grid_size.width() * pixel_size.width(),
            grid_size.height() * pixel_size.height(),
        );
        let window = winit::window::WindowBuilder::new()
            .with_inner_size(size)
            .with_min_inner_size(size)
            .with_max_inner_size(size)
            .build(&event_loop)
            .unwrap();
        let hidpi_factor = window.scale_factor();
        let p_size = size.to_physical::<f64>(hidpi_factor);
        let surface_texture = pixels::SurfaceTexture::new(
            p_size.width.round() as u32,
            p_size.height.round() as u32,
            &window,
        );
        let pixels =
            pixels::Pixels::new(grid_size.width(), grid_size.height(), surface_texture)
                .unwrap();
        Self {
            _window: window,
            pixels,
        }
    }

    pub fn draw<'a>(
        &mut self,
        cells: impl Iterator<Item = WaveCellRef<'a>>,
        image_patterns: &ImagePatterns,
    ) {
        let frame = self.pixels.get_frame_mut();
        for (cell, pixel) in cells.zip(frame.chunks_exact_mut(4)) {
            let [r, g, b, a] = image_patterns.weighted_average_colour(&cell).0;
            pixel[0] = r;
            pixel[1] = g;
            pixel[2] = b;
            pixel[3] = a;
        }
        let _ = self.pixels.render();
    }
}
