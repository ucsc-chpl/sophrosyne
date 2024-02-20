@group(0) @binding(0) var<storage, read_write> A: array<u32>;
@group(0) @binding(1) var<storage, read_write> B: array<u32>;
@group(0) @binding(2) var<storage, read_write> C: array<u32>;

@workgroup_size(1)
@compute 
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;

    // Perform the addition
    C[idx] = A[idx] + B[idx];
}