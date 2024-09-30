pub mod keyboard;
pub mod motion_device;
pub mod mouse;

/// Represents a controller.
///
/// A controller is responsible for handling events and
/// is used by structs that implement `Camera` to fetch inputs.
pub trait Controller {
    /// Handle an event, usually by filtering by event type and
    /// updating the controller's state accordingly.
    fn handle_event(&mut self, event: &winit::event::Event<()>);

    /// Fetch the inputs from the controller's state.
    fn fetch_input(&mut self) -> Vec<super::Input>;
}
