pub mod model;

/// Source code of the shader, this module defines
/// all the structs used in the shader.
pub(crate) mod source {
    vulkano_shaders::shader! {
        shaders: {
            compute: {
                ty: "compute",
                path: r"src/shader/ray_trace.comp",
            },
        }
    }
}

pub use source::{BvhBuffer, CameraBuffer, Materials, ModelsBuffer, TrianglesBuffer};

#[derive(Debug, Clone)]
/// This struct is used at the initialization of the application.
///
/// It contains the paths of the models and their positions.
pub struct SceneDescriptor {
    /// A vector of path to `.obj files`.
    pub model_paths: Vec<String>,
    /// A vector of positions for the models.
    ///
    /// They represent translations that will be applied
    /// to models on load.
    pub positions: Vec<[f32; 3]>,
}
