use std::sync::Arc;

use vulkano::{
    device::{Device, Queue},
    image::{view::ImageView, ImageUsage},
    swapchain::{self, Surface, SurfaceInfo, Swapchain, SwapchainCreateInfo},
    sync::GpuFuture,
};
use winit::{dpi::LogicalSize, window::CursorGrabMode};

#[derive(Clone, Debug)]
/// Represents a window.
pub struct Window {
    /// Inner `winit` window.
    window: Arc<winit::window::Window>,
    /// The swapchain of the window.
    swapchain: Arc<Swapchain>,
    /// The final views of the swapchain.
    image_views: Vec<Arc<ImageView>>,
    /// Whether the swapchain needs to be recreated.
    recreate_swapchain: bool,
    /// The index of the image to be rendered.
    image_index: u32,
    /// The present mode of the window.
    present_mode: PresentMode,
}

impl Window {
    #[must_use]
    /// Creates a new window.
    ///
    /// ## Panics
    ///
    /// The function will panic if anything goes wrong during window creation.
    pub fn new(
        event_loop: &winit::event_loop::EventLoop<()>,
        device: &Arc<Device>,
        window_descriptor: &WindowDescriptor,
    ) -> Self {
        let mut winit_window_builder: winit::window::WindowBuilder =
            winit::window::WindowBuilder::new().with_title(&window_descriptor.title);

        winit_window_builder = match window_descriptor.mode {
            Mode::BorderlessFullscreen => winit_window_builder.with_fullscreen(Some(
                winit::window::Fullscreen::Borderless(event_loop.primary_monitor()),
            )),
            Mode::Fullscreen => {
                winit_window_builder.with_fullscreen(Some(if cfg!(target_os = "macos") {
                    winit::window::Fullscreen::Borderless(event_loop.primary_monitor())
                } else {
                    // FIXME: Exclusive fullscreen
                    winit::window::Fullscreen::Exclusive({
                        let video_mode = Self::get_best_videomode(
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
                    })
                }))
            }
            Mode::Windowed => {
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
            match winit_window.set_cursor_grab(if cfg!(target_os = "macos") {
                CursorGrabMode::Locked
            } else {
                CursorGrabMode::Confined
            }) {
                Ok(()) => (),
                Err(winit::error::ExternalError::NotSupported(_)) => {
                    tracing::warn!("Cursor confinement is not supported on this platform");
                }
                Err(err) => tracing::error!("Error confining cursor: {err:?}"),
            }
        }

        winit_window.set_cursor_visible(window_descriptor.cursor_visible);

        let window = Arc::new(winit_window);

        let (swapchain, final_views) =
            Self::create_swapchain(device.clone(), &window, window_descriptor);

        Self {
            window,
            recreate_swapchain: false,
            image_index: 0,
            present_mode: window_descriptor.present_mode,
            swapchain,
            image_views: final_views,
        }
    }

    #[must_use]
    /// Creates a new swapchain.
    fn create_swapchain(
        device: Arc<Device>,
        window: &Arc<winit::window::Window>,
        window_descriptor: &WindowDescriptor,
    ) -> (Arc<Swapchain>, Vec<Arc<ImageView>>) {
        let surface = Surface::from_window(device.instance().clone(), window.clone()).unwrap();
        let surface_capabilities = device
            .physical_device()
            .surface_capabilities(&surface, SurfaceInfo::default())
            .unwrap();

        assert!(
            device
                .physical_device()
                .surface_formats(&surface, SurfaceInfo::default())
                .unwrap()
                .iter()
                .any(|(format, _)| *format == vulkano::format::Format::R8G8B8A8_UNORM),
            "required surface format R8G8B8A8_UNORM is not supported"
        );

        let mut available_swapchain_present_modes = device
            .physical_device()
            .surface_present_modes(&surface, SurfaceInfo::default())
            .unwrap();

        let present_mode = if available_swapchain_present_modes
            .any(|p| p == window_descriptor.present_mode.into())
        {
            window_descriptor.present_mode
        } else {
            /// This present mode is guaranteed to be supported,
            /// so we can safely fall back to it.
            const FALLBACK_PRESENT_MODE: PresentMode = PresentMode::Fifo;
            tracing::warn!(
                "request present mode {:?} not supported, falling back to {:?}",
                window_descriptor.present_mode,
                FALLBACK_PRESENT_MODE
            );
            FALLBACK_PRESENT_MODE
        };

        let (swapchain, images) = Swapchain::new(
            device,
            surface,
            SwapchainCreateInfo {
                min_image_count: surface_capabilities.min_image_count + 1,
                image_format: vulkano::format::Format::R8G8B8A8_UNORM,
                image_extent: window.inner_size().into(),
                image_usage: ImageUsage::STORAGE | ImageUsage::COLOR_ATTACHMENT,
                present_mode: present_mode.into(),
                ..Default::default()
            },
        )
        .unwrap();

        let images_views = images
            .into_iter()
            .map(|image| ImageView::new_default(image).unwrap())
            .collect::<Vec<_>>();

        (swapchain, images_views)
    }

    #[must_use]
    /// Returns the best video mode of the given monitor.
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

    /// Recreates the swapchain and its views.
    fn recreate_swapchain_and_views(&mut self) {
        let [desired_width, desired_height]: [u32; 2] = self.window.inner_size().into();

        if desired_width == 0 || desired_height == 0 {
            return;
        }

        let (new_swapchain, new_images) = self
            .swapchain
            .recreate(SwapchainCreateInfo {
                image_extent: [desired_width, desired_height],
                present_mode: self.present_mode.into(),
                ..self.swapchain.create_info()
            })
            .expect("failed to recreate swapchain");

        self.swapchain = new_swapchain;
        let new_images = new_images
            .into_iter()
            .map(|image| ImageView::new_default(image).unwrap())
            .collect::<Vec<_>>();
        self.image_views = new_images;
        #[cfg(target_os = "ios")]
        unsafe {
            self.surface.update_ios_sublayer_on_resize();
        }

        self.recreate_swapchain = false;

        // TODO: Recreate command buffers
        todo!("recreate command buffers");
    }
}

impl super::RenderSurface for Window {
    #[must_use]
    #[inline]
    fn size(&self) -> (u32, u32) {
        let size = self.window.inner_size();
        (size.width, size.height)
    }

    #[must_use]
    #[inline]
    fn views(&self) -> &[Arc<vulkano::image::view::ImageView>] {
        &self.image_views
    }

    #[must_use = "The function returns a future that must be awaited"]
    /// Acquires the next image to be rendered.
    ///
    /// ## Errors
    ///
    /// The function will return a non-fatal error if the swapchain couldn't be acquired.
    fn acquire(&mut self) -> Result<(u32, Box<dyn vulkano::sync::GpuFuture>), super::AcquireError> {
        if self.recreate_swapchain {
            self.recreate_swapchain_and_views();
            // on_recreate_swapchain(&self.final_views);
        }

        let (image_index, suboptimal, acquire_future) =
            match swapchain::acquire_next_image(self.swapchain.clone(), None)
                .map_err(vulkano::Validated::unwrap)
            {
                Ok(r) => r,
                Err(vulkano::VulkanError::OutOfDate) => {
                    self.recreate_swapchain = true;
                    return Err(super::AcquireError);
                }
                Err(e) => panic!("failed to acquire next image: {e}"),
            };

        self.recreate_swapchain |= suboptimal;
        self.image_index = image_index;

        Ok((image_index, acquire_future.boxed()))
    }

    /// Presents the rendered image to the swapchain.
    ///
    /// ## Errors
    ///
    /// The function will return a non-fatal error if the swapchain couldn't be presented.
    fn present(
        &mut self,
        render_future: Box<dyn vulkano::sync::GpuFuture>,
        queue: &Arc<Queue>,
    ) -> Result<(), super::PresentError> {
        let future = render_future
            .then_swapchain_present(
                queue.clone(),
                swapchain::SwapchainPresentInfo::swapchain_image_index(
                    self.swapchain.clone(),
                    self.image_index,
                ),
            )
            .then_signal_fence_and_flush();

        match future.map_err(vulkano::Validated::unwrap) {
            Ok(future) => {
                future.wait(None).unwrap_or_else(|e| {
                    tracing::error!("An error occured while rendering next frame: {e}");
                });
                Ok(())
            }
            Err(vulkano::VulkanError::OutOfDate) => {
                self.recreate_swapchain = true;
                Ok(())
            }
            Err(e) => {
                tracing::error!("Failed to flush rendering future: {e}");
                Err(super::PresentError)
            }
        }
    }
}

#[allow(clippy::module_name_repetitions)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Represents the mode of the window.
pub enum Mode {
    Windowed,
    BorderlessFullscreen,
    Fullscreen,
}

#[allow(clippy::module_name_repetitions)]
#[derive(Debug, Clone)]
/// Represents a window descriptor.
pub struct WindowDescriptor {
    pub width: u32,
    pub height: u32,
    pub position: Option<[f32; 2]>,
    pub resize_constraints: ResizeConstraints,
    pub title: String,
    pub resizable: bool,
    pub cursor_visible: bool,
    pub cursor_locked: bool,
    pub mode: Mode,
    pub present_mode: PresentMode,
}

impl Default for WindowDescriptor {
    fn default() -> Self {
        Self {
            title: "Title".to_string(),
            width: 1280,
            height: 720,
            position: None,
            resize_constraints: ResizeConstraints::default(),
            resizable: true,
            cursor_locked: false,
            cursor_visible: true,
            mode: Mode::Windowed,
            present_mode: PresentMode::Fifo,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Represents the present mode of the window.
pub enum PresentMode {
    /// The image is presented immediately.
    Immediate,
    /// Images get queued and the first one is presented.
    Mailbox,
    /// Two imaages are queued and the first one is presented.
    ///
    /// This present mode is the only one to be guaranteed to be supported.
    Fifo,
}

impl From<PresentMode> for vulkano::swapchain::PresentMode {
    fn from(mode: PresentMode) -> Self {
        match mode {
            PresentMode::Immediate => Self::Immediate,
            PresentMode::Mailbox => Self::Mailbox,
            PresentMode::Fifo => Self::Fifo,
        }
    }
}

#[allow(clippy::module_name_repetitions)]
#[derive(Debug, Clone, Copy)]
/// Represents the constraints for resizing a window.
pub struct ResizeConstraints {
    /// The minimum width of the window.
    pub min_width: u32,
    /// The minimum height of the window.
    pub min_height: u32,
    /// The maximum width of the window.
    pub max_width: u32,
    /// The maximum height of the window.
    pub max_height: u32,
}

impl Default for ResizeConstraints {
    fn default() -> Self {
        Self {
            min_width: 180,
            min_height: 120,
            max_width: u32::MAX,
            max_height: u32::MAX,
        }
    }
}

impl ResizeConstraints {
    #[must_use]
    /// Checks the constraints and returns a new `ResizeConstraints` with valid values.
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
