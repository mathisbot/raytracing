//! RayTracing Engine

// Hide console window on Windows in release builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use rt_engine::RayTracingApp;

fn main() {
    tracing_subscriber::fmt()
        .with_max_level(if cfg!(debug_assertions) {
            tracing::Level::TRACE
        } else {
            tracing::Level::INFO
        })
        .init();

    let first_person_camera = rt_engine::control::camera::first_person::FirstPerson::default();

    let keyboard = rt_engine::control::controller::keyboard::Keyboard::default();
    let mouse = rt_engine::control::controller::mouse::Mouse::default();

    let config = rt_engine::RayTracingAppConfig {
        render_surface_type: rt_engine::RenderSurfaceType::Window(
            rt_engine::render::window::WindowDescriptor {
                width: 1024,
                height: 720,
                title: "RayTracer".to_string(),
                cursor_visible: false,
                cursor_locked: true,
                position: None,
                resizable: false,
                mode: rt_engine::render::window::Mode::Windowed,
                present_mode: rt_engine::render::window::PresentMode::Fifo,
                resize_constraints: rt_engine::render::window::ResizeConstraints::default(),
            },
        ),
        camera: Box::new(first_person_camera),
        controllers: vec![Box::new(keyboard), Box::new(mouse)],
    };

    // let config = rt_engine::RayTracingAppConfig {
    //     render_surface_type: rt_engine::RenderSurfaceType::Image(
    //         rt_engine::render::image::ImageDescriptor {
    //             path: "output.png".into(),
    //             width: 3840,
    //             height: 2160,
    //         },
    //     ),
    //     camera: Box::new(first_person_camera),
    //     controllers: vec![],
    // };

    let app = RayTracingApp::new(config);

    app.run();
}
