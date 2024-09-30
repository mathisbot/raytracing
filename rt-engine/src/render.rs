//! This module, aside from defining the `Renderer` struct, also defines the `RenderSurface` trait.
//!
//! The latter is used to abstract the rendering surface, which can be any surface suitable for rendering.

use std::sync::Arc;
use vulkano::{
    buffer::Subbuffer,
    command_buffer::{self, allocator::StandardCommandBufferAllocator, AutoCommandBufferBuilder},
    descriptor_set::{
        allocator::StandardDescriptorSetAllocator, PersistentDescriptorSet, WriteDescriptorSet,
    },
    device::{Device, Queue},
    image::view::ImageView,
    pipeline::{
        compute::ComputePipelineCreateInfo, layout::PipelineDescriptorSetLayoutCreateInfo,
        ComputePipeline, Pipeline, PipelineLayout, PipelineShaderStageCreateInfo,
    },
    sync::GpuFuture,
};

#[cfg(feature = "image")]
pub mod image;
pub mod window;

#[derive(Copy, Clone, Debug)]
pub struct AcquireError;
#[derive(Copy, Clone, Debug)]
pub struct PresentError;

#[allow(clippy::module_name_repetitions)]
/// The type of a render command buffer.
pub type RenderCommandBuffer =
    Arc<vulkano::command_buffer::PrimaryAutoCommandBuffer<Arc<StandardCommandBufferAllocator>>>;

#[allow(clippy::module_name_repetitions)]
// TODO: enum render surface "one time" and "swapchain" ?
// Or add init function to render surface?
/// Represents a render surface.
pub trait RenderSurface {
    /// Returns the size of the render surface.
    fn size(&self) -> (u32, u32);
    /// Returns the views of the render surface.
    ///
    /// Views must be in the same order as the one used for indexing when returning index from `acquire()`.
    /// This function is used to generate command buffers.
    fn views(&self) -> &[Arc<ImageView>];
    /// Acquires the next image view.
    ///
    /// The returned index must be using the same order as the one used for `views()`.
    ///
    /// ## Errors
    ///
    /// This function returns an error if the image view cannot be acquired.
    fn acquire(&mut self) -> Result<(u32, Box<dyn vulkano::sync::GpuFuture>), AcquireError>;
    /// Presents the rendered image.
    ///
    /// ## Errors
    ///
    /// This function returns an error if the image cannot be presented.
    fn present(
        &mut self,
        render_future: Box<dyn vulkano::sync::GpuFuture>,
        queue: &Arc<Queue>,
    ) -> Result<(), PresentError>;
}

#[derive(Clone)]
/// Represents the buffers used by the renderer.
pub struct Buffers {
    /// The camera uniform buffer.
    pub camera_uniform: Subbuffer<crate::shader::CameraBuffer>,
    /// The triangles buffer.
    pub triangles_buffer: Subbuffer<crate::shader::TrianglesBuffer>,
    /// The materials buffer.
    pub materials_buffer: Subbuffer<crate::shader::Materials>,
    /// The models buffer.
    pub models_buffer: Subbuffer<crate::shader::ModelsBuffer>,
    /// The BVHs buffer.
    pub bvhs_buffer: Subbuffer<crate::shader::BvhBuffer>,
}

/// Represents a renderer.
pub struct Renderer {
    /// The queue used by the renderer.
    queue: Arc<Queue>,
    /// The compute pipeline used by the renderer.
    pipeline: Arc<ComputePipeline>,
    /// The render surface used by the renderer.
    render_surface: Box<dyn RenderSurface>,
    /// The render command buffers used by the renderer.
    render_command_buffers: Box<[RenderCommandBuffer]>,
}

impl Renderer {
    #[must_use]
    /// Creates a new renderer.
    ///
    /// ## Panics
    ///
    /// This function panics if the renderer cannot be created, typically due to pipeline creation failure.
    pub fn new(
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        descriptor_set_allocator: &Arc<StandardDescriptorSetAllocator>,
        command_buffer_allocator: &Arc<StandardCommandBufferAllocator>,
        render_surface: Box<dyn RenderSurface>,
        buffers: &Buffers,
    ) -> Self {
        let (width, height) = render_surface.size();

        let pipeline = {
            let stage = {
                let shader = crate::shader::source::load_compute(device.clone()).unwrap();
                PipelineShaderStageCreateInfo::new(shader.entry_point("main").unwrap())
            };
            tracing::trace!("Shader loaded");

            let layout = PipelineLayout::new(
                device.clone(),
                PipelineDescriptorSetLayoutCreateInfo::from_stages(&[stage.clone()])
                    .into_pipeline_layout_create_info(device.clone())
                    .unwrap(),
            )
            .unwrap();

            ComputePipeline::new(
                device.clone(),
                None,
                ComputePipelineCreateInfo::stage_layout(stage, layout),
            )
            .unwrap()
        };
        tracing::debug!("Pipeline created");

        let work_group_count = [(width + 15) / 16, (height + 15) / 16, 1];
        let descriptor_set_layout = pipeline.layout().set_layouts().first().unwrap();
        let render_command_buffers = render_surface
            .views()
            .iter()
            .map(|view| {
                let descriptor_set = PersistentDescriptorSet::new(
                    descriptor_set_allocator,
                    descriptor_set_layout.clone(),
                    [
                        // TODO: Add buffers
                        WriteDescriptorSet::image_view(0, view.clone()),
                        WriteDescriptorSet::buffer(1, buffers.camera_uniform.clone()),
                        WriteDescriptorSet::buffer(2, buffers.triangles_buffer.clone()),
                        WriteDescriptorSet::buffer(3, buffers.materials_buffer.clone()),
                        WriteDescriptorSet::buffer(4, buffers.models_buffer.clone()),
                        WriteDescriptorSet::buffer(5, buffers.bvhs_buffer.clone()),
                    ],
                    [],
                )
                .unwrap();

                let mut builder = AutoCommandBufferBuilder::primary(
                    command_buffer_allocator,
                    queue.queue_family_index(),
                    command_buffer::CommandBufferUsage::MultipleSubmit,
                )
                .unwrap();

                builder
                    .bind_pipeline_compute(pipeline.clone())
                    .unwrap()
                    .bind_descriptor_sets(
                        vulkano::pipeline::PipelineBindPoint::Compute,
                        pipeline.layout().clone(),
                        0,
                        vec![descriptor_set],
                    )
                    .unwrap()
                    .dispatch(work_group_count)
                    .unwrap();
                builder.build().unwrap()
            })
            .collect::<Vec<_>>()
            .into_boxed_slice();
        tracing::debug!("Command buffers created");

        Self {
            queue: queue.clone(),
            pipeline,
            render_surface,
            render_command_buffers,
        }
    }

    /// Recreates the command buffers, typically when the render surface is resized.
    ///
    /// ## Panics
    ///
    /// This function panics if the command buffers cannot be recreated, typically if the pipeline is out of date
    /// or if the render surface is invalid.
    pub fn recreate_command_buffers(
        &mut self,
        descriptor_set_allocator: &Arc<StandardDescriptorSetAllocator>,
        command_buffer_allocator: &Arc<StandardCommandBufferAllocator>,
        render_surface: &Arc<dyn RenderSurface>,
    ) {
        let (width, height) = render_surface.size();

        let work_group_count = [(width + 15) / 16, (height + 15) / 16, 1];
        let descriptor_set_layout = self.pipeline.layout().set_layouts().first().unwrap();

        self.render_command_buffers = render_surface
            .views()
            .iter()
            .map(|view| {
                let descriptor_set = PersistentDescriptorSet::new(
                    descriptor_set_allocator,
                    descriptor_set_layout.clone(),
                    [
                        WriteDescriptorSet::image_view(0, view.clone()),
                        // WriteDescriptorSet::buffer(1, buffers.camera_uniform.clone()),
                        // WriteDescriptorSet::buffer(2, buffers.triangles_buffer.clone()),
                        // WriteDescriptorSet::buffer(3, buffers.materials_buffer.clone()),
                        // WriteDescriptorSet::buffer(4, buffers.models_buffer.clone()),
                        // WriteDescriptorSet::buffer(5, buffers.bvhs_buffer.clone()),
                    ],
                    [],
                )
                .unwrap();

                let mut builder = AutoCommandBufferBuilder::primary(
                    command_buffer_allocator,
                    self.queue.queue_family_index(),
                    command_buffer::CommandBufferUsage::MultipleSubmit,
                )
                .unwrap();

                builder
                    .bind_pipeline_compute(self.pipeline.clone())
                    .unwrap()
                    .bind_descriptor_sets(
                        vulkano::pipeline::PipelineBindPoint::Compute,
                        self.pipeline.layout().clone(),
                        0,
                        vec![descriptor_set],
                    )
                    .unwrap()
                    .dispatch(work_group_count)
                    .unwrap();
                builder.build().unwrap()
            })
            .collect::<Vec<_>>()
            .into_boxed_slice();

        tracing::trace!("Command buffers recreated");
    }

    /// Renders the scene.
    ///
    /// ## Panics
    ///
    /// This function panics if the renderer cannot render the scene, typically due to an error
    /// during rendering on the GPU.
    pub fn render(&mut self, on_waiting_for_render: &mut dyn FnMut(u32)) {
        let (view_index, future) = self.render_surface.acquire().unwrap();

        let render_future = future
            .then_execute(
                self.queue.clone(),
                self.render_command_buffers[view_index as usize].clone(),
            )
            .unwrap()
            .then_signal_fence_and_flush()
            .unwrap();

        on_waiting_for_render(view_index);

        self.render_surface
            .present(render_future.boxed(), &self.queue)
            .unwrap();
    }
}
