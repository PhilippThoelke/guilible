use std::{
    sync::{atomic, mpsc, Arc},
    thread,
};

use crate::ui;
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
    buffer_size: u64,
    bind_group_layout: &wgpu::BindGroupLayout,
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
    pub fn request_staging(&mut self, min_size: u64) -> StagingBuffer {
        while self.buffer_size < min_size {
            // grow the buffer size and discard all available buffers
            self.buffer_size *= 2;
            println!("increasing buffer size to {}", self.buffer_size);
            self.staging_buffers.clear();
            self.storage_buffers.clear();
        }

        // check if there are any available buffers
        let result = match self
            .staging_buffers
            .iter()
            .filter(|b| b.ready.load(atomic::Ordering::SeqCst))
            .next()
        {
            Some(result) => result.clone(),
            None => {
                // no buffer chains available, create a new one
                create_staging_buffer(&self.device_arc, self.buffer_size)
            }
        };

        result.ready.store(false, atomic::Ordering::SeqCst);
        result
    }

    pub fn request_storage(&mut self, min_size: u64) -> StorageBuffer {
        while self.buffer_size < min_size {
            // grow the buffer size and discard all available buffers
            self.buffer_size *= 2;
            println!("increasing buffer size to {}", self.buffer_size);
            self.staging_buffers.clear();
            self.storage_buffers.clear();
        }

        // check if there are any available buffers
        let result = match self
            .storage_buffers
            .iter()
            .filter(|b| b.ready.load(atomic::Ordering::SeqCst))
            .next()
        {
            Some(result) => result.clone(),
            None => {
                // no buffer chains available, create a new one
                create_storage_buffer(&self.device_arc, self.buffer_size, &self.bind_group_layout)
            }
        };

        result.ready.store(false, atomic::Ordering::SeqCst);
        result
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
    data: &[f32],
) {
    // copy staging buffer to storage buffer
    let mut encoder = device_arc.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("staging-to-storage encoder"),
    });
    encoder.copy_buffer_to_buffer(
        &staging.buffer,
        0,
        &storage.buffer,
        0,
        data.len() as u64 * 4,
    );

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
                let mut buffer_pool = create_buffer_pool(BufferPoolDescriptor {
                    device_arc: descriptor.device_arc.clone(),
                    bind_group_layout: descriptor.bind_group_layout,
                    initial_buffer_size: 1024,
                });

                while alive.load(atomic::Ordering::SeqCst) {
                    // receive data from the UI thread
                    let ui_data = descriptor.ui_worker.recv();
                    let num_bytes = (ui_data.data.len() * 4) as u64;

                    // request a mapped staging buffer from the pool
                    let staging_buffer = buffer_pool.request_staging(num_bytes);

                    // retrieve mapped slice from the staging buffer
                    let mut view = staging_buffer
                        .buffer
                        .slice(0..num_bytes)
                        .get_mapped_range_mut();
                    let floats: &mut [f32] = bytemuck::cast_slice_mut(&mut view);
                    // copy UI data into staging buffer
                    floats.copy_from_slice(&ui_data.data);
                    drop(view);
                    staging_buffer.buffer.unmap();

                    // copy staging buffer to a storage buffer
                    // TODO: investigate if we want to request the storage buffer here or before writing to the staging buffer
                    let storage_buffer = buffer_pool.request_storage(num_bytes);
                    staging_to_storage(
                        staging_buffer,
                        &storage_buffer,
                        &descriptor.device_arc,
                        &descriptor.queue_arc,
                        &ui_data.data,
                    );

                    // send the storage buffer to the render thread
                    let message = TransferWorkerMessage {
                        storage_buffer: storage_buffer,
                        num_instances: ui_data.num_instances,
                    };
                    match sender.try_send(message) {
                        Ok(_) => {}
                        Err(mpsc::TrySendError::Full(_)) => {}
                        Err(mpsc::TrySendError::Disconnected(_)) => {
                            // send failed, exit the loop and clean up
                            break;
                        }
                    }
                }

                // clean up
                descriptor.ui_worker.stop_and_join();
            })
            .expect("failed to spawn transfer worker"),
    }
}

pub struct TransferWorkerDescriptor {
    pub device_arc: Arc<wgpu::Device>,
    pub queue_arc: Arc<wgpu::Queue>,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub ui_worker: ui::UIWorker,
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
        println!("stopping transfer worker");
        self.alive.store(false, atomic::Ordering::SeqCst);
        self.worker_handle
            .join()
            .expect("failed to join transfer worker thread");
    }
}
