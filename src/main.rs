use std::sync::Arc;

use pollster::FutureExt;
use wgpu::{self, include_wgsl, Color};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::Window;

#[derive(Default)]
struct AppState<'a> {
    render_state: Option<RenderState<'a>>,
    window: Option<Arc<Window>>,
}

impl<'a> ApplicationHandler for AppState<'a> {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        println!("application resumed");
        let win_arc = Arc::new(
            event_loop
                .create_window(Window::default_attributes())
                .unwrap(),
        );
        self.window = Some(win_arc.clone());
        self.render_state = Some(RenderState::new(win_arc.clone()));
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                self.render_state.as_mut().unwrap().resize(Some(size));
            }
            WindowEvent::RedrawRequested => {
                // Request next frame
                self.window.as_ref().unwrap().request_redraw();

                // Render the frame
                match self.render_state.as_mut().unwrap().render() {
                    Ok(_) => {}
                    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                        self.render_state.as_mut().unwrap().resize(None);
                    }
                    Err(wgpu::SurfaceError::OutOfMemory | wgpu::SurfaceError::Other) => {
                        println!("Surface error, exiting");
                        event_loop.exit();
                    }
                    Err(wgpu::SurfaceError::Timeout) => {
                        println!("Surface timeout");
                    }
                }
            }
            _ => {}
        }
    }
}

struct RenderState<'a> {
    surface: wgpu::Surface<'a>,
    config: wgpu::SurfaceConfiguration,
    device: wgpu::Device,
    queue: wgpu::Queue,
    render_pipeline: wgpu::RenderPipeline,
    window: Arc<Window>,
    last_render_time: Option<std::time::Instant>,
}

impl<'a> RenderState<'a> {
    fn new(window: Arc<Window>) -> Self {
        println!("creating render state");

        let window_size = window.inner_size();

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let surface = instance.create_surface(window.clone()).unwrap();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .block_on()
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    label: None,
                    memory_hints: wgpu::MemoryHints::default(),
                },
                None,
            )
            .block_on()
            .unwrap();

        let surface_formats = surface.get_capabilities(&adapter).formats;
        let surface_format = surface_formats
            .iter()
            .find(|format| format.is_srgb())
            .copied()
            .unwrap_or(surface_formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: window_size.width,
            height: window_size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let shader = device.create_shader_module(include_wgsl!("shader.wgsl"));
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[],
                push_constant_ranges: &[],
            });
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        Self {
            surface,
            config,
            device,
            queue,
            render_pipeline,
            window,
            last_render_time: None,
        }
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        if let Some(last_render_time) = self.last_render_time {
            let now = std::time::Instant::now();
            let elapsed = now - last_render_time;
            self.window
                .set_title(&format!("FPS: {:.2}", 1.0 / elapsed.as_secs_f64()));
        }
        self.last_render_time = Some(std::time::Instant::now());

        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.draw(0..3, 0..1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));

        output.present();
        Ok(())
    }

    fn resize(&mut self, size: Option<winit::dpi::PhysicalSize<u32>>) {
        match size {
            Some(size) => {
                if size.width == 0 || size.height == 0 {
                    return;
                }

                self.config.width = size.width;
                self.config.height = size.height;
                self.surface.configure(&self.device, &self.config);
            }
            None => {
                // Reconfigure the surface with the current size
                self.surface.configure(&self.device, &self.config);
            }
        }
    }
}

fn main() {
    let event_loop = EventLoop::new().unwrap();
    // TODO: we might need ControlFlow::Poll to handle updating the UI based on incoming data?
    event_loop.set_control_flow(ControlFlow::Wait);

    let mut app = AppState::default();
    let _ = event_loop.run_app(&mut app);
}
