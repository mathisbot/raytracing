use std::f32::consts::{FRAC_PI_2, TAU};

#[inline]
#[must_use]
fn dot(a: &[f32; 3], b: &[f32; 3]) -> f32 {
    a[0].mul_add(b[0], a[1].mul_add(b[1], a[2] * b[2]))
}

#[inline]
#[must_use]
fn cross(a: &[f32; 3], b: &[f32; 3]) -> [f32; 3] {
    [
        a[1].mul_add(b[2], -(a[2] * b[1])),
        a[2].mul_add(b[0], -(a[0] * b[2])),
        a[0].mul_add(b[1], -(a[1] * b[0])),
    ]
}

#[inline]
#[must_use]
fn normalize(a: &[f32; 3]) -> [f32; 3] {
    let dot = dot(a, a);
    // if dot == 0.0 {
    //     return [0.0, 0.0, 0.0];
    // }
    let inv_len = 1.0 / dot.sqrt();
    [a[0] * inv_len, a[1] * inv_len, a[2] * inv_len]
}

// Assert that UP is normalized
const UP: [f32; 3] = [0.0, 1.0, 0.0];

#[derive(Copy, Clone, Debug)]
pub struct Camera {
    position: [f32; 3],
    direction: [f32; 3],
    yaw: f32,
    pitch: f32,
}

impl Camera {
    const DEFAULT_YAW: f32 = 0.0;
    const DEFAULT_PITCH: f32 = 0.0;

    fn view_direction(yaw: f32, pitch: f32) -> [f32; 3] {
        [
            -yaw.sin() * pitch.cos(),
            -pitch.sin(),
            -yaw.cos() * pitch.cos(),
        ]
    }

    #[must_use]
    pub fn with_position(position: [f32; 3]) -> Self {
        Self {
            position,
            direction: Self::view_direction(Self::DEFAULT_YAW, Self::DEFAULT_PITCH),
            yaw: Self::DEFAULT_YAW,
            pitch: Self::DEFAULT_PITCH,
        }
    }

    #[must_use]
    pub const fn position(&self) -> [f32; 3] {
        self.position
    }

    #[must_use]
    pub const fn view(&self) -> [f32; 3] {
        self.direction
    }

    pub fn process_keyboard_input(&mut self, keyboard: super::Keyboard, delta_seconds: f32) {
        const SPEED: f32 = 10.0;
        let relative_speed = SPEED * delta_seconds;

        if keyboard.forward_pressed() {
            self.position = [
                self.direction[0].mul_add(relative_speed, self.position[0]),
                self.direction[1].mul_add(relative_speed, self.position[1]),
                self.direction[2].mul_add(relative_speed, self.position[2]),
            ];
        }

        if keyboard.backward_pressed() {
            self.position = [
                self.direction[0].mul_add(-relative_speed, self.position[0]),
                self.direction[1].mul_add(-relative_speed, self.position[1]),
                self.direction[2].mul_add(-relative_speed, self.position[2]),
            ];
        }

        if keyboard.left_pressed() {
            self.position = [
                self.direction[2].mul_add(relative_speed, self.position[0]),
                self.position[1],
                self.direction[0].mul_add(-relative_speed, self.position[2]),
            ];
        }

        if keyboard.right_pressed() {
            self.position = [
                self.direction[2].mul_add(-relative_speed, self.position[0]),
                self.position[1],
                self.direction[0].mul_add(relative_speed, self.position[2]),
            ];
        }

        if keyboard.up_pressed() {
            self.position[1] += relative_speed;
        }

        if keyboard.down() {
            self.position[1] -= relative_speed;
        }
    }

    pub fn process_mouse_input(&mut self, delta_x: f32, delta_y: f32) {
        const SENSITIVITY: f32 = 0.00075;

        self.yaw = delta_x.mul_add(SENSITIVITY, self.yaw) % TAU;
        self.pitch = delta_y
            .mul_add(SENSITIVITY, self.pitch)
            .clamp(-FRAC_PI_2 + 0.01, FRAC_PI_2 - 0.01);

        self.direction = Self::view_direction(self.yaw, self.pitch);
    }

    pub fn up_right(&self) -> ([f32; 3], [f32; 3]) {
        let right = normalize(&cross(&self.direction, &UP));
        let up = cross(&right, &self.direction);
        (up, right)
    }
}
