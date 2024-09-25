use std::sync::Arc;
#[cfg(target_os = "macos")]
use vulkano::instance::InstanceCreateFlags;
use vulkano::{
    command_buffer::allocator::{
        StandardCommandBufferAllocator, StandardCommandBufferAllocatorCreateInfo,
    },
    descriptor_set::allocator::{
        StandardDescriptorSetAllocator, StandardDescriptorSetAllocatorCreateInfo,
    },
    device::{
        physical::{PhysicalDevice, PhysicalDeviceType},
        Device, DeviceCreateInfo, DeviceExtensions, Features, Queue, QueueCreateInfo, QueueFlags,
    },
    instance::{Instance, InstanceCreateInfo},
    memory::allocator::StandardMemoryAllocator,
    swapchain::Surface,
    Version, VulkanLibrary,
};
use winit::event_loop::EventLoop;

#[derive(Debug)]
pub struct VulkanoConfig {
    pub instance_create_info: InstanceCreateInfo,
    pub device_extensions: DeviceExtensions,
    pub device_features: Features,
}

impl Default for VulkanoConfig {
    #[inline]
    fn default() -> Self {
        let device_extensions = DeviceExtensions::empty();
        Self {
            instance_create_info: InstanceCreateInfo {
                #[cfg(target_os = "macos")]
                flags: InstanceCreateFlags::ENUMERATE_PORTABILITY,
                application_version: Version::major_minor(0, 1),
                #[cfg(target_os = "macos")]
                enabled_extensions: InstanceExtensions {
                    khr_portability_enumeration: true,
                    ..InstanceExtensions::empty()
                },
                ..Default::default()
            },
            device_extensions,
            // TODO: Acceleration structures ?
            // They're not supported on all devices,
            // but ray tracing is heavy so it won't run on weak devices anyway
            // Vulkano doesn't support them as of now
            device_features: Features::empty(),
        }
    }
}

#[allow(clippy::module_name_repetitions)]
#[derive(Debug)]
pub struct VulkanoContext {
    instance: Arc<Instance>,
    device: Arc<Device>,
    compute_queue: Arc<Queue>,
    transfer_queue: Arc<Queue>,
    memory_allocator: Arc<StandardMemoryAllocator>,
    command_buffer_allocator: Arc<StandardCommandBufferAllocator>,
    descriptor_set_allocator: Arc<StandardDescriptorSetAllocator>,
}

impl VulkanoContext {
    pub fn new(mut config: VulkanoConfig, event_loop: &EventLoop<()>) -> Self {
        let library = VulkanLibrary::new().unwrap();

        tracing::debug!("Vulkan library loaded");

        config.instance_create_info.enabled_extensions = Surface::required_extensions(event_loop)
            .union(&config.instance_create_info.enabled_extensions);

        assert!(
            library
                .supported_extensions()
                .contains(&config.instance_create_info.enabled_extensions),
            "Vulkan library does not support required extensions"
        );

        config.device_extensions = config.device_extensions.union(&DeviceExtensions {
            khr_swapchain: true,
            ..DeviceExtensions::empty()
        });

        let instance =
            Instance::new(library, config.instance_create_info).expect("failed to create instance");

        let physical_device = instance
            .enumerate_physical_devices()
            .expect("failed to enumerate physical devices")
            .filter(|p| p.supported_extensions().contains(&config.device_extensions))
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
            &config.device_extensions,
            &config.device_features,
        );

        let memory_allocator = Arc::new(StandardMemoryAllocator::new_default(device.clone()));

        let command_buffer_allocator = StandardCommandBufferAllocator::new(
            device.clone(),
            StandardCommandBufferAllocatorCreateInfo::default(),
        )
        .into();

        let descriptor_set_allocator = StandardDescriptorSetAllocator::new(
            device.clone(),
            StandardDescriptorSetAllocatorCreateInfo::default(),
        )
        .into();

        Self {
            instance,
            device,
            compute_queue,
            transfer_queue,
            memory_allocator,
            command_buffer_allocator,
            descriptor_set_allocator,
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
            .find(|(_i, q)| q.queue_flags.intersects(QueueFlags::COMPUTE))
            .map(|(i, _)| i)
            .expect("could not find a queue that supports graphics");

        // Try finding a separate queue for transfer
        let queue_family_transfer = physical_device
            .queue_family_properties()
            .iter()
            .enumerate()
            .map(|(i, q)| (u32::try_from(i).unwrap(), q))
            .find(|(i, q)| {
                q.queue_flags.intersects(QueueFlags::TRANSFER) && *i != queue_family_compute
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
            DeviceCreateInfo {
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

    #[must_use]
    pub const fn instance(&self) -> &Arc<Instance> {
        &self.instance
    }

    #[must_use]
    pub const fn device(&self) -> &Arc<Device> {
        &self.device
    }

    #[must_use]
    pub const fn compute_queue(&self) -> &Arc<Queue> {
        &self.compute_queue
    }

    #[must_use]
    pub const fn transfer_queue(&self) -> &Arc<Queue> {
        &self.transfer_queue
    }

    #[must_use]
    pub const fn memory_allocator(&self) -> &Arc<StandardMemoryAllocator> {
        &self.memory_allocator
    }

    #[must_use]
    pub const fn command_buffer_allocator(&self) -> &Arc<StandardCommandBufferAllocator> {
        &self.command_buffer_allocator
    }

    #[must_use]
    pub const fn descriptor_set_allocator(&self) -> &Arc<StandardDescriptorSetAllocator> {
        &self.descriptor_set_allocator
    }
}
