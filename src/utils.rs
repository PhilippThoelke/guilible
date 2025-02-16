use bytemuck::NoUninit;
use online_statistics::{self, stats::Univariate};

#[repr(C)]
#[derive(Clone, Copy, NoUninit)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

#[derive(Default)]
pub struct Stats {
    pub mean: online_statistics::mean::Mean<f64>,
    pub variance: online_statistics::variance::Variance<f64>,
}

impl Stats {
    pub fn update(&mut self, value: f64) {
        self.mean.update(value);
        self.variance.update(value);
    }
}

impl std::fmt::Display for Stats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "μ = {:>5.2}ms ± {:>5.2}ms",
            self.mean.get() * 1000.0,
            self.variance.get().sqrt() * 1000.0,
        )
    }
}
