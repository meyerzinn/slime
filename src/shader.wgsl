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
fn fs_clear(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(0.0);
}

// Fragment shader

@group(0) @binding(0)
var t_slime_trail: texture_2d<f32>;
@group(0) @binding(1)
var s_slime_trail: sampler;

@fragment
fn fs_blur(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    // let texel = vec2<f32>(1.0) / vec2<f32>(textureDimensions(t_slime_trail));
    let dims = textureDimensions(t_slime_trail);
    let texel = 1.0 / vec2<f32>(dims);

    var color = vec3<f32>(0.0);
    for (var i = -2; i < 3; i++) {
        for (var j = -2; j < 3; j++) {
            color += textureSample(t_slime_trail, s_slime_trail, uv + texel * vec2<f32>(f32(i), f32(j))).rgb;
        }
    }
    color *= 0.995 / 25.0;
    if (color.r < 0.005) {
        color.r = 0.0;
    }
    if (color.g < 0.005) {
        color.g = 0.0;
    }
    if (color.b < 0.005) {
        color.b = 0.0;
    }
    return vec4<f32>(color, 1.0);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(t_slime_trail, s_slime_trail, in.uv);
}


// https://www.pcg-random.org/
fn pcg(v: u32) -> u32
{
	let state: u32 = v * 747796405u + 2891336453u;
	let word: u32 = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
	return (word >> 22u) ^ word;
}

@group(1) @binding(0) var agents_old: texture_1d<f32>;
@group(1) @binding(1) var agents_new: texture_storage_1d<rgba32float, write>;
@group(1) @binding(2) var texture: texture_storage_2d<rgba8unorm, write>;

@compute
@workgroup_size(256)
fn cs_init(@builtin(num_workgroups) num_workgroups: vec3<u32>, @builtin(local_invocation_index) local_invocation_index: u32) {
    let total_kernels: u32 = num_workgroups.x * num_workgroups.y * num_workgroups.z;
    let total_agents: u32 = u32(textureDimensions(agents_old));
    let agents_per_kernel = (total_agents + (total_kernels - 1u)) / total_kernels;

    let start = agents_per_kernel * local_invocation_index;
    for (var index = start; index < min(total_agents, start + agents_per_kernel); index++) {
        var random = pcg(index);
        let species = random & 0x3u;
        random >>= 2u;
        let x = random & 0x3FFu;
        random >>= 10u;
        let y = random & 0x3FFu;
        random >>= 10u;
        let angle = random & 0x3FFu;
        
        let x_n = f32(x) / f32(0x3FFu); // in [0, 1]
        let y_n = f32(y) / f32(0x3FFu); // in [0, 1]
        let angle_n = f32(angle) / f32(0x400u) * 2.0 * 3.14159; // in [0, 2 * PI] (todo: normalize)

        let position = vec2<f32>(x_n, y_n);
        textureStore(agents_new, index, vec4<f32>(position.x, position.y, angle_n, f32(species)));    
    }
}

const VELOCITY = vec2<f32>(0.0005, 0.0005);

@compute
@workgroup_size(256)
fn cs_main(@builtin(local_invocation_index) index: u32) {
    if (index >= u32(textureDimensions(agents_old))) {
        return;
    }
    let agent_old : vec4<f32> = textureLoad(agents_old, index, 0);
    let position_old = vec2<f32>(agent_old.r, agent_old.g);
    let angle_old = agent_old.b;
    let species = u32(agent_old.a);

    var angle_new = angle_old;
    var heading = vec2<f32>(cos(angle_old), sin(angle_old));
    var position_new = heading * VELOCITY + position_old;
    var edge_normal = vec2<f32>(0.0, 0.0);
    if (position_new.x < 0.0) {
        edge_normal.x = 1.0;
    }
    if (position_new.x > 1.0) {
        edge_normal.x = -1.0;
    }
    if (position_new.y < 0.0) {
        edge_normal.y = 1.0;
    }
    if (position_new.y > 1.0) {
        edge_normal.y = -1.0;
    }
    heading -= 2.0 * edge_normal * dot(heading, edge_normal);
    angle_new = atan2(heading.y, heading.x);
    position_new = clamp(position_new, vec2<f32>(0.0), vec2<f32>(1.0));
    
    var color: vec3<f32>;
    switch (species) {
        case 0u: {
            color = vec3<f32>(1.0, 0.0, 0.0);
        }
        case 1u: {
            color = vec3<f32>(0.0, 1.0, 0.0);
        }
        case 2u: {
            color = vec3<f32>(0.0, 0.0, 1.0);
        }
        case 3u: {
            color = vec3<f32>(1.0, 1.0, 1.0);
        }
        default: {
            color = vec3<f32>(0.0, 0.0, 0.0);
        }
    }

    // render slime mold to texture
    let tex_dims = textureDimensions(texture);
    let crit_dim = f32(max(tex_dims.x, tex_dims.y));
    let coords = vec2<u32>(clamp(position_new * crit_dim, vec2<f32>(0.0), vec2<f32>(crit_dim)));
    textureStore(texture, coords, vec4<f32>(color, 1.0));

    // update the agent
    textureStore(agents_new, index, vec4<f32>(position_new.x, position_new.y, angle_new, f32(species)));
}
