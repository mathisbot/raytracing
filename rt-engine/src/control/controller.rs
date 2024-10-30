//! This module contains the necessary trait used to handle different kind
//! of controllers, i.e. input sources.
//!
//! To implement a controller, simply create a struct with internal states and
//! implement the `Controller` trait for it.
//! Add it to the list of controllers in the main app struct and it will be
//! automatically handled by the event loop.

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
    ///
    /// ## Note
    ///
    /// If you want to handle an external controller which events
    /// are not supported by `winit`, filter `winit::event::Event::MainEventsCleared`
    /// which happens once per frame and update the controller's state accordingly.
    fn handle_event(&mut self, event: &winit::event::Event<()>);

    /// Fetch the inputs from the controller's state.
    ///
    /// This will be used by the `Camera` to update its state.
    fn fetch_input(&mut self) -> super::Inputs;
}
