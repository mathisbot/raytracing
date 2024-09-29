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
