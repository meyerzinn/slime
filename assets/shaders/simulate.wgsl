#import bevy_core_pipeline::fullscreen_vertex_shader
#import "shaders/pcg.wgsl"

@group(3) @binding(0)
var<uniform> random_seed: u32;

@group(0) @binding(0)
var<storage, read_write> agents: array<Agent>; // someday: bind write-only (https://github.com/gfx-rs/wgpu/issues/2897)

@group(1) @binding(0)
var t_trails_prev: texture_2d<f32>;

@group(2) @binding(0)
var t_trails_next: texture_storage_2d<rgba8unorm, write>;

struct Agent {
  pos: vec2<f32>,
  angle: f32,
  species: u32,
}

const TWO_PI: f32 = 6.28318530718;

@compute
@workgroup_size(256, 1, 1)
// Initializes the simulation.
fn init(@builtin(local_invocation_index) local_id: u32,
        @builtin(num_workgroups) num_workgroups: vec3<u32>)
{
  seed(random_seed);
  seed(local_id);

  let total_kernels: u32 = num_workgroups.x * num_workgroups.y * num_workgroups.z;
  let total_agents: u32 = arrayLength(&agents);
  let agents_per_kernel = (total_agents + (total_kernels - 1u)) / total_kernels;

  let start = agents_per_kernel * local_id;
  for (var index = start; index < min(start + agents_per_kernel, total_agents); index++) {
    var agent: Agent;
    agent.pos = vec2<f32>(rand_f32(), rand_f32());
    agent.angle = TWO_PI * rand_f32();
    agent.species = 32u;
    agents[index] = agent;
  }
}

const VELOCITY: f32 = 0.0001;

@compute
@workgroup_size(256, 1, 1)
// Updates the simulation.
fn update(@builtin(local_invocation_index) local_id: u32,
          @builtin(num_workgroups) num_workgroups: vec3<u32>)
{
  seed(random_seed);
  seed(local_id);

  let total_kernels: u32 = num_workgroups.x * num_workgroups.y * num_workgroups.z;
  let total_agents: u32 = arrayLength(&agents);
  let agents_per_kernel = (total_agents + (total_kernels - 1u)) / total_kernels;

  let start = agents_per_kernel * local_id;  
  for (var index = start; index < min(start + agents_per_kernel, total_agents); index++) {
    var agent: Agent = agents[index];
    var heading = vec2<f32>(cos(agent.angle), sin(agent.angle));
    agent.pos += VELOCITY * heading;
    var edge_normal = vec2<f32>(0.0, 0.0);
    if (agent.pos.x < 0.0) {
      agent.pos.x = 0.0;
      edge_normal.x = 1.0;
    }
    if (agent.pos.x > 1.0) {
      agent.pos.x = 1.0;
      edge_normal.x = -1.0;
    }
    if (agent.pos.y < 0.0) {
      agent.pos.y = 0.0;
      edge_normal.y = 1.0;
    }
    if (agent.pos.y > 1.0) {
      agent.pos.y = 1.0;
      edge_normal.y = -1.0;
    }
    heading -= 2.0 * edge_normal * dot(heading, edge_normal);
    agent.angle = atan2(heading.y, heading.x);
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

  let dims = vec2<f32>(textureDimensions(t_trails_next));
  for (var index = start; index < min(start + agents_per_kernel, total_agents); index++) {
    var agent: Agent = agents[index];
    let coords = vec2<u32>(clamp(floor(agent.pos * dims), vec2<f32>(0.0), dims - 1.0));
    // @todo compute color ("released chemical") for the agent
    let color = vec4<f32>(1.0, 0.0, 0.0, 1.0);
    textureStore(t_trails_next, coords, color);
  }
}

@group(3) @binding(0)
var<uniform> direction: vec2<i32>;

var<private> BLUR_COEFFS : array<f32, 5> = array<f32, 5>(0.0702702703, 0.3162162162, 0.1135135135, 0.3162162162, 0.0702702703);

@fragment
fn blur_fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let dims = vec2<u32>(textureDimensions(t_trails_prev));
    let coords_upper_left = vec2<u32>(floor(in.position.xy));
    let coords = vec2<u32>(coords_upper_left.x, dims.y - coords_upper_left.y - 1u);

    var color = vec3<f32>(0.0);
    for (var i = 0; i < 5; i++) {
      let off = vec2<i32>(coords) + direction * (i - 2);
      if (off.x >= 0 && off.x < i32(dims.x) && off.y >= 0 && off.y < i32(dims.y)) {
        color += textureLoad(t_trails_prev, vec2<u32>(off), 0).rgb * BLUR_COEFFS[i]; 
      }
    }
    return vec4(color, 1.0);
}