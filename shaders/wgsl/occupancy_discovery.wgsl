@group(0) @binding(0) var<storage, read_write> count: array<atomic<u32>>;
@group(0) @binding(1) var<storage, read_write> poll: array<u32>;
@group(0) @binding(2) var<storage, read_write> scratchpad: array<atomic<u32>>;

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

            // Do busy so other workgroups can participate.
            for (var i = 0; i < 1000; i++) {
                atomicAdd(&scratchpad[0], 1u);
            }

            // Close the poll.
            poll[0] = 1u;
        }
    }
}