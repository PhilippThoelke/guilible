use crate::render;
use crate::utils;
use rayon::prelude::*;

pub struct QuadManager {
    pub quads: Vec<render::Quad>,
}

impl QuadManager {
    pub fn add_quad(&mut self, x: f32, y: f32, w: f32, h: f32, color: utils::Color) {
        self.quads.push(render::Quad { x, y, w, h, color });
    }
}

pub struct UIState {
    quad_manager: QuadManager,
}

impl UIState {
    pub fn new() -> Self {
        let quad_manager = QuadManager { quads: Vec::new() };
        Self { quad_manager }
    }

    pub fn setup(&mut self) {
        let n = 2000;
        let quad_size = 0.001;
        for i in 0..n {
            for j in 0..n {
                self.quad_manager.add_quad(
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
    }

    pub fn update(&mut self, start_time: std::time::Instant) {
        let delta = start_time.elapsed().as_secs_f32();
        let n = (self.quad_manager.quads.len() as f32).sqrt();

        self.quad_manager
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

    pub fn quads(&self) -> &[f32] {
        bytemuck::cast_slice(&self.quad_manager.quads)
    }

    pub fn num_quads(&self) -> u32 {
        self.quad_manager.quads.len() as u32
    }
}
