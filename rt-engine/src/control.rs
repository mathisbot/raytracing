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

/// Represents a set of inputs.
#[derive(Default, Debug, Clone, Copy)]
pub struct Inputs((u8, f32, f32));

impl Inputs {
    /// This function accumulates the inputs.
    pub fn accumulate(&mut self, inputs: Self) {
        self.0.0 |= inputs.0.0;
        self.0.1 += inputs.0.1;
        self.0.2 += inputs.0.2;
    }

    /// This function deccumulates the inputs.
    pub fn deccumulate(&mut self, inputs: Self) {
        self.0.0 &= !inputs.0.0;
        self.0.1 -= inputs.0.1;
        self.0.2 -= inputs.0.2;
    }
}

// Transparency between Inputs and Input
impl From<Input> for Inputs {
    fn from(input: Input) -> Self {
        match input {
            Input::Forward => Self((1 << 0, 0.0, 0.0)),
            Input::Backward => Self((1 << 1, 0.0, 0.0)),
            Input::Left => Self((1 << 2, 0.0, 0.0)),
            Input::Right => Self((1 << 3, 0.0, 0.0)),
            Input::Up => Self((1 << 4, 0.0, 0.0)),
            Input::Down => Self((1 << 5, 0.0, 0.0)),
            Input::Yaw(yaw) => Self((1 << 6, yaw, 0.0)),
            Input::Pitch(pitch) => Self((1 << 7, 0.0, pitch)),
        }
    }
}

// Transparency between Inputs and Input
impl From<&[Input]> for Inputs {
    fn from(inputs_list: &[Input]) -> Self {
        let mut inputs = Self::default();
        for input in inputs_list {
            inputs.accumulate((*input).into());
        }
        inputs
    }
}

// This method allocates, so it should only be used once
// by the camera to iterate over the inputs.
impl From<Inputs> for Box<[Input]> {
    fn from(inputs: Inputs) -> Self {
        let mut inputs_vec = Vec::new();
        if inputs.0.0 & (1 << 0) != 0 {
            inputs_vec.push(Input::Forward);
        }
        if inputs.0.0 & (1 << 1) != 0 {
            inputs_vec.push(Input::Backward);
        }
        if inputs.0.0 & (1 << 2) != 0 {
            inputs_vec.push(Input::Left);
        }
        if inputs.0.0 & (1 << 3) != 0 {
            inputs_vec.push(Input::Right);
        }
        if inputs.0.0 & (1 << 4) != 0 {
            inputs_vec.push(Input::Up);
        }
        if inputs.0.0 & (1 << 5) != 0 {
            inputs_vec.push(Input::Down);
        }
        if inputs.0.0 & (1 << 6) != 0 {
            inputs_vec.push(Input::Yaw(inputs.0.1));
        }
        if inputs.0.0 & (1 << 7) != 0 {
            inputs_vec.push(Input::Pitch(inputs.0.2));
        }
        inputs_vec.into()
    }
}
