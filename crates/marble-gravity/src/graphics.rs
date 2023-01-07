use crate::spheretree::Sphere;
use cgmath::{prelude::*, Matrix3, Matrix4, Quaternion, Vector2, Vector3};
use instant::Instant;
use physics::BODIES;
use std::{
    collections::VecDeque,
    mem,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};
use wgpu::util::DeviceExt;

const FRAME_TIME_HISTORY_COUNT: usize = 30;

#[repr(C)]
#[derive(Copy, Clone)]
struct Uniforms {
    sun_direction: Vector3<f32>,
    ray_splits: u32,
    pub(self) window_size: Vector2<f32>,
    _padding2: [u32; 2],
    pub(self) view_to_world_space: Matrix4<f32>,
}
impl Uniforms {
    pub fn new() -> Self {
        Self {
            sun_direction: Vector3::unit_x(),
            window_size: Vector2::zero(),
            ray_splits: 4,
            _padding2: [0; 2],
            view_to_world_space: Matrix4::one(),
        }
    }
}
unsafe impl bytemuck::Pod for Uniforms {}
unsafe impl bytemuck::Zeroable for Uniforms {}

pub struct Parameters {
    pub texture_format: wgpu::TextureFormat,
    pub present_mode: wgpu::PresentMode,
}

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
    staging_belt: wgpu::util::StagingBelt,
    glyph_brush: wgpu_glyph::GlyphBrush<()>,
    window_size: (u32, u32),
    this_frame_render_time_nanos_or_zero: Arc<AtomicU64>,
    fps_latest_instant: Instant,
    fps_recent_frame_and_render_time: VecDeque<[Duration; 2]>,
    fps_display: f32,
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
            size: ((2 * BODIES - 1) * mem::size_of::<Sphere>()) as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let uniforms_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Uniforms buffer"),
            size: mem::size_of::<Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let (skybox_texture_view, skybox_sampler) =
            make_skybox_texture_view_and_sampler(&device, &queue);
        let render_tasks = make_render_tasks(
            &parameters,
            &device,
            &body_buffer,
            &uniforms_buffer,
            &skybox_texture_view,
            &skybox_sampler,
        );

        let font = wgpu_glyph::ab_glyph::FontArc::try_from_slice(include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../assets/Roboto-Regular-Digits.ttf"
        )))
        .unwrap();
        let glyph_brush = wgpu_glyph::GlyphBrushBuilder::using_font(font)
            .build(&device, parameters.texture_format);

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
            staging_belt: wgpu::util::StagingBelt::new(1024),
            glyph_brush,
            window_size: size,
            this_frame_render_time_nanos_or_zero: Arc::new(AtomicU64::new(10_000_000)),
            fps_latest_instant: Instant::now(),
            fps_recent_frame_and_render_time: std::iter::once([Duration::from_millis(10); 2])
                .collect(),
            fps_display: 100.0,
        }
    }
    pub fn change_ray_splits(&mut self, delta: i8) {
        match delta {
            1 if self.uniforms.ray_splits < 4 => {
                self.uniforms.ray_splits += 1;
                log::info!("Incremented to ray_splits={}", self.uniforms.ray_splits);
            }
            -1 if self.uniforms.ray_splits > 0 => {
                self.uniforms.ray_splits -= 1;
                log::info!("Decremented to ray_splits={}", self.uniforms.ray_splits);
            }
            -1 | 1 => {}
            other => unreachable!("{}", other),
        }
        self.uniforms_are_new = true;
    }
    #[cfg(target_arch = "wasm32")]
    pub fn window_size(&self) -> (u32, u32) {
        self.window_size
    }
    pub fn resize(&mut self, (w, h): (u32, u32)) {
        self.window_size = (w, h);
        self.uniforms.window_size = Vector2::from((w as f32, h as f32));
        self.uniforms_are_new = true;
        configure_surface(
            &self.parameters,
            &self.device,
            &self.surface,
            self.window_size,
        );
    }
    pub fn get_recent_avg_frame_and_render_time(&self) -> [Duration; 2] {
        self.device.poll(wgpu::MaintainBase::Poll);
        let [f, r] = self
            .fps_recent_frame_and_render_time
            .iter()
            .copied()
            .reduce(|[f1, r1], [f2, r2]| [f1 + f2, r1 + r2])
            .unwrap();
        let n = self.fps_recent_frame_and_render_time.len() as u32;
        [
            (f + Instant::now().duration_since(self.fps_latest_instant)) / n,
            r / n,
        ]
    }
    pub fn render(
        &mut self,
        bodies: Vec<Sphere>,
        rotation: Quaternion<f32>,
        update_fps_display: bool,
    ) {
        let now_pre_render = Instant::now();
        // Copy state to GPU
        {
            self.queue
                .write_buffer(&self.body_buffer, 0, bytemuck::cast_slice(&bodies));
            let sun_direction = rotation.conjugate().rotate_vector(Vector3::unit_x());
            let view_to_world_space = Matrix4::from(Matrix3::from_cols(
                rotation.rotate_vector(Vector3::unit_x()),
                rotation.rotate_vector(Vector3::unit_y()),
                rotation.rotate_vector(Vector3::unit_z()),
            ));
            if sun_direction != self.uniforms.sun_direction
                || view_to_world_space != self.uniforms.view_to_world_space
            {
                self.uniforms_are_new = true;
                self.uniforms.sun_direction = sun_direction;
                self.uniforms.view_to_world_space = view_to_world_space;
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
        let render_time = {
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

            self.glyph_brush.queue(wgpu_glyph::Section {
                screen_position: (5.0, 5.0),
                bounds: (self.window_size.0 as f32, self.window_size.1 as f32),
                text: vec![wgpu_glyph::Text::new({
                    let fps = self.fps_display;
                    let precision = (2 - fps.log10().ceil() as isize).max(0) as usize;
                    &format!("{fps:.precision$}")
                })
                .with_color([0.5, 0.5, 0.5, 1.0])
                .with_scale(32.0)],
                layout: wgpu_glyph::Layout::default_single_line(),
            });
            self.glyph_brush
                .draw_queued(
                    &self.device,
                    &mut self.staging_belt,
                    &mut encoder,
                    surface_texture_view,
                    self.window_size.0,
                    self.window_size.1,
                )
                .unwrap();
            self.staging_belt.finish();

            let render_time = loop {
                let render_time = self
                    .this_frame_render_time_nanos_or_zero
                    .load(Ordering::SeqCst);
                if render_time == 0 {
                    self.device.poll(wgpu::Maintain::Wait);
                    continue;
                }
                self.this_frame_render_time_nanos_or_zero
                    .compare_exchange(render_time, 0, Ordering::SeqCst, Ordering::SeqCst)
                    .unwrap();
                break render_time;
            };
            self.queue.submit(std::iter::once(encoder.finish()));
            self.queue.on_submitted_work_done({
                let slot = self.this_frame_render_time_nanos_or_zero.clone();
                move || {
                    let render_time: u64 = Instant::now()
                        .checked_duration_since(now_pre_render)
                        .unwrap()
                        .as_nanos()
                        .try_into()
                        .unwrap();
                    slot.compare_exchange(0, render_time, Ordering::SeqCst, Ordering::SeqCst)
                        .unwrap();
                }
            });
            surface_texture.present();
            self.staging_belt.recall();
            render_time
        };
        {
            let now = Instant::now();
            let frame_time = now.duration_since(self.fps_latest_instant);
            self.fps_latest_instant = now;

            while self.fps_recent_frame_and_render_time.len() > FRAME_TIME_HISTORY_COUNT {
                self.fps_recent_frame_and_render_time.pop_front();
            }
            self.fps_recent_frame_and_render_time
                .push_back([frame_time, Duration::from_nanos(render_time)]);

            if update_fps_display {
                self.fps_display = (self.fps_recent_frame_and_render_time.len() as f32)
                    / self
                        .fps_recent_frame_and_render_time
                        .iter()
                        .map(|[f, _]| f.as_secs_f32())
                        .sum::<f32>();
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
    );
}

fn make_skybox_texture_view_and_sampler(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> (wgpu::TextureView, wgpu::Sampler) {
    let texture = {
        let png: [&[u8]; 6] = [
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../assets/skybox/right.png"
            )),
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../assets/skybox/left.png"
            )),
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../assets/skybox/top.png"
            )),
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../assets/skybox/bottom.png"
            )),
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../assets/skybox/front.png"
            )),
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../assets/skybox/back.png"
            )),
        ];
        let images = png.map(|p| image::load_from_memory(p).unwrap().into_rgba8());
        let (width, height) = images[0].dimensions();
        assert_eq!(width, height);
        for im in &images {
            assert_eq!((width, height), im.dimensions());
        }
        device.create_texture_with_data(
            queue,
            &wgpu::TextureDescriptor {
                label: Some("skybox texture"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: {
                        assert_eq!(images.len(), 6);
                        6
                    },
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING,
            },
            &images.map(|im| im.into_raw()).concat(),
        )
    };
    let texture_view = texture.create_view(&wgpu::TextureViewDescriptor {
        dimension: Some(wgpu::TextureViewDimension::Cube),
        ..Default::default()
    });
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Nearest,
        mipmap_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });
    (texture_view, sampler)
}

fn make_render_tasks(
    parameters: &Parameters,
    device: &wgpu::Device,
    body_buffer: &wgpu::Buffer,
    uniforms_buffer: &wgpu::Buffer,
    skybox_texture_view: &wgpu::TextureView,
    skybox_sampler: &wgpu::Sampler,
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
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::TextureView(skybox_texture_view),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: wgpu::BindingResource::Sampler(skybox_sampler),
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
                    ty: wgpu::BufferBindingType::Uniform,
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
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::Cube,
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                },
                count: None, // See above
            },
            wgpu::BindGroupLayoutEntry {
                binding: 3,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
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

    let vertex_module = device.create_shader_module(wgpu::include_wgsl!(concat!(
        env!("OUT_DIR"),
        "/shader.vert.wgsl"
    )));
    let fragment_module = device.create_shader_module(wgpu::include_wgsl!(concat!(
        env!("OUT_DIR"),
        "/shader.frag.wgsl"
    )));

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
