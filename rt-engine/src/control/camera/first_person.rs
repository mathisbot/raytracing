use super::super::Input;

#[derive(Copy, Clone, Debug)]
/// Represents a first person camera.
pub struct FirstPerson {
    position: [f32; 3],
    direction: [f32; 3],
    up: [f32; 3],
    right: [f32; 3],
    yaw: f32,
    pitch: f32,
    speed: f32,
    sensitivity: f32,
}

impl FirstPerson {
    pub fn set_sentivity(&mut self, sensitivity: f32) {
        self.sensitivity = sensitivity;
    }
}

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
    fn direction(&self) -> [f32; 3] {
        self.direction
    }

    fn position(&self) -> [f32; 3] {
        self.position
    }

    fn up(&self) -> [f32; 3] {
        self.up
    }

    fn right(&self) -> [f32; 3] {
        self.right
    }

    fn process_inputs(&mut self, inputs: &[Input], delta_seconds: f32) {
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
