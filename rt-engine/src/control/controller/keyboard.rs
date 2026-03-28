use super::super::{Input, Inputs};
use winit::keyboard::{Key, NamedKey};

#[derive(Copy, Clone, Debug, Default)]
/// Represents the state of a keyboard.
pub struct Keyboard(Inputs);

impl super::Controller for Keyboard {
    fn handle_event(&mut self, event: &winit::event::Event<()>) {
        if let winit::event::Event::WindowEvent {
            event:
                winit::event::WindowEvent::KeyboardInput {
                    event: key_event, ..
                },
            ..
        } = event
        {
            // TODO: Personalize key bindings.
            let mask = match key_event.logical_key.as_ref() {
                Key::Character("z" | "Z") => Input::Forward,
                Key::Character("q" | "Q") => Input::Left,
                Key::Character("s" | "S") => Input::Backward,
                Key::Character("d" | "D") => Input::Right,
                Key::Named(NamedKey::Space) => Input::Up,
                Key::Named(NamedKey::Shift) => Input::Down,
                _ => return,
            };

            match key_event.state {
                winit::event::ElementState::Pressed => self.0.accumulate(mask.into()),
                winit::event::ElementState::Released => self.0.deccumulate(mask.into()),
            }
        }
    }

    #[inline]
    fn fetch_input(&mut self) -> Inputs {
        self.0
    }
}
