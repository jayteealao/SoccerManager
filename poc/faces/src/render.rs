//! Off-screen wgpu renderer. Draws a list of meshes into a 512x512 colour
//! image and a matching depth image (near = white, background = black).
//! Works on a software adapter (Mesa lavapipe) as well as on a GPU.

use bytemuck::{Pod, Zeroable};
use glam::Vec3;
use wgpu::util::DeviceExt;

pub const SIZE: u32 = 512;
const SAMPLES: u32 = 4;
const COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const DEPTH_OUT_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Debug, Default)]
pub struct Vertex {
    pub pos: [f32; 3],
    pub nrm: [f32; 3],
    pub uv: [f32; 2],
    /// Per-kind extra data (skin: brow, lip, beard, scalp masks; hair: tangent).
    pub aux: [f32; 4],
    /// Skin: jersey mask, crow's-feet mask, unused, unused.
    pub aux2: [f32; 4],
}

/// Shader kinds; must match `shader.wgsl`.
pub mod kind {
    pub const SKIN: u32 = 0;
    pub const EYE: u32 = 1;
    pub const HAIR_CARD: u32 = 2;
    pub const HAIR_SHELL: u32 = 3;
    pub const HAIR_TUBE: u32 = 4;
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Debug, Default)]
pub struct DrawParams {
    pub kind: u32,
    pub _pad: [u32; 3],
    /// Skin albedo, or hair root colour (linear RGB).
    pub colour: [f32; 4],
    /// Iris colour, or hair tip colour, or brow/stubble colour for skin.
    pub colour2: [f32; 4],
    /// Skin: skin_age, stubble, brow density, lip tint.
    /// Hair: alpha cutoff, shell height, shell max length, coil radius.
    pub p0: [f32; 4],
    /// Eye centre (xyz), or hair: coil frequency, cell size, thickness, seed.
    pub p1: [f32; 4],
}

pub struct Draw {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub params: DrawParams,
    /// RGBA8 alpha texture for hair cards (width, height, pixels).
    pub texture: Option<(u32, u32, Vec<u8>)>,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Globals {
    view_proj: [[f32; 4]; 4],
    view: [[f32; 4]; 4],
    cam_pos: [f32; 4],
    /// x = near view depth of the head, y = far view depth.
    depth_range: [f32; 4],
}

pub struct Camera {
    pub eye: Vec3,
    pub target: Vec3,
    pub fov_y_deg: f32,
}

impl Camera {
    /// Portrait framing for the FLAME head (PoC 7), which is smaller than
    /// MakeHuman's.
    pub fn portrait_flame(yaw_deg: f32) -> Self {
        let target = Vec3::new(0.0, 6.95, 0.6);
        let dist = 12.0;
        let yaw = yaw_deg.to_radians();
        let eye = target + Vec3::new(yaw.sin() * dist, 0.5, yaw.cos() * dist);
        Self { eye, target, fov_y_deg: 14.0 }
    }

    /// Closer three-quarter view for judging hair.
    pub fn hair_closeup(yaw_deg: f32) -> Self {
        let target = Vec3::new(0.0, 7.55, 0.45);
        let yaw = yaw_deg.to_radians();
        let eye = target + Vec3::new(yaw.sin() * 11.0, 1.2, yaw.cos() * 11.0);
        Self { eye, target, fov_y_deg: 14.5 }
    }

    /// A portrait camera for the MakeHuman head (decimetre units).
    pub fn portrait(yaw_deg: f32) -> Self {
        let target = Vec3::new(0.0, 7.08, 0.55);
        let dist = 14.0;
        let yaw = yaw_deg.to_radians();
        let eye = target + Vec3::new(yaw.sin() * dist, 0.6, yaw.cos() * dist);
        Self { eye, target, fov_y_deg: 14.0 }
    }
}

pub struct Frame {
    pub colour: Vec<u8>,
    pub depth: Vec<u8>,
}

pub struct Renderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    opaque: wgpu::RenderPipeline,
    hair: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    pub adapter_info: String,
}

impl Renderer {
    pub fn new() -> Self {
        pollster::block_on(Self::new_async())
    }

    async fn new_async() -> Self {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: None,
                apply_limit_buckets: false,
            })
            .await
            .expect("no wgpu adapter; on Linux without a GPU install mesa-vulkan-drivers (lavapipe)");
        let info = adapter.get_info();
        let adapter_info = format!("{} ({:?}, {})", info.name, info.backend, info.driver);
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .expect("request device");

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("faces"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let make = |entry: &str, a2c: bool| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(entry),
                layout: Some(&pl),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &[Some(wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<Vertex>() as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x2, 3 => Float32x4, 4 => Float32x4],
                    })],
                },
                primitive: wgpu::PrimitiveState { cull_mode: None, ..Default::default() },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: Some(true),
                    depth_compare: Some(wgpu::CompareFunction::Less),
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: SAMPLES,
                    mask: !0,
                    alpha_to_coverage_enabled: a2c,
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(entry),
                    compilation_options: Default::default(),
                    targets: &[
                        Some(wgpu::ColorTargetState {
                            format: COLOR_FORMAT,
                            blend: None,
                            write_mask: wgpu::ColorWrites::ALL,
                        }),
                        Some(wgpu::ColorTargetState {
                            format: DEPTH_OUT_FORMAT,
                            blend: None,
                            write_mask: wgpu::ColorWrites::ALL,
                        }),
                    ],
                }),
                multiview_mask: None,
                cache: None,
            })
        };
        let opaque = make("fs_opaque", false);
        let hair = make("fs_hair", true);
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });
        Self { device, queue, opaque, hair, layout, sampler, adapter_info }
    }

    pub fn render(&self, draws: &[Draw], cam: &Camera) -> Frame {
        let d = &self.device;
        let size = wgpu::Extent3d { width: SIZE, height: SIZE, depth_or_array_layers: 1 };
        let tex = |format, samples, usage| {
            d.create_texture(&wgpu::TextureDescriptor {
                label: None,
                size,
                mip_level_count: 1,
                sample_count: samples,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage,
                view_formats: &[],
            })
        };
        let att = wgpu::TextureUsages::RENDER_ATTACHMENT;
        let out = att | wgpu::TextureUsages::COPY_SRC;
        let msaa_c = tex(COLOR_FORMAT, SAMPLES, att);
        let msaa_d = tex(DEPTH_OUT_FORMAT, SAMPLES, att);
        let res_c = tex(COLOR_FORMAT, 1, out);
        let res_d = tex(DEPTH_OUT_FORMAT, 1, out);
        let zbuf = tex(wgpu::TextureFormat::Depth32Float, SAMPLES, att);
        let v = |t: &wgpu::Texture| t.create_view(&Default::default());
        let (msaa_cv, msaa_dv, res_cv, res_dv, zv) = (v(&msaa_c), v(&msaa_d), v(&res_c), v(&res_d), v(&zbuf));

        let view = glam::camera::rh::view::look_at_mat4(cam.eye, cam.target, Vec3::Y);
        let proj = glam::camera::rh::proj::directx::perspective(cam.fov_y_deg.to_radians(), 1.0, 1.0, 40.0);
        let dist = (cam.eye - cam.target).length();
        let globals = Globals {
            view_proj: (proj * view).to_cols_array_2d(),
            view: view.to_cols_array_2d(),
            cam_pos: cam.eye.extend(1.0).to_array(),
            depth_range: [dist - 2.2, dist + 1.6, 0.0, 0.0],
        };
        let gbuf = d.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::bytes_of(&globals),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        let white = (1, 1, vec![255u8; 4]);
        struct Prepared {
            vb: wgpu::Buffer,
            ib: wgpu::Buffer,
            n: u32,
            bg: wgpu::BindGroup,
            hair: bool,
        }
        let prepared: Vec<Prepared> = draws
            .iter()
            .filter(|dr| !dr.indices.is_empty())
            .map(|dr| {
                let (tw, th, px) = dr.texture.as_ref().unwrap_or(&white);
                let t = d.create_texture_with_data(
                    &self.queue,
                    &wgpu::TextureDescriptor {
                        label: None,
                        size: wgpu::Extent3d { width: *tw, height: *th, depth_or_array_layers: 1 },
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: wgpu::TextureDimension::D2,
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        usage: wgpu::TextureUsages::TEXTURE_BINDING,
                        view_formats: &[],
                    },
                    wgpu::util::TextureDataOrder::LayerMajor,
                    px,
                );
                let pbuf = d.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: None,
                    contents: bytemuck::bytes_of(&dr.params),
                    usage: wgpu::BufferUsages::UNIFORM,
                });
                let tv = t.create_view(&Default::default());
                let bg = d.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: None,
                    layout: &self.layout,
                    entries: &[
                        wgpu::BindGroupEntry { binding: 0, resource: gbuf.as_entire_binding() },
                        wgpu::BindGroupEntry { binding: 1, resource: pbuf.as_entire_binding() },
                        wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&tv) },
                        wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::Sampler(&self.sampler) },
                    ],
                });
                Prepared {
                    vb: d.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: None,
                        contents: bytemuck::cast_slice(&dr.vertices),
                        usage: wgpu::BufferUsages::VERTEX,
                    }),
                    ib: d.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: None,
                        contents: bytemuck::cast_slice(&dr.indices),
                        usage: wgpu::BufferUsages::INDEX,
                    }),
                    n: dr.indices.len() as u32,
                    bg,
                    hair: dr.params.kind == kind::HAIR_CARD || dr.params.kind == kind::HAIR_SHELL,
                }
            })
            .collect();

        let mut enc = d.create_command_encoder(&Default::default());
        {
            let bgc = wgpu::Color { r: 0.16, g: 0.18, b: 0.21, a: 1.0 };
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[
                    Some(wgpu::RenderPassColorAttachment {
                        view: &msaa_cv,
                        depth_slice: None,
                        resolve_target: Some(&res_cv),
                        ops: wgpu::Operations { load: wgpu::LoadOp::Clear(bgc), store: wgpu::StoreOp::Discard },
                    }),
                    Some(wgpu::RenderPassColorAttachment {
                        view: &msaa_dv,
                        depth_slice: None,
                        resolve_target: Some(&res_dv),
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Discard,
                        },
                    }),
                ],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &zv,
                    depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Discard }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            // Opaque first, then alpha-to-coverage hair.
            for hair in [false, true] {
                pass.set_pipeline(if hair { &self.hair } else { &self.opaque });
                for p in prepared.iter().filter(|p| p.hair == hair) {
                    pass.set_bind_group(0, &p.bg, &[]);
                    pass.set_vertex_buffer(0, p.vb.slice(..));
                    pass.set_index_buffer(p.ib.slice(..), wgpu::IndexFormat::Uint32);
                    pass.draw_indexed(0..p.n, 0, 0..1);
                }
            }
        }
        let colour = self.read(&mut enc, &res_c);
        let depth = self.read(&mut enc, &res_d);
        self.queue.submit([enc.finish()]);
        let colour = self.finish_read(colour);
        let depth = self.finish_read(depth);
        // Depth PNG is grey: keep one channel replicated.
        Frame { colour, depth }
    }

    fn read(&self, enc: &mut wgpu::CommandEncoder, t: &wgpu::Texture) -> wgpu::Buffer {
        let buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: (SIZE * SIZE * 4) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        enc.copy_texture_to_buffer(
            t.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &buf,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(SIZE * 4),
                    rows_per_image: None,
                },
            },
            wgpu::Extent3d { width: SIZE, height: SIZE, depth_or_array_layers: 1 },
        );
        buf
    }

    fn finish_read(&self, buf: wgpu::Buffer) -> Vec<u8> {
        let slice = buf.slice(..);
        slice.map_async(wgpu::MapMode::Read, |r| r.expect("map"));
        self.device.poll(wgpu::PollType::wait_indefinitely()).expect("poll");
        let data = slice.get_mapped_range().expect("mapped range").to_vec();
        drop(buf);
        data
    }
}

impl Default for Renderer {
    fn default() -> Self {
        Self::new()
    }
}
