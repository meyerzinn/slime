//! Projects the agents into a texture.

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(
    @builtin(vertex_index) in_vertex_index: u32,
) -> VertexOutput {
    var out: VertexOutput;

    // we just need four vertices, one for each corner of the screen
    // arranged in a triangle strip:
    // 2      3
    //
    // 0      1
    
    // clip space:
    // 0 (0b00) => (-1, -1)  
    // 1 (0b01) => ( 1, -1)  
    // 2 (0b10) => (-1,  1)   
    // 3 (0b11) => ( 1,  1)

    let x = f32(i32((in_vertex_index & 1u) << 1u) - 1);
    let y = f32(2 * i32(in_vertex_index >> 1u) - 1);
    out.clip_position = vec4<f32>(x, y, 0.0, 1.0);

    // texture space:
    // 0 (0b00) => (0, 0)
    // 1 (0b01) => (1, 0)
    // 2 (0b10) => (0, 1)
    // 3 (0b11) => (1, 1)

    let u = f32(in_vertex_index & 1u);
    let v = f32(in_vertex_index >> 1u);
    out.uv = vec2<f32>(u, v);

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<u32> {
    return vec4<u32>(0xFFFFFFFFu, 0u, 0u, 0xFFFFFFFFu);
}