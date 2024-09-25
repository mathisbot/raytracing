//! RayTracing Engine

// Hide console window on Windows in release builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use raytracing_engine::RayTracerApp;
use raytracing_engine::VulkanoConfig;
use winit::event_loop::EventLoop;

fn main() {
    tracing_subscriber::fmt()
        .with_max_level(if cfg!(debug_assertions) {
            tracing::Level::TRACE
        } else {
            tracing::Level::INFO
        })
        .init();

    let event_loop = EventLoop::new();
    let app = RayTracerApp::new(VulkanoConfig::default(), &event_loop);

    app.run(event_loop);
}
