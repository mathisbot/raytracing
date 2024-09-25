use vulkano::pipeline::compute::ComputePipelineCreateInfo;
use vulkano::pipeline::layout::PipelineDescriptorSetLayoutCreateInfo;
use vulkano::pipeline::{ComputePipeline, PipelineLayout, PipelineShaderStageCreateInfo};

use vulkano::shader::ShaderModule;

use std::sync::Arc;

pub fn new(
    device: &Arc<vulkano::device::Device>,
    shader_module: &Arc<ShaderModule>,
    entry_point: &str,
) -> Arc<ComputePipeline> {
    let stage = PipelineShaderStageCreateInfo::new(
        shader_module
            .entry_point(entry_point)
            .expect("unable to find shader entrypoint"),
    );
    let layout = PipelineLayout::new(
        device.clone(),
        PipelineDescriptorSetLayoutCreateInfo::from_stages(&[stage.clone()])
            .into_pipeline_layout_create_info(device.clone())
            .unwrap(),
    )
    .unwrap();

    ComputePipeline::new(
        device.clone(),
        None,
        ComputePipelineCreateInfo::stage_layout(stage, layout),
    )
    .expect("failed to create compute pipeline")
}
