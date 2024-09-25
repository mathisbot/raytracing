#[derive(Copy, Clone, Debug, Default)]
pub struct Mouse {
    mouse_delta: (f32, f32),
}

impl Mouse {
    /// This function should be called to handle mouse movement events.
    /// Movements are stacked into an inner variable and can be fetched later
    /// with `fetch_mouse_delta`.
    pub fn handle_mousemove(&mut self, axis: winit::event::AxisId, value: f32) {
        match axis {
            0 => self.mouse_delta.0 -= value,
            1 => self.mouse_delta.1 += value,
            _ => unreachable!("unexpected axis id"),
        }
    }

    /// This function should be called to fetch the accumulated mouse movements.
    /// It returns the accumulated mouse movements and resets the inner variable.
    pub fn fetch_mouse_delta(&mut self) -> (f32, f32) {
        core::mem::take(&mut self.mouse_delta)
    }
}
