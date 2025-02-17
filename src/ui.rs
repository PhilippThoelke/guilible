use crate::utils;
use bytemuck::NoUninit;
use rayon::prelude::*;

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
    pub quads: Vec<Quad>,
}

impl QuadManager {
    pub fn add_quad(&mut self, x: f32, y: f32, w: f32, h: f32, color: utils::Color) {
        self.quads.push(Quad { x, y, w, h, color });
    }
}

pub fn setup(quad_manager: &mut QuadManager) {
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

pub fn update(quad_manager: &mut QuadManager, start_time: std::time::Instant) {
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
