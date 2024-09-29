use super::super::Input;

#[derive(Copy, Clone, Debug, Default)]
pub struct Keyboard(u8);

impl super::Controller for Keyboard {
    fn handle_event(&mut self, event: &winit::event::Event<()>) {
        if let winit::event::Event::WindowEvent {
            event:
                winit::event::WindowEvent::KeyboardInput {
                    input:
                        winit::event::KeyboardInput {
                            state,
                            virtual_keycode: Some(key),
                            ..
                        },
                    ..
                },
            ..
        } = event
        {
            // TODO: Personalize key bindings.
            let mask = match key {
                winit::event::VirtualKeyCode::Z => 0b0001,
                winit::event::VirtualKeyCode::Q => 0b0010,
                winit::event::VirtualKeyCode::S => 0b0100,
                winit::event::VirtualKeyCode::D => 0b1000,
                winit::event::VirtualKeyCode::Space => 0b1_0000,
                winit::event::VirtualKeyCode::LShift => 0b10_0000,
                _ => 0,
            };

            match state {
                winit::event::ElementState::Pressed => self.0 |= mask,
                winit::event::ElementState::Released => self.0 &= !mask,
            }
        }
    }

    fn fetch_input(&mut self) -> Vec<Input> {
        let mut inputs = Vec::new();

        if self.0 & 0b0001 != 0 {
            inputs.push(Input::Forward);
        }
        if self.0 & 0b0010 != 0 {
            inputs.push(Input::Left);
        }
        if self.0 & 0b0100 != 0 {
            inputs.push(Input::Backward);
        }
        if self.0 & 0b1000 != 0 {
            inputs.push(Input::Right);
        }

        if self.0 & 0b1_0000 != 0 {
            inputs.push(Input::Up);
        }

        if self.0 & 0b10_0000 != 0 {
            inputs.push(Input::Down);
        }

        inputs
    }
}
