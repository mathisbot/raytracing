use crate::shader::source::{Bvh, Triangle};
use vulkano::padded::Padded;

impl Bvh {
    #[must_use]
    #[inline]
    /// Compute the cost of a BVH node
    fn bvh_cost(min_bound: [f32; 3], max_bound: [f32; 3], count: u32) -> f64 {
        let dx = max_bound[0] - min_bound[0];
        let dy = max_bound[1] - min_bound[1];
        let dz = max_bound[2] - min_bound[2];

        let surface_area = f64::from(dx.mul_add(dy + dz, dy * dz));

        // Using f64 as the return type to avoid loss of precision when count is large
        // This slows down the BVH construction,
        // but allows for better performances when the BVH is used
        f64::from(count) * surface_area
    }

    #[inline]
    /// Grow the bounding box to include the given point
    fn grow_to_include(min_bound: &mut [f32; 3], max_bound: &mut [f32; 3], point: &[f32; 3]) {
        min_bound[0] = min_bound[0].min(point[0]);
        min_bound[1] = min_bound[1].min(point[1]);
        min_bound[2] = min_bound[2].min(point[2]);
        max_bound[0] = max_bound[0].max(point[0]);
        max_bound[1] = max_bound[1].max(point[1]);
        max_bound[2] = max_bound[2].max(point[2]);
    }

    #[must_use]
    /// Evaluate the cost of a split at the given position
    ///
    /// `split_axis` is the axis to split on, must be 0, 1, or 2
    fn evaluate_split(
        split_axis: usize,
        split_position: f32,
        triangles: &[Padded<Triangle, 8>],
    ) -> f64 {
        let mut min_bound_left = [f32::INFINITY; 3];
        let mut max_bound_left = [f32::NEG_INFINITY; 3];
        let mut min_bound_right = [f32::INFINITY; 3];
        let mut max_bound_right = [f32::NEG_INFINITY; 3];
        let mut count_left = 0;
        let mut count_right = 0;

        for triangle in triangles {
            let (min_bound, max_bound, count) = if triangle
                .vertices
                .iter()
                .any(|vertex| vertex[split_axis] < split_position)
            {
                (&mut min_bound_left, &mut max_bound_left, &mut count_left)
            } else {
                (&mut min_bound_right, &mut max_bound_right, &mut count_right)
            };

            for vertex in triangle.vertices {
                Self::grow_to_include(min_bound, max_bound, &vertex);
            }
            *count += 1;
        }

        Self::bvh_cost(min_bound_left, max_bound_left, count_left)
            + Self::bvh_cost(min_bound_right, max_bound_right, count_right)
    }

    #[must_use]
    /// Find the best split position for the given axis
    fn choose_split(bvh: Self, triangles: &[Padded<Triangle, 8>]) -> (usize, f32, f64) {
        /// The number of different split positions to test.
        const SPLIT_TEST_COUNT: u8 = 5;
        /// The minimum number of triangles in a leaf node.
        const MIN_TRIANGLES: usize = 2;

        if triangles.len() <= MIN_TRIANGLES {
            return (0, 0.0, f64::INFINITY);
        }

        let mut best_split_pos = 0.0;
        let mut best_split_axis = 0;
        let mut best_cost = f64::INFINITY;

        for axis in 0..3 {
            let delta = bvh.max_bound[axis] - bvh.min_bound[axis];

            for i in 0..SPLIT_TEST_COUNT {
                let split_lambda = f32::from(i + 1) / f32::from(SPLIT_TEST_COUNT + 1);
                let split_pos = split_lambda.mul_add(delta, bvh.min_bound[axis]);

                let cost = Self::evaluate_split(axis, split_pos, triangles);
                if cost < best_cost {
                    best_cost = cost;
                    best_split_pos = split_pos;
                    best_split_axis = axis;
                }
            }
        }

        (best_split_axis, best_split_pos, best_cost)
    }

    /// Recursively split the BVH
    fn split(bvhs: &mut Vec<Padded<Self, 4>>, triangles: &mut [Padded<Triangle, 8>]) {
        let start_bvh_len = u32::try_from(bvhs.len()).expect("too many BVHs");
        let bvh = bvhs.last_mut().unwrap();
        let triangle_offset = bvh.triangle_offset;
        let parent_cost = Self::bvh_cost(*bvh.min_bound, bvh.max_bound, bvh.triangle_count);

        let (split_axis, split_position, split_cost) = Self::choose_split(**bvh, triangles);

        if split_cost < 0.9 * parent_cost {
            let mut bvh_left = Self {
                min_bound: bvh.max_bound.into(),
                max_bound: *bvh.min_bound,
                left_offset: 0,
                right_offset: 0,
                triangle_offset,
                triangle_count: 0,
            };
            let mut bvh_right = Self {
                min_bound: bvh.max_bound.into(),
                max_bound: *bvh.min_bound,
                left_offset: 0,
                right_offset: 0,
                triangle_offset, // incorrect value
                triangle_count: 0,
            };

            for i in 0..triangles.len() {
                let left = triangles[i]
                    .vertices
                    .iter()
                    .any(|vertex| vertex[split_axis] < split_position);

                triangles.swap(i, bvh_left.triangle_count as usize);
                let triangle = *triangles[bvh_left.triangle_count as usize];

                let target_bvh = if left { &mut bvh_left } else { &mut bvh_right };
                target_bvh.triangle_count += 1;
                for vertex in triangle.vertices {
                    Self::grow_to_include(
                        &mut target_bvh.min_bound,
                        &mut target_bvh.max_bound,
                        &vertex,
                    );
                }
            }

            bvh.left_offset = start_bvh_len;
            // bvh is dropped here, so we can safely borrow bvhs again
            bvhs.push(bvh_left.into());
            Self::split(bvhs, &mut triangles[..bvh_left.triangle_count as usize]);

            // so that we need to borrow bvh again
            bvhs[start_bvh_len as usize - 1].right_offset =
                u32::try_from(bvhs.len()).expect("too many BVHs");
            bvh_right.triangle_offset = triangle_offset + bvh_left.triangle_count;
            bvhs.push(bvh_right.into());
            Self::split(bvhs, &mut triangles[bvh_left.triangle_count as usize..]);
        }
    }

    /// Build a BVH
    pub fn build(
        bvhs: &mut Vec<Padded<Self, 4>>,
        triangles: &mut [Padded<Triangle, 8>],
        triangle_offset: u32,
    ) {
        let mut min_bound = [f32::INFINITY; 3];
        let mut max_bound = [f32::NEG_INFINITY; 3];

        for triangle in &*triangles {
            for vertex in triangle.vertices {
                Self::grow_to_include(&mut min_bound, &mut max_bound, &vertex);
            }
        }

        bvhs.push(
            Self {
                min_bound: min_bound.into(),
                max_bound,
                left_offset: 0,
                right_offset: 0,
                triangle_offset,
                triangle_count: u32::try_from(triangles.len()).expect("too many triangles"),
            }
            .into(),
        );

        Self::split(bvhs, triangles);
    }
}
