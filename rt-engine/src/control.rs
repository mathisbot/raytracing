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
