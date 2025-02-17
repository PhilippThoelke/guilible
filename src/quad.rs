use std::sync::Arc;

use crate::transfer;
use crate::ui;
use wgpu::include_wgsl;

pub struct QuadRenderer {
    render_pipeline: wgpu::RenderPipeline,
    transfer_worker: transfer::TransferWorker,
}

impl QuadRenderer {
    pub fn new(
        device_arc: Arc<wgpu::Device>,
        queue_arc: Arc<wgpu::Queue>,
        texture_out_format: wgpu::TextureFormat,
    ) -> QuadRenderer {
        let shader = device_arc.create_shader_module(include_wgsl!("quad_shader.wgsl"));
        let bind_group_layout =
            device_arc.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("quad bind group layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let render_pipeline_layout =
            device_arc.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("render pipeline layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });
        let render_pipeline = device_arc.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("quad render pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: size_of::<ui::Quad>() as u64,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &[wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x2,
                        offset: 0,
                        shader_location: 0,
                    }],
                }],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: texture_out_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Cw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let ui_worker = ui::create_ui_worker(device_arc.clone());
        let transfer_worker =
            transfer::create_transfer_worker(transfer::TransferWorkerDescriptor {
                device_arc,
                queue_arc,
                bind_group_layout,
                ui_worker,
            });

        QuadRenderer {
            render_pipeline,
            transfer_worker,
        }
    }

    pub fn render(&mut self, render_pass: &mut wgpu::RenderPass) -> Vec<transfer::StorageBuffer> {
        let msg = self.transfer_worker.recv();

        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, &msg.storage_buffer.bind_group, &[]);
        render_pass.set_vertex_buffer(0, msg.storage_buffer.buffer.slice(..));
        render_pass.draw(0..4, 0..msg.num_instances);

        vec![msg.storage_buffer]
    }

    pub fn stop_and_join(self) {
        self.transfer_worker.stop_and_join();
    }
}
