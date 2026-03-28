use crate::shader::source::{Bvh, Model, Triangle};
use vulkano::padded::Padded;

impl Model {
    #[must_use]
    /// Load a model from the given `.obj` source file
    ///
    /// ## Panics
    ///
    /// This function panics if the model cannot be loaded, typically due to an invalid `.obj` file.
    pub fn load(
        triangles: &mut Vec<Padded<Triangle, 8>>,
        bvhs: &mut Vec<Padded<Bvh, 4>>,
        src: &str,
        position: &[f32; 3],
    ) -> Self {
        let triangle_offset = triangles.len();
        let bvh_index = u32::try_from(bvhs.len()).expect("too many BVHs");

        let start = std::time::Instant::now();

        let (models, materials) =
            tobj::load_obj(src, &tobj::GPU_LOAD_OPTIONS).expect("failed to load OBJ file");
        if let Err(error) = materials {
            tracing::warn!(
                "Material file couldn't be loaded for model '{src}': {error}. Continuing without materials."
            );
        }

        let total_new_triangles = models
            .iter()
            .map(|model| model.mesh.indices.len() / 3)
            .sum::<usize>();
        triangles.reserve(total_new_triangles);

        for model in &models {
            let mesh = &model.mesh;

            let uv_at = |vertex_index: usize| -> [f32; 2] {
                let uv_index = vertex_index * 2;
                match (
                    mesh.texcoords.get(uv_index),
                    mesh.texcoords.get(uv_index + 1),
                ) {
                    (Some(&u), Some(&v)) => [u, v],
                    _ => [0.0, 0.0],
                }
            };

            for i in (0..mesh.indices.len()).step_by(3) {
                let a = mesh.indices[i] as usize;
                let b = mesh.indices[i + 1] as usize;
                let c = mesh.indices[i + 2] as usize;

                let a0 = a * 3;
                let b0 = b * 3;
                let c0 = c * 3;

                let a_pos = [
                    mesh.positions[a0] + position[0],
                    mesh.positions[a0 + 1] + position[1],
                    mesh.positions[a0 + 2] + position[2],
                ];
                let b_pos = [
                    mesh.positions[b0] + position[0],
                    mesh.positions[b0 + 1] + position[1],
                    mesh.positions[b0 + 2] + position[2],
                ];
                let c_pos = [
                    mesh.positions[c0] + position[0],
                    mesh.positions[c0 + 1] + position[1],
                    mesh.positions[c0 + 2] + position[2],
                ];

                let ab = [
                    b_pos[0] - a_pos[0],
                    b_pos[1] - a_pos[1],
                    b_pos[2] - a_pos[2],
                ];
                let ac = [
                    c_pos[0] - a_pos[0],
                    c_pos[1] - a_pos[1],
                    c_pos[2] - a_pos[2],
                ];
                let normal = [
                    ab[1].mul_add(ac[2], -(ab[2] * ac[1])),
                    ab[2].mul_add(ac[0], -(ab[0] * ac[2])),
                    ab[0].mul_add(ac[1], -(ab[1] * ac[0])),
                ];

                let triangle = Triangle {
                    vertices: [a_pos.into(), b_pos.into(), c_pos.into()],
                    normal: normal.into(),
                    uv: [uv_at(a), uv_at(b), uv_at(c)],
                };

                triangles.push(triangle.into());
            }
        }

        Bvh::build(
            bvhs,
            &mut triangles[triangle_offset..],
            u32::try_from(triangle_offset).expect("too many triangles"),
        );

        let bvh_count = u32::try_from(bvhs.len()).expect("too many BVHs") - bvh_index;

        tracing::trace!(
            "Model loaded in {:?} with {} triangles and {} BVH nodes",
            start.elapsed(),
            triangles.len() - triangle_offset,
            bvh_count
        );

        Self {
            bvh_index,
            // TODO: Material ID
            material_id: 0,
        }
    }
}
