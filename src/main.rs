mod quad;
mod utils;

use pollster::FutureExt;
use quad::QuadRenderer;
use std::sync::Arc;
use wgpu;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::Window;

#[derive(Default)]
struct AppState<'window> {
    render_state: Option<RenderState<'window>>,
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

struct RenderState<'window> {
    surface: wgpu::Surface<'window>,
    config: wgpu::SurfaceConfiguration,
    device_arc: Arc<wgpu::Device>,
    queue_arc: Arc<wgpu::Queue>,
    window: Arc<Window>,

    start_time: std::time::Instant,
    last_render_time: Option<std::time::Instant>,

    quad_renderer: QuadRenderer,
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

        let (_device, _queue) = adapter
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
        let device_arc = Arc::new(_device);
        let queue_arc = Arc::new(_queue);

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
        surface.configure(&device_arc, &config);

        let mut quad_renderer =
            QuadRenderer::new(device_arc.clone(), queue_arc.clone(), config.format);

        let n = 317;
        println!("generating {} quads", n * n);
        for i in 0..n {
            for j in 0..n {
                quad_renderer.add_quad(
                    (i as f32 / n as f32) * 2.0 - 1.0,
                    (j as f32 / n as f32) * 2.0 - 1.0,
                    0.005,
                    0.005,
                    utils::Color {
                        r: i as f32 / n as f32,
                        g: j as f32 / n as f32,
                        b: ((i as f32 / n as f32) * 2.0 - 1.0)
                            * ((j as f32 / n as f32) * 2.0 - 1.0),
                        a: 1.0,
                    },
                );
            }
        }

        Self {
            surface,
            config,
            device_arc,
            queue_arc,
            window,
            start_time: std::time::Instant::now(),
            last_render_time: None,
            quad_renderer,
        }
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        // calculate delta time and update window title
        let now = std::time::Instant::now();
        let delta_time = now
            .duration_since(self.last_render_time.unwrap_or(now))
            .as_secs_f64();
        if self.last_render_time.is_some() && delta_time > 0.0 {
            self.window
                .set_title(&format!("FPS: {:.2}", 1.0 / delta_time));
        }
        self.last_render_time = Some(now);

        // update quads
        let t = self.start_time.elapsed().as_secs_f32() * 2.0;
        let n = (self.quad_renderer.size() as f32).sqrt() as usize;
        for i in 0..self.quad_renderer.size() {
            let row = i / n;
            let col = i % n;

            let (_, _, w, h, color) = self.quad_renderer.get_quad(i).unwrap();
            let center = n as f32 / 2.0;
            let dx = row as f32 - center;
            let dy = col as f32 - center;
            let distance = (dx * dx + dy * dy).sqrt() * 0.5;
            self.quad_renderer.set_quad(
                i,
                (row as f32 / n as f32) * 2.0 - 1.0 + ((t + distance).sin() * 0.01),
                (col as f32 / n as f32) * 2.0 - 1.0 + ((t + distance).cos() * 0.01),
                w,
                h,
                color,
            );
        }

        // grab the current texture from the surface
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        //create a render encoder
        let mut encoder = self
            .device_arc
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("command encoder"),
            });

        {
            // initialize render pass
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            // queue rendering for all elements
            self.quad_renderer.render(&mut render_pass);
        }

        // submit the render encoder
        self.queue_arc.submit(std::iter::once(encoder.finish()));
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
                self.surface.configure(&self.device_arc, &self.config);
            }
            None => {
                // Reconfigure the surface with the current size
                self.surface.configure(&self.device_arc, &self.config);
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
