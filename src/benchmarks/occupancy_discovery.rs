use easywg::*;

pub async fn occupancy_discovery() {
    let device = Device::new().await;

    log::info!("Device name: {:?}", device.adapter_info.name);
    log::info!("Using backend: {:?}", device.adapter_info.backend);

    let num_workgroups = 1024;
    let workgroup_size = 1;

    // Define host buffers.
    let mut h_count = vec![0u32; 0];

    device
        .launch_compute(
            "../shaders/wgsl/occupancy_discovery.wgsl",
            &mut vec![&mut h_count],
            num_workgroups,
            workgroup_size,
        )
        .await;

    // Print results.
    // Results are automatically mapped back to the host.
}