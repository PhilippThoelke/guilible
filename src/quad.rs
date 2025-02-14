use std::mem;
use std::sync::Arc;

use crate::utils;
use bytemuck::NoUninit;
use wgpu::include_wgsl;

#[repr(C)]
#[derive(Clone, Copy, NoUninit)]
struct Quad {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    color: utils::Color,
}

pub struct QuadRenderer {
    device_arc: Arc<wgpu::Device>,
    queue_arc: Arc<wgpu::Queue>,

    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: Option<wgpu::BindGroup>,
    quad_buffer: Option<wgpu::Buffer>,

    render_pipeline: wgpu::RenderPipeline,

    quads: Vec<Quad>,
    dirty: Vec<bool>,
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
                    array_stride: size_of::<Quad>() as u64,
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

        QuadRenderer {
            device_arc,
            queue_arc,
            bind_group_layout,
            bind_group: None,
            quad_buffer: None,
            render_pipeline,
            quads: Vec::new(),
            dirty: Vec::new(),
        }
    }

    pub fn add_quad(&mut self, x: f32, y: f32, w: f32, h: f32, color: utils::Color) {
        self.quads.push(Quad {
            x: x,
            y: y,
            w: w,
            h: h,
            color: color,
        });
        self.dirty.push(true);

        let size = mem::size_of::<Quad>();

        // create or grow the buffer if it's too small
        if self.quad_buffer.is_none()
            || self.quad_buffer.as_ref().unwrap().size() < (self.quads.len() * size) as u64
        {
            self.grow_buffer();
        }
    }

    pub fn size(&self) -> usize {
        self.quads.len()
    }

    pub fn get_quad(&self, index: usize) -> Option<(f32, f32, f32, f32, utils::Color)> {
        if index < self.quads.len() {
            let quad = self.quads[index];
            Some((quad.x, quad.y, quad.w, quad.h, quad.color))
        } else {
            None
        }
    }

    pub fn set_quad(&mut self, index: usize, x: f32, y: f32, w: f32, h: f32, color: utils::Color) {
        if index < self.quads.len() {
            self.quads[index] = Quad {
                x: x,
                y: y,
                w: w,
                h: h,
                color: color,
            };
            self.dirty[index] = true;
        }
    }

    pub fn render(&mut self, render_pass: &mut wgpu::RenderPass) {
        if self.quads.is_empty() {
            return;
        }
        let size = mem::size_of::<Quad>();
        let buffer = self.quad_buffer.as_ref().unwrap();
        let len = self.quads.len();
        let mut i = 0;

        while i < len {
            if self.dirty[i] {
                let start = i;
                // find contiguous block of dirty quads and clear flags
                while i < len && self.dirty[i] {
                    self.dirty[i] = false;
                    i += 1;
                }
                let data = bytemuck::cast_slice(&self.quads[start..i]);
                self.queue_arc
                    .write_buffer(buffer, (start * size) as u64, data);
            } else {
                self.dirty[i] = false;
                i += 1;
            }
        }

        // submit the draw call
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.quad_buffer.as_ref().unwrap().slice(..));
        render_pass.draw(0..4, 0..self.quads.len() as u32);
    }

    fn grow_buffer(&mut self) {
        let new_size = self
            .quad_buffer
            .as_ref()
            .map_or(mem::size_of::<Quad>() as u64 * 32, |buffer| {
                buffer.size() * 2
            });

        let (new_buffer, new_bind_group) = self.create_buffer(new_size);
        self.quad_buffer = Some(new_buffer);
        self.bind_group = Some(new_bind_group);

        // mark all quads as dirty
        self.dirty = vec![true; self.quads.len()];
    }

    fn create_buffer(&self, size: u64) -> (wgpu::Buffer, wgpu::BindGroup) {
        let buffer = self.device_arc.create_buffer(&wgpu::BufferDescriptor {
            label: Some("quad buffer"),
            size: size,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::VERTEX,
            mapped_at_creation: false,
        });

        let bind_group = self
            .device_arc
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("quad bind group"),
                layout: &self.bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &buffer,
                        offset: 0,
                        size: None,
                    }),
                }],
            });

        (buffer, bind_group)
    }
}
