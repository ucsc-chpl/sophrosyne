@group(0) @binding(0) var<storage, read_write> count: array<atomic<u32>>;
@group(0) @binding(1) var<storage, read_write> poll: array<u32>;
@group(0) @binding(2) var<storage, read_write> scratchpad: array<atomic<u32>>;

const LOCAL_MEM_SIZE: u32 = 1;
var<workgroup> local_mem: array<u32, LOCAL_MEM_SIZE>;

@workgroup_size(1)
@compute 
fn main(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
) {
    // Single representative thread from each workgroup takes part in the discovery protocol.
    if local_id.x == 0 {
        if poll[0] == 0 {
            // Poll is open, increment counter.
            atomicAdd(&count[0], 1u);

            // Do busy work so other workgroups can participate.
            for (var i = 0; i < 100000; i++) {
                atomicAdd(&scratchpad[(i + i32(global_id.x))  % 10000], 1u);
            }

            // Close the poll.
            poll[0] = 1u;
        }

        // Do work on local memory so it's not compiled away.
        local_mem[global_id.x % LOCAL_MEM_SIZE] += 1u;
    }
}