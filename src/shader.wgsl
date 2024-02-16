@group(0)
@binding(0)
var<storage, read_write> a: array<u32>;

@compute
@workgroup_size(1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    a[global_id.x] = global_id.x;
}