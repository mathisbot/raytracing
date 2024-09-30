use std::io::BufWriter;
use std::{path::PathBuf, sync::Arc};

use png;

use vulkano::buffer::Subbuffer;
use vulkano::command_buffer::allocator::StandardCommandBufferAllocator;
use vulkano::command_buffer::{CopyImageToBufferInfo, PrimaryAutoCommandBuffer};
use vulkano::device::Queue;
use vulkano::image::view::{ImageView, ImageViewCreateInfo};
use vulkano::image::{ImageCreateInfo, ImageUsage};
use vulkano::memory::allocator::{AllocationCreateInfo, StandardMemoryAllocator};
use vulkano::sync::{self, GpuFuture};

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
    image_view: [Arc<ImageView>; 1],
    /// CPU accessible buffer
    inner_buffer: Subbuffer<[u8]>,
    /// Transfer queue will be used to copy the image to the buffer
    compute_queue: Arc<Queue>,
    /// Command buffer used to copy the image to the buffer
    command_buffer: Arc<PrimaryAutoCommandBuffer<Arc<StandardCommandBufferAllocator>>>,
    /// Used to benchmark the rendering time.
    start_time: std::time::Instant,
}

impl Image {
    #[must_use]
    /// Creates a new image from the given image descriptor.
    ///
    /// ## Panics
    ///
    /// This function will panic if the inner image / buffer creation fails.
    pub fn new(
        image_descriptor: &ImageDescriptor,
        memory_allocator: Arc<StandardMemoryAllocator>,
        command_buffer_allocator: &Arc<StandardCommandBufferAllocator>,
        compute_queue: Arc<Queue>,
    ) -> Self {
        let ImageDescriptor {
            path,
            width,
            height,
        } = image_descriptor;

        let image = vulkano::image::Image::new(
            memory_allocator.clone(),
            ImageCreateInfo {
                format: vulkano::format::Format::R8G8B8A8_UNORM,
                extent: [*width, *height, 1],
                usage: ImageUsage::TRANSFER_SRC | ImageUsage::STORAGE,
                ..Default::default()
            },
            AllocationCreateInfo::default(),
        )
        .unwrap();

        let image_view = ImageView::new(image.clone(), ImageViewCreateInfo::from_image(&image))
            .expect("Could not create image view");

        let inner_buffer = vulkano::buffer::Buffer::new_unsized(
            memory_allocator,
            vulkano::buffer::BufferCreateInfo {
                usage: vulkano::buffer::BufferUsage::TRANSFER_DST,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: vulkano::memory::allocator::MemoryTypeFilter::PREFER_HOST
                    | vulkano::memory::allocator::MemoryTypeFilter::HOST_RANDOM_ACCESS,
                ..Default::default()
            },
            u64::from(*width)
                * u64::from(*height)
                * vulkano::format::Format::R8G8B8A8_UNORM.block_size()
                * size_of::<u8>() as u64,
        )
        .unwrap();

        let command_buffer = {
            let mut builder = vulkano::command_buffer::AutoCommandBufferBuilder::primary(
                command_buffer_allocator,
                compute_queue.queue_family_index(),
                vulkano::command_buffer::CommandBufferUsage::OneTimeSubmit,
            )
            .unwrap();

            builder
                .copy_image_to_buffer(CopyImageToBufferInfo::image_buffer(
                    image,
                    inner_buffer.clone(),
                ))
                .unwrap();

            builder.build().unwrap()
        };

        Self {
            path: path.clone(),
            width: *width,
            height: *height,
            image_view: [image_view],
            inner_buffer,
            compute_queue,
            command_buffer,
            start_time: std::time::Instant::now(),
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
        &self.image_view
    }

    #[must_use = "The function returns a future that must be awaited"]
    #[inline]
    fn acquire(&mut self) -> Result<(u32, Box<dyn vulkano::sync::GpuFuture>), super::AcquireError> {
        Ok((0, Box::new(sync::now(self.compute_queue.device().clone()))))
    }

    fn present(
        &mut self,
        render_future: Box<dyn vulkano::sync::GpuFuture>,
        _queue: &std::sync::Arc<vulkano::device::Queue>,
    ) -> Result<(), super::PresentError> {
        let future = render_future.then_signal_fence_and_flush();

        match future.map_err(vulkano::Validated::unwrap) {
            Ok(future) => {
                let future = future
                    .then_execute(self.compute_queue.clone(), self.command_buffer.clone())
                    .unwrap()
                    .then_signal_fence_and_flush()
                    .unwrap();

                let file = std::fs::File::create(&self.path).unwrap();
                let file_writer = &mut BufWriter::new(file);

                let mut encoder = png::Encoder::new(file_writer, self.width, self.height);
                encoder.set_color(png::ColorType::Rgba);
                encoder.set_depth(png::BitDepth::Eight);

                let mut png_writer = encoder.write_header().unwrap();

                future.wait(None).unwrap();

                let reader = self.inner_buffer.read().unwrap();

                png_writer.write_image_data(&reader).unwrap();

                let elapsed = self.start_time.elapsed();
                tracing::info!(
                    "Image succesfully rendered and saved to {:?} in {:?}.",
                    self.path,
                    elapsed
                );

                Ok(())
            }
            Err(e) => {
                tracing::error!("Failed to flush rendering future: {e}");
                Err(super::PresentError)
            }
        }
    }
}

#[allow(clippy::module_name_repetitions)]
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
