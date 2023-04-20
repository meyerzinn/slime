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