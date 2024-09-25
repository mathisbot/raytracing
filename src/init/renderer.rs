use crate::init::context::VulkanoContext;
use crate::init::window::WindowDescriptor;
use std::sync::Arc;
use vulkano::swapchain::{SurfaceInfo, SwapchainPresentInfo};
use vulkano::{
    device::{Device, Queue},
    format::Format,
    image::{view::ImageView, ImageUsage},
    swapchain::{self, PresentMode, Surface, Swapchain, SwapchainCreateInfo},
    sync::{self, GpuFuture},
    Validated, VulkanError,
};
use winit::window::Window;

#[allow(clippy::module_name_repetitions)]
pub struct VulkanoWindowRenderer {
    compute_queue: Arc<Queue>,
    swapchain: Arc<Swapchain>,
    final_views: Vec<Arc<ImageView>>,
    recreate_swapchain: bool,
    previous_frame_end: Option<Box<dyn GpuFuture>>,
    image_index: u32,
    present_mode: PresentMode,
}

impl VulkanoWindowRenderer {
    #[must_use]
    pub fn new(
        vulkano_context: &VulkanoContext,
        window: &Arc<Window>,
        descriptor: &WindowDescriptor,
    ) -> Self {
        let (swapchain, final_views) =
            Self::create_swapchain(vulkano_context.device().clone(), window, descriptor);

        tracing::debug!("Swapchain created");

        let previous_frame_end = Some(sync::now(vulkano_context.device().clone()).boxed());

        Self {
            compute_queue: vulkano_context.compute_queue().clone(),
            swapchain,
            final_views,
            recreate_swapchain: false,
            previous_frame_end,
            image_index: 0,
            present_mode: descriptor.present_mode,
        }
    }

    fn create_swapchain(
        device: Arc<Device>,
        window: &Arc<Window>,
        window_descriptor: &WindowDescriptor,
    ) -> (Arc<Swapchain>, Vec<Arc<ImageView>>) {
        let surface = Surface::from_window(device.instance().clone(), window.clone()).unwrap();
        let surface_capabilities = device
            .physical_device()
            .surface_capabilities(&surface, swapchain::SurfaceInfo::default())
            .unwrap();

        assert!(
            device
                .physical_device()
                .surface_formats(&surface, swapchain::SurfaceInfo::default())
                .unwrap()
                .iter()
                .any(|(format, _)| *format == Format::R8G8B8A8_UNORM),
            "required surface format R8G8B8A8_UNORM is not supported"
        );

        let mut available_swapchain_present_modes = device
            .physical_device()
            .surface_present_modes(&surface, SurfaceInfo::default())
            .unwrap();

        let present_mode =
            if available_swapchain_present_modes.any(|p| p == window_descriptor.present_mode) {
                window_descriptor.present_mode
            } else {
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
                image_format: Format::R8G8B8A8_UNORM,
                image_extent: window.inner_size().into(),
                image_usage: ImageUsage::STORAGE | ImageUsage::COLOR_ATTACHMENT,
                present_mode,
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

    pub fn set_present_mode(&mut self, present_mode: PresentMode) {
        if self.present_mode != present_mode {
            self.present_mode = present_mode;
            self.recreate_swapchain = true;
        }
    }

    #[must_use]
    pub fn swapchain_format(&self) -> Format {
        self.final_views[self.image_index() as usize].format()
    }

    #[must_use]
    pub fn swapchain_image_count(&self) -> u32 {
        u32::try_from(self.final_views.len()).unwrap()
    }

    #[must_use]
    pub const fn image_index(&self) -> u32 {
        self.image_index
    }

    #[must_use]
    pub fn compute_queue(&self) -> Arc<Queue> {
        self.compute_queue.clone()
    }

    #[must_use]
    pub fn surface(&self) -> Arc<Surface> {
        self.swapchain.surface().clone()
    }

    #[must_use]
    pub fn swapchain_image_size(&self) -> [u32; 2] {
        self.final_views[0].image().extent()[0..2]
            .try_into()
            .unwrap()
    }

    #[must_use]
    pub fn swapchain_image_view(&self) -> Arc<ImageView> {
        self.final_views[self.image_index as usize].clone()
    }

    #[must_use]
    pub fn get_swapchain_image_view(&self, index: u32) -> Arc<ImageView> {
        self.final_views[index as usize].clone()
    }

    pub fn resize(&mut self) {
        self.recreate_swapchain = true;
    }

    /// Begin rendering pass.
    /// Finish rendering by calling [`VulkanoWindowRenderer::present`].
    pub fn acquire(
        &mut self,
        on_recreate_swapchain: impl FnOnce(&[Arc<ImageView>]),
    ) -> Result<Box<dyn GpuFuture>, VulkanError> {
        if self.recreate_swapchain {
            self.recreate_swapchain_and_views();
            on_recreate_swapchain(&self.final_views);
        }

        let (image_index, suboptimal, acquire_future) =
            match swapchain::acquire_next_image(self.swapchain.clone(), None)
                .map_err(Validated::unwrap)
            {
                Ok(r) => r,
                Err(VulkanError::OutOfDate) => {
                    self.recreate_swapchain = true;
                    return Err(VulkanError::OutOfDate);
                }
                Err(e) => panic!("failed to acquire next image: {e}"),
            };

        self.recreate_swapchain |= suboptimal;
        self.image_index = image_index;

        Ok(self
            .previous_frame_end
            .take()
            .unwrap()
            .join(acquire_future)
            .boxed())
    }

    /// Finishes rendering by presenting the swapchain.
    pub fn present(&mut self, after_future: Box<dyn GpuFuture>) {
        let future = after_future
            .then_swapchain_present(
                self.compute_queue(),
                SwapchainPresentInfo::swapchain_image_index(
                    self.swapchain.clone(),
                    self.image_index,
                ),
            )
            .then_signal_fence_and_flush();

        match future.map_err(Validated::unwrap) {
            Ok(future) => {
                future.wait(None).unwrap_or_else(|e| {
                    tracing::error!("An error occured while rendering next frame: {e}");
                });

                self.previous_frame_end = Some(future.boxed());
            }
            Err(VulkanError::OutOfDate) => {
                self.recreate_swapchain = true;
                self.previous_frame_end =
                    Some(sync::now(self.compute_queue().device().clone()).boxed());
            }
            Err(e) => {
                tracing::error!("Failed to flush rendering future: {e}");
                self.previous_frame_end =
                    Some(sync::now(self.compute_queue().device().clone()).boxed());
            }
        }
    }

    fn recreate_swapchain_and_views(&mut self) {
        let image_extent: [u32; 2] = self.swapchain_image_size();

        if image_extent.contains(&0) {
            return;
        }

        let (new_swapchain, new_images) = self
            .swapchain
            .recreate(SwapchainCreateInfo {
                image_extent,
                present_mode: self.present_mode,
                ..self.swapchain.create_info()
            })
            .expect("failed to recreate swapchain");

        self.swapchain = new_swapchain;
        let new_images = new_images
            .into_iter()
            .map(|image| ImageView::new_default(image).unwrap())
            .collect::<Vec<_>>();
        self.final_views = new_images;
        #[cfg(target_os = "ios")]
        unsafe {
            self.surface.update_ios_sublayer_on_resize();
        }

        // FIXME: Recreate command buffers, ...
        todo!("recreating swapchain");

        #[allow(unreachable_code)]
        {
            self.recreate_swapchain = false;
        }
    }
}
