#[derive(Copy, Clone, Debug, Default)]
pub struct Keyboard(u8);

impl Keyboard {
    const FORWARD: u8 = 1 << 0;
    const LEFT: u8 = 1 << 1;
    const RIGHT: u8 = 1 << 2;
    const BACKWARD: u8 = 1 << 3;
    const UP: u8 = 1 << 4;
    const DOWN: u8 = 1 << 5;

    // TODO: Configurable key bindings (using HashMap? Lookup table?)
    pub fn handle_keypress(&mut self, event: &winit::event::KeyboardInput) {
        let pressed = u8::from(event.state == winit::event::ElementState::Pressed);
        match event.scancode {
            0x11 => self.0 = self.0 & !(Self::FORWARD) | (Self::FORWARD * pressed),
            0x1E => self.0 = self.0 & !(Self::LEFT) | (Self::LEFT * pressed),
            0x20 => self.0 = self.0 & !(Self::RIGHT) | (Self::RIGHT * pressed),
            0x1F => self.0 = self.0 & !(Self::BACKWARD) | (Self::BACKWARD * pressed),
            0x39 => self.0 = self.0 & !(Self::UP) | (Self::UP * pressed),
            0x2A => self.0 = self.0 & !(Self::DOWN) | (Self::DOWN * pressed),
            _ => (),
        };
    }

    #[must_use]
    pub const fn forward_pressed(self) -> bool {
        self.0 & Self::FORWARD == Self::FORWARD
    }

    #[must_use]
    pub const fn left_pressed(self) -> bool {
        self.0 & Self::LEFT == Self::LEFT
    }

    #[must_use]
    pub const fn right_pressed(self) -> bool {
        self.0 & Self::RIGHT == Self::RIGHT
    }

    #[must_use]
    pub const fn backward_pressed(self) -> bool {
        self.0 & Self::BACKWARD == Self::BACKWARD
    }

    #[must_use]
    pub const fn up_pressed(self) -> bool {
        self.0 & Self::UP == Self::UP
    }

    #[must_use]
    pub const fn down(self) -> bool {
        self.0 & Self::DOWN == Self::DOWN
    }
}
