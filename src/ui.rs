use crate::utils;
use bytemuck::NoUninit;
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

pub struct UIWorkerMessage {
    pub data: Vec<f32>,
    pub num_instances: u32,
}

pub fn create_ui_worker() -> UIWorker {
    // TODO: check if we want a buffer size of 0 or 1 in the sync_channel
    let (sender, receiver) = std::sync::mpsc::sync_channel(0);
    let alive = Arc::new(atomic::AtomicBool::new(true));

    UIWorker {
        receiver,
        alive: alive.clone(),
        worker_handle: thread::Builder::new()
            .name("ui worker".to_string())
            .spawn(move || {
                let start = std::time::Instant::now();

                let mut quad_manager = QuadManager { quads: Vec::new() };
                let n = 100;
                let quad_size = 0.01;
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
                                b: ((i as f32 / n as f32) * 2.0 - 1.0)
                                    * ((j as f32 / n as f32) * 2.0 - 1.0),
                                a: 1.0,
                            },
                        );
                    }
                }

                while alive.load(atomic::Ordering::SeqCst) {
                    for quad in &mut quad_manager.quads {
                        quad.x += start.elapsed().as_secs_f32().cos() * 0.001;
                        quad.y += start.elapsed().as_secs_f32().cos() * 0.001;
                    }

                    let message = UIWorkerMessage {
                        data: quad_manager
                            .quads
                            .iter()
                            .flat_map(|quad| -> [f32; 8] { quad.into() })
                            .collect(),
                        num_instances: quad_manager.quads.len() as u32,
                    };

                    match sender.try_send(message) {
                        Ok(_) => {}
                        Err(mpsc::TrySendError::Full(_)) => {}
                        Err(mpsc::TrySendError::Disconnected(_)) => {
                            break;
                        }
                    }
                }
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
        println!("stopping ui worker");
        self.alive.store(false, atomic::Ordering::SeqCst);
        self.worker_handle
            .join()
            .expect("failed to join ui worker thread");
    }
}
