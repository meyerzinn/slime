// pcg hash function
var<private> rng_state: u32;

fn seed(x: u32) {
	rng_state = mix(rng_state, hash(x));
}

fn mix(x: u32, y: u32) -> u32 {
    return 19u * x + 47u * y + 101u;
}

// Returns a random u32.
fn rand_u32() -> u32 {
    let state = rng_state;
    rng_state = rng_state * 747796405u + 2891336453u;
    let word = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
    return (word >> 22u) ^ word;
}

// Returns an f32 in the range [0, 1)
fn rand_f32() -> f32 {
    return f32(rand_u32()) / f32(0xFFFFFFFFu);
}

fn hash(v: u32) -> u32
{
	let state: u32 = v * 747796405u + 2891336453u;
	let word: u32 = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
	return (word >> 22u) ^ word;
}
