use crate::init::context::VulkanoContext;
use vulkano::{
    buffer::{
        AllocateBufferError, Buffer, BufferContents, BufferCreateInfo, BufferUsage, Subbuffer,
    },
    command_buffer::{CommandBufferExecFuture, CopyBufferInfo},
    memory::allocator::{AllocationCreateInfo, MemoryTypeFilter},
    sync::{self, future::FenceSignalFuture, GpuFuture},
    Validated,
};

pub type SendBufferFuture = FenceSignalFuture<CommandBufferExecFuture<sync::future::NowFuture>>;

pub fn new_staging_buffer<T>(
    context: &VulkanoContext,
    data_len: u64,
) -> Result<Subbuffer<T>, Validated<AllocateBufferError>>
where
    T: BufferContents + ?Sized,
{
    Buffer::new_unsized(
        context.memory_allocator().clone(),
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
    )
}

pub fn send_staging_to_device<T>(
    context: &VulkanoContext,
    data_len: u64,
    staging_buffer: Subbuffer<T>,
    usage: BufferUsage,
) -> Result<(Subbuffer<T>, SendBufferFuture), Validated<AllocateBufferError>>
where
    T: BufferContents + ?Sized,
{
    let destination_buffer = Buffer::new_unsized(
        context.memory_allocator().clone(),
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
        &context.command_buffer_allocator().clone(),
        context.transfer_queue().queue_family_index(),
        vulkano::command_buffer::CommandBufferUsage::OneTimeSubmit,
    )
    .unwrap();
    builder.copy_buffer(CopyBufferInfo::buffers(
        staging_buffer,
        destination_buffer.clone(),
    ))?;
    let command_buffer = builder.build().unwrap();

    let future = sync::now(context.device().clone())
        .then_execute(context.transfer_queue().clone(), command_buffer)
        .unwrap()
        .then_signal_fence_and_flush()
        .unwrap();

    Ok((destination_buffer, future))
}

pub fn new_uniform<T>(
    context: &VulkanoContext,
) -> Result<Subbuffer<T>, Validated<AllocateBufferError>>
where
    T: BufferContents,
{
    Buffer::new_sized(
        context.memory_allocator().clone(),
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
