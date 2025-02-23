use crate::construct;
use crate::render::Renderer;
use crate::utils;
use pollster::FutureExt;
use std::sync::Arc;
use wgpu;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::window::Window;

#[derive(Default)]
pub struct Application<'win> {
    state: Option<State<'win>>,
    window: Option<Arc<Window>>,
}

impl<'win> ApplicationHandler for Application<'win> {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        println!("starting guilible");
        println!("├─ creating window");

        let win_arc = Arc::new(
            event_loop
                .create_window(Window::default_attributes())
                .unwrap(),
        );
        self.window = Some(win_arc.clone());
        self.state = Some(State::new(win_arc.clone()));
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                println!("\nclosing guilible");

                // stop the renderer and drop the state
                if let Some(state) = self.state.take() {
                    state.renderer.stop_and_join();
                    println!("╰─ render    : {}", state.stats);
                }

                // drop the window and exit the event loop
                self.window.take();
                event_loop.exit();
            }

            WindowEvent::Resized(size) => {
                if let Some(render_state) = self.state.as_mut() {
                    render_state.resize(Some(size));
                }
            }

            WindowEvent::RedrawRequested => {
                // Request next frame
                self.window.as_ref().unwrap().request_redraw();

                // Render the frame
                if let Some(render_state) = self.state.as_mut() {
                    match render_state.render() {
                        Ok(_) => {}
                        Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                            println!("surface lost or outdated, recomfiguring");
                            render_state.resize(None);
                        }
                        Err(wgpu::SurfaceError::OutOfMemory | wgpu::SurfaceError::Other) => {
                            println!("surface error, exiting");
                            event_loop.exit();
                        }
                        Err(wgpu::SurfaceError::Timeout) => {
                            println!("surface timeout");
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

struct State<'win> {
    surface: wgpu::Surface<'win>,
    config: wgpu::SurfaceConfiguration,
    device_arc: Arc<wgpu::Device>,
    queue_arc: Arc<wgpu::Queue>,
    window: Arc<Window>,

    last_render_time: Option<std::time::Instant>,
    stats: utils::Stats,

    renderer: Renderer,
}

impl<'win> State<'win> {
    fn new(window: Arc<Window>) -> Self {
        let window_size = window.inner_size();

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let surface = instance
            .create_surface(window.clone())
            .expect("failed to create surface");

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .block_on()
            .expect("failed to find an adapter");

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
            .expect("failed to create device");

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

        let renderer = Renderer::new(device_arc.clone(), queue_arc.clone(), config.format);

        println!("╰─ ready");

        Self {
            surface,
            config,
            device_arc,
            queue_arc,
            window,
            last_render_time: None,
            stats: utils::Stats::default(),
            renderer,
        }
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let _delta_time = self.update_timing();
        let render_start_time = std::time::Instant::now();

        // grab the current texture from the surface
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // create a render encoder
        let mut encoder = self
            .device_arc
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("render command encoder"),
            });

        // keep track of storage buffers to be recycled after rendering
        let mut storage_buffers = Vec::<construct::StorageBuffer>::new();

        {
            // initialize render pass
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            // queue rendering for all elements
            storage_buffers.extend(self.renderer.render(&mut render_pass));
        }

        // submit the render encoder
        self.queue_arc.submit(std::iter::once(encoder.finish()));

        // recycle storage buffers
        self.queue_arc.on_submitted_work_done(move || {
            for storage_buffer in storage_buffers.iter() {
                storage_buffer
                    .ready
                    .store(true, std::sync::atomic::Ordering::SeqCst);
            }
        });

        // present the frame
        output.present();

        // update stats
        self.stats.update(render_start_time.elapsed().as_secs_f64());

        Ok(())
    }

    fn update_timing(&mut self) -> f64 {
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
        delta_time
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
