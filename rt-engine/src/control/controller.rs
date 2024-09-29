pub mod keyboard;
pub mod motion_device;

pub trait Controller {
    fn handle_event(&mut self, event: &winit::event::Event<()>);
    fn fetch_input(&mut self) -> Vec<super::Input>;
}
