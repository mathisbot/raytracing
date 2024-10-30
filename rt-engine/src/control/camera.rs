//! This module contains the necessary trait used to handle different kind
//! of cameras.
//!
//! To implement a camera, simply create a struct with internal states and
//! implement the `Camera` trait for it.
//! Mention it in the main app struct and it will be automatically handled
//! by the event loop.

pub mod first_person;

/// Represents a camera.
///
/// It is expected that all vectors (except for `position`) are normalized.
pub trait Camera {
    /// Returns the direction the camera is facing.
    ///
    /// The direction must be normalized.
    fn direction(&self) -> [f32; 3];
    /// Returns the position of the camera.
    ///
    /// The position is in world space.
    fn position(&self) -> [f32; 3];
    /// Returns the up vector of the camera.
    ///
    /// The up vector must be normalized.
    fn up(&self) -> [f32; 3];
    /// Returns the right vector of the camera.
    ///
    /// The right vector must be normalized.
    fn right(&self) -> [f32; 3];

    /// Processes the inputs and updates the camera.
    ///
    /// Typically, this means updating the camera's position, orientation, etc.
    fn process_inputs(&mut self, inputs: super::Inputs, delta_seconds: f32);
}
