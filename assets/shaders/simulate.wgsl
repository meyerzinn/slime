@group(0) @binding(0)
var<storage, read_write> agents: array<Agent>; // someday: bind write-only (https://github.com/gfx-rs/wgpu/issues/2897)

@group(1) @binding(0)
var t_trails_prev: texture_2d<f32>;

@group(1) @binding(1)
var t_trails_next: texture_storage_2d<rgba8unorm, write>;

struct Agent {
  pos: vec2<f32>,
  angle: f32,
  species: u32,
}

@compute
@workgroup_size(256, 1, 1)
// Initializes the simulation.
fn init(@builtin(local_invocation_index) local_id: u32,
        @builtin(num_workgroups) num_workgroups: vec3<u32>)
{
  let total_kernels: u32 = num_workgroups.x * num_workgroups.y * num_workgroups.z;
  let total_agents: u32 = arrayLength(&agents);
  let agents_per_kernel = (total_agents + (total_kernels - 1u)) / total_kernels;

  let start = agents_per_kernel * local_id;
  for (var index = start; index < min(start + agents_per_kernel, total_agents); index++) {
    var agent: Agent;
    agent.pos = vec2<f32>(0.5, 0.5);
    agent.angle = 1.0;
    agent.species = 32u;
    agents[index] = agent;
  }
}

@compute
@workgroup_size(256, 1, 1)
// Updates the simulation.
fn update(@builtin(local_invocation_index) local_id: u32,
          @builtin(num_workgroups) num_workgroups: vec3<u32>)
{
  let total_kernels: u32 = num_workgroups.x * num_workgroups.y * num_workgroups.z;
  let total_agents: u32 = arrayLength(&agents);
  let agents_per_kernel = (total_agents + (total_kernels - 1u)) / total_kernels;

  let start = agents_per_kernel * local_id;
  for (var index = start; index < min(start + agents_per_kernel, total_agents); index++) {
    var agent: Agent = agents[index];
    agent.pos += 0.0001 * vec2<f32>(1.0);
    agent.angle = 2.0;
    agents[index] = agent;
  }
}


@compute
@workgroup_size(256, 1, 1)
// Projects the agents onto the new texture.
fn project(@builtin(local_invocation_index) local_id: u32,
           @builtin(num_workgroups) num_workgroups: vec3<u32>)
{
  let total_kernels: u32 = num_workgroups.x * num_workgroups.y * num_workgroups.z;
  let total_agents: u32 = arrayLength(&agents);
  let agents_per_kernel = (total_agents + (total_kernels - 1u)) / total_kernels;

  let start = agents_per_kernel * local_id;

  let dims = vec2<f32>(textureDimensions(t_trails_prev));
  for (var index = start; index < min(start + agents_per_kernel, total_agents); index++) {
    var agent: Agent = agents[index];
    let coords = vec2<u32>(clamp(agent.pos * dims, vec2<f32>(0.0), dims));
    // @todo compute color ("released chemical") for the agent
    let color = vec4<f32>(1.0, 0.0, 0.0, 1.0);
    textureStore(t_trails_next, coords, color);
  }
}

const TWO_PI: f32 = 6.28318530718;

// sigma = 10
var<private> BLUR_COEFFS: array<f32, 33> = array<f32, 33>(
	0.012318109844189502,
	0.014381474814203989,
	0.016623532195728208,
	0.019024086115486723,
	0.02155484948872149,
	0.02417948052890078,
	0.02685404941667096,
	0.0295279624870386,
	0.03214534135442581,
	0.03464682117793548,
	0.0369716985390341,
	0.039060328279673276,
	0.040856643282313365,
	0.04231065439216247,
	0.043380781642569775,
	0.044035873841196206,
	0.04425662519949865,
	0.044035873841196206,
	0.043380781642569775,
	0.04231065439216247,
	0.040856643282313365,
	0.039060328279673276,
	0.0369716985390341,
	0.03464682117793548,
	0.03214534135442581,
	0.0295279624870386,
	0.02685404941667096,
	0.02417948052890078,
	0.02155484948872149,
	0.019024086115486723,
	0.016623532195728208,
	0.014381474814203989,
	0.012318109844189502
);


@group(2) @binding(0)
var<uniform> direction: vec2<i32>;

fn compute_blur(at: vec2<i32>) -> vec4<f32> {
  let dims = textureDimensions(t_trails_prev);

}

@compute
@workgroup_size(256, 1, 1)
// Projects the agents onto the new texture.
fn blur(@builtin(local_invocation_index) local_id: u32,
        @builtin(num_workgroups) num_workgroups: vec3<u32>)
{
  let dims = vec2<u32>(textureDimensions(t_trails_prev));
  let total_pixels: u32 = dims.x * dims.y; 
  let total_kernels: u32 = num_workgroups.x * num_workgroups.y * num_workgroups.z;
  let pixels_per_kernel: u32 = (total_pixels + (total_kernels - 1u)) / total_kernels;

  var count = 0u;
  let start: u32 = pixels_per_kernel * local_id;
  var x: u32 = start / dims.x;
  var y: u32 = start - (x * dims.x);
  while (count < pixels_per_kernel) {
    let coords = vec2<u32>(x, y);
    let lower = vec2<i32>(0);
    let upper = vec2<i32>(dims - 1u);
    var color = vec3<f32>(0.0);
    for (var i = 0; i < 33; i++) {
      let tc = clamp(vec2<i32>(coords) + direction * (i - 16), vec2<i32>(0), vec2<i32>(dims - 1u));
      color += BLUR_COEFFS[i] * textureLoad(t_trails_prev, tc, 0).rgb;
    }
    textureStore(t_trails_next, coords, vec4<f32>(color, 1.0));

    // update indices
    count++;
    y++;
    if (y >= dims.y) {
      y = 0u;
      x++;
      if (x >= dims.x) {
        break;
      }
    }
  }  
}


// struct VertexOutput {
//     @builtin(position) clip_position: vec4<f32>,
//     @location(0) uv: vec2<f32>,
// };

// @vertex
// fn blur_vertex(
//     @builtin(vertex_index) in_vertex_index: u32,
// ) -> VertexOutput {
//     var out: VertexOutput;

//     // we just need four vertices, one for each corner of the screen
//     // arranged in a triangle strip:
//     // 2      3
//     //
//     // 0      1
    
//     // clip space:
//     // 0 (0b00) => (-1, -1)  
//     // 1 (0b01) => ( 1, -1)  
//     // 2 (0b10) => (-1,  1)   
//     // 3 (0b11) => ( 1,  1)

//     let x = f32(i32((in_vertex_index & 1u) << 1u) - 1);
//     let y = f32(2 * i32(in_vertex_index >> 1u) - 1);
//     out.clip_position = vec4<f32>(x, y, 0.0, 1.0);

//     // texture space:
//     // 0 (0b00) => (0, 0)
//     // 1 (0b01) => (1, 0)
//     // 2 (0b10) => (0, 1)
//     // 3 (0b11) => (1, 1)

//     let u = f32(in_vertex_index & 1u);
//     let v = f32(in_vertex_index >> 1u);
//     out.uv = vec2<f32>(u, v);

//     return out;
// }

// @fragment
// fn blur_fragment(in: VertexOutput) -> @location(0) vec4<f32> {
//     // todo: blur
//     let dims = vec2<f32>(textureDimensions(t_trails_prev));
//     let coords = vec2<u32>(clamp(in.uv * dims, vec2<f32>(0.0), dims));

//     return textureLoad(t_trails_prev, coords, 0);
// }