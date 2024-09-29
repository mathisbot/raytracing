pub mod camera;
pub mod controller;

#[derive(Debug, Clone, Copy)]
pub enum Input {
    Forward,
    Backward,
    Left,
    Right,
    Up,
    Down,
    Yaw(f32),
    Pitch(f32),
}
