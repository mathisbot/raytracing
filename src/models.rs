use vulkano::{
    buffer::{BufferUsage, Subbuffer},
    sync::GpuFuture,
};

use crate::init::context::VulkanoContext;

mod bvh;
mod load;

#[allow(clippy::module_name_repetitions)]
pub struct LoadedModels {
    pub triangles_buffer: Subbuffer<crate::shaders::TrianglesBuffer>,
    pub materials_buffer: Subbuffer<crate::shaders::Materials>,
    pub models_buffer: Subbuffer<crate::shaders::ModelsBuffer>,
    pub bvhs_buffer: Subbuffer<crate::shaders::BvhBuffer>,
    pub future: Box<dyn vulkano::sync::GpuFuture>,
}

impl LoadedModels {
    #[must_use]
    pub fn load(context: &VulkanoContext, model_paths: &[String], positions: &[[f32; 3]]) -> Self {
        assert_eq!(
            model_paths.len(),
            positions.len(),
            "model_paths and positions must have the same length"
        );

        let mut triangles = Vec::new();
        let mut bvhs = Vec::new();
        let mut models = Vec::with_capacity(model_paths.len());
        for (model_path, model_position) in model_paths.iter().zip(positions) {
            let model =
                crate::shaders::Model::load(&mut triangles, &mut bvhs, model_path, model_position);
            models.push(model);
        }

        let (triangles_buffer, triangles_future) = {
            use crate::shaders::TrianglesBuffer;
            let staging_buffer = crate::buffers::new_staging_buffer::<TrianglesBuffer>(
                context,
                triangles.len() as u64,
            )
            .unwrap();
            staging_buffer
                .write()
                .unwrap()
                .triangles
                .copy_from_slice(&triangles);
            crate::buffers::send_staging_to_device(
                context,
                triangles.len() as u64,
                staging_buffer,
                BufferUsage::STORAGE_BUFFER,
            )
            .unwrap()
        };
        let (materials_buffer, material_future) = {
            use crate::shaders::{Material, Materials};
            let data = [Material {
                color: [0.8, 0.6, 0.6],
                albedo: 1.0,
                smoothness: 0.98,
                emission_strength: 0.0,
            }
            .into()];
            let staging_buffer =
                crate::buffers::new_staging_buffer::<Materials>(context, data.len() as u64)
                    .unwrap();
            staging_buffer
                .write()
                .unwrap()
                .materials
                .copy_from_slice(&data);
            crate::buffers::send_staging_to_device(
                context,
                data.len() as u64,
                staging_buffer,
                BufferUsage::STORAGE_BUFFER,
            )
            .unwrap()
        };
        let (models_buffer, models_future) = {
            use crate::shaders::ModelsBuffer;
            let data = &models;
            let staging_buffer =
                crate::buffers::new_staging_buffer::<ModelsBuffer>(context, data.len() as u64)
                    .unwrap();
            staging_buffer.write().unwrap().models.copy_from_slice(data);
            crate::buffers::send_staging_to_device(
                context,
                data.len() as u64,
                staging_buffer,
                BufferUsage::STORAGE_BUFFER,
            )
            .unwrap()
        };
        let (bvhs_buffer, bvh_future) = {
            use crate::shaders::BvhBuffer;
            let staging_buffer =
                crate::buffers::new_staging_buffer::<BvhBuffer>(context, bvhs.len() as u64)
                    .unwrap();
            staging_buffer.write().unwrap().bvhs.copy_from_slice(&bvhs);
            crate::buffers::send_staging_to_device(
                context,
                bvhs.len() as u64,
                staging_buffer,
                BufferUsage::STORAGE_BUFFER,
            )
            .unwrap()
        };

        let future = triangles_future
            .join(material_future)
            .join(models_future)
            .join(bvh_future);

        Self {
            triangles_buffer,
            materials_buffer,
            models_buffer,
            bvhs_buffer,
            future: Box::new(future),
        }
    }
}
