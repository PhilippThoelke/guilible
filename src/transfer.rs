use std::{
    sync::{atomic, mpsc, Arc},
    thread,
};

use crate::ui;
use crate::utils;
use wgpu;

#[derive(Clone)]
struct StagingBuffer {
    buffer: wgpu::Buffer,
    ready: Arc<atomic::AtomicBool>,
}

#[derive(Clone)]
pub struct StorageBuffer {
    pub buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
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

fn create_storage_buffer(
    device_arc: &Arc<wgpu::Device>,
    bind_group_layout: &wgpu::BindGroupLayout,
    buffer_size: u64,
) -> StorageBuffer {
    let storage = device_arc.create_buffer(&wgpu::BufferDescriptor {
        label: Some("pool storage buffer"),
        size: buffer_size,
        usage: wgpu::BufferUsages::COPY_DST
            | wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::VERTEX,
        mapped_at_creation: false,
    });
    let bind_group = device_arc.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("quad bind group"),
        layout: &bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                buffer: &storage,
                offset: 0,
                size: None,
            }),
        }],
    });

    StorageBuffer {
        buffer: storage,
        bind_group,
        ready: Arc::new(atomic::AtomicBool::new(true)),
    }
}

fn create_buffer_pool(descriptor: BufferPoolDescriptor) -> BufferPool {
    BufferPool {
        device_arc: descriptor.device_arc,
        bind_group_layout: descriptor.bind_group_layout,
        staging_buffers: Vec::new(),
        storage_buffers: Vec::new(),
        buffer_size: descriptor.initial_buffer_size,
    }
}

struct BufferPoolDescriptor {
    device_arc: Arc<wgpu::Device>,
    bind_group_layout: wgpu::BindGroupLayout,
    initial_buffer_size: u64,
}

struct BufferPool {
    device_arc: Arc<wgpu::Device>,
    bind_group_layout: wgpu::BindGroupLayout,
    staging_buffers: Vec<StagingBuffer>,
    storage_buffers: Vec<StorageBuffer>,
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

    pub fn request_storage(&mut self, min_size: Option<u64>) -> StorageBuffer {
        self.check_size(min_size);

        // check if there are any available buffers
        let result = match self
            .storage_buffers
            .iter()
            .filter(|b| b.ready.load(atomic::Ordering::SeqCst))
            .next()
        {
            Some(result) => result.clone(),
            None => {
                // no storage buffers available, create a new one
                let new_buffer = create_storage_buffer(
                    &self.device_arc,
                    &self.bind_group_layout,
                    self.buffer_size,
                );
                // TODO: investigate if it really is faster to keep a pool of storage buffers
                // self.storage_buffers.push(new_buffer.clone());
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
                self.storage_buffers.clear();
            }
        }
    }
}

pub struct TransferWorkerMessage {
    pub storage_buffer: StorageBuffer,
    pub num_instances: u32,
}

fn staging_to_storage(
    staging: StagingBuffer,
    storage: &StorageBuffer,
    device_arc: &Arc<wgpu::Device>,
    queue_arc: &Arc<wgpu::Queue>,
    num_bytes: u64,
) {
    // copy staging buffer to storage buffer
    let mut encoder = device_arc.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("staging-to-storage encoder"),
    });
    encoder.copy_buffer_to_buffer(&staging.buffer, 0, &storage.buffer, 0, num_bytes);

    // submit the copy command
    queue_arc.submit(std::iter::once(encoder.finish()));

    // re-map the staging buffer after the copy is done and mark it as ready once it has been mapped
    let device_arc = device_arc.clone();
    queue_arc.on_submitted_work_done(move || {
        staging
            .buffer
            .slice(..)
            .map_async(wgpu::MapMode::Write, move |result| match result {
                Ok(_) => {
                    // mark the buffer as ready
                    staging.ready.store(true, atomic::Ordering::SeqCst);
                }
                Err(e) => {
                    eprintln!("failed to re-map staging buffer: {:?}", e);
                }
            });
        // poll the device to avoid BufferAsyncError
        device_arc.poll(wgpu::Maintain::Poll);
    });
}

pub fn create_transfer_worker(descriptor: TransferWorkerDescriptor) -> TransferWorker {
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    let alive = Arc::new(atomic::AtomicBool::new(true));

    TransferWorker {
        receiver,
        alive: alive.clone(),
        worker_handle: thread::Builder::new()
            .name("transfer worker".to_string())
            .spawn(move || {
                // create quad manager and setup example quads
                // Note: this will happen outside of the library
                let mut quad_manager = ui::QuadManager { quads: Vec::new() };
                ui::setup(&mut quad_manager);

                // create buffer pool
                let mut buffer_pool = create_buffer_pool(BufferPoolDescriptor {
                    device_arc: descriptor.device_arc.clone(),
                    bind_group_layout: descriptor.bind_group_layout,
                    initial_buffer_size: 1024,
                });

                let worker_start = std::time::Instant::now();
                let mut stats = utils::Stats::default();
                while alive.load(atomic::Ordering::SeqCst) {
                    // start measuring time
                    let loop_start = std::time::Instant::now();

                    // update quads
                    // Note: this will happen outside of the library
                    ui::update(&mut quad_manager, worker_start);
                    let num_bytes = (quad_manager.quads.len() * 8 * 4) as u64;

                    // request staging and storage buffers
                    let staging_buffer = buffer_pool.request_staging(Some(num_bytes));
                    let storage_buffer = buffer_pool.request_storage(Some(num_bytes));

                    // pack quad data into a flat array
                    let data = bytemuck::cast_slice(&quad_manager.quads);

                    // prepare staging buffer for writing
                    let mut view = staging_buffer
                        .buffer
                        .slice(0..num_bytes)
                        .get_mapped_range_mut();
                    let floats: &mut [f32] = bytemuck::cast_slice_mut(&mut view);

                    // copy data into staging buffer
                    floats.copy_from_slice(&data);
                    drop(view);
                    staging_buffer.buffer.unmap();

                    // copy staging buffer to a storage buffer
                    staging_to_storage(
                        staging_buffer,
                        &storage_buffer,
                        &descriptor.device_arc,
                        &descriptor.queue_arc,
                        num_bytes,
                    );

                    // send the storage buffer to the render thread
                    let message = TransferWorkerMessage {
                        storage_buffer,
                        num_instances: quad_manager.quads.len() as u32,
                    };

                    // update statistics (data receive until message sent)
                    stats.update(loop_start.elapsed().as_secs_f64());

                    // send message to the render thread (blocks until the previous message has been consumed)
                    match sender.try_send(message) {
                        Ok(_) => {}
                        Err(mpsc::TrySendError::Full(_)) => {}
                        Err(mpsc::TrySendError::Disconnected(_)) => {
                            // send failed, exit the loop and clean up
                            break;
                        }
                    }
                }

                // print statistics
                println!("├─ transfer  (cpu→gpu) : {}", stats);
            })
            .expect("failed to spawn transfer worker"),
    }
}

pub struct TransferWorkerDescriptor {
    pub device_arc: Arc<wgpu::Device>,
    pub queue_arc: Arc<wgpu::Queue>,
    pub bind_group_layout: wgpu::BindGroupLayout,
}

pub struct TransferWorker {
    receiver: mpsc::Receiver<TransferWorkerMessage>,
    alive: Arc<atomic::AtomicBool>,
    worker_handle: std::thread::JoinHandle<()>,
}

impl TransferWorker {
    pub fn recv(&self) -> TransferWorkerMessage {
        self.receiver
            .recv()
            .expect("failed to receive message from transfer worker")
    }

    pub fn stop_and_join(self) {
        self.alive.store(false, atomic::Ordering::SeqCst);
        self.worker_handle
            .join()
            .expect("failed to join transfer worker thread");
    }
}
