struct Quad {
    center: vec2<f32>,
    size: vec2<f32>,
    color: vec4<f32>,
};

struct VertexOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@group(0) @binding(0) var<storage, read> quad_buffer: array<Quad>;

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32, @builtin(instance_index) instance_index: u32) -> VertexOut {
    let quad = quad_buffer[instance_index];

    // calculate the position of the vertex
    let x = quad.center.x + quad.size.x * select(-0.5, 0.5, vertex_index / 2u == 0u);
    let y = quad.center.y + quad.size.y * select(-0.5, 0.5, vertex_index % 2u == 0u);

    // return the vertex position and color
    return VertexOut(
        vec4<f32>(x, y, 0.0, 1.0),
        quad.color
    );
}


@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    return in.color;
}