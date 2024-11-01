use super::super::{Input, Inputs};

#[derive(Copy, Clone, Debug)]
/// Represents a first person camera.
pub struct FirstPerson {
    /// The position of the camera.
    position: [f32; 3],
    /// The direction the camera is facing.
    ///
    /// The direction is only stored for speed purposes,
    /// the direction is only really computed when the pitch or yaw changes.
    direction: [f32; 3],
    /// The up vector of the camera.
    ///
    /// The up vector is only stored for speed purposes,
    /// the up vector is only really computed when the pitch or yaw changes.
    up: [f32; 3],
    /// The right vector of the camera.
    ///
    /// The right vector is only stored for speed purposes,
    /// the right vector is only really computed when the pitch or yaw changes.
    right: [f32; 3],
    /// The yaw of the camera.
    yaw: f32,
    /// The pitch of the camera.
    pitch: f32,
    /// The speed of the camera.
    speed: f32,
    /// The sensitivity of the camera.
    sensitivity: f32,
}

impl FirstPerson {
    #[must_use]
    pub fn from_position_yaw_pitch(position: [f32; 3], yaw: f32, pitch: f32) -> Self {
        let direction = [
            yaw.to_radians().cos() * pitch.to_radians().cos(),
            pitch.to_radians().sin(),
            yaw.to_radians().sin() * pitch.to_radians().cos(),
        ];

        let right = [-yaw.to_radians().sin(), 0.0, yaw.to_radians().cos()];

        let mut up = [
            right[1].mul_add(direction[2], -(right[2] * direction[1])),
            right[2].mul_add(direction[0], -(right[0] * direction[2])),
            right[0].mul_add(direction[1], -(right[1] * direction[0])),
        ];

        // normalize(&mut self.direction); // This is not necessary, as the direction is normalized by the pitch and yaw.
        // normalize(&mut self.right); // This is not necessary, as the right vector is normalized by the yaw.
        normalize(&mut up);

        Self {
            position,
            direction,
            up,
            right,
            yaw,
            pitch,
            ..Default::default()
        }
    }

    #[inline]
    /// Sets the sensitivity of the camera.
    pub fn set_sentivity(&mut self, sensitivity: f32) {
        self.sensitivity = sensitivity;
    }

    #[inline]
    /// Sets the speed of the camera.
    pub fn set_speed(&mut self, speed: f32) {
        self.speed = speed;
    }
}

#[inline]
/// Normalizes in-place a 3D vector.
fn normalize(v: &mut [f32; 3]) {
    let inv_length = 1.0 / (v[2].mul_add(v[2], v[0].mul_add(v[0], v[1] * v[1]))).sqrt();
    v[0] *= inv_length;
    v[1] *= inv_length;
    v[2] *= inv_length;
}

impl Default for FirstPerson {
    fn default() -> Self {
        Self {
            position: [0.0, 0.0, 0.0],
            direction: [0.0, 0.0, -1.0],
            up: [0.0, 1.0, 0.0],
            right: [-1.0, 0.0, 0.0],
            yaw: 270.0,
            pitch: 0.0,
            speed: 10.0,
            sensitivity: 0.03,
        }
    }
}

impl super::Camera for FirstPerson {
    #[must_use]
    #[inline]
    fn direction(&self) -> [f32; 3] {
        self.direction
    }

    #[must_use]
    #[inline]
    fn position(&self) -> [f32; 3] {
        self.position
    }

    #[must_use]
    #[inline]
    fn up(&self) -> [f32; 3] {
        self.up
    }

    #[must_use]
    #[inline]
    fn right(&self) -> [f32; 3] {
        self.right
    }

    fn process_inputs(&mut self, inputs: Inputs, delta_seconds: f32) {
        let inputs = Into::<Box<[Input]>>::into(inputs);
        if inputs.is_empty() {
            return;
        }

        let relative_speed = self.speed * delta_seconds;

        for input in inputs {
            match input {
                Input::Forward => {
                    self.position[0] += self.direction[0] * relative_speed;
                    self.position[1] += self.direction[1] * relative_speed;
                    self.position[2] += self.direction[2] * relative_speed;
                }
                Input::Backward => {
                    self.position[0] -= self.direction[0] * relative_speed;
                    self.position[1] -= self.direction[1] * relative_speed;
                    self.position[2] -= self.direction[2] * relative_speed;
                }
                Input::Left => {
                    self.position[0] -= self.right[0] * relative_speed;
                    self.position[1] -= self.right[1] * relative_speed;
                    self.position[2] -= self.right[2] * relative_speed;
                }
                Input::Right => {
                    self.position[0] += self.right[0] * relative_speed;
                    self.position[1] += self.right[1] * relative_speed;
                    self.position[2] += self.right[2] * relative_speed;
                }
                Input::Up => {
                    self.position[0] += self.up[0] * relative_speed;
                    self.position[1] += self.up[1] * relative_speed;
                    self.position[2] += self.up[2] * relative_speed;
                }
                Input::Down => {
                    self.position[0] -= self.up[0] * relative_speed;
                    self.position[1] -= self.up[1] * relative_speed;
                    self.position[2] -= self.up[2] * relative_speed;
                }
                Input::Yaw(value) => {
                    self.yaw -= value * self.sensitivity;
                }
                Input::Pitch(value) => {
                    self.pitch -= value * self.sensitivity;
                }
            }
        }

        self.direction = [
            self.yaw.to_radians().cos() * self.pitch.to_radians().cos(),
            self.pitch.to_radians().sin(),
            self.yaw.to_radians().sin() * self.pitch.to_radians().cos(),
        ];

        self.right = [
            -self.yaw.to_radians().sin(),
            0.0,
            self.yaw.to_radians().cos(),
        ];

        self.up = [
            self.right[1].mul_add(self.direction[2], -(self.right[2] * self.direction[1])),
            self.right[2].mul_add(self.direction[0], -(self.right[0] * self.direction[2])),
            self.right[0].mul_add(self.direction[1], -(self.right[1] * self.direction[0])),
        ];

        // normalize(&mut self.direction); // This is not necessary, as the direction is normalized by the pitch and yaw.
        // normalize(&mut self.right); // This is not necessary, as the right vector is normalized by the yaw.
        normalize(&mut self.up);
    }
}
