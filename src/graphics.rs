use crate::{physics::BODIES, spheretree::Sphere};
use cgmath::{prelude::*, Quaternion, Vector2, Vector3};
use std::{
    mem,
    time::{Duration, Instant},
};
use winit::dpi::PhysicalSize;

#[repr(C)]
#[derive(Copy, Clone)]
struct Uniforms {
    pub(self) source_direction: Vector3<f32>,
    _padding: u32,
    pub(self) window_size: Vector2<f32>,
}
impl Uniforms {
    pub fn new() -> Self {
        Self {
            source_direction: Vector3::zero(),
            _padding: 0,
            window_size: Vector2::zero(),
        }
    }
}
unsafe impl bytemuck::Pod for Uniforms {}
unsafe impl bytemuck::Zeroable for Uniforms {}

pub struct Parameters {
    pub texture_format: wgpu::TextureFormat,
    pub present_mode: wgpu::PresentMode,
}

// TODO Use push constants instead of uniforms
pub struct Graphics {
    parameters: Parameters,
    queue: wgpu::Queue,
    device: wgpu::Device,
    surface: wgpu::Surface,
    body_buffer: wgpu::Buffer,
    uniforms_buffer: wgpu::Buffer,
    uniforms: Uniforms,
    uniforms_are_new: bool,
    render_tasks: wgpu::RenderBundle,
    window_size: (u32, u32),
    fps_latest_print: Instant,
    fps_frame_count: u32,
}
impl Graphics {
    pub async fn initialize(
        parameters: Parameters,
        surface: wgpu::Surface,
        device_and_queue: (wgpu::Device, wgpu::Queue),
        size: (u32, u32),
    ) -> Self {
        let (device, queue) = device_and_queue;

        let mut uniforms = Uniforms::new();
        uniforms.window_size = Vector2::from(size).cast().unwrap();
        configure_surface(&parameters, &device, &surface, size);

        let body_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Body buffer"),
            size: ((2 * BODIES - 1) * mem::size_of::<Sphere>() as u32) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let uniforms_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Uniforms buffer"),
            size: mem::size_of::<Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let render_tasks = make_render_tasks(&parameters, &device, &body_buffer, &uniforms_buffer);

        Self {
            parameters,
            queue,
            device,
            surface,
            body_buffer,
            uniforms_buffer,
            uniforms,
            uniforms_are_new: true,
            render_tasks,
            window_size: size,
            fps_latest_print: Instant::now(),
            fps_frame_count: 0,
        }
    }
    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        let size: (f32, f32) = (new_size.width as f32, new_size.height as f32);
        self.window_size = new_size.into();
        self.uniforms.window_size = Vector2::from(size);
        self.uniforms_are_new = true;
        configure_surface(
            &self.parameters,
            &self.device,
            &self.surface,
            self.window_size,
        );
    }
    pub fn render(&mut self, bodies: Vec<Sphere>, rotation: Quaternion<f32>) {
        // Copy state to GPU
        {
            self.queue
                .write_buffer(&self.body_buffer, 0, bytemuck::cast_slice(&bodies));
            let source_dir = rotation
                .conjugate()
                .rotate_vector(Vector3::new(0.0, -1.0, 0.0));
            if source_dir != self.uniforms.source_direction {
                self.uniforms_are_new = true;
                self.uniforms.source_direction = source_dir;
            }
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
            let surface_texture = self
                .surface
                .get_current_texture()
                .or_else(|error| {
                    log::debug!(
                        "retrying `wgpu::Surface::get_current_texture` once after: {error:?}"
                    );
                    configure_surface(
                        &self.parameters,
                        &self.device,
                        &self.surface,
                        self.window_size,
                    );
                    self.surface.get_current_texture()
                })
                .unwrap();

            let surface_texture_view =
                &surface_texture
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor {
                        label: Some("frame texture view"),
                        format: None,
                        dimension: None,
                        aspect: wgpu::TextureAspect::All,
                        base_mip_level: 0,
                        mip_level_count: None,
                        base_array_layer: 0,
                        array_layer_count: None,
                    });

            let mut encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Command encoder"),
                });
            encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("render pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: surface_texture_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.0,
                                g: 0.0,
                                b: 0.0,
                                a: 1.0,
                            }),
                            store: true,
                        },
                    })],
                    depth_stencil_attachment: None,
                })
                .execute_bundles(std::iter::once(&self.render_tasks));

            self.queue.submit(std::iter::once(encoder.finish()));
            surface_texture.present();
        }
        {
            let now = Instant::now();
            let span = now.duration_since(self.fps_latest_print);
            if span > Duration::from_secs(1) {
                let fps = self.fps_frame_count as f64 / span.as_secs_f64();
                let precision = (2.0 - fps.log10().ceil()).max(0.0) as usize;
                log::info!("{:.1$} FPS", fps, precision);
                self.fps_latest_print = now;
                self.fps_frame_count = 1;
            } else {
                self.fps_frame_count += 1;
            }
        }
    }
}

fn configure_surface(
    parameters: &Parameters,
    device: &wgpu::Device,
    surface: &wgpu::Surface,
    (width, height): (u32, u32),
) {
    surface.configure(
        device,
        &wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: parameters.texture_format,
            width,
            height,
            present_mode: parameters.present_mode,
            alpha_mode: wgpu::CompositeAlphaMode::Opaque,
        },
    )
}

fn make_render_tasks(
    parameters: &Parameters,
    device: &wgpu::Device,
    body_buffer: &wgpu::Buffer,
    uniforms_buffer: &wgpu::Buffer,
) -> wgpu::RenderBundle {
    let mut bundle_encoder =
        device.create_render_bundle_encoder(&wgpu::RenderBundleEncoderDescriptor {
            label: Some("Render bundle encoder descriptor"),
            color_formats: &[Some(parameters.texture_format)],
            depth_stencil: None,
            sample_count: 1,
            multiview: None,
        });
    let bind_group_layout = make_bind_group_layout(device);
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Bind group"),
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: body_buffer,
                    offset: 0,
                    size: None,
                }),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: uniforms_buffer,
                    offset: 0,
                    size: None,
                }),
            },
        ],
    });
    let pipeline = make_pipeline(parameters, device, &bind_group_layout);

    bundle_encoder.set_pipeline(&pipeline);
    bundle_encoder.set_bind_group(0, &bind_group, &[]);
    bundle_encoder.draw(0..4, 0..1);
    bundle_encoder.finish(&wgpu::RenderBundleDescriptor {
        label: Some("Render bundle"),
    })
}

fn make_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Body buffer layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None, // Only applicable to sampled textures
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None, // See above
            },
        ],
    })
}

fn make_pipeline(
    parameters: &Parameters,
    device: &wgpu::Device,
    bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    // All uniforms reside in the same bind group (since nothing is ever swapped out).
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Pipeline layout"),
        bind_group_layouts: &[bind_group_layout],
        push_constant_ranges: &[],
    });

    let vertex_module =
        device.create_shader_module(wgpu::include_wgsl!("../target/shader.vert.wgsl"));
    let fragment_module =
        device.create_shader_module(wgpu::include_wgsl!("../target/shader.frag.wgsl"));

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Render pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &vertex_module,
            entry_point: "main",
            buffers: &[],
        },
        fragment: Some(wgpu::FragmentState {
            module: &fragment_module,
            entry_point: "main",
            targets: &[Some(wgpu::ColorTargetState {
                format: parameters.texture_format,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        // Cover the viewport with 4 points hardcoded in the vertex shader
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleStrip,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            unclipped_depth: false,
            polygon_mode: wgpu::PolygonMode::Fill,
            conservative: false,
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        multiview: None,
    })
}
