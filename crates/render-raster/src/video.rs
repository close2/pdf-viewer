//! A decoded video picture converted to RGB on the graphics device (RFC 0015 section 6, round 4;
//! ADR 3529): the three planes uploaded as `R8Uint` textures, one compute pass of `video.wgsl`, and
//! the pixels read back as straight RGBA8.
//!
//! The formula is `pdf_render::video`'s, and `render_cpu::video` computes it as the oracle; the
//! two are compared per pixel in `tests/video_conversion.rs`.
//!
//! The pipeline is compiled on the first picture and never in the device's bring-up (CLAUDE.md
//! principle 2: nothing on the launch path waits for a shader a page does not need). The plane
//! textures and the output buffers are kept for the next picture of the same size, so a clip's
//! frames after its first allocate nothing.

use std::time::{Duration, Instant};

use pdf_render::video::{Conversion, Planes};
use raster_gpu::wgpu;

/// The side of `video.wgsl`'s workgroup, in pixels each way.
const SIDE: u32 = 16;

/// What converting one picture cost, for the measurement and for a host that paces frames.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct VideoCost {
    /// Writing the three planes, the pass and its copy submitted, and the device finished with
    /// them: the conversion as a window that keeps the pixels on the device would pay it.
    pub on_device: Duration,
    /// Mapping the output and copying the rows out: what reading the frame back adds.
    pub read_back: Duration,
}

/// The compiled pass and, once a picture has been converted, the resources of its size.
#[derive(Debug)]
pub(crate) struct VideoPass {
    pipeline: wgpu::ComputePipeline,
    sized: Option<Sized>,
}

/// The textures and buffers of one picture size.
#[derive(Debug)]
struct Sized {
    width: u32,
    height: u32,
    planes: [wgpu::Texture; 3],
    params: wgpu::Buffer,
    output: wgpu::Buffer,
    readback: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    row_words: u32,
}

/// Why the device did not convert a picture.
pub(crate) type Refusal = String;

impl VideoPass {
    /// Parses `video.wgsl` and builds its pipeline, or says what the adapter refused.
    pub(crate) fn compile(gpu: &wgpu::Device) -> Result<Self, Refusal> {
        const LABEL: &str = "render-raster video conversion";
        let (module, refused) = captured(gpu, || {
            gpu.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(LABEL),
                source: wgpu::ShaderSource::Wgsl(include_str!("video.wgsl").into()),
            })
        });
        if let Some(detail) = refused {
            return Err(format!("the shader {LABEL}: {detail}"));
        }
        let (pipeline, refused) = captured(gpu, || {
            gpu.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(LABEL),
                layout: None,
                module: &module,
                entry_point: Some("convert"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            })
        });
        match refused {
            Some(detail) => Err(format!("the pipeline {LABEL}: {detail}")),
            None => Ok(Self {
                pipeline,
                sized: None,
            }),
        }
    }

    /// Converts `planes` by `conversion` and reads the pixels back: tight RGBA8 rows.
    pub(crate) fn convert(
        &mut self,
        gpu: &wgpu::Device,
        queue: &wgpu::Queue,
        planes: &Planes<'_>,
        conversion: &Conversion,
    ) -> Result<(Vec<u8>, VideoCost), Refusal> {
        let started = Instant::now();
        let pipeline = &self.pipeline;
        let sized = Self::sized_for(&mut self.sized, pipeline, gpu, planes.width, planes.height)?;
        let (submission, refused) = captured(gpu, || {
            submit(gpu, queue, pipeline, sized, planes, conversion)
        });
        if let Some(detail) = refused {
            return Err(format!("the conversion's submission: {detail}"));
        }
        let (sender, receiver) = std::sync::mpsc::channel();
        sized
            .readback
            .map_async(wgpu::MapMode::Read, .., move |result| {
                // The poll below runs this callback; the receiver outlives it.
                let _ = sender.send(result);
            });
        gpu.poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: None,
        })
        .map_err(|error| format!("the device, waiting for the conversion: {error}"))?;
        let on_device = started.elapsed();
        let mapped = Instant::now();
        match receiver.recv() {
            Ok(Ok(())) => {}
            Ok(Err(error)) => return Err(format!("mapping the converted picture: {error}")),
            Err(_) => return Err("the map callback was dropped without running".to_owned()),
        }
        let pixels = {
            let view = sized
                .readback
                .get_mapped_range(..)
                .map_err(|error| format!("reading the converted picture: {error}"))?;
            unpad(&view, planes.width, planes.height, sized.row_words)
        };
        sized.readback.unmap();
        Ok((
            pixels,
            VideoCost {
                on_device,
                read_back: mapped.elapsed(),
            },
        ))
    }

    /// The resources for a `width` by `height` picture, made where the last picture's differ.
    fn sized_for<'kept>(
        kept: &'kept mut Option<Sized>,
        pipeline: &wgpu::ComputePipeline,
        gpu: &wgpu::Device,
        width: u32,
        height: u32,
    ) -> Result<&'kept Sized, Refusal> {
        if kept
            .as_ref()
            .is_none_or(|sized| (sized.width, sized.height) != (width, height))
        {
            *kept = None;
            let (made, refused) = captured(gpu, || make_sized(gpu, pipeline, width, height));
            if let Some(detail) = refused {
                return Err(format!(
                    "the resources of a {width} by {height} picture: {detail}"
                ));
            }
            *kept = Some(made?);
        }
        kept.as_ref()
            .ok_or_else(|| "the resources were made and are gone".to_owned())
    }
}

/// The planes written, the pass and its copy recorded and submitted.
fn submit(
    gpu: &wgpu::Device,
    queue: &wgpu::Queue,
    pipeline: &wgpu::ComputePipeline,
    sized: &Sized,
    planes: &Planes<'_>,
    conversion: &Conversion,
) -> wgpu::SubmissionIndex {
    let (chroma_width, chroma_height) = planes.chroma_size();
    for (texture, (bytes, (width, height))) in sized.planes.iter().zip([
        (planes.y, (planes.width, planes.height)),
        (planes.cb, (chroma_width, chroma_height)),
        (planes.cr, (chroma_width, chroma_height)),
    ]) {
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            bytes,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width),
                rows_per_image: Some(height),
            },
            extent(width, height),
        );
    }
    queue.write_buffer(
        &sized.params,
        0,
        &params(planes, conversion, sized.row_words),
    );
    let mut encoder = gpu.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("render-raster video conversion"),
    });
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("render-raster video conversion"),
            timestamp_writes: None,
        });
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, &sized.bind_group, &[]);
        pass.dispatch_workgroups(planes.width.div_ceil(SIDE), planes.height.div_ceil(SIDE), 1);
    }
    encoder.copy_buffer_to_buffer(&sized.output, 0, &sized.readback, 0, sized.output.size());
    queue.submit([encoder.finish()])
}

fn make_sized(
    gpu: &wgpu::Device,
    pipeline: &wgpu::ComputePipeline,
    width: u32,
    height: u32,
) -> Result<Sized, Refusal> {
    // A row of the output is a whole number of 256-byte copy rows, so that a later sink can copy
    // the buffer into a texture without repacking it.
    let row_words = width.next_multiple_of(64);
    let bytes = u64::from(row_words)
        .checked_mul(u64::from(height))
        .and_then(|words| words.checked_mul(4))
        .ok_or_else(|| format!("a {width} by {height} picture's bytes"))?;
    let plane = |label, width, height| {
        gpu.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: extent(width, height),
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Uint,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        })
    };
    let (chroma_width, chroma_height) = (width.div_ceil(2), height.div_ceil(2));
    let planes = [
        plane("video luma", width, height),
        plane("video cb", chroma_width, chroma_height),
        plane("video cr", chroma_width, chroma_height),
    ];
    let buffer = |label, size, usage| {
        gpu.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size,
            usage,
            mapped_at_creation: false,
        })
    };
    let params = buffer(
        "video params",
        PARAMS_BYTES as u64,
        wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    );
    let output = buffer(
        "video pixels",
        bytes,
        wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
    );
    let readback = buffer(
        "video readback",
        bytes,
        wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
    );
    let views = planes
        .each_ref()
        .map(|texture| texture.create_view(&wgpu::TextureViewDescriptor::default()));
    let [luma, cb, cr] = &views;
    let bind_group = gpu.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("video conversion"),
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: params.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(luma),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::TextureView(cb),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: wgpu::BindingResource::TextureView(cr),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: output.as_entire_binding(),
            },
        ],
    });
    Ok(Sized {
        width,
        height,
        planes,
        params,
        output,
        readback,
        bind_group,
        row_words,
    })
}

/// `video.wgsl`'s `Params`: eight `u32`s, four `f32`s, then the two `vec4<f32>` rows.
const PARAMS_BYTES: usize = 80;

#[expect(
    clippy::cast_possible_truncation,
    reason = "the formula's numbers are given to the shader in `f32`, which is the device's \
              arithmetic and the reason the comparison has a tolerance"
)]
fn params(planes: &Planes<'_>, conversion: &Conversion, row_words: u32) -> [u8; PARAMS_BYTES] {
    let affine = conversion.affine();
    let (chroma_width, chroma_height) = planes.chroma_size();
    let words = [
        planes.width,
        planes.height,
        chroma_width,
        chroma_height,
        u32::from(conversion.site_x),
        u32::from(conversion.site_y),
        row_words,
        0,
    ];
    let floats = [
        affine.luma as f32,
        affine.luma_offset as f32,
        0.0,
        0.0,
        affine.cb[0] as f32,
        affine.cb[1] as f32,
        affine.cb[2] as f32,
        0.0,
        affine.cr[0] as f32,
        affine.cr[1] as f32,
        affine.cr[2] as f32,
        0.0,
    ];
    let mut bytes = [0u8; PARAMS_BYTES];
    let fields = words
        .iter()
        .map(|word| word.to_le_bytes())
        .chain(floats.iter().map(|float| float.to_le_bytes()));
    for (at, field) in bytes.as_chunks_mut::<4>().0.iter_mut().zip(fields) {
        *at = field;
    }
    bytes
}

fn extent(width: u32, height: u32) -> wgpu::Extent3d {
    wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    }
}

/// The picture's rows out of the padded buffer.
fn unpad(padded: &[u8], width: u32, height: u32, row_words: u32) -> Vec<u8> {
    let row = (width as usize).saturating_mul(4);
    let stride = (row_words as usize).saturating_mul(4);
    padded
        .chunks(stride)
        .take(height as usize)
        .flat_map(|line| line.get(..row).unwrap_or(line))
        .copied()
        .collect()
}

/// `make` under the device's validation and internal error scopes, and what either caught.
fn captured<T>(gpu: &wgpu::Device, create: impl FnOnce() -> T) -> (T, Option<String>) {
    let internal = gpu.push_error_scope(wgpu::ErrorFilter::Internal);
    let validation = gpu.push_error_scope(wgpu::ErrorFilter::Validation);
    let made = create();
    let invalid = pollster::block_on(validation.pop());
    let failed = pollster::block_on(internal.pop());
    (made, invalid.or(failed).map(|error| error.to_string()))
}
