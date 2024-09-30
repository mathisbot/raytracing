//! This module contains the control system of the engine.
//!
//! It is used to move the camera around and interact with the scene.

pub mod camera;
pub mod controller;

#[derive(Debug, Clone, Copy)]
/// Represents an input.
pub enum Input {
    /// Move forward.
    Forward,
    /// Move backward.
    Backward,
    /// Move left.
    Left,
    /// Move right.
    Right,
    /// Move up.
    Up,
    /// Move down.
    Down,
    /// Yaw.
    Yaw(f32),
    /// Pitch.
    Pitch(f32),
}
