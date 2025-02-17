use crate::utils;
use bytemuck::NoUninit;
use rayon::prelude::*;
use std::{
    sync::{atomic, mpsc, Arc},
    thread,
};

#[repr(C)]
#[derive(Clone, Copy, NoUninit)]
pub struct Quad {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    color: utils::Color,
}

impl From<&Quad> for [f32; 8] {
    fn from(quad: &Quad) -> [f32; 8] {
        [
            quad.x,
            quad.y,
            quad.w,
            quad.h,
            quad.color.r,
            quad.color.g,
            quad.color.b,
            quad.color.a,
        ]
    }
}

pub struct QuadManager {
    quads: Vec<Quad>,
}

impl QuadManager {
    pub fn add_quad(&mut self, x: f32, y: f32, w: f32, h: f32, color: utils::Color) {
        self.quads.push(Quad { x, y, w, h, color });
    }
}

fn setup(quad_manager: &mut QuadManager) {
    let n = 2000;
    let quad_size = 0.001;
    println!("generating {} quads", n * n);
    for i in 0..n {
        for j in 0..n {
            quad_manager.add_quad(
                (i as f32 / n as f32) - 0.5,
                (j as f32 / n as f32) - 0.5,
                quad_size,
                quad_size,
                utils::Color {
                    r: i as f32 / n as f32,
                    g: j as f32 / n as f32,
                    b: ((i as f32 / n as f32) * 2.0 - 1.0) * ((j as f32 / n as f32) * 2.0 - 1.0),
                    a: 1.0,
                },
            );
        }
    }
}

fn update(quad_manager: &mut QuadManager, start_time: std::time::Instant) {
    let delta = start_time.elapsed().as_secs_f32();
    let n = (quad_manager.quads.len() as f32).sqrt();

    quad_manager
        .quads
        .par_iter_mut()
        .enumerate()
        .for_each(|(i, quad)| {
            let i = i as f32;
            let x = i % n;
            let y = i / n;
            quad.x = (x / n) - 0.5 + (delta * 1.0 + x / n * 6.0).sin() * 0.4;
            quad.y = (y / n) - 0.5 + (delta * 1.0 + y / n * 6.0).cos() * 0.4;
        });
}

#[derive(Clone)]
pub struct StagingBuffer {
    pub buffer: wgpu::Buffer,
    pub ready: Arc<atomic::AtomicBool>,
}

fn create_staging_buffer(device_arc: &Arc<wgpu::Device>, buffer_size: u64) -> StagingBuffer {
    StagingBuffer {
        buffer: device_arc.create_buffer(&wgpu::BufferDescriptor {
            label: Some("pool staging buffer"),
            size: buffer_size,
            usage: wgpu::BufferUsages::MAP_WRITE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: true,
        }),
        ready: Arc::new(atomic::AtomicBool::new(true)),
    }
}

fn create_buffer_pool(descriptor: BufferPoolDescriptor) -> BufferPool {
    BufferPool {
        device_arc: descriptor.device_arc,
        staging_buffers: Vec::new(),
        buffer_size: descriptor.initial_buffer_size,
    }
}

struct BufferPoolDescriptor {
    device_arc: Arc<wgpu::Device>,
    initial_buffer_size: u64,
}

struct BufferPool {
    device_arc: Arc<wgpu::Device>,
    staging_buffers: Vec<StagingBuffer>,
    buffer_size: u64,
}

impl BufferPool {
    pub fn request_staging(&mut self, min_size: Option<u64>) -> StagingBuffer {
        self.check_size(min_size);

        // check if there are any available buffers
        let result = match self
            .staging_buffers
            .iter()
            .filter(|b| b.ready.load(atomic::Ordering::SeqCst))
            .next()
        {
            Some(result) => result.clone(),
            None => {
                // no mapped staging buffer available, create a new one
                let new_buffer = create_staging_buffer(&self.device_arc, self.buffer_size);
                self.staging_buffers.push(new_buffer.clone());
                new_buffer
            }
        };

        result.ready.store(false, atomic::Ordering::SeqCst);
        result
    }

    fn check_size(&mut self, min_size: Option<u64>) {
        if let Some(min_size) = min_size {
            while self.buffer_size < min_size {
                // grow the buffer size and discard all available buffers
                self.buffer_size *= 2;
                println!("increasing buffer size to {}", self.buffer_size);
                self.staging_buffers.clear();
            }
        }
    }
}

pub struct UIWorkerMessage {
    pub staging_buffer: StagingBuffer,
    pub num_instances: u32,
}

pub fn create_ui_worker(device_arc: Arc<wgpu::Device>) -> UIWorker {
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    let alive = Arc::new(atomic::AtomicBool::new(true));

    UIWorker {
        receiver,
        alive: alive.clone(),
        worker_handle: thread::Builder::new()
            .name("ui worker".to_string())
            .spawn(move || {
                let worker_start = std::time::Instant::now();

                // create quad manager and setup example quads
                // TODO: this will be outsourced once guilible becomes a library
                let mut quad_manager = QuadManager { quads: Vec::new() };
                setup(&mut quad_manager);

                // create staging buffer pool
                let mut buffer_pool = create_buffer_pool(BufferPoolDescriptor {
                    device_arc,
                    initial_buffer_size: 1024,
                });

                let mut stats = utils::Stats::default();
                while alive.load(atomic::Ordering::SeqCst) {
                    let loop_start = std::time::Instant::now();

                    let num_bytes = (quad_manager.quads.len() * 8 * 4) as u64;
                    let staging_buffer = buffer_pool.request_staging(Some(num_bytes));

                    // update quads
                    update(&mut quad_manager, worker_start);

                    // pack quad data into a flat array
                    let data = bytemuck::cast_slice(&quad_manager.quads);

                    let mut view = staging_buffer
                        .buffer
                        .slice(0..num_bytes)
                        .get_mapped_range_mut();
                    let floats: &mut [f32] = bytemuck::cast_slice_mut(&mut view);
                    // copy UI data into staging buffer
                    floats.copy_from_slice(&data);
                    drop(view);
                    staging_buffer.buffer.unmap();

                    // construct message
                    let message = UIWorkerMessage {
                        staging_buffer,
                        num_instances: quad_manager.quads.len() as u32,
                    };

                    // update statistics (loop start to message sent)
                    stats.update(loop_start.elapsed().as_secs_f64());

                    // send message to the transfer thread (blocks until the previous message has been consumed)
                    match sender.try_send(message) {
                        Ok(_) => {}
                        Err(mpsc::TrySendError::Full(_)) => {}
                        Err(mpsc::TrySendError::Disconnected(_)) => {
                            break;
                        }
                    }
                }

                println!("├─ ui          (cpu)   : {}", stats);
            })
            .expect("failed to spawn ui worker"),
    }
}

pub struct UIWorker {
    receiver: mpsc::Receiver<UIWorkerMessage>,
    alive: Arc<atomic::AtomicBool>,
    worker_handle: std::thread::JoinHandle<()>,
}

impl UIWorker {
    pub fn recv(&self) -> UIWorkerMessage {
        self.receiver
            .recv()
            .expect("failed to receive message from ui worker")
    }

    pub fn stop_and_join(self) {
        self.alive.store(false, atomic::Ordering::SeqCst);
        self.worker_handle
            .join()
            .expect("failed to join ui worker thread");
    }
}
