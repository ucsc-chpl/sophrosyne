use easywg::*;

pub async fn atomic_throughput() {
    let device = Device::new().await;

    log::info!("Device name: {:?}", device.adapter_info.name);
    log::info!("Using backend: {:?}", device.adapter_info.backend);

    let n: usize = 16;
    let test_iters = 64;
    let rmw_iters = 16;

    let workgroup_size = 4;
    // TODO: Use occupancy discovery to get # of workgroups.
    let workgroups = 3;
    let contention = 1;
    let padding = 1;

    // Define host buffers.
    let mut h_res = vec![0u32; workgroups * workgroup_size];
    let mut h_iters = vec![16u32; 1];
    h_iters[0] = rmw_iters;
    let mut h_indexes = vec![0u32; workgroups * workgroup_size];
    for i in 0..workgroups * workgroup_size {
        // Contiguous access.
        h_indexes[i] = ((i / contention) * padding) as u32;
    }

    for _ in 0..test_iters {
        device
            .launch_compute(
                "shaders/wgsl/atomic_fa_relaxed.wgsl",
                &mut vec![&mut h_res, &mut h_iters, &mut h_indexes],
                n as u32,
                1,
            )
            .await;
    }

}
