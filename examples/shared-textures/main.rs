use std::collections::HashMap;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowId},
};

const IMAGE_SIZE: u32 = 512;
fn image() -> Vec<u8> {
    let png_bytes: &[u8] = &include_bytes!("images/wgpu.png")[..];
    let png = std::io::Cursor::new(png_bytes);
    let decoder = png::Decoder::new(png);
    let (info, mut reader) = decoder.read_info().expect("can read info");
    let mut buf = vec![0; info.buffer_size()];
    reader.next_frame(&mut buf).expect("can read png frame");
    buf
}

struct Viewport {
    #[used]
    window: Window,
    surface: wgpu::Surface,
    sc_desc: wgpu::SwapChainDescriptor,
    swap_chain: wgpu::SwapChain,
}

impl Viewport {
    fn new(
        window: Window,
        device: &wgpu::Device,
        swapchain_format: wgpu::TextureFormat,
        instance: &wgpu::Instance,
    ) -> Self {
        let surface = unsafe { instance.create_surface(&window) };
        Self::with_surface(window, device, swapchain_format, surface)
    }
    fn with_surface(
        window: Window,
        device: &wgpu::Device,
        swapchain_format: wgpu::TextureFormat,
        surface: wgpu::Surface,
    ) -> Self {
        let size = window.inner_size();

        let sc_desc = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
            format: swapchain_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
        };

        let swap_chain = device.create_swap_chain(&surface, &sc_desc);

        Viewport {
            window,
            surface,
            sc_desc,
            swap_chain,
        }
    }
    fn resize(&mut self, device: &wgpu::Device, size: winit::dpi::PhysicalSize<u32>) {
        self.sc_desc.width = size.width;
        self.sc_desc.height = size.height;
        self.swap_chain = device.create_swap_chain(&self.surface, &self.sc_desc);
    }
    fn get_current_frame(&mut self) -> wgpu::SwapChainTexture {
        self.swap_chain
            .get_current_frame()
            .expect("Failed to acquire next swap chain texture")
            .output
    }
}

async fn run(
    event_loop: EventLoop<()>,
    mut windows: Vec<Window>,
    swapchain_format: wgpu::TextureFormat,
) {
    let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);
    let first_window = windows.remove(0);
    let first_surface = unsafe { instance.create_surface(&first_window) };
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::Default,
            // Request an adapter which can render to our surface
            compatible_surface: Some(&first_surface),
        })
        .await
        .expect("Failed to find an appropiate adapter");

    // Create the logical device and command queue
    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                features: wgpu::Features::empty(),
                limits: wgpu::Limits::default(),
                shader_validation: true,
            },
            None,
        )
        .await
        .expect("Failed to create device");

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStage::FRAGMENT,
                ty: wgpu::BindingType::SampledTexture {
                    component_type: wgpu::TextureComponentType::Float,
                    multisampled: false,
                    dimension: wgpu::TextureViewDimension::D2,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStage::FRAGMENT,
                ty: wgpu::BindingType::Sampler { comparison: false },
                count: None,
            },
        ],
    });

    // Load the shaders from disk
    let vs_module = device.create_shader_module(wgpu::include_spirv!("shader.vert.spv"));
    let fs_module = device.create_shader_module(wgpu::include_spirv!("shader.frag.spv"));

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
        layout: Some(&pipeline_layout),
        vertex_stage: wgpu::ProgrammableStageDescriptor {
            module: &vs_module,
            entry_point: "main",
        },
        fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
            module: &fs_module,
            entry_point: "main",
        }),
        // Use the default rasterizer state: no culling, no depth bias
        rasterization_state: None,
        primitive_topology: wgpu::PrimitiveTopology::TriangleStrip,
        color_states: &[swapchain_format.into()],
        depth_stencil_state: None,
        vertex_state: wgpu::VertexStateDescriptor {
            index_format: wgpu::IndexFormat::Uint16,
            vertex_buffers: &[],
        },
        sample_count: 1,
        sample_mask: !0,
        alpha_to_coverage_enabled: false,
    });

    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: None,
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });

    let texture = device.create_texture(&wgpu::TextureDescriptor {
        size: wgpu::Extent3d {
            width: IMAGE_SIZE,
            height: IMAGE_SIZE,
            depth: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::COPY_DST,
        label: None,
    });

    {
        let image = image();
        queue.write_texture(
            wgpu::TextureCopyView {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            &image,
            wgpu::TextureDataLayout {
                offset: 0,
                bytes_per_row: 4 * IMAGE_SIZE,
                rows_per_image: 0,
            },
            wgpu::Extent3d {
                width: IMAGE_SIZE,
                height: IMAGE_SIZE,
                depth: 1,
            },
        );
    }
    let texture_view = texture.create_view(&Default::default());

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&texture_view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
        ],
        label: None,
    });

    let mut viewports: HashMap<WindowId, Viewport> = windows
        .into_iter()
        .map(|window| {
            (
                window.id(),
                Viewport::new(window, &device, swapchain_format, &instance),
            )
        })
        .collect();
    viewports.insert(
        first_window.id(),
        Viewport::with_surface(first_window, &device, swapchain_format, first_surface),
    );

    event_loop.run(move |event, _, control_flow| {
        // Have the closure take ownership of the resources.
        // `event_loop.run` never returns, therefore we must do this to ensure
        // the resources are properly cleaned up.
        let _ = (
            &instance,
            &adapter,
            &vs_module,
            &fs_module,
            &pipeline_layout,
        );

        *control_flow = ControlFlow::Wait;
        match event {
            Event::WindowEvent {
                window_id,
                event: WindowEvent::Resized(size),
                ..
            } => {
                // Recreate the swap chain with the new size
                if let Some(viewport) = viewports.get_mut(&window_id) {
                    viewport.resize(&device, size);
                }
            }
            Event::RedrawRequested(window_id) => {
                if let Some(viewport) = viewports.get_mut(&window_id) {
                    let frame = viewport.get_current_frame();
                    let mut encoder = device
                        .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
                    {
                        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                            color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                                attachment: &frame.view,
                                resolve_target: None,
                                ops: wgpu::Operations {
                                    load: wgpu::LoadOp::Clear(wgpu::Color::GREEN),
                                    store: true,
                                },
                            }],
                            depth_stencil_attachment: None,
                        });
                        rpass.set_pipeline(&render_pipeline);
                        rpass.set_bind_group(0, &bind_group, &[]);
                        rpass.draw(0..4, 0..1);
                    }

                    queue.submit(Some(encoder.finish()));
                }
            }
            Event::WindowEvent {
                window_id,
                event: WindowEvent::CloseRequested,
                ..
            } => {
                viewports.remove(&window_id);
                if viewports.is_empty() {
                    *control_flow = ControlFlow::Exit
                }
            }
            _ => {}
        }
    });
}

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    {
        const WINDOW_SIZE: u32 = 128;
        const WINDOW_PADDING: u32 = 16;
        const WINDOW_TITLEBAR: u32 = 32;
        const WINDOW_OFFSET: u32 = WINDOW_SIZE + WINDOW_PADDING;
        let event_loop = EventLoop::new();
        let windows: Vec<_> = (0..16)
            .map(|i| {
                let window = winit::window::WindowBuilder::new()
                    .with_title(format!("Window #{}", i))
                    .with_inner_size(winit::dpi::PhysicalSize::new(WINDOW_SIZE, WINDOW_SIZE))
                    .build(&event_loop)
                    .unwrap();
                window.set_outer_position(winit::dpi::PhysicalPosition::new(
                    WINDOW_PADDING + (i % 4) * WINDOW_OFFSET,
                    WINDOW_PADDING + (i / 4) * (WINDOW_OFFSET + WINDOW_TITLEBAR),
                ));
                window
            })
            .collect();

        subscriber::initialize_default_subscriber(None);
        // Temporarily avoid srgb formats for the swapchain on the web
        futures::executor::block_on(run(
            event_loop,
            windows,
            wgpu::TextureFormat::Bgra8UnormSrgb,
        ));
    }
    #[cfg(target_arch = "wasm32")]
    {
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
        panic!("wasm32 is not supported")
    }
}
