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

#[derive(Debug, Clone, Copy)]
#[allow(clippy::module_name_repetitions)]
/// This struct is used at the initialization of the application.
///
/// It contains the parameters for the shader.
pub struct ShaderDescriptor {
    /// Max number of bounces for a ray.
    pub max_bounces: u8,
    /// Max number of samples for a pixel.
    pub samples: u16,
}

impl From<ShaderDescriptor> for source::ShaderConstants {
    fn from(descriptor: ShaderDescriptor) -> Self {
        Self {
            max_bounce_count: u32::from(descriptor.max_bounces),
            nb_samples: u32::from(descriptor.samples),
        }
    }
}
