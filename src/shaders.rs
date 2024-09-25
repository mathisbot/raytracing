vulkano_shaders::shader! {
    shaders: {
        compute: {
            ty: "compute",
            path: r"src/shaders/ray_trace.comp",
        },
    }
}
