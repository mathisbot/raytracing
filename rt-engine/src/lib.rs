//! # Ray Tracing Core Engine
//!
//! This crate provides the core engine for ray tracing applications. It is designed to be used as a library in other applications.
//!
//! ## Features
//!
//! - Ray Tracing
//! - Vulkan
//! - Bounding Volume Hierarchies
//! - Camera handling
//! - Wide controller devices support
//! - Model Loading
//! - Window Rendering
//! - Image Rendering

#![warn(clippy::pedantic, clippy::nursery)]
#![warn(
    clippy::cognitive_complexity,
    clippy::dbg_macro,
    clippy::debug_assert_with_mut_call,
    clippy::doc_link_with_quotes,
    clippy::doc_markdown,
    clippy::empty_line_after_outer_attr,
    clippy::empty_structs_with_brackets,
    clippy::float_cmp,
    clippy::float_cmp_const,
    clippy::float_equality_without_abs,
    keyword_idents,
    clippy::missing_const_for_fn,
    missing_copy_implementations,
    clippy::missing_docs_in_private_items,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::mod_module_files,
    non_ascii_idents,
    noop_method_call,
    clippy::option_if_let_else,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::semicolon_if_nothing_returned,
    clippy::unseparated_literal_suffix,
    clippy::shadow_unrelated,
    clippy::similar_names,
    clippy::suspicious_operation_groupings,
    unused_crate_dependencies,
    unused_extern_crates,
    unused_import_braces,
    clippy::unused_self,
    clippy::use_debug,
    clippy::used_underscore_binding,
    clippy::useless_let_if_seq,
    clippy::wildcard_dependencies,
    clippy::wildcard_imports
)]

use std::sync::Arc;

#[cfg(feature = "image")]
use render::image::{Image, ImageDescriptor};

use render::{window::WindowDescriptor, Buffers, RenderSurface, Renderer};
use vulkano::{
    command_buffer::allocator::{
        StandardCommandBufferAllocator, StandardCommandBufferAllocatorCreateInfo,
    },
    descriptor_set::allocator::{
        StandardDescriptorSetAllocator, StandardDescriptorSetAllocatorCreateInfo,
    },
    device::{
        physical::{PhysicalDevice, PhysicalDeviceType},
        Device, DeviceExtensions, DeviceFeatures, Queue, QueueCreateInfo,
    },
    instance::{Instance, InstanceCreateInfo},
    memory::allocator::StandardMemoryAllocator,
    swapchain::Surface,
    VulkanLibrary,
};

/// Handles everything related to the camera.
pub mod control;
/// Handles rendering on a surface.
pub mod render;
/// Shader source code and implementations
/// of the shader structs.
pub mod shader;

/// Utils to handle staging buffers.
mod buffer;

/// Represents the context of the ray tracing application.
struct Context {
    /// The Vulkan device.
    device: Arc<Device>,
    /// The compute queue.
    compute_queue: Arc<Queue>,
    /// The transfer queue.
    transfer_queue: Arc<Queue>,
    /// The memory allocator.
    memory_allocator: Arc<StandardMemoryAllocator>,
    /// The descriptor set allocator.
    descriptor_set_allocator: Arc<StandardDescriptorSetAllocator>,
    /// The command buffer allocator.
    command_buffer_allocator: Arc<StandardCommandBufferAllocator>,
}

impl Context {
    #[must_use]
    /// Creates a new context for the ray tracing application.
    pub fn new(
        config: &RayTracingAppConfig,
        event_loop: Option<&winit::event_loop::EventLoop<()>>,
    ) -> Self {
        let library = VulkanLibrary::new().expect("failed to load Vulkan library");

        tracing::debug!("Vulkan library loaded");

        let instance_extensions = match config.render_surface_type {
            RenderSurfaceType::Window(_) => Surface::required_extensions(event_loop.unwrap())
                .expect("failed to query required surface extensions"),
            #[cfg(feature = "image")]
            RenderSurfaceType::Image(_) => vulkano::instance::InstanceExtensions::empty(),
        };
        assert!(
            library
                .supported_extensions()
                .contains(&instance_extensions),
            "Vulkan library does not support required extensions"
        );

        let device_extensions = match config.render_surface_type {
            RenderSurfaceType::Window(_) => DeviceExtensions {
                khr_storage_buffer_storage_class: true,
                khr_swapchain: true,
                ..DeviceExtensions::empty()
            },
            #[cfg(feature = "image")]
            RenderSurfaceType::Image(_) => DeviceExtensions::empty(),
        };

        let instance = Instance::new(
            library,
            InstanceCreateInfo {
                #[cfg(target_os = "macos")]
                flags: vulkano::instance::InstanceCreateFlags::ENUMERATE_PORTABILITY,
                application_version: vulkano::Version::major_minor(1, 0),
                #[cfg(target_os = "macos")]
                enabled_extensions: vulkano::instance::InstanceExtensions {
                    khr_portability_enumeration: true,
                    ..Default::default()
                }
                .union(&instance_extensions),
                #[cfg(not(target_os = "macos"))]
                enabled_extensions: instance_extensions,
                ..Default::default()
            },
        )
        .expect("failed to create instance");

        tracing::debug!("Vulkan instance created");

        let physical_device = instance
            .enumerate_physical_devices()
            .expect("failed to enumerate physical devices")
            .filter(|p| p.supported_extensions().contains(&device_extensions))
            .min_by_key(|p| match p.properties().device_type {
                PhysicalDeviceType::DiscreteGpu => 1,
                PhysicalDeviceType::IntegratedGpu => 2,
                PhysicalDeviceType::VirtualGpu => 3,
                PhysicalDeviceType::Cpu => 4,
                PhysicalDeviceType::Other => 5,
                _ => 6,
            })
            .expect("failed to find a suitable physical device");

        tracing::info!("Using device {}", physical_device.properties().device_name,);

        let (device, compute_queue, transfer_queue) = Self::create_device(
            physical_device,
            &device_extensions,
            &DeviceFeatures::empty(),
        );

        tracing::debug!("Vulkan device created");

        Self {
            device: device.clone(),
            compute_queue,
            transfer_queue,
            memory_allocator: Arc::new(StandardMemoryAllocator::new_default(device.clone())),
            descriptor_set_allocator: Arc::new(StandardDescriptorSetAllocator::new(
                device.clone(),
                StandardDescriptorSetAllocatorCreateInfo::default(),
            )),
            command_buffer_allocator: Arc::new(StandardCommandBufferAllocator::new(
                device,
                StandardCommandBufferAllocatorCreateInfo::default(),
            )),
        }
    }

    #[must_use]
    /// Creates a new Vulkan device.
    fn create_device(
        physical_device: Arc<PhysicalDevice>,
        device_extensions: &DeviceExtensions,
        device_features: &DeviceFeatures,
    ) -> (Arc<Device>, Arc<Queue>, Arc<Queue>) {
        let queue_family_compute = physical_device
            .queue_family_properties()
            .iter()
            .enumerate()
            .map(|(i, q)| (u32::try_from(i).unwrap(), q))
            .find(|(_i, q)| {
                q.queue_flags
                    .intersects(vulkano::device::QueueFlags::COMPUTE)
            })
            .map(|(i, _)| i)
            .expect("could not find a queue that supports graphics");

        // Try finding a separate queue for transfer
        let queue_family_transfer = physical_device
            .queue_family_properties()
            .iter()
            .enumerate()
            .map(|(i, q)| (u32::try_from(i).unwrap(), q))
            .find(|(i, q)| {
                q.queue_flags
                    .intersects(vulkano::device::QueueFlags::TRANSFER)
                    && *i != queue_family_compute
            })
            .map(|(i, _)| i);

        let queue_create_infos = vec![
            QueueCreateInfo {
                queue_family_index: queue_family_compute,
                ..Default::default()
            },
            queue_family_transfer.map_or_else(QueueCreateInfo::default, |transfer_queue| {
                QueueCreateInfo {
                    queue_family_index: transfer_queue,
                    ..Default::default()
                }
            }),
        ];

        let (device, mut queues) = Device::new(
            physical_device,
            vulkano::device::DeviceCreateInfo {
                queue_create_infos,
                enabled_extensions: *device_extensions,
                enabled_features: *device_features,
                ..Default::default()
            },
        )
        .expect("failed to create device");

        let compute_queue = queues.next().unwrap();
        let transfer_queue = queue_family_transfer.map_or_else(
            || compute_queue.clone(),
            |_| queues.next().expect("Failed to get transfer queue"),
        );

        (device, compute_queue, transfer_queue)
    }
}

/// Event loop handler for window-based rendering.
struct WindowAppLoop {
    /// Vulkan context.
    context: Context,
    /// Window descriptor.
    window_descriptor: WindowDescriptor,
    /// Shader parameters.
    shader_descriptor: crate::shader::ShaderDescriptor,
    /// Input controllers.
    controllers: Vec<Box<dyn crate::control::controller::Controller>>,
    /// Active camera.
    camera: Box<dyn crate::control::camera::Camera>,
    /// Renderer instance.
    renderer: Option<Renderer>,
    /// GPU buffers.
    buffers: Buffers,
    /// Timestamp of previous frame.
    start: std::time::Instant,
    /// User callback invoked while waiting for render.
    on_waiting_for_render: Box<dyn FnMut(u32)>,
}

impl winit::application::ApplicationHandler for WindowAppLoop {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);

        if self.renderer.is_none() {
            let render_surface = Box::new(crate::render::window::Window::new(
                event_loop,
                &self.context.device,
                &self.window_descriptor,
            ));

            self.renderer = Some(Renderer::new(
                &self.context.device,
                &self.context.compute_queue,
                &self.context.descriptor_set_allocator,
                &self.context.command_buffer_allocator,
                render_surface,
                &self.buffers,
                self.shader_descriptor,
            ));
        }
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        if matches!(event, winit::event::WindowEvent::CloseRequested) {
            event_loop.exit();
        }

        let controller_event = winit::event::Event::WindowEvent { window_id, event };
        for controller in &mut self.controllers {
            controller.handle_event(&controller_event);
        }
    }

    fn device_event(
        &mut self,
        _event_loop: &winit::event_loop::ActiveEventLoop,
        device_id: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        let controller_event = winit::event::Event::DeviceEvent { device_id, event };
        for controller in &mut self.controllers {
            controller.handle_event(&controller_event);
        }
    }

    fn about_to_wait(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        let controller_event = winit::event::Event::AboutToWait;
        for controller in &mut self.controllers {
            controller.handle_event(&controller_event);
        }

        let elapsed = self.start.elapsed().as_secs_f32();
        self.start = std::time::Instant::now();

        let mut inputs = crate::control::Inputs::default();
        for controller in &mut self.controllers {
            inputs.accumulate(controller.fetch_input());
        }
        self.camera.process_inputs(inputs, elapsed);

        let mut camera_handle = self.buffers.camera_uniform.write().unwrap();
        camera_handle.camera.position = self.camera.position().into();
        camera_handle.camera.view = self.camera.direction().into();
        camera_handle.camera.up = self.camera.up().into();
        camera_handle.camera.right = self.camera.right();
        drop(camera_handle);

        if let Some(renderer) = &mut self.renderer {
            renderer.render(&mut self.on_waiting_for_render);
        }
    }
}

/// The main ray tracing application.
pub struct RayTracingApp {
    /// The configuration of the ray tracing application.
    config: RayTracingAppConfig,
    /// Vulkan context.
    context: Context,
    /// The renderer.
    renderer: Option<Renderer>,
    /// The GPU buffers.
    buffers: Buffers,
    /// The optional event loop.
    event_loop: Option<winit::event_loop::EventLoop<()>>,
}

impl RayTracingApp {
    #[must_use]
    /// Creates a new ray tracing application from the given configuration.
    ///
    /// ## Panics
    ///
    /// This function will panic if the application encounters any errors during initialization.
    pub fn new(config: RayTracingAppConfig) -> Self {
        let event_loop = match config.render_surface_type {
            RenderSurfaceType::Window(_) => Some(
                winit::event_loop::EventLoop::new().expect("failed to create winit event loop"),
            ),
            #[cfg(feature = "image")]
            RenderSurfaceType::Image(_) => None,
        };
        let context = Context::new(&config, event_loop.as_ref());

        let buffers = Self::init_gpu_buffers(&config, &context);

        let renderer = match &config.render_surface_type {
            RenderSurfaceType::Window(_) => None,
            #[cfg(feature = "image")]
            RenderSurfaceType::Image(descriptor) => {
                let render_surface: Box<dyn RenderSurface> = Box::new(Image::new(
                    descriptor,
                    context.memory_allocator.clone(),
                    &context.command_buffer_allocator,
                    context.compute_queue.clone(),
                ));

                Some(Renderer::new(
                    &context.device,
                    &context.compute_queue,
                    &context.descriptor_set_allocator,
                    &context.command_buffer_allocator,
                    render_surface,
                    &buffers,
                    config.shader_descriptor,
                ))
            }
        };

        tracing::debug!("Successfully initialized");

        Self {
            config,
            context,
            renderer,
            buffers,
            event_loop,
        }
    }

    #[must_use]
    /// Initializes the GPU buffers.
    fn init_gpu_buffers(config: &RayTracingAppConfig, context: &Context) -> Buffers {
        let camera_uniform = {
            use crate::shader::source::{Camera, CameraBuffer};
            let data = Camera {
                position: config.camera.position().into(),
                view: config.camera.direction().into(),
                up: config.camera.up().into(),
                right: config.camera.right(),
            };
            let buffer =
                crate::buffer::new_uniform::<CameraBuffer>(&context.memory_allocator).unwrap();
            buffer.write().unwrap().camera = data;
            buffer
        };
        tracing::trace!("Camera buffer initialized");

        let shader::model::LoadedModels {
            triangles_buffer,
            materials_buffer,
            models_buffer,
            bvhs_buffer,
        } = shader::model::LoadedModels::load(
            &context.memory_allocator,
            &context.command_buffer_allocator,
            &context.transfer_queue,
            &config.scene_descriptor,
        );

        Buffers {
            camera_uniform,
            triangles_buffer,
            materials_buffer,
            models_buffer,
            bvhs_buffer,
        }
    }

    #[must_use]
    /// Returns the buffers used in the shader.
    pub fn buffers(&self) -> Buffers {
        self.buffers.clone()
    }

    /// Run the application.
    ///
    /// ## Note
    ///
    /// Use the argument `on_waiting_for_render` to update anything unrelated to rendering while waiting for the render to complete.
    ///
    /// ## Panics
    ///
    /// This function will panic if the application encounters any errors during runtime.
    /// Typically, this can happen if there is a concurrency issue or if the application is unable to render.
    pub fn run(self, mut on_waiting_for_render: Box<dyn FnMut(u32)>) {
        match self.config.render_surface_type {
            RenderSurfaceType::Window(_) => {
                let Self {
                    context,
                    event_loop,
                    config:
                        RayTracingAppConfig {
                            render_surface_type,
                            shader_descriptor,
                            controllers,
                            camera,
                            ..
                        },
                    renderer,
                    buffers,
                    ..
                } = self;

                let window_descriptor = match render_surface_type {
                    RenderSurfaceType::Window(descriptor) => descriptor,
                    #[cfg(feature = "image")]
                    RenderSurfaceType::Image(_) => unreachable!(),
                };

                let mut app = WindowAppLoop {
                    context,
                    window_descriptor,
                    shader_descriptor,
                    controllers,
                    camera,
                    renderer,
                    buffers,
                    start: std::time::Instant::now(),
                    on_waiting_for_render,
                };

                event_loop
                    .unwrap()
                    .run_app(&mut app)
                    .expect("winit event loop run failed");
            }
            #[cfg(feature = "image")]
            RenderSurfaceType::Image(_) => {
                let Self { mut renderer, .. } = self;
                renderer
                    .as_mut()
                    .expect("image renderer was not initialized")
                    .render(&mut on_waiting_for_render);
            }
        }
    }

    /// Run the application without a per-frame callback.
    pub fn run_forever(self) {
        self.run(Box::new(|_| {}));
    }
}

/// The configuration of the ray tracing application.
pub struct RayTracingAppConfig {
    /// The type of render surface to use.
    pub render_surface_type: RenderSurfaceType,
    /// The camera to use.
    pub camera: Box<dyn control::camera::Camera>,
    /// The controllers to use.
    pub controllers: Vec<Box<dyn control::controller::Controller>>,
    /// Scene data to render.
    pub scene_descriptor: shader::SceneDescriptor,
    /// Shader parameters.
    pub shader_descriptor: shader::ShaderDescriptor,
}

impl RayTracingAppConfig {
    #[must_use]
    /// Creates a new configuration with explicit fields.
    pub fn new(
        render_surface_type: RenderSurfaceType,
        camera: Box<dyn control::camera::Camera>,
        controllers: Vec<Box<dyn control::controller::Controller>>,
        scene_descriptor: shader::SceneDescriptor,
        shader_descriptor: shader::ShaderDescriptor,
    ) -> Self {
        Self {
            render_surface_type,
            camera,
            controllers,
            scene_descriptor,
            shader_descriptor,
        }
    }
}

#[non_exhaustive]
#[derive(Clone, Debug)]
// TODO: Remove and use only `RenderSurface` trait.
/// The type of render surface to use.
pub enum RenderSurfaceType {
    /// A window.
    Window(WindowDescriptor),
    #[cfg(feature = "image")]
    /// An image.
    Image(ImageDescriptor),
}
