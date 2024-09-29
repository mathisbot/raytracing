#![warn(clippy::pedantic, clippy::nursery)]
#![allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]

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
        Device, DeviceExtensions, Features, Queue, QueueCreateInfo,
    },
    instance::{Instance, InstanceCreateInfo},
    memory::allocator::StandardMemoryAllocator,
    swapchain::Surface,
    VulkanLibrary,
};

pub mod control;
pub mod render; // TODO: Make private ?
pub mod shader;

mod buffer;

struct Context {
    device: Arc<Device>,
    compute_queue: Arc<Queue>,
    transfer_queue: Arc<Queue>,
    memory_allocator: Arc<StandardMemoryAllocator>,
    descriptor_set_allocator: Arc<StandardDescriptorSetAllocator>,
    command_buffer_allocator: Arc<StandardCommandBufferAllocator>,
}

impl Context {
    pub fn new(
        config: &RayTracingAppConfig,
        event_loop: Option<&winit::event_loop::EventLoop<()>>,
    ) -> Self {
        let library = VulkanLibrary::new().expect("failed to load Vulkan library");

        tracing::debug!("Vulkan library loaded");

        let instance_extensions = match config.render_surface_type {
            RenderSurfaceType::Window(_) => Surface::required_extensions(event_loop.unwrap()),
            #[cfg(feature = "image")]
            RenderSurfaceType::Image(_) => vulkano::instance::InstanceExtensions::empty(),
            // _ => todo!(),
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
            // _ => todo!(),
        };

        let instance = Instance::new(
            library,
            InstanceCreateInfo {
                #[cfg(target_os = "macos")]
                flags: InstanceCreateFlags::ENUMERATE_PORTABILITY,
                application_version: vulkano::Version::major_minor(1, 0),
                #[cfg(target_os = "macos")]
                enabled_extensions: InstanceExtensions {
                    khr_portability_enumeration: true,
                    ..InstanceExtensions::empty()
                }
                .union(instance_extensions),
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

        let (device, compute_queue, transfer_queue) =
            Self::create_device(physical_device, &device_extensions, &Features::empty());

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

    fn create_device(
        physical_device: Arc<PhysicalDevice>,
        device_extensions: &DeviceExtensions,
        device_features: &Features,
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

pub struct RayTracingApp {
    config: RayTracingAppConfig,
    renderer: Renderer,
    buffers: Buffers,
    event_loop: Option<winit::event_loop::EventLoop<()>>,
}

impl RayTracingApp {
    pub fn new(config: RayTracingAppConfig) -> Self {
        let event_loop = match config.render_surface_type {
            RenderSurfaceType::Window(_) => Some(winit::event_loop::EventLoop::new()),
            #[cfg(feature = "image")]
            RenderSurfaceType::Image(_) => None,
            // _ => todo!(),
        };
        let context = Context::new(&config, event_loop.as_ref());

        let render_surface: Box<dyn RenderSurface> = match &config.render_surface_type {
            RenderSurfaceType::Window(descriptor) => Box::new(crate::render::window::Window::new(
                event_loop.as_ref().unwrap(),
                &context.device,
                descriptor,
            )),
            #[cfg(feature = "image")]
            RenderSurfaceType::Image(descriptor) => Box::new(Image::new(&descriptor)),
            // _ => todo!(),
        };

        // TODO: Let user specify buffer content
        let buffers = Self::init_gpu_buffers(&context);

        let renderer = Renderer::new(
            &context.device,
            &context.compute_queue,
            &context.descriptor_set_allocator,
            &context.command_buffer_allocator,
            render_surface,
            &buffers,
        );

        tracing::debug!("Successfully initialized");

        Self {
            config,
            renderer,
            buffers,
            event_loop,
        }
    }

    #[must_use]
    fn init_gpu_buffers(context: &Context) -> Buffers {
        let camera_uniform = {
            use crate::shader::source::{Camera, CameraBuffer};
            let data = Camera {
                position: [0.0, 0.0, 0.0].into(),
                view: [0.0, 0.0, -1.0].into(),
                up: [0.0, 1.0, 0.0].into(),
                right: [1.0, 0.0, 0.0],
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
            &[
                "assets/models/cottage/cottage_FREE.obj".to_string(),
                "assets/models/gun/Pistol_02.obj".to_string(),
            ],
            &[[0.0, -3.0, -10.0], [0.0, 0.0, 0.0]],
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
    pub fn buffers(&self) -> Buffers {
        self.buffers.clone()
    }

    pub fn run(self) {
        match self.config.render_surface_type {
            RenderSurfaceType::Window(_) => {
                let Self {
                    event_loop,
                    config:
                        RayTracingAppConfig {
                            mut controllers,
                            mut camera,
                            ..
                        },
                    mut renderer,
                    buffers,
                    ..
                } = self;

                let mut start = std::time::Instant::now();

                event_loop.unwrap().run(move |event, _, control_flow| {
                    for controller in &mut controllers {
                        controller.handle_event(&event);
                    }
                    match event {
                        winit::event::Event::WindowEvent {
                            event: winit::event::WindowEvent::CloseRequested,
                            ..
                        } => {
                            *control_flow = winit::event_loop::ControlFlow::Exit;
                        }
                        // TODO: Resize
                        // winit::event::Event::WindowEvent {
                        //     event: winit::event::WindowEvent::Resized(size),
                        //     ..
                        // } => {
                        //     self.resize(size);
                        // }
                        winit::event::Event::MainEventsCleared => {
                            let elapsed = start.elapsed().as_secs_f32();
                            start = std::time::Instant::now();

                            let inputs = controllers
                                .iter_mut()
                                .flat_map(|controller| controller.fetch_input())
                                .collect::<Vec<_>>();
                            camera.process_inputs(&inputs, elapsed);

                            let mut camera_handle = buffers.camera_uniform.write().unwrap();
                            camera_handle.camera.position = camera.position().into();
                            camera_handle.camera.view = camera.direction().into();
                            camera_handle.camera.up = camera.up().into();
                            camera_handle.camera.right = camera.right();
                            drop(camera_handle);

                            // tracing::trace!("FPS: {}", 1.0 / elapsed);

                            renderer.render(&mut |_| {}).unwrap();
                        }
                        _ => {}
                    }
                });
            }
            #[cfg(feature = "image")]
            RenderSurfaceType::Image(_) => {
                self.render(&mut |_| {});
            } // _ => todo!(),
        }
    }
}

pub struct RayTracingAppConfig {
    pub render_surface_type: RenderSurfaceType,
    pub camera: Box<dyn control::camera::Camera>,
    pub controllers: Vec<Box<dyn control::controller::Controller>>,
}

#[non_exhaustive]
#[derive(Clone, Debug)]
pub enum RenderSurfaceType {
    Window(WindowDescriptor),
    #[cfg(feature = "image")]
    Image(ImageDescriptor),
}
