# Ray Tracing Engine

Ray tracing engine built in Rust using Vulkano.

## Features

- Ray tracing capabilities for realistic rendering
- Support for loading 3D models
- BVH implementation for efficient rendering
- Basic material handling
- Free camera
- Wide controller support
- Rendering on a window or an image
- Multi-platform

## Usage

To run the engine, use `cargo run --release`.

Please note that you will need the Vulkan Runtime to use the application.
The Runtime is very likely to be shipped with your graphics driver on Windows and Linux,
but you might need to install it on macOS.
It is also easier (but not mandatory) to have the Vulkan SDK installed when compiling the application,
especially for shader compilation.

For obvious reasons, I didn't include the `.obj` files of the models.
You will have to find suitable models and modify `lib.rs` to load include their paths.

Temporarily, you can only edit the materials in `rt-engine/src/shader/models.rs`.

## Performances

I managed to run a smooth 120 fps on average with 10 rays per pixel and 6 bounces in the scene shown in the screenshots (1024x720), with an AMD Radeon RX 7900 XTX.

Note that because of the BVH-based collision detection, performances can drop depending on how big is the model on your screen.
I still managed to run a minimum of 60 fps, even when staying very to close to the most densely populated regions of the models.

## Screenshots

![Basic Scene](./.github/images/basic_scene.webp)

*Basic scene featuring two models with a reflective material.*

## To-Do

- Textures / Materials
- Optimization

## Contributing

This is a personal project with an educational purpose.
As such, contributions are not welcome.
However, I'd be delighted to read about any issues you may encounter, and possible suggestions if you wish.

## License

This project is licensed under the [GNU GPLv3](https://opensource.org/license/gpl-3-0).
