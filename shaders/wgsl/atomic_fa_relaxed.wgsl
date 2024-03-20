@group(0) @binding(0) var<storage, read_write> res: array<atomic<u32>>;
@group(0) @binding(1) var<storage, read_write> iters: array<u32>;
@group(0) @binding(2) var<storage, read_write> indexes: array<u32>;

@workgroup_size(1)
@compute 
fn main(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
) {
    var index = indexes[global_id.x];
    for (var i = 0u; i < iters[0]; i++) {
        atomicAdd(&res[index], 1u);
    }
}