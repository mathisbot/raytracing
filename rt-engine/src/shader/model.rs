use std::sync::Arc;

use vulkano::{
    buffer::{BufferUsage, Subbuffer},
    command_buffer::allocator::StandardCommandBufferAllocator,
    device::Queue,
    memory::allocator::StandardMemoryAllocator,
    sync::GpuFuture,
};

/// The module containing the BVH construction implementation.
mod bvh;
/// The module containing the model loading implementation.
mod load;

#[allow(clippy::module_name_repetitions)]
/// Represents a loaded scene with models.
pub struct LoadedModels {
    /// The buffer containing the triangles of the models.
    pub triangles_buffer: Subbuffer<crate::shader::TrianglesBuffer>,
    /// The buffer containing the materials of the models.
    pub materials_buffer: Subbuffer<crate::shader::Materials>,
    /// The buffer containing the models.
    pub models_buffer: Subbuffer<crate::shader::ModelsBuffer>,
    /// The buffer containing the BVHs of the models.
    pub bvhs_buffer: Subbuffer<crate::shader::BvhBuffer>,
}

impl LoadedModels {
    #[must_use]
    /// Load the models from the given paths and positions.
    ///
    /// ## Panics
    ///
    /// This function will panic if one of the models cannot be loaded,
    /// or if the given positions and paths do not have the same length.
    pub fn load(
        memory_allocator: &Arc<StandardMemoryAllocator>,
        command_buffer_allocator: &Arc<StandardCommandBufferAllocator>,
        queue: &Arc<Queue>,
        model_paths: &[String],
        positions: &[[f32; 3]],
    ) -> Self {
        assert_eq!(
            model_paths.len(),
            positions.len(),
            "model_paths and positions must have the same length"
        );

        let mut triangles = Vec::new();
        let mut bvhs = Vec::new();
        let mut models = Vec::with_capacity(model_paths.len());
        for (model_path, model_position) in model_paths.iter().zip(positions) {
            let model = crate::shader::source::Model::load(
                &mut triangles,
                &mut bvhs,
                model_path,
                model_position,
            );
            models.push(model);
        }

        let (triangles_buffer, triangles_future) = {
            use crate::shader::TrianglesBuffer;
            let staging_buffer = crate::buffer::new_staging::<TrianglesBuffer>(
                memory_allocator,
                triangles.len() as u64,
            )
            .unwrap();
            staging_buffer
                .write()
                .unwrap()
                .triangles
                .copy_from_slice(&triangles);
            crate::buffer::send_staging_to_device(
                memory_allocator,
                command_buffer_allocator,
                queue,
                triangles.len() as u64,
                staging_buffer,
                BufferUsage::STORAGE_BUFFER,
            )
            .unwrap()
        };
        let (materials_buffer, material_future) = {
            use crate::shader::source::{Material, Materials};
            let data = [Material {
                color: [0.8, 0.6, 0.6],
                albedo: 1.0,
                smoothness: 0.98,
                emission_strength: 0.0,
            }
            .into()];
            let staging_buffer =
                crate::buffer::new_staging::<Materials>(memory_allocator, data.len() as u64)
                    .unwrap();
            staging_buffer
                .write()
                .unwrap()
                .materials
                .copy_from_slice(&data);
            crate::buffer::send_staging_to_device(
                memory_allocator,
                command_buffer_allocator,
                queue,
                data.len() as u64,
                staging_buffer,
                BufferUsage::STORAGE_BUFFER,
            )
            .unwrap()
        };
        let (models_buffer, models_future) = {
            use crate::shader::ModelsBuffer;
            let data = &models;
            let staging_buffer =
                crate::buffer::new_staging::<ModelsBuffer>(memory_allocator, data.len() as u64)
                    .unwrap();
            staging_buffer.write().unwrap().models.copy_from_slice(data);
            crate::buffer::send_staging_to_device(
                memory_allocator,
                command_buffer_allocator,
                queue,
                data.len() as u64,
                staging_buffer,
                BufferUsage::STORAGE_BUFFER,
            )
            .unwrap()
        };
        let (bvhs_buffer, bvh_future) = {
            use crate::shader::BvhBuffer;
            let staging_buffer =
                crate::buffer::new_staging::<BvhBuffer>(memory_allocator, bvhs.len() as u64)
                    .unwrap();
            staging_buffer.write().unwrap().bvhs.copy_from_slice(&bvhs);
            crate::buffer::send_staging_to_device(
                memory_allocator,
                command_buffer_allocator,
                queue,
                bvhs.len() as u64,
                staging_buffer,
                BufferUsage::STORAGE_BUFFER,
            )
            .unwrap()
        };

        triangles_future
            .join(material_future)
            .join(models_future)
            .join(bvh_future)
            .then_signal_fence()
            .wait(None)
            .unwrap();

        Self {
            triangles_buffer,
            materials_buffer,
            models_buffer,
            bvhs_buffer,
        }
    }
}
