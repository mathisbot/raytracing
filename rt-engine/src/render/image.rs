use std::io::BufWriter;
use std::{path::PathBuf, sync::Arc};

use png;

use vulkano::image::view::ImageView;
use vulkano::sync::GpuFuture;

pub struct Image {
    path: PathBuf,
    width: u32,
    height: u32,
    inner_buffer: [Arc<ImageView>; 1],
}

impl Image {
    pub fn new(image_descriptor: &ImageDescriptor) -> Self {
        let ImageDescriptor {
            path,
            width,
            height,
        } = image_descriptor;

        let image_view = todo!();

        Self {
            path: path.clone(),
            width: *width,
            height: *height,
            inner_buffer: [image_view],
        }
    }
}

impl super::RenderSurface for Image {
    fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    fn views(&self) -> &[Arc<ImageView>] {
        &self.inner_buffer
    }

    fn acquire(&mut self) -> Result<(u32, Box<dyn vulkano::sync::GpuFuture>), super::AcquireError> {
        unimplemented!()
    }

    fn present(
        &mut self,
        render_future: Box<dyn vulkano::sync::GpuFuture>,
        queue: &std::sync::Arc<vulkano::device::Queue>,
    ) -> Result<(), super::PresentError> {
        let render_future = render_future.then_signal_fence_and_flush().unwrap();

        let file = std::fs::File::create(&self.path).unwrap();
        let writer = &mut BufWriter::new(file);

        let mut encoder = png::Encoder::new(writer, self.width, self.height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);

        let mut writer = encoder.write_header().unwrap();

        render_future.wait(None).unwrap();

        // retrieve the image data from the inner_buffer
        let data = todo!();

        writer.write_image_data(data).unwrap();

        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct ImageDescriptor {
    pub path: PathBuf,
    pub width: u32,
    pub height: u32,
}
