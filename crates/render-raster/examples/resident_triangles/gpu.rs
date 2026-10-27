//! The device half: the mesh uploaded once, recorded once as a render bundle, and drawn at a
//! view that is one uniform.
//!
//! Nothing here is shared with raster's own device: the probe prices a lane raster does not
//! have, and borrowing raster's device would put that device's state into the figure.

use std::time::{Duration, Instant};

use raster_gpu::wgpu;
use wgpu::util::DeviceExt;

use crate::mesh::{Mesh, STRIDE};

/// The colour target: straight-alpha RGBA8, which is what the oracle's raster holds.
const COLOUR: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
/// The winding count lives here, eight bits per sample.
const STENCIL: wgpu::TextureFormat = wgpu::TextureFormat::Stencil8;

/// What the device was, for the figure's line.
pub(crate) struct Adapter {
    pub(crate) name: String,
    pub(crate) driver: String,
}

/// What putting the page on the device cost, once.
pub(crate) struct Upload {
    pub(crate) buffers: Duration,
    pub(crate) pipelines: Duration,
    pub(crate) bundle: Duration,
}

/// One zoom step's cost.
#[derive(Clone, Copy)]
pub(crate) struct Step {
    /// The uniform written, the pass encoded and submitted: the host's share.
    pub(crate) host: Duration,
    /// The pass on the device, from its two timestamps.
    pub(crate) device: Duration,
    /// Submission to completion, as the host waits for it.
    pub(crate) wall: Duration,
}

pub(crate) struct Lane {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pub(crate) adapter: Adapter,
    width: u32,
    height: u32,
    uniform: wgpu::Buffer,
    bundle: wgpu::RenderBundle,
    multisampled: wgpu::TextureView,
    stencil: wgpu::TextureView,
    resolved: wgpu::Texture,
    queries: wgpu::QuerySet,
    resolve: wgpu::Buffer,
    stamps: wgpu::Buffer,
    period: f32,
    pub(crate) upload: Upload,
}

/// Why the lane could not be built on this machine.
#[derive(Debug)]
pub(crate) enum Refusal {
    NoAdapter(Vec<String>),
    Device(wgpu::RequestDeviceError),
    Samples { samples: u32, adapter: String },
    Image { width: u32, height: u32, limit: u32 },
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoAdapter(seen) => write!(f, "no adapter matched; saw {seen:?}"),
            Self::Device(error) => write!(f, "the adapter refused a device: {error}"),
            Self::Samples { samples, adapter } => {
                write!(
                    f,
                    "{adapter} offers no {samples}-sample colour and stencil target"
                )
            }
            Self::Image {
                width,
                height,
                limit,
            } => write!(
                f,
                "a {width}x{height} image is past the device's {limit}-texel texture"
            ),
        }
    }
}

fn adapter(wanted: Option<&str>) -> Result<wgpu::Adapter, Refusal> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::VULKAN,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let adapters = pollster::block_on(instance.enumerate_adapters(wgpu::Backends::VULKAN));
    let names: Vec<String> = adapters.iter().map(|a| a.get_info().name).collect();
    adapters
        .into_iter()
        .find(|a| {
            let info = a.get_info();
            match wanted {
                Some(name) => info.name.to_lowercase().contains(&name.to_lowercase()),
                None => info.device_type != wgpu::DeviceType::Cpu,
            }
        })
        .ok_or(Refusal::NoAdapter(names))
}

impl Lane {
    /// Brings a device up on the adapter whose name contains `wanted` (the first hardware
    /// adapter when `None`), uploads `mesh`, and records it.
    pub(crate) fn new(
        wanted: Option<&str>,
        samples: u32,
        (width, height): (u32, u32),
        mesh: &Mesh,
    ) -> Result<Self, Refusal> {
        let adapter = adapter(wanted)?;
        let info = adapter.get_info();
        let offered = |format| {
            adapter
                .get_texture_format_features(format)
                .flags
                .sample_count_supported(samples)
        };
        if !offered(COLOUR) || !offered(STENCIL) {
            return Err(Refusal::Samples {
                samples,
                adapter: info.name,
            });
        }
        let wanted_features = wgpu::Features::TIMESTAMP_QUERY
            | wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES;
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("resident triangles"),
            required_features: adapter.features() & wanted_features,
            required_limits: adapter.limits(),
            ..Default::default()
        }))
        .map_err(Refusal::Device)?;

        let started = Instant::now();
        let vertices = |label, bytes: &[u8]| {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(label),
                // A buffer of no vertices is still a buffer the bundle may bind.
                contents: if bytes.is_empty() {
                    &[0; STRIDE]
                } else {
                    bytes
                },
                usage: wgpu::BufferUsages::VERTEX,
            })
        };
        let fan = vertices("fan", &mesh.fan);
        let curve = vertices("curve", &mesh.curve);
        let cover = vertices("cover", &mesh.cover);
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("view"),
            size: 32,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let limit = device.limits().max_texture_dimension_2d;
        let mut textures = Vec::with_capacity(mesh.images.len());
        for image in &mesh.images {
            if image.width > limit || image.height > limit {
                return Err(Refusal::Image {
                    width: image.width,
                    height: image.height,
                    limit,
                });
            }
            // No mip chain: a minified image is sampled from its full grid, so this prices
            // the step's upper bound and none of §8.9.5's averaging (RFC 0013 §4).
            let texture = device.create_texture_with_data(
                &queue,
                &wgpu::TextureDescriptor {
                    label: Some("image"),
                    size: wgpu::Extent3d {
                        width: image.width,
                        height: image.height,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: COLOUR,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                },
                wgpu::util::TextureDataOrder::LayerMajor,
                &image.data,
            );
            // §8.9.5.3: a magnified image without `/Interpolate` is drawn as its samples.
            let magnify = if image.interpolate {
                wgpu::FilterMode::Linear
            } else {
                wgpu::FilterMode::Nearest
            };
            let filter = device.create_sampler(&wgpu::SamplerDescriptor {
                mag_filter: magnify,
                min_filter: wgpu::FilterMode::Linear,
                ..Default::default()
            });
            textures.push((
                texture.create_view(&wgpu::TextureViewDescriptor::default()),
                filter,
            ));
        }
        queue.submit([]);
        let _ = device.poll(wgpu::PollType::wait_indefinitely());
        let buffers = started.elapsed();

        let started = Instant::now();
        let pipelines = Pipelines::new(&device, samples);
        let buffers_bound = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("view"),
            layout: &pipelines.layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform.as_entire_binding(),
            }],
        });
        let images: Vec<wgpu::BindGroup> = textures
            .iter()
            .map(|(view, sampler)| {
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("image"),
                    layout: &pipelines.textures,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(sampler),
                        },
                    ],
                })
            })
            .collect();
        let pipelines_spent = started.elapsed();

        let started = Instant::now();
        let mut recorder =
            device.create_render_bundle_encoder(&wgpu::RenderBundleEncoderDescriptor {
                label: Some("the page"),
                color_formats: &[Some(COLOUR)],
                depth_stencil: Some(wgpu::RenderBundleDepthStencil {
                    format: STENCIL,
                    depth_read_only: true,
                    stencil_read_only: false,
                }),
                sample_count: samples,
                multiview: None,
            });
        recorder.set_bind_group(0, &buffers_bound, &[]);
        for batch in &mesh.batches {
            if let Some(image) = batch.image {
                recorder.set_pipeline(&pipelines.image);
                recorder.set_bind_group(1, &images[image as usize], &[]);
                recorder.set_vertex_buffer(0, cover.slice(..));
                recorder.draw(batch.cover.clone(), 0..1);
                continue;
            }
            let rule = usize::from(batch.even_odd);
            for (pipeline, buffer, range) in [
                (&pipelines.fan[rule], &fan, &batch.fan),
                (&pipelines.curve[rule], &curve, &batch.curve),
                (&pipelines.cover[rule], &cover, &batch.cover),
            ] {
                if range.is_empty() {
                    continue;
                }
                recorder.set_pipeline(pipeline);
                recorder.set_vertex_buffer(0, buffer.slice(..));
                recorder.draw(range.clone(), 0..1);
            }
        }
        let bundle = recorder.finish(&wgpu::RenderBundleDescriptor {
            label: Some("the page"),
        });
        let bundle_spent = started.elapsed();

        let target = |label, format, usage, samples| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: samples,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage,
                view_formats: &[],
            })
        };
        let attachment = wgpu::TextureUsages::RENDER_ATTACHMENT;
        let multisampled = target("samples", COLOUR, attachment, samples)
            .create_view(&wgpu::TextureViewDescriptor::default());
        let stencil = target("winding", STENCIL, attachment, samples)
            .create_view(&wgpu::TextureViewDescriptor::default());
        let resolved = target(
            "resolved",
            COLOUR,
            attachment | wgpu::TextureUsages::COPY_SRC,
            1,
        );
        let queries = device.create_query_set(&wgpu::QuerySetDescriptor {
            label: Some("pass"),
            ty: wgpu::QueryType::Timestamp,
            count: 2,
        });
        let resolve = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("stamps resolved"),
            size: 16,
            usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let stamps = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("stamps"),
            size: 16,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let period = queue.get_timestamp_period();
        Ok(Self {
            device,
            queue,
            adapter: Adapter {
                name: info.name,
                driver: format!("{} {}", info.driver, info.driver_info),
            },
            width,
            height,
            uniform,
            bundle,
            multisampled,
            stencil,
            resolved,
            queries,
            resolve,
            stamps,
            period,
            upload: Upload {
                buffers,
                pipelines: pipelines_spent,
                bundle: bundle_spent,
            },
        })
    }

    /// Draws the page at `view` — §8.3.2's six numbers, page space to device pixels.
    pub(crate) fn step(&self, view: [f32; 6]) -> Step {
        let started = Instant::now();
        let mut uniform = [0_u8; 32];
        #[expect(clippy::cast_precision_loss)] // a target's size, far under 2^24
        let values = [
            view[0],
            view[1],
            view[2],
            view[3],
            view[4],
            view[5],
            self.width as f32,
            self.height as f32,
        ];
        for (slot, value) in uniform.chunks_exact_mut(4).zip(values) {
            slot.copy_from_slice(&value.to_le_bytes());
        }
        self.queue.write_buffer(&self.uniform, 0, &uniform);
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("step"),
            });
        {
            let resolved = self
                .resolved
                .create_view(&wgpu::TextureViewDescriptor::default());
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("the page"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.multisampled,
                    depth_slice: None,
                    resolve_target: Some(&resolved),
                    ops: wgpu::Operations {
                        // §11.4.7's white backdrop, which the oracle paints under every page.
                        load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                        store: wgpu::StoreOp::Discard,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.stencil,
                    depth_ops: None,
                    stencil_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(0),
                        store: wgpu::StoreOp::Discard,
                    }),
                }),
                timestamp_writes: Some(wgpu::RenderPassTimestampWrites {
                    query_set: &self.queries,
                    beginning_of_pass_write_index: Some(0),
                    end_of_pass_write_index: Some(1),
                }),
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.execute_bundles([&self.bundle]);
        }
        encoder.resolve_query_set(&self.queries, 0..2, &self.resolve, 0);
        encoder.copy_buffer_to_buffer(&self.resolve, 0, &self.stamps, 0, 16);
        self.queue.submit([encoder.finish()]);
        let host = started.elapsed();
        let _ = self.device.poll(wgpu::PollType::wait_indefinitely());
        let wall = started.elapsed().saturating_sub(host);

        self.stamps.slice(..).map_async(wgpu::MapMode::Read, |_| {});
        let _ = self.device.poll(wgpu::PollType::wait_indefinitely());
        let ticks = {
            let bytes = self
                .stamps
                .slice(..)
                .get_mapped_range()
                .expect("the stamps were mapped above");
            let read = |at: usize| {
                let mut word = [0_u8; 8];
                word.copy_from_slice(&bytes[at..at + 8]);
                u64::from_le_bytes(word)
            };
            read(8).saturating_sub(read(0))
        };
        self.stamps.unmap();
        #[expect(clippy::cast_precision_loss)] // a pass's tick count, far under 2^52
        let device = Duration::from_secs_f64(ticks as f64 * f64::from(self.period) * 1e-9);
        Step { host, device, wall }
    }

    /// The last step's resolved pixels, straight-alpha RGBA, tightly packed.
    pub(crate) fn read(&self) -> Vec<u8> {
        let row = self.width * 4;
        let padded = row.div_ceil(256) * 256;
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback"),
            size: u64::from(padded) * u64::from(self.height),
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("read"),
            });
        encoder.copy_texture_to_buffer(
            self.resolved.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded),
                    rows_per_image: None,
                },
            },
            wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit([encoder.finish()]);
        buffer.slice(..).map_async(wgpu::MapMode::Read, |_| {});
        let _ = self.device.poll(wgpu::PollType::wait_indefinitely());
        let mapped = buffer
            .slice(..)
            .get_mapped_range()
            .expect("the readback was mapped above");
        let (row, padded) = (row as usize, padded as usize);
        mapped
            .chunks_exact(padded)
            .flat_map(|line| line[..row].iter().copied())
            .collect()
    }
}

/// The seven pipelines: a fan, a curve and a cover stage under each of §8.5.3.3's two rules,
/// and an image's textured quad.
struct Pipelines {
    layout: wgpu::BindGroupLayout,
    fan: [wgpu::RenderPipeline; 2],
    curve: [wgpu::RenderPipeline; 2],
    cover: [wgpu::RenderPipeline; 2],
    textures: wgpu::BindGroupLayout,
    image: wgpu::RenderPipeline,
}

impl Pipelines {
    fn new(device: &wgpu::Device, samples: u32) -> Self {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("resident triangles"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("view"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let textures = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("image"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        // A path's stages read the view alone; an image's read its texture as well.
        let stage_layout = |groups: &[Option<&wgpu::BindGroupLayout>]| {
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("view"),
                bind_group_layouts: groups,
                immediate_size: 0,
            })
        };
        let path_layout = stage_layout(&[Some(&layout)]);
        let image_layout = stage_layout(&[Some(&layout), Some(&textures)]);
        let attributes = wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Unorm8x4];
        let build = |vertex: &str, fragment: &str, stencil: wgpu::StencilState, write: bool| {
            let pipeline_layout = if write && vertex == "image_vertex" {
                &image_layout
            } else {
                &path_layout
            };
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(fragment),
                layout: Some(pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: Some(vertex),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &[Some(wgpu::VertexBufferLayout {
                        array_stride: STRIDE as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &attributes,
                    })],
                },
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: STENCIL,
                    depth_write_enabled: Some(false),
                    depth_compare: Some(wgpu::CompareFunction::Always),
                    stencil,
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: samples,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                fragment: Some(wgpu::FragmentState {
                    module: &module,
                    entry_point: Some(fragment),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: COLOUR,
                        blend: write.then_some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: if write {
                            wgpu::ColorWrites::ALL
                        } else {
                            wgpu::ColorWrites::empty()
                        },
                    })],
                }),
                multiview_mask: None,
                cache: None,
            })
        };
        // Nonzero counts up through a front face and down through a back face; even-odd
        // flips every bit, so a sample crossed an odd number of times holds 0xff.
        let count = |even_odd: bool| {
            let (front, back) = if even_odd {
                (
                    wgpu::StencilOperation::Invert,
                    wgpu::StencilOperation::Invert,
                )
            } else {
                (
                    wgpu::StencilOperation::IncrementWrap,
                    wgpu::StencilOperation::DecrementWrap,
                )
            };
            let face = |op| wgpu::StencilFaceState {
                compare: wgpu::CompareFunction::Always,
                fail_op: wgpu::StencilOperation::Keep,
                depth_fail_op: wgpu::StencilOperation::Keep,
                pass_op: op,
            };
            wgpu::StencilState {
                front: face(front),
                back: face(back),
                read_mask: 0xff,
                write_mask: 0xff,
            }
        };
        // The cover paints where the count passes the rule and leaves zero behind, so the
        // next batch starts from a clear stencil without a clear.
        let test = |even_odd: bool| {
            let face = wgpu::StencilFaceState {
                compare: wgpu::CompareFunction::NotEqual,
                fail_op: wgpu::StencilOperation::Keep,
                depth_fail_op: wgpu::StencilOperation::Keep,
                pass_op: wgpu::StencilOperation::Zero,
            };
            wgpu::StencilState {
                front: face,
                back: face,
                read_mask: if even_odd { 1 } else { 0xff },
                write_mask: 0xff,
            }
        };
        let untouched = wgpu::StencilState {
            front: wgpu::StencilFaceState::IGNORE,
            back: wgpu::StencilFaceState::IGNORE,
            read_mask: 0,
            write_mask: 0,
        };
        let rules = |make: &dyn Fn(bool) -> wgpu::RenderPipeline| [make(false), make(true)];
        Self {
            fan: rules(&|eo| build("stencil_vertex", "fan_fragment", count(eo), false)),
            curve: rules(&|eo| build("stencil_vertex", "curve_fragment", count(eo), false)),
            cover: rules(&|eo| build("cover_vertex", "cover_fragment", test(eo), true)),
            image: build("image_vertex", "image_fragment", untouched, true),
            layout,
            textures,
        }
    }
}
