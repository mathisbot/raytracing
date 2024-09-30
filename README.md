# Ray Tracing Engine

Ray tracing engine built in Rust using Vulkano.

## Features

- Ray tracing capabilities for realistic rendering
- Support for loading 3D models
- BVH implementation for efficient rendering
- Basic material handling
- Free camera
- Multi-platform

## Usage

For the time being, it is not possible to configure the engine other than by modifying the source code.

Several variables are of interest:
- `max_depth` in `shader/ray_trace.comp`: BVH max depth
- `nb_samples` in `shader/ray_trace.comp`: Samples per pixels
- `max_bounce_count` in `shader/ray_trace.comp`: Maximum number of bounces for a single ray
- Model path in `lib.rs`
- Materials in `shader/models.rs`

Keep in mind that for obvious reasons, I didn't include the `.obj` files of the models.
You will have to find suitable models and modify `lib.rs` to load include their paths.

To run the engine, use `cargo run --release`.

Please note that you will need the Vulkan Runtime to use the application.
The Runtime is very likely to be shipped with your graphics driver on Windows and Linux,
but you might need to install it on macOS.
It is also easier (but not mandatory) to have the Vulkan SDK installed when compiling the application,
especially for shader compilation.

## Performances

I managed to run a smooth 120 fps at minimum with 10 rays per pixel and 5 max bounce in the scene shown in the screenshots (1280x720), with an AMD Radeon RX 7900 XTX.

Because of the BVH-based collision detection, performances can drop depending on how big is the model on your screen.

## Screenshots

![Basic Scene](./.github/images/basic_scene.webp)

*Basic scene featuring two models with a reflective material.*

## To-Do

- Customizable buffers
- Textures
- Optimization

## Contributing

This is a personal project with an educational purpose.
As such, contributions are not welcome.
However, I'd be delighted to read about any issues you may encounter, and possible suggestions if you wish.

## License

This project is licensed under the [GNU GPLv3](https://opensource.org/license/gpl-3-0).
