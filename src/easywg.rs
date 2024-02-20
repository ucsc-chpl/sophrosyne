use std::mem::size_of_val;
use std::path::Path;
use regex::Regex;

fn read_wgsl<P: AsRef<Path>>(path: P) -> String {
    std::fs::read_to_string(path).expect("Failed to read WGSL file")
}

fn modify_workgroup_size(wgsl_code: &str, new_size: u32) -> String {
    let re = Regex::new(r"@workgroup_size\(\d+\)").unwrap();
    re.replace(wgsl_code, format!("@workgroup_size({})", new_size).as_str()).to_string()
}

pub struct Device {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pub adapter_info: wgpu::AdapterInfo,
}

impl Device {
    pub async fn new() -> Self {
        let instance = wgpu::Instance::default();
        // TODO: Provide way to select adapter.
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .unwrap();
        log::info!("Created adapter");
        let adapter_info = adapter.get_info();
        // TODO: Provide way to select device and requested features.
        use wgpu::Features;
        let device_descriptor = wgpu::DeviceDescriptor {
            label: None,
            required_features: Features::TIMESTAMP_QUERY,
            required_limits: Default::default(),
        };
        let (device, queue) = adapter
            .request_device(&device_descriptor, None)
            .await
            .unwrap();
        log::info!("Created device and queue");

        Self {
            device,
            queue,
            adapter_info,
        }
    }

    pub async fn launch_compute(
        &self,
        wgsl_path: &str,
        buffers: &mut Vec<&mut Vec<u32>>,
        num_workgroups: u32,
        workgroup_size: u32,
    ) {
        let shader_src = modify_workgroup_size(&read_wgsl(wgsl_path), workgroup_size);
        let shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("WGSL Shader"),
                source: wgpu::ShaderSource::Wgsl(shader_src.into()),
            });
        log::info!("Created shader module.");

        // For each provided host buffer, create a device local storage buffer.
        let mut storage_buffers = Vec::new();
        for host_buf in buffers.iter() {
            // Storage buffers are only used for read-write access from the shader.
            // They need COPY_DST so we can copy the host buffer to the device buffer and
            // COPY_SRC so we can copy the device buffer back to the staging buffers.
            let device_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: None,
                size: (host_buf.len() * std::mem::size_of::<u32>()) as wgpu::BufferAddress,
                usage: wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::COPY_DST
                    | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            });

            // Copy host data to device buffer.
            self.queue
                .write_buffer(&device_buf, 0, bytemuck::cast_slice(host_buf));

            storage_buffers.push(device_buf);
        }
        log::info!("Created storage buffers and copied over host data.");

        // Create corresponding staging buffers for each storage buffer.
        let mut output_staging_buffers = Vec::new();
        for storage_buffer in &storage_buffers {
            // Staging buffers can be mapped to host memory (MAP_READ) so that we can read shader results on the host.
            // We later use CommandEncoder::copy_buffer_to_buffer to copy the device buffers to the staging buffers.
            let staging_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: None,
                size: storage_buffer.size(),
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });
            output_staging_buffers.push(staging_buffer);
        }
        log::info!("Created output staging buffers.");

        // Create bind groups.
        let bind_group_layout_entries: Vec<wgpu::BindGroupLayoutEntry> = (0..storage_buffers.len())
            .map(|binding| wgpu::BindGroupLayoutEntry {
                binding: binding as u32,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            })
            .collect();

        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: None,
                    entries: &bind_group_layout_entries,
                });

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bind_group_layout,
            entries: &storage_buffers
                .iter()
                .enumerate()
                .map(|(i, buffer)| wgpu::BindGroupEntry {
                    binding: i as u32,
                    resource: buffer.as_entire_binding(),
                })
                .collect::<Vec<_>>(),
        });
        log::info!("Created bind group");

        // Create pipeline.
        let pipeline_layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });

        let pipeline = self
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: None,
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "main",
            });
        log::info!("Created compute pipeline");

        // Record commands
        let mut command_encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        {
            let mut compute_pass =
                command_encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: None,
                    timestamp_writes: None,
                });
            compute_pass.set_pipeline(&pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            compute_pass.dispatch_workgroups(num_workgroups, 1, 1);
        }
        // We finish the compute pass by dropping it.

        // GPU storage buffer -> staging buffer
        for i in 0..storage_buffers.len() {
            command_encoder.copy_buffer_to_buffer(
                &storage_buffers[i],
                0,
                &output_staging_buffers[i],
                0,
                storage_buffers[i].size(),
            );
        }

        // Finalize the command encoder and submit it to the GPU.
        self.queue.submit(Some(command_encoder.finish()));
        log::info!("Submitted commands.");

        for (host_buf, staging_buf) in buffers.iter_mut().zip(output_staging_buffers.iter()) {
            let buf_slice = staging_buf.slice(..);
            let (tx, rx) = flume::bounded(1);
            buf_slice.map_async(wgpu::MapMode::Read, move |r| tx.send(r).unwrap());
            self.device.poll(wgpu::Maintain::Wait).panic_on_timeout();
            rx.recv_async().await.unwrap().unwrap();
            {
                let view = buf_slice.get_mapped_range();
                host_buf.copy_from_slice(bytemuck::cast_slice(&view));
            }
        }
        log::info!("Results written to host buffers.");
    }
}

pub struct WgpuContext {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::ComputePipeline,
    bind_group: wgpu::BindGroup,
    storage_buffer: wgpu::Buffer,
    output_staging_buffer: wgpu::Buffer,
}

impl WgpuContext {
    pub async fn new(buffer_size: usize) -> WgpuContext {
        let instance = wgpu::Instance::default();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .unwrap();
        log::info!("Using backend: {:?}", adapter.get_info().backend);
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default(), None)
            .await
            .unwrap();

        // Our shader, kindly compiled with Naga.
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!(
                "shader.wgsl"
            ))),
        });

        // This is where the GPU will read from and write to.
        let storage_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: buffer_size as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        // For portability reasons, WebGPU draws a distinction between memory that is
        // accessible by the CPU and memory that is accessible by the GPU. Only
        // buffers accessible by the CPU can be mapped and accessed by the CPU and
        // only buffers visible to the GPU can be used in shaders. In order to get
        // data from the GPU, we need to use CommandEncoder::copy_buffer_to_buffer
        // (which we will later) to copy the buffer modified by the GPU into a
        // mappable, CPU-accessible buffer which we'll create here.
        let output_staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: buffer_size as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        // This can be though of as the function signature for our CPU-GPU function.
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    // Going to have this be None just to be safe.
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        // This ties actual resources stored in the GPU to our metaphorical function
        // through the binding slots we defined above.
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: storage_buffer.as_entire_binding(),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "main",
        });

        WgpuContext {
            device,
            queue,
            pipeline,
            bind_group,
            storage_buffer,
            output_staging_buffer,
        }
    }
}

pub async fn run() {
    let mut input = [0u32; 256];
    let ctx = WgpuContext::new(size_of_val(&input)).await;
    for n in 0..256 {
        input[n] = n as u32;
    }
    compute(&mut input, &ctx).await;
}

pub async fn compute(host_buf: &mut [u32], context: &WgpuContext) {
    log::info!("Running the compute shader.");
    // Local buffer contents -> GPU storage buffer.
    context
        .queue
        .write_buffer(&context.storage_buffer, 0, bytemuck::cast_slice(host_buf));
    log::info!("Wrote input data to the GPU.");

    let mut command_encoder = context
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

    {
        let mut compute_pass = command_encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: None,
            timestamp_writes: None,
        });
        compute_pass.set_pipeline(&context.pipeline);
        compute_pass.set_bind_group(0, &context.bind_group, &[]);
        compute_pass.dispatch_workgroups(host_buf.len() as u32, 1, 1);
    }
    // We finish the comput pass by dropping it.

    // GPU storage buffer -> staging buffer
    command_encoder.copy_buffer_to_buffer(
        &context.storage_buffer,
        0,
        &context.output_staging_buffer,
        0,
        context.storage_buffer.size(),
    );

    // Finalize the command encoder and submit it to the GPU.
    context.queue.submit(Some(command_encoder.finish()));
    log::info!("Submitted commands.");

    // Get a buffer slice that represents the entire buffer.
    let buffer_slice = context.output_staging_buffer.slice(..);

    // WebGPU, for safety reasons, only allows either the GPU
    // or CPU to access a buffer's contents at a time. We need to "map" the buffer which means
    // flipping ownership of the buffer over to the CPU and making access legal. We do this
    // with `BufferSlice::map_async`.
    //
    // The problem is that map_async is not an async function so we can't await it. What
    // we need to do instead is pass in a closure that will be executed when the slice is
    // either mapped or the mapping has failed.
    //
    // The problem with this is that we don't have a reliable way to wait in the main
    // code for the buffer to be mapped and even worse, calling get_mapped_range or
    // get_mapped_range_mut prematurely will cause a panic, not return an error.
    //
    // Using channels solves this as awaiting the receiving of a message from
    // the passed closure will force the outside code to wait. It also doesn't hurt
    // if the closure finishes before the outside code catches up as the message is
    // buffered and receiving will just pick that up.
    //
    // It may also be worth noting that although on native, the usage of asynchronous
    // channels is wholly unnecessary, for the sake of portability to WASM (std channels
    // don't work on WASM,) we'll use async channels that work on both native and WASM.
    let (sender, receiver) = flume::bounded(1);
    buffer_slice.map_async(wgpu::MapMode::Read, move |r| sender.send(r).unwrap());
    // In order for the mapping to be completed, one of three things must happen.
    // One of those can be calling `Device::poll`. This isn't necessary on the web as devices
    // are polled automatically but natively, we need to make sure this happens manually.
    // `Maintain::Wait` will cause the thread to wait on native but not on WebGpu.
    context.device.poll(wgpu::Maintain::Wait).panic_on_timeout();
    log::info!("Device polled.");
    // Now we await the receiving and panic if anything went wrong because we're lazy.
    receiver.recv_async().await.unwrap().unwrap();
    log::info!("Result received.");
    // NOW we can call get_mapped_range.
    {
        let view = buffer_slice.get_mapped_range();
        host_buf.copy_from_slice(bytemuck::cast_slice(&view));
    }
    log::info!("Results written to local buffer.");
    // We need to make sure all `BufferView`'s are dropped before we do what we're about
    // to do.
    // Unmap so that we can copy to the staging buffer in the next iteration.
    context.output_staging_buffer.unmap();
}
