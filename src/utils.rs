use bytemuck::NoUninit;

#[repr(C)]
#[derive(Clone, Copy, NoUninit)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}
