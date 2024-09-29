pub mod first_person;

/// Represents a camera.
///
/// It is expected that all vectors (except for `position`) are normalized.
pub trait Camera {
    fn direction(&self) -> [f32; 3];
    fn position(&self) -> [f32; 3];
    fn up(&self) -> [f32; 3];
    fn right(&self) -> [f32; 3];

    fn process_inputs(&mut self, inputs: &[super::Input], delta_seconds: f32);
}
