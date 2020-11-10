use crate::{physics::BODIES, spheretree::Sphere};
use anyhow::*;
use cgmath::Vector2;
use log::info;
use std::mem;
use wgpu::*;
use winit::{dpi::PhysicalSize, window::Window};

const TEXTURE_FORMAT: TextureFormat = TextureFormat::Bgra8UnormSrgb;

#[repr(C)]
#[derive(Copy, Clone)]
struct Uniforms {
    window_size: Vector2<f32>,
}
impl Uniforms {
    pub fn new((width, height): (u32, u32)) -> Self {
        Self {
            window_size: Vector2::new(width as f32, height as f32),
        }
    }
}
unsafe impl bytemuck::Pod for Uniforms {}
unsafe impl bytemuck::Zeroable for Uniforms {}

// TODO Use push constants instead of uniforms
pub struct Graphics {
    queue: Queue,
    device: Device,
    surface: Surface,
    swap_chain: SwapChain,
    body_buffer: Buffer,
    uniforms_buffer: Buffer,
    uniforms: Uniforms,
    uniforms_are_new: bool,
    render_tasks: RenderBundle,
}
impl Graphics {
    pub async fn initialize(window: &Window) -> Result<Self> {
        let instance = Instance::new(BackendBit::PRIMARY);
        let surface = unsafe { instance.create_surface(window) };
        let adapter = make_adapter(&instance, &surface).await?;
        let (device, queue) = make_device_and_queue(&adapter).await?;

        info!("Found and acquired adapter:\n{:?}", adapter.get_info());

        let size: (u32, u32) = window.inner_size().into();
        let swap_chain = make_swap_chain(&device, &surface, size);
        let uniforms = Uniforms::new(size);

        let body_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Body buffer"),
            size: ((2 * BODIES - 1) * mem::size_of::<Sphere>() as u32) as u64,
            usage: BufferUsage::STORAGE | BufferUsage::COPY_DST,
            // I dont quite understand what this means, but I think it is just a feature that I won't be using
            mapped_at_creation: false,
        });
        let uniforms_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Uniforms buffer"),
            size: mem::size_of::<Uniforms>() as u64,
            usage: BufferUsage::UNIFORM | BufferUsage::COPY_DST,
            mapped_at_creation: false, // See above
        });

        let render_tasks = make_render_tasks(&device, &body_buffer, &uniforms_buffer);

        Ok(Self {
            queue,
            device,
            surface,
            swap_chain,
            body_buffer,
            uniforms_buffer,
            uniforms,
            uniforms_are_new: true,
            render_tasks,
        })
    }
    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        self.uniforms = Uniforms::new(new_size.into());
        self.uniforms_are_new = true;
        self.swap_chain = make_swap_chain(&self.device, &self.surface, new_size.into());
    }
    pub fn render(&mut self, bodies: Vec<Sphere>) {
        // Copy state to GPU
        {
            self.queue
                .write_buffer(&self.body_buffer, 0, bytemuck::cast_slice(&bodies));
            if self.uniforms_are_new {
                self.queue.write_buffer(
                    &self.uniforms_buffer,
                    0,
                    bytemuck::cast_slice(&[self.uniforms]),
                );
                self.uniforms_are_new = false;
            }
        }
        // Render
        {
            let swap_chain_frame = self.swap_chain.get_current_frame().unwrap();
            let mut encoder = self
                .device
                .create_command_encoder(&CommandEncoderDescriptor {
                    label: Some("Command encoder"),
                });
            encoder
                .begin_render_pass(&RenderPassDescriptor {
                    color_attachments: &[RenderPassColorAttachmentDescriptor {
                        attachment: &swap_chain_frame.output.view,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Load,
                            store: true,
                        },
                    }],
                    depth_stencil_attachment: None,
                })
                .execute_bundles(std::iter::once(&self.render_tasks));

            self.queue.submit(std::iter::once(encoder.finish()));
        }
    }
}

fn make_render_tasks(
    device: &Device,
    body_buffer: &Buffer,
    uniforms_buffer: &Buffer,
) -> RenderBundle {
    let mut bundle_encoder = device.create_render_bundle_encoder(&RenderBundleEncoderDescriptor {
        label: Some("Render bundle encoder descriptor"),
        color_formats: &[TEXTURE_FORMAT],
        depth_stencil_format: None,
        sample_count: 1,
    });
    let bind_group_layout = make_bind_group_layout(device);
    let bind_group = device.create_bind_group(&BindGroupDescriptor {
        label: Some("Bind group"),
        layout: &bind_group_layout,
        entries: &[
            BindGroupEntry {
                binding: 0,
                resource: BindingResource::Buffer(body_buffer.slice(..)),
            },
            BindGroupEntry {
                binding: 1,
                resource: BindingResource::Buffer(uniforms_buffer.slice(..)),
            },
        ],
    });
    let pipeline = make_pipeline(device, &bind_group_layout);

    bundle_encoder.set_pipeline(&pipeline);
    bundle_encoder.set_bind_group(0, &bind_group, &[]);
    bundle_encoder.draw(0..4, 0..1);
    bundle_encoder.finish(&RenderBundleDescriptor {
        label: Some("Render bundle"),
    })
}

fn make_bind_group_layout(device: &Device) -> BindGroupLayout {
    device.create_bind_group_layout(&BindGroupLayoutDescriptor {
        label: Some("Body buffer layout"),
        entries: &[
            BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStage::FRAGMENT,
                ty: BindingType::StorageBuffer {
                    dynamic: false,
                    min_binding_size: None, // TODO Revisit when I understand
                    readonly: true,
                },
                count: None, // Only applicable to sampled textures
            },
            BindGroupLayoutEntry {
                binding: 1,
                visibility: ShaderStage::FRAGMENT,
                ty: BindingType::UniformBuffer {
                    dynamic: false,
                    min_binding_size: None, // TODO See above
                },
                count: None, // See above
            },
        ],
    })
}

fn make_pipeline(device: &Device, bind_group_layout: &BindGroupLayout) -> RenderPipeline {
    // All uniforms reside in the same bind group (since nothing is ever swapped out).
    let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: Some("Pipeline layout"),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    let vertex_module = device.create_shader_module(include_spirv!("../target/shader.vert.spv"));
    let fragment_module = device.create_shader_module(include_spirv!("../target/shader.frag.spv"));

    device.create_render_pipeline(&RenderPipelineDescriptor {
        label: Some("Render pipeline"),
        layout: Some(&pipeline_layout),
        vertex_stage: ProgrammableStageDescriptor {
            module: &vertex_module,
            entry_point: "main",
        },
        fragment_stage: Some(ProgrammableStageDescriptor {
            module: &fragment_module,
            entry_point: "main",
        }),
        rasterization_state: None, // Default I guess?
        // Cover the viewport with 4 points hardcoded in the vertex shader
        primitive_topology: PrimitiveTopology::TriangleStrip,
        color_states: &[ColorStateDescriptor {
            format: TEXTURE_FORMAT,
            alpha_blend: BlendDescriptor::REPLACE,
            color_blend: BlendDescriptor::REPLACE,
            write_mask: ColorWrite::ALL,
        }],
        depth_stencil_state: None,
        vertex_state: VertexStateDescriptor {
            index_format: IndexFormat::Uint16,
            vertex_buffers: &[],
        },
        sample_count: 1,
        sample_mask: !0,
        alpha_to_coverage_enabled: false,
    })
}

fn make_swap_chain(device: &Device, surface: &Surface, (width, height): (u32, u32)) -> SwapChain {
    device.create_swap_chain(
        surface,
        &SwapChainDescriptor {
            usage: TextureUsage::OUTPUT_ATTACHMENT,
            format: TEXTURE_FORMAT,
            width,
            height,
            present_mode: PresentMode::Fifo,
        },
    )
}

async fn make_adapter(instance: &Instance, surface: &Surface) -> Result<Adapter> {
    instance
        .request_adapter(&RequestAdapterOptionsBase {
            power_preference: PowerPreference::default(),
            compatible_surface: Some(&surface),
        })
        .await
        .context("Failed to acquire adapter")
}

async fn make_device_and_queue(adapter: &Adapter) -> Result<(Device, Queue)> {
    adapter
        .request_device(
            &DeviceDescriptor {
                features: Features::empty(),
                limits: Limits::default(),
                shader_validation: true,
            },
            None, // Trace path
        )
        .await
        .context("Failed to acquire device")
}
