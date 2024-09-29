pub mod first_person;

/// Represents a camera.
///
/// It is expected that all vectors (except for `position`) are normalized.
pub trait Camera {
    /// Returns the direction the camera is facing.
    fn direction(&self) -> [f32; 3];
    /// Returns the position of the camera.
    fn position(&self) -> [f32; 3];
    /// Returns the up vector of the camera.
    fn up(&self) -> [f32; 3];
    /// Returns the right vector of the camera.
    fn right(&self) -> [f32; 3];

    /// Processes the inputs and updates the camera.
    fn process_inputs(&mut self, inputs: &[super::Input], delta_seconds: f32);
}
