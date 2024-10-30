use super::super::{Input, Inputs};

#[derive(Copy, Clone, Debug, Default)]
/// Represents the state of a motion device.
/// This includes the mouse, as well as the joystick of a gamepad.
pub struct Mouse(f32, f32);

impl super::Controller for Mouse {
    fn handle_event(&mut self, event: &winit::event::Event<()>) {
        if let winit::event::Event::DeviceEvent {
            event:
                winit::event::DeviceEvent::MouseMotion {
                    delta: (delta_x, delta_y),
                },
            ..
        } = event
        {
            #[allow(clippy::cast_possible_truncation)]
            {
                self.0 -= *delta_x as f32;
                self.1 += *delta_y as f32;
            }
        }
    }

    #[must_use]
    fn fetch_input(&mut self) -> Inputs {
        let yaw = core::mem::take(&mut self.0);
        let pitch = core::mem::take(&mut self.1);

        let mut inputs = Inputs::default();
        if yaw != 0.0 {
            inputs.accumulate(Input::Yaw(yaw).into());
        }
        if pitch != 0.0 {
            inputs.accumulate(Input::Pitch(pitch).into());
        }

        inputs
    }
}
