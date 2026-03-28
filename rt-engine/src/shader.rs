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

impl SceneDescriptor {
    #[allow(clippy::missing_const_for_fn)]
    #[must_use]
    /// Creates a scene descriptor from model paths and positions.
    pub fn new(model_paths: Vec<String>, positions: Vec<[f32; 3]>) -> Self {
        Self {
            model_paths,
            positions,
        }
    }

    #[must_use]
    /// Convenience constructor for a scene with a single model.
    pub fn single_model(model_path: String, position: [f32; 3]) -> Self {
        Self {
            model_paths: vec![model_path],
            positions: vec![position],
        }
    }
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

impl ShaderDescriptor {
    #[must_use]
    /// Creates shader parameters.
    pub const fn new(max_bounces: u8, samples: u16) -> Self {
        Self {
            max_bounces,
            samples,
        }
    }
}

impl Default for ShaderDescriptor {
    fn default() -> Self {
        Self {
            max_bounces: 6,
            samples: 16,
        }
    }
}

impl From<ShaderDescriptor> for source::ShaderConstants {
    fn from(descriptor: ShaderDescriptor) -> Self {
        Self {
            max_bounce_count: u32::from(descriptor.max_bounces),
            nb_samples: u32::from(descriptor.samples),
        }
    }
}
