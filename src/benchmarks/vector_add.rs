use easywg::*;

#[allow(unused)]
pub async fn vector_add() {
    let device = Device::new().await;

    log::info!("Device name: {:?}", device.adapter_info.name);
    log::info!("Using backend: {:?}", device.adapter_info.backend);

    let n: usize = 16;

    // Define host buffers.
    let mut h_a = vec![1u32; n];
    let mut h_b = vec![1u32; n];
    let mut h_c = vec![0u32; n];

    device
        .launch_compute(
            "../shaders/wgsl/vector_add.wgsl",
            &mut vec![&mut h_a, &mut h_b, &mut h_c],
            n as u32,
            1,
        )
        .await;

    // Print results.
    // Results are automatically mapped back to the host.
    for i in 0..n {
        log::info!("{} + {} = {}", h_a[i], h_b[i], h_c[i]);
    }
    
}