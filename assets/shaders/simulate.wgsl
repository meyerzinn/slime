#import bevy_core_pipeline::fullscreen_vertex_shader
#import "shaders/utils.wgsl"

struct SimulationOptions {
  evaporation: f32,
  // @todo: repellants
}

struct Agent {
  pos: vec2<f32>,
  angle: f32,
}

struct Species {
  color: vec3<f32>,
  speed: f32,
  turn_speed: f32,
  view_distance: f32,
  field_of_view: f32,
}

@group(0) @binding(0)
var<storage, read_write> agents: array<Agent>; // someday: bind write-only (https://github.com/gfx-rs/wgpu/issues/2897)
@group(0) @binding(1)
var<uniform> species: Species;

@group(1) @binding(0)
var t_trails_prev: texture_2d<f32>;
@group(1) @binding(1)
var s_trails_prev: sampler;

@group(2) @binding(0)
var t_trails_next: texture_storage_2d<rgba8unorm, write>;

@group(3) @binding(0)
var<uniform> random_seed: u32;

@group(4) @binding(0)
var<uniform> options: SimulationOptions;

const TWO_PI: f32 = 6.28318530718;

@compute
@workgroup_size(256, 1, 1)
// Initializes the simulation.
fn init(@builtin(local_invocation_index) local_id: u32,
        @builtin(num_workgroups) num_workgroups: vec3<u32>)
{
  seed(random_seed);
  seed(local_id);
  seed(u32(species.color.r * 255.0));
  seed(u32(species.color.g * 255.0));
  seed(u32(species.color.b * 255.0));

  let total_kernels: u32 = num_workgroups.x * num_workgroups.y * num_workgroups.z;
  let total_agents: u32 = arrayLength(&agents);
  let agents_per_kernel = (total_agents + (total_kernels - 1u)) / total_kernels;

  let start = agents_per_kernel * local_id;
  for (var index = start; index < min(start + agents_per_kernel, total_agents); index++) {
    let r = 0.5 * clamp(sqrt(rand_f32()), 0.0, 1.0);
    let t = rand_f32() * TWO_PI;
    var agent: Agent;
    agent.pos = 0.5 + r * vec2<f32>(cos(t), sin(t));
    agent.angle = atan2(0.5 - agent.pos.y, 0.5 - agent.pos.x);
    agents[index] = agent;
  }
}

const STEER_NUM_SAMPLES: u32 = 3u;

// returns the new heading for an agent
fn steer(agent: Agent) -> f32 {
  let angle_delta = (species.turn_speed * 2.0) / f32(STEER_NUM_SAMPLES - 1u);
  var angle = agent.angle - species.turn_speed;
  var t = agent.angle;
  var t_sim = 0.0;
  for (var i = 0u; i < STEER_NUM_SAMPLES; i++) {
    let dir = vec2<f32>(cos(angle), sin(angle));
    let wc =  species.view_distance * dir + agent.pos;
    // @todo: is this needed? we could skip and just take black if that is faster
    if (wc.x < 0.0 || wc.y < 0.0 || wc.x >= 1.0 || wc.y >= 1.0) {
      continue;
    }
    let tc = world_to_tex(vec2<u32>(textureDimensions(t_trails_prev)), wc);
    let s = textureLoad(t_trails_prev, tc, 0).rgb;
    let d = dot(species.color, s) * 2.0 - 1.0;
    if (d > t_sim) {
      t_sim = d;
      t = angle;
    }
    angle += angle_delta;
  }
  return t;
}

// converts world coordinates to texel index, assuming pos.x and pos.y are in [0.0, 1.0].
fn world_to_tex(dims: vec2<u32>, pos: vec2<f32>) -> vec2<u32> {
  let scaled = pos * vec2<f32>(dims);
  return vec2<u32>(clamp(floor(scaled), vec2<f32>(0.0), vec2<f32>(dims - 1u)));
}

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

  let dims = textureDimensions(t_trails_prev);

  let start = agents_per_kernel * local_id;  
  for (var index = start; index < min(start + agents_per_kernel, total_agents); index++) {
    var agent: Agent = agents[index];

    agent.angle = steer(agent);
    var heading = vec2<f32>(cos(agent.angle), sin(agent.angle));
    agent.pos += species.speed * heading;
    
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
    // slightly perturb the heading by up to 0.1 degrees
    agent.angle = atan2(heading.y, heading.x) + 0.00174533 * (rand_f32() - 0.5);
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
  let dims = vec2<u32>(textureDimensions(t_trails_next));
  for (var index = start; index < min(start + agents_per_kernel, total_agents); index++) {
    let agent: Agent = agents[index];
    let texel = world_to_tex(dims, agent.pos);
    textureStore(t_trails_next, texel, vec4(species.color, 1.0));
  }
}

@group(3) @binding(0)
var<uniform> direction: vec2<i32>;

const BLUR_SAMPLE_COUNT: i32 = 4;

var<private> BLUR_OFFSETS : array<f32, 4> = array<f32, 4>(
    -2.431625915613778,
    -0.4862426846689484,
    1.4588111840004858,
    3.0
);

var<private> BLUR_WEIGHTS : array<f32, 4> = array<f32, 4>(
    0.24696196374528634,
    0.34050702333458593,
    0.30593582919679174,
    0.10659518372333592
);


@fragment
fn blur_fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let dims = vec2<f32>(textureDimensions(t_trails_prev));
    var color = vec3<f32>(0.0);
    for (var i = 0; i < BLUR_SAMPLE_COUNT; i++)
    {
        let offset: vec2<f32> = vec2<f32>(direction) * BLUR_OFFSETS[i] / dims;
        let weight: f32 = BLUR_WEIGHTS[i];
        color += textureSample(t_trails_prev, s_trails_prev, in.uv + offset).rgb * weight;
    }
    color = clamp(color - options.evaporation, vec3<f32>(0.0), vec3<f32>(1.0));
    return vec4(color, 1.0);
}