
@group(0) @binding(0) var<storage, read> vertex_buffer: array<vec2<f32>>;

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4<f32> {
    return vec4<f32>(vertex_buffer[vertex_index], 0.0, 1.0);
}


@fragment
fn fs_main(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 0.0, 0.0, 1.0);
}