use crate::init::context::VulkanoContext;
use crate::init::renderer::VulkanoWindowRenderer;
use std::sync::Arc;
use vulkano::swapchain::PresentMode;
use winit::{
    dpi::{LogicalSize, PhysicalSize},
    window::{CursorGrabMode, Window},
};

#[allow(clippy::module_name_repetitions)]
pub struct VulkanoWindow {
    window: Arc<Window>,
    renderer: VulkanoWindowRenderer,
}

impl VulkanoWindow {
    #[must_use]
    pub fn new<T>(
        event_loop: &winit::event_loop::EventLoopWindowTarget<T>,
        vulkano_context: &VulkanoContext,
        window_descriptor: &WindowDescriptor,
    ) -> Self {
        let mut winit_window_builder: winit::window::WindowBuilder =
            winit::window::WindowBuilder::new().with_title(&window_descriptor.title);

        winit_window_builder = match window_descriptor.mode {
            WindowMode::BorderlessFullscreen => winit_window_builder.with_fullscreen(Some(
                winit::window::Fullscreen::Borderless(event_loop.primary_monitor()),
            )),
            WindowMode::Fullscreen => {
                #[cfg(target_os = "macos")]
                {
                    winit_window_builder.with_fullscreen(Some(
                        winit::window::Fullscreen::Borderless(event_loop.primary_monitor()),
                    ))
                }
                #[cfg(not(target_os = "macos"))]
                {
                    winit_window_builder.with_fullscreen(Some(
                        winit::window::Fullscreen::Exclusive({
                            let video_mode = get_best_videomode(
                                &event_loop
                                    .primary_monitor()
                                    .expect("could not find primary monitor"),
                            );
                            tracing::debug!(
                                "Best video mode: {}x{} @ {}Hz",
                                video_mode.size().width,
                                video_mode.size().height,
                                video_mode.refresh_rate_millihertz() / 1000
                            );
                            video_mode
                        }),
                    ))
                }
            }
            WindowMode::Windowed => {
                let WindowDescriptor {
                    width,
                    height,
                    position,
                    ..
                } = window_descriptor;

                if let Some(position) = position {
                    winit_window_builder =
                        winit_window_builder.with_position(winit::dpi::LogicalPosition::new(
                            f64::from(position[0]),
                            f64::from(position[1]),
                        ));
                }
                winit_window_builder.with_inner_size(LogicalSize::new(*width, *height))
            }
            .with_resizable(window_descriptor.resizable),
            WindowMode::SizedFullscreen => panic!("unsupported window mode"),
        };

        let constraints = window_descriptor.resize_constraints.check_constraints();
        let min_inner_size = LogicalSize {
            width: constraints.min_width,
            height: constraints.min_height,
        };
        let max_inner_size = LogicalSize {
            width: constraints.max_width,
            height: constraints.max_height,
        };

        winit_window_builder =
            if constraints.max_width < u32::MAX && constraints.max_height < u32::MAX {
                winit_window_builder
                    .with_min_inner_size(min_inner_size)
                    .with_max_inner_size(max_inner_size)
            } else {
                winit_window_builder.with_min_inner_size(min_inner_size)
            };

        let winit_window = winit_window_builder.build(event_loop).unwrap();

        if let Some(monitor) = winit_window.current_monitor() {
            if let Some(name) = monitor.name() {
                tracing::info!("Window created on monitor {}", name);
            }
        }

        if window_descriptor.cursor_locked {
            match winit_window.set_cursor_grab(CursorGrabMode::Confined) {
                Ok(()) => (),
                Err(winit::error::ExternalError::NotSupported(_)) => {
                    tracing::warn!("Cursor confinement is not supported on this platform");
                }
                Err(err) => panic!("{err:?}"),
            }
        }

        winit_window.set_cursor_visible(window_descriptor.cursor_visible);

        let window = Arc::new(winit_window);

        Self {
            renderer: VulkanoWindowRenderer::new(vulkano_context, &window, window_descriptor),
            window,
        }
    }

    #[must_use]
    pub fn renderer_mut(&mut self) -> &mut VulkanoWindowRenderer {
        &mut self.renderer
    }

    #[must_use]
    pub const fn renderer(&self) -> &VulkanoWindowRenderer {
        &self.renderer
    }

    #[must_use]
    pub fn window_size(&self) -> PhysicalSize<u32> {
        self.window.inner_size()
    }

    #[must_use]
    pub fn resolution(&self) -> (f64, f64) {
        let size = self.window.inner_size();
        let scale_factor = self.window.scale_factor();
        (
            f64::from(size.width) / scale_factor,
            f64::from(size.height) / scale_factor,
        )
    }

    #[must_use]
    pub fn aspect_ratio(&self) -> f64 {
        let (w, h): (u32, u32) = self.window_size().into();
        f64::from(w) / f64::from(h)
    }
}

fn get_best_videomode(monitor: &winit::monitor::MonitorHandle) -> winit::monitor::VideoMode {
    monitor
        .video_modes()
        .max_by(|a, b| {
            (a.size().width, a.size().height, a.refresh_rate_millihertz()).cmp(&(
                b.size().width,
                b.size().height,
                b.refresh_rate_millihertz(),
            ))
        })
        .unwrap()
}

#[allow(clippy::module_name_repetitions)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowMode {
    Windowed,
    BorderlessFullscreen,
    SizedFullscreen,
    Fullscreen,
}

#[allow(clippy::module_name_repetitions)]
#[derive(Debug, Clone)]
pub struct WindowDescriptor {
    pub width: u32,
    pub height: u32,
    pub position: Option<[f32; 2]>,
    pub resize_constraints: WindowResizeConstraints,
    pub title: String,
    pub present_mode: PresentMode,
    pub resizable: bool,
    pub cursor_visible: bool,
    pub cursor_locked: bool,
    pub mode: WindowMode,
}

impl Default for WindowDescriptor {
    fn default() -> Self {
        Self {
            title: "Title".to_string(),
            width: 1280,
            height: 720,
            position: None,
            resize_constraints: WindowResizeConstraints::default(),
            present_mode: PresentMode::Fifo,
            resizable: true,
            cursor_locked: false,
            cursor_visible: true,
            mode: WindowMode::Windowed,
        }
    }
}

#[allow(clippy::module_name_repetitions)]
#[derive(Debug, Clone, Copy)]
pub struct WindowResizeConstraints {
    pub min_width: u32,
    pub min_height: u32,
    pub max_width: u32,
    pub max_height: u32,
}

impl Default for WindowResizeConstraints {
    fn default() -> Self {
        Self {
            min_width: 180,
            min_height: 120,
            max_width: u32::MAX,
            max_height: u32::MAX,
        }
    }
}

impl WindowResizeConstraints {
    #[must_use]
    pub fn check_constraints(&self) -> Self {
        let Self {
            mut min_width,
            mut min_height,
            mut max_width,
            mut max_height,
        } = self;
        min_width = min_width.max(1);
        min_height = min_height.max(1);
        if max_width < min_width {
            tracing::debug!(
                "The given maximum width {} is smaller than the minimum width {}",
                max_width,
                min_width
            );
            max_width = min_width;
        }
        if max_height < min_height {
            tracing::debug!(
                "The given maximum height {} is smaller than the minimum height {}",
                max_height,
                min_height
            );
            max_height = min_height;
        }
        Self {
            min_width,
            min_height,
            max_width,
            max_height,
        }
    }
}
