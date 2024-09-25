#![deny(clippy::all)]
#![warn(clippy::pedantic, clippy::nursery)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]

mod buffers;
mod controls;
mod init;
mod models;
mod pipeline;
mod shaders;

pub use init::context::VulkanoConfig;

use std::sync::Arc;

use crate::init::context::VulkanoContext;
use crate::init::window::{VulkanoWindow, WindowDescriptor, WindowMode, WindowResizeConstraints};
use models::LoadedModels;
use vulkano::buffer::Subbuffer;
use vulkano::command_buffer::allocator::StandardCommandBufferAllocator;
use vulkano::command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage};
use vulkano::descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet};
use vulkano::pipeline::Pipeline;
use vulkano::sync::GpuFuture;
use winit::event;
use winit::event_loop::{ControlFlow, EventLoop};

#[derive(Clone)]
pub struct Buffers {
    pub camera_uniform: Subbuffer<crate::shaders::CameraBuffer>,
    pub triangles_buffer: Subbuffer<crate::shaders::TrianglesBuffer>,
    pub materials_buffer: Subbuffer<crate::shaders::Materials>,
    pub models_buffer: Subbuffer<crate::shaders::ModelsBuffer>,
    pub bvhs_buffer: Subbuffer<crate::shaders::BvhBuffer>,
}

pub type RenderCommandBuffer =
    Arc<vulkano::command_buffer::PrimaryAutoCommandBuffer<Arc<StandardCommandBufferAllocator>>>;

pub struct RayTracerApp {
    context: VulkanoContext,
    window: VulkanoWindow,
    compute_pipeline: Arc<vulkano::pipeline::ComputePipeline>,
    buffers: Buffers,
    render_command_buffers: Box<[RenderCommandBuffer]>,
}

impl RayTracerApp {
    #[must_use]
    pub fn new(config: VulkanoConfig, event_loop: &EventLoop<()>) -> Self {
        let context = VulkanoContext::new(config, event_loop);

        let window = VulkanoWindow::new(
            event_loop,
            &context,
            &WindowDescriptor {
                width: 1024,
                height: 720,
                title: "RayTracer".to_string(),
                cursor_visible: false,
                cursor_locked: true,
                position: None,
                resizable: false,
                // FIXME: Exclusive fullscreen mode
                mode: WindowMode::Windowed,
                present_mode: vulkano::swapchain::PresentMode::Fifo,
                resize_constraints: WindowResizeConstraints::default(),
            },
        );

        let shader = crate::shaders::load_compute(context.device().clone())
            .expect("failed to create shader module");

        let compute_pipeline = crate::pipeline::new(context.device(), &shader, "main");

        let buffers = Self::init_gpu_buffers(&context);

        let render_command_buffers =
            Self::init_render_command_buffers(&context, &compute_pipeline, &window, &buffers);

        Self {
            context,
            window,
            compute_pipeline,
            buffers,
            render_command_buffers,
        }
    }

    #[must_use]
    pub const fn context(&self) -> &VulkanoContext {
        &self.context
    }

    #[must_use]
    pub const fn window(&self) -> &VulkanoWindow {
        &self.window
    }

    #[must_use]
    pub fn window_mut(&mut self) -> &mut VulkanoWindow {
        &mut self.window
    }

    #[must_use]
    pub fn compute_pipeline(&self) -> &vulkano::pipeline::ComputePipeline {
        &self.compute_pipeline
    }

    #[must_use]
    pub const fn buffers(&self) -> &Buffers {
        &self.buffers
    }

    #[must_use]
    pub const fn render_command_buffers(&self) -> &[RenderCommandBuffer] {
        &self.render_command_buffers
    }

    #[must_use]
    pub const fn render_command_buffer(&self, index: usize) -> &RenderCommandBuffer {
        &self.render_command_buffers[index]
    }

    #[must_use]
    fn init_gpu_buffers(context: &VulkanoContext) -> Buffers {
        let camera_uniform = {
            use crate::shaders::{Camera, CameraBuffer};
            let data = Camera {
                position: [0.0, 0.0, 0.0].into(),
                view: [0.0, 0.0, -1.0].into(),
                up: [0.0, 1.0, 0.0].into(),
                right: [1.0, 0.0, 0.0],
            };
            let buffer = crate::buffers::new_uniform::<CameraBuffer>(context).unwrap();
            buffer.write().unwrap().camera = data;
            buffer
        };

        let LoadedModels {
            triangles_buffer,
            materials_buffer,
            models_buffer,
            bvhs_buffer,
            future: model_future,
        } = models::LoadedModels::load(
            context,
            &[
                "assets/models/cottage/cottage_FREE.obj".to_string(),
                "assets/models/gun/Pistol_02.obj".to_string(),
            ],
            &[[0.0, -3.0, -10.0], [0.0, 0.0, 0.0]],
        );

        model_future
            .then_signal_fence_and_flush()
            .unwrap()
            .wait(None)
            .unwrap();

        Buffers {
            camera_uniform,
            triangles_buffer,
            materials_buffer,
            models_buffer,
            bvhs_buffer,
        }
    }

    #[must_use]
    fn init_render_command_buffers(
        context: &VulkanoContext,
        compute_pipeline: &Arc<vulkano::pipeline::ComputePipeline>,
        window: &VulkanoWindow,
        buffers: &Buffers,
    ) -> Box<[RenderCommandBuffer]> {
        let (width, height): (u32, u32) = window.window_size().into();

        let work_group_count = [(width + 15) / 16, (height + 15) / 16, 1];

        let descriptor_set_layout = compute_pipeline.layout().set_layouts().first().unwrap();
        let mut render_command_buffers = Vec::new();
        for i in 0..window.renderer().swapchain_image_count() {
            let image_view = window.renderer().get_swapchain_image_view(i);

            let descriptor_set = PersistentDescriptorSet::new(
                context.descriptor_set_allocator(),
                descriptor_set_layout.clone(),
                [
                    WriteDescriptorSet::image_view(0, image_view),
                    WriteDescriptorSet::buffer(1, buffers.camera_uniform.clone()),
                    WriteDescriptorSet::buffer(2, buffers.triangles_buffer.clone()),
                    WriteDescriptorSet::buffer(3, buffers.materials_buffer.clone()),
                    WriteDescriptorSet::buffer(4, buffers.models_buffer.clone()),
                    WriteDescriptorSet::buffer(5, buffers.bvhs_buffer.clone()),
                ],
                [],
            )
            .unwrap();

            render_command_buffers.push({
                let mut builder = AutoCommandBufferBuilder::primary(
                    &context.command_buffer_allocator().clone(),
                    context.compute_queue().queue_family_index(),
                    CommandBufferUsage::SimultaneousUse,
                )
                .unwrap();

                builder
                    .bind_pipeline_compute(compute_pipeline.clone())
                    .unwrap()
                    .bind_descriptor_sets(
                        vulkano::pipeline::PipelineBindPoint::Compute,
                        compute_pipeline.layout().clone(),
                        0,
                        vec![descriptor_set],
                    )
                    .unwrap()
                    .dispatch(work_group_count)
                    .unwrap();
                builder.build().unwrap()
            });
        }

        render_command_buffers.into_boxed_slice()
    }

    pub fn run(self, event_loop: EventLoop<()>) {
        let Self {
            context,
            mut window,
            buffers,
            render_command_buffers,
            ..
        } = self;

        let mut physics_start = std::time::Instant::now();

        let mut keyboard = crate::controls::Keyboard::default();
        let mut mouse = crate::controls::Mouse::default();
        let mut camera = crate::controls::Camera::with_position([0.0, 0.0, 10.0]);

        event_loop.run(move |event, _, control_flow| match event {
            event::Event::WindowEvent {
                event: event::WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            event::Event::MainEventsCleared => {
                let mut camera_handle = buffers.camera_uniform.write().unwrap();
                camera_handle.camera.position = camera.position().into();
                camera_handle.camera.view = camera.view().into();
                let (up, right) = camera.up_right();
                camera_handle.camera.right = right;
                camera_handle.camera.up = up.into();
                drop(camera_handle);

                let renderer = window.renderer_mut();
                let future = renderer
                    .acquire(|_| tracing::debug!("Swapchain recreated"))
                    .unwrap();
                let image_index = renderer.image_index();

                let render_future = future
                    .then_execute(
                        context.compute_queue().clone(),
                        render_command_buffers[image_index as usize].clone(),
                    )
                    .unwrap()
                    .then_signal_fence_and_flush() // Frame starts to render here
                    .unwrap();

                let delta_seconds = physics_start.elapsed().as_secs_f32();
                physics_start = std::time::Instant::now();

                camera.process_keyboard_input(keyboard, delta_seconds);

                let (delta_x, delta_y) = mouse.fetch_mouse_delta();
                camera.process_mouse_input(delta_x, delta_y);

                renderer.present(render_future.boxed());

                // let fps = 1.0 / delta_seconds;
                // tracing::trace!("FPS: {:.1}", fps);
            }
            event::Event::WindowEvent {
                event: event::WindowEvent::KeyboardInput { input, .. },
                ..
            } => {
                keyboard.handle_keypress(&input);
            }
            event::Event::DeviceEvent {
                event: event::DeviceEvent::Motion { axis, value },
                ..
            } => {
                #[allow(clippy::cast_possible_truncation)]
                mouse.handle_mousemove(axis, value as f32);
            }
            _ => (),
        })
    }
}
