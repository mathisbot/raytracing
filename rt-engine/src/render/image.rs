use std::io::BufWriter;
use std::{path::PathBuf, sync::Arc};

use png;

use vulkano::image::view::ImageView;
use vulkano::sync::GpuFuture;

/// Represents an image that can be rendered to.
///
/// The image is saved to the disk when `present` is called.
pub struct Image {
    /// Used to save the image to the disk.
    path: PathBuf,
    /// The width of the image.
    width: u32,
    /// The height of the image.
    height: u32,
    /// The internal image view of the image.
    inner_buffer: [Arc<ImageView>; 1],
}

impl Image {
    #[must_use]
    /// Creates a new image from the given image descriptor.
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
    #[must_use]
    #[inline]
    fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    #[must_use]
    #[inline]
    fn views(&self) -> &[Arc<ImageView>] {
        &self.inner_buffer
    }

    #[must_use]
    fn acquire(&mut self) -> Result<(u32, Box<dyn vulkano::sync::GpuFuture>), super::AcquireError> {
        // TODO: acquire the image view
        todo!("acquire the image view");
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

        // TODO: retrieve the image data from the inner_buffer
        let data = todo!();

        writer.write_image_data(data).unwrap();

        Ok(())
    }
}

#[derive(Clone, Debug)]
/// Represents an image descriptor.
pub struct ImageDescriptor {
    /// The path to save the image to.
    pub path: PathBuf,
    /// The width of the image.
    pub width: u32,
    /// The height of the image.
    pub height: u32,
}
