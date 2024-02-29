@group(0) @binding(0) var<storage, read_write> count: array<atomic<u32>>;
@group(0) @binding(1) var<storage, read_write> poll_open: array<u32>;

@workgroup_size(1)
@compute 
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
}