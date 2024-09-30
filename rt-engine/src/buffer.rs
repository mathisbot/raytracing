use std::sync::Arc;

use vulkano::{
    buffer::{
        AllocateBufferError, Buffer, BufferContents, BufferCreateInfo, BufferUsage, Subbuffer,
    },
    command_buffer::{
        allocator::StandardCommandBufferAllocator, CommandBufferExecFuture, CopyBufferInfo,
    },
    device::Queue,
    memory::allocator::{AllocationCreateInfo, MemoryTypeFilter, StandardMemoryAllocator},
    sync::{self, future::FenceSignalFuture, GpuFuture},
    Validated,
};

/// The future type for sending a buffer to the device.
pub type SendBufferFuture = FenceSignalFuture<CommandBufferExecFuture<sync::future::NowFuture>>;

#[must_use = "The function returns a future that must be awaited and a buffer that must be used"]
/// Sends the given data to the device,
/// returning the destination buffer and the send future.
pub fn send_to_device<T>(
    memory_allocator: &Arc<StandardMemoryAllocator>,
    command_buffer_allocator: &Arc<StandardCommandBufferAllocator>,
    queue: &Arc<Queue>,
    data_len: u64,
    usage: BufferUsage,
    fill_buffer: impl FnOnce(&mut T),
) -> Result<(Subbuffer<T>, SendBufferFuture), Validated<AllocateBufferError>>
where
    T: BufferContents + ?Sized,
{
    let staging_buffer = Buffer::new_unsized(
        memory_allocator.clone(),
        BufferCreateInfo {
            usage: BufferUsage::TRANSFER_SRC,
            ..Default::default()
        },
        AllocationCreateInfo {
            memory_type_filter: MemoryTypeFilter::PREFER_HOST
                | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
            ..Default::default()
        },
        data_len,
    )?;

    fill_buffer(&mut staging_buffer.write().unwrap());

    let destination_buffer = Buffer::new_unsized(
        memory_allocator.clone(),
        BufferCreateInfo {
            usage: usage | BufferUsage::TRANSFER_DST,
            ..Default::default()
        },
        AllocationCreateInfo {
            memory_type_filter: MemoryTypeFilter::PREFER_DEVICE,
            ..Default::default()
        },
        data_len,
    )?;

    let mut builder = vulkano::command_buffer::AutoCommandBufferBuilder::primary(
        command_buffer_allocator,
        queue.queue_family_index(),
        vulkano::command_buffer::CommandBufferUsage::OneTimeSubmit,
    )
    .unwrap();
    builder.copy_buffer(CopyBufferInfo::buffers(
        staging_buffer,
        destination_buffer.clone(),
    ))?;
    let command_buffer = builder.build().unwrap();

    let future = sync::now(queue.device().clone())
        .then_execute(queue.clone(), command_buffer)
        .unwrap()
        .then_signal_fence_and_flush()
        .unwrap();

    Ok((destination_buffer, future))
}

#[must_use = "The function returns a buffer that must be used"]
/// Creates a new uniform buffer.
pub fn new_uniform<T>(
    memory_allocator: &Arc<StandardMemoryAllocator>,
) -> Result<Subbuffer<T>, Validated<AllocateBufferError>>
where
    T: BufferContents,
{
    Buffer::new_sized(
        memory_allocator.clone(),
        BufferCreateInfo {
            usage: BufferUsage::UNIFORM_BUFFER,
            ..Default::default()
        },
        AllocationCreateInfo {
            memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                | MemoryTypeFilter::HOST_RANDOM_ACCESS,
            ..Default::default()
        },
    )
}
