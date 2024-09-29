use crate::shader::source::{Bvh, Model, Triangle};
use vulkano::padded::Padded;

impl Model {
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
        // TODO: Materials
        let _materials = materials.expect("failed to load materials");

        for model in &models {
            let mesh = &model.mesh;
            for i in (0..mesh.indices.len()).step_by(3) {
                let a = mesh.indices[i] as usize;
                let b = mesh.indices[i + 1] as usize;
                let c = mesh.indices[i + 2] as usize;

                let triangle = Triangle {
                    vertices: [
                        [
                            mesh.positions[a * 3] + position[0],
                            mesh.positions[a * 3 + 1] + position[1],
                            mesh.positions[a * 3 + 2] + position[2],
                        ]
                        .into(),
                        [
                            mesh.positions[b * 3] + position[0],
                            mesh.positions[b * 3 + 1] + position[1],
                            mesh.positions[b * 3 + 2] + position[2],
                        ]
                        .into(),
                        [
                            mesh.positions[c * 3] + position[0],
                            mesh.positions[c * 3 + 1] + position[1],
                            mesh.positions[c * 3 + 2] + position[2],
                        ]
                        .into(),
                    ],
                    normal: {
                        let ab = [
                            mesh.positions[b * 3] - mesh.positions[a * 3],
                            mesh.positions[b * 3 + 1] - mesh.positions[a * 3 + 1],
                            mesh.positions[b * 3 + 2] - mesh.positions[a * 3 + 2],
                        ];
                        let ac = [
                            mesh.positions[c * 3] - mesh.positions[a * 3],
                            mesh.positions[c * 3 + 1] - mesh.positions[a * 3 + 1],
                            mesh.positions[c * 3 + 2] - mesh.positions[a * 3 + 2],
                        ];
                        [
                            ab[1].mul_add(ac[2], -(ab[2] * ac[1])),
                            ab[2].mul_add(ac[0], -(ab[0] * ac[2])),
                            ab[0].mul_add(ac[1], -(ab[1] * ac[0])),
                        ]
                    }
                    .into(),
                    uv: [
                        [mesh.texcoords[a * 2], mesh.texcoords[a * 2 + 1]],
                        [mesh.texcoords[b * 2], mesh.texcoords[b * 2 + 1]],
                        [mesh.texcoords[c * 2], mesh.texcoords[c * 2 + 1]],
                    ],
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
