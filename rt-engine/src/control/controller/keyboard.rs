use super::super::{Input, Inputs};

#[derive(Copy, Clone, Debug, Default)]
/// Represents the state of a keyboard.
pub struct Keyboard(Inputs);

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
                winit::event::VirtualKeyCode::Z => Input::Forward,
                winit::event::VirtualKeyCode::Q => Input::Left,
                winit::event::VirtualKeyCode::S => Input::Backward,
                winit::event::VirtualKeyCode::D => Input::Right,
                winit::event::VirtualKeyCode::Space => Input::Up,
                winit::event::VirtualKeyCode::LShift => Input::Down,
                _ => return,
            };

            match state {
                winit::event::ElementState::Pressed => self.0.accumulate(mask.into()),
                winit::event::ElementState::Released => self.0.deccumulate(mask.into()),
            }
        }
    }

    #[must_use]
    #[inline]
    fn fetch_input(&mut self) -> Inputs {
        self.0
    }
}
