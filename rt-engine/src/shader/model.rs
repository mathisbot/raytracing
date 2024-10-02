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

#[derive(Clone)]
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
        scene_descriptor: &super::SceneDescriptor,
    ) -> Self {
        let super::SceneDescriptor {
            model_paths,
            positions,
        } = scene_descriptor;

        assert_eq!(
            model_paths.len(),
            positions.len(),
            "model_paths and positions must have the same length"
        );

        let mut triangles = Vec::new();
        let mut bvhs = Vec::new();
        let models = model_paths
            .iter()
            .zip(positions)
            .map(|(path, position)| {
                crate::shader::source::Model::load(&mut triangles, &mut bvhs, path, position)
            })
            .collect::<Vec<_>>();

        let (triangles_buffer, triangles_future) = {
            use crate::shader::TrianglesBuffer;

            crate::buffer::send_to_device(
                memory_allocator,
                command_buffer_allocator,
                queue,
                triangles.len() as u64,
                BufferUsage::STORAGE_BUFFER,
                |data: &mut TrianglesBuffer| data.triangles.copy_from_slice(&triangles),
            )
            .unwrap()
        };

        let (materials_buffer, material_future) = {
            use crate::shader::source::{Material, Materials};

            let materials = [Material {
                color: [0.8, 0.6, 0.6],
                albedo: 1.0,
                smoothness: 0.98,
                emission_strength: 0.0,
            }
            .into()];

            crate::buffer::send_to_device(
                memory_allocator,
                command_buffer_allocator,
                queue,
                materials.len() as u64,
                BufferUsage::STORAGE_BUFFER,
                |data: &mut Materials| data.materials.copy_from_slice(&materials),
            )
            .unwrap()
        };

        let (models_buffer, models_future) = {
            use crate::shader::ModelsBuffer;

            crate::buffer::send_to_device(
                memory_allocator,
                command_buffer_allocator,
                queue,
                models.len() as u64,
                BufferUsage::STORAGE_BUFFER,
                |data: &mut ModelsBuffer| data.models.copy_from_slice(&models),
            )
            .unwrap()
        };

        let (bvhs_buffer, bvh_future) = {
            use crate::shader::BvhBuffer;

            crate::buffer::send_to_device(
                memory_allocator,
                command_buffer_allocator,
                queue,
                bvhs.len() as u64,
                BufferUsage::STORAGE_BUFFER,
                |data: &mut BvhBuffer| data.bvhs.copy_from_slice(&bvhs),
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
