use super::super::Input;

#[derive(Copy, Clone, Debug, Default)]
/// Represents the state of a motion device.
/// This includes the mouse, as well as the joystick of a gamepad.
pub struct MotionDevice(f32, f32);

impl super::Controller for MotionDevice {
    fn handle_event(&mut self, event: &winit::event::Event<()>) {
        if let winit::event::Event::DeviceEvent {
            event: winit::event::DeviceEvent::Motion { axis, value },
            ..
        } = event
        {
            match axis {
                #[allow(clippy::cast_possible_truncation)]
                0 => self.0 -= *value as f32,
                #[allow(clippy::cast_possible_truncation)]
                1 => self.1 += *value as f32,
                _ => unreachable!("Unexpected axis"),
            }
        }
    }

    #[must_use]
    fn fetch_input(&mut self) -> Vec<Input> {
        let yaw = core::mem::take(&mut self.0);
        let pitch = core::mem::take(&mut self.1);

        let mut inputs = Vec::with_capacity(2);
        if yaw != 0.0 {
            inputs.push(Input::Yaw(yaw));
        }
        if pitch != 0.0 {
            inputs.push(Input::Pitch(pitch));
        }

        inputs
    }
}
