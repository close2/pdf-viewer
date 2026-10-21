//! Tier 1's price: copy out, map, and convert to straight alpha, exactly once.
//!
//! Section 6.1 of the brief measured this path as the item that dominates an offscreen frame
//! — about 1.2 GB/s of copy, map and demultiply at 4× scale — which is why it lives
//! behind [`Target::Readback`](crate::target::Target::Readback) alone and why
//! `Timings::readback` prices it separately: brief section 11.1 asks how much of the old backend's
//! fixed cost this was, and the split is the answer.
//!
//! The premultiplied→straight conversion here is the boundary conversion of brief section 3, done
//! once; its rounding rule is raster's own and is recorded in `raster/doc/adr/0005`.
//!
//! **A frame put onto a backdrop converts nothing** ([`Target::ReadbackOnto`], the caller's
//! `doc/adr/2150`): `onto.wgsl` composites the copy over the caller's opaque colour in the
//! buffer the copy lands in, every pixel leaves opaque, and the host's work is the map and a
//! row copy. The caller measured its own three passes over the straight raster — premultiply,
//! medium, demultiply — at 2.2 ms of a 3.3 ms page on this machine's RADV.
//!
//! [`Target::ReadbackOnto`]: crate::target::Target::ReadbackOnto

use crate::error::{PipelineProblem, RenderError};
use crate::frame::Raster;
use crate::target::Backdrop;

/// `onto.wgsl`'s pipeline, compiled on the first frame that asks for a backdrop and kept —
/// never on the startup path, the compute lane's rule (ADR 0080) — or the refusal that stopped
/// it, kept as the compute lane keeps one, so a later frame is refused in the same words
/// without a failing compile inside it (ADR 1962).
pub(crate) type KeptOnto = Option<Result<wgpu::ComputePipeline, PipelineProblem>>;

/// The side of `onto.wgsl`'s workgroup, in pixels each way.
const ONTO_SIDE: u32 = 16;

/// Parse `onto.wgsl` and build its pipeline, or name what this adapter refused.
fn compile_onto(gpu: &wgpu::Device) -> Result<wgpu::ComputePipeline, PipelineProblem> {
    const LABEL: &str = "raster readback onto";
    let (module, refused) = crate::pipeline::captured(gpu, || {
        gpu.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(LABEL),
            source: wgpu::ShaderSource::Wgsl(crate::shaders::ONTO.into()),
        })
    });
    if let Some(detail) = refused {
        return Err(PipelineProblem::Shader {
            shader: LABEL,
            detail,
        });
    }
    let (pipeline, refused) = crate::pipeline::captured(gpu, || {
        gpu.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some(LABEL),
            layout: None,
            module: &module,
            entry_point: Some("onto"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        })
    });
    match refused {
        Some(detail) => Err(PipelineProblem::ComputePipeline {
            pipeline: LABEL,
            detail,
        }),
        None => Ok(pipeline),
    }
}

/// `onto.wgsl`'s `Params`, as the bytes the uniform holds: eight `u32`s in declaration order.
fn onto_params(backdrop: &Backdrop, width: u32, height: u32, bytes_per_row: u32) -> [u8; 32] {
    let [left, top, right, bottom] = backdrop.keep.unwrap_or([0, 0, width, height]);
    let [r, g, b] = backdrop.colour;
    let colour = u32::from_le_bytes([r, g, b, u8::MAX]);
    let words = [
        left,
        top,
        right,
        bottom,
        colour,
        width,
        height,
        bytes_per_row / 4,
    ];
    let mut bytes = [0_u8; 32];
    for (slot, word) in bytes.chunks_exact_mut(4).zip(words) {
        slot.copy_from_slice(&word.to_le_bytes());
    }
    bytes
}

/// A copy of a finished target on its way to system memory: submitted, not yet mapped.
///
/// The copy is submitted with the frame and mapped when the frame is collected, so a frame
/// another one is walked beside waits for its pixels only once (ADR 1595).
pub(crate) struct PendingCopy {
    buffer: wgpu::Buffer,
    width: u32,
    height: u32,
    bytes_per_row: u32,
    /// The copy's own submission, which the map waits on and nothing later.
    submission: wgpu::SubmissionIndex,
    /// Whether `onto.wgsl` put the copy onto a backdrop, so that it holds opaque pixels whose
    /// premultiplied and straight bytes are one and nothing is converted.
    opaque: bool,
    /// What compiling `onto.wgsl`'s pipeline cost, where this copy was the first to need it —
    /// the one-off a frame's `Timings::phases` names, as every first-use compile is.
    pub(crate) compiled: Option<std::time::Duration>,
}

impl PendingCopy {
    /// Map the copy once the device has made it, and convert premultiplied to straight alpha.
    pub(crate) fn collect(self, gpu: &wgpu::Device) -> Result<Raster, RenderError> {
        let pixels = map_and_convert(
            gpu,
            &self.buffer,
            &self.submission,
            self.width,
            self.height,
            self.bytes_per_row,
            self.opaque,
        )?;
        Ok(Raster::new(self.width, self.height, pixels))
    }
}

/// A copy's row pitch — each row padded to `COPY_BYTES_PER_ROW_ALIGNMENT` — and its whole size
/// in bytes, or `None` where either overflows.
pub(crate) fn copy_extent(width: u32, height: u32) -> Option<(u32, u64)> {
    let bytes_per_row = width
        .checked_mul(4)?
        .checked_next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)?;
    let size = u64::from(bytes_per_row).checked_mul(u64::from(height))?;
    Some((bytes_per_row, size))
}

/// `onto.wgsl`'s work for one copy: the storage buffer the target is copied into and put onto
/// its backdrop in, and the bind group that names it.
struct OntoPass<'p> {
    staged: wgpu::Buffer,
    group: wgpu::BindGroup,
    pipeline: &'p wgpu::ComputePipeline,
}

impl<'p> OntoPass<'p> {
    /// The pipeline, compiled here where this is the first copy to need it — the duration then
    /// comes back beside the pass — and the buffers of one copy of `extent`'s size.
    ///
    /// A refusal kept from an earlier frame refuses this one in the same words, before any of
    /// the copy's buffers is made (ADR 1962). A copy past `max_frame_bytes` is refused by name,
    /// since a buffer past it is one this device cannot bind as storage.
    fn prepare(
        gpu: &wgpu::Device,
        queue: &wgpu::Queue,
        (backdrop, kept): (&Backdrop, &'p mut KeptOnto),
        (width, height, bytes_per_row, size): (u32, u32, u32, u64),
        max_frame_bytes: u64,
    ) -> Result<(Self, Option<std::time::Duration>), RenderError> {
        if size > max_frame_bytes {
            return Err(RenderError::FrameBudgetExceeded {
                needed: size,
                budget: max_frame_bytes,
            });
        }
        let mut compiled = None;
        let pipeline = kept
            .get_or_insert_with(|| {
                let started = std::time::Instant::now();
                let pipeline = compile_onto(gpu);
                compiled = Some(started.elapsed());
                pipeline
            })
            .as_ref()
            .map_err(|problem| RenderError::from(problem.clone()))?;
        // A mappable buffer cannot be bound as storage, so the frame is copied into one that
        // can, put onto the backdrop there, and copied on: a second copy of the device's own,
        // which is what lets the host convert nothing.
        let staged = gpu.create_buffer(&wgpu::BufferDescriptor {
            label: Some("raster readback onto"),
            size,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let params = gpu.create_buffer(&wgpu::BufferDescriptor {
            label: Some("raster readback onto params"),
            size: 32,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(
            &params,
            0,
            &onto_params(backdrop, width, height, bytes_per_row),
        );
        let group = gpu.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("raster readback onto"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: params.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: staged.as_entire_binding(),
                },
            ],
        });
        Ok((
            Self {
                staged,
                group,
                pipeline,
            },
            compiled,
        ))
    }

    /// Put the copy onto its backdrop, then copy it on into the buffer the host maps.
    fn record(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        into: &wgpu::Buffer,
        (width, height, size): (u32, u32, u64),
    ) {
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("raster readback onto"),
                timestamp_writes: None,
            });
            pass.set_pipeline(self.pipeline);
            pass.set_bind_group(0, &self.group, &[]);
            pass.dispatch_workgroups(width.div_ceil(ONTO_SIDE), height.div_ceil(ONTO_SIDE), 1);
        }
        encoder.copy_buffer_to_buffer(&self.staged, 0, into, 0, size);
    }
}

/// Copy the finished target out: record the copy and submit it behind the frame's passes —
/// through `onto.wgsl` first where the frame asked for a backdrop.
///
/// # Errors
///
/// A target too large to copy; a backdrop's frame past [`Limits::max_frame_bytes`], which is
/// the largest buffer this device binds as storage ([`Device::backdrop_fits`] asks the same
/// question first); and the pipeline's refusal.
///
/// [`Limits::max_frame_bytes`]: crate::device::Limits::max_frame_bytes
/// [`Device::backdrop_fits`]: crate::device::Device::backdrop_fits
pub(crate) fn copy_out(
    gpu: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    width: u32,
    height: u32,
    limits: crate::device::Limits,
    onto: Option<(&Backdrop, &mut KeptOnto)>,
) -> Result<PendingCopy, RenderError> {
    let (bytes_per_row, size) = copy_extent(width, height).ok_or(RenderError::TargetTooLarge {
        width,
        height,
        limit: limits.max_target_size,
    })?;
    let (onto, compiled) = match onto {
        Some(asked) => {
            let (pass, compiled) = OntoPass::prepare(
                gpu,
                queue,
                asked,
                (width, height, bytes_per_row, size),
                limits.max_frame_bytes,
            )?;
            (Some(pass), compiled)
        }
        None => (None, None),
    };
    let buffer = gpu.create_buffer(&wgpu::BufferDescriptor {
        label: Some("raster readback"),
        size,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = gpu.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("raster readback copy"),
    });
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: onto.as_ref().map_or(&buffer, |pass| &pass.staged),
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row),
                rows_per_image: None,
            },
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    if let Some(pass) = &onto {
        pass.record(&mut encoder, &buffer, (width, height, size));
    }
    let submission = queue.submit([encoder.finish()]);
    Ok(PendingCopy {
        buffer,
        width,
        height,
        bytes_per_row,
        submission,
        opaque: onto.is_some(),
        compiled,
    })
}

/// Map the copy-out buffer and demultiply straight from it, without a staging `Vec`:
/// `read_buffer`'s `to_vec` would be a second full-target copy — 8 MB at page size — of
/// bytes this reads once and discards (ADR 0022). An `opaque` copy is already straight and
/// is only taken out of its row padding.
fn map_and_convert(
    gpu: &wgpu::Device,
    buffer: &wgpu::Buffer,
    submission: &wgpu::SubmissionIndex,
    width: u32,
    height: u32,
    bytes_per_row: u32,
    opaque: bool,
) -> Result<Vec<u8>, RenderError> {
    await_map(gpu, buffer, submission)?;
    let pixels = {
        let view = buffer
            .get_mapped_range(..)
            .map_err(|e| RenderError::ReadbackFailed {
                detail: e.to_string(),
            })?;
        if opaque {
            unpad(&view, width, height, bytes_per_row)
        } else {
            demultiply(&view, width, height, bytes_per_row)
        }
    };
    buffer.unmap();
    Ok(pixels)
}

/// Map a `MAP_READ` buffer that `submission` wrote, and copy its bytes out.
pub(crate) fn read_buffer(
    gpu: &wgpu::Device,
    buffer: &wgpu::Buffer,
    submission: &wgpu::SubmissionIndex,
) -> Result<Vec<u8>, RenderError> {
    await_map(gpu, buffer, submission)?;
    let bytes = {
        let view = buffer
            .get_mapped_range(..)
            .map_err(|e| RenderError::ReadbackFailed {
                detail: e.to_string(),
            })?;
        view.to_vec()
    };
    buffer.unmap();
    Ok(bytes)
}

/// Ask for the map and block until the device has done it.
///
/// **The wait is for the submission that wrote the buffer, and no later one** (ADR 1595): a
/// frame submitted after this one is on the device behind it, and waiting for the most recent
/// submission would wait for that frame's passes too.
fn await_map(
    gpu: &wgpu::Device,
    buffer: &wgpu::Buffer,
    submission: &wgpu::SubmissionIndex,
) -> Result<(), RenderError> {
    let (sender, receiver) = std::sync::mpsc::channel();
    buffer.map_async(wgpu::MapMode::Read, .., move |result| {
        // The poll below drives this callback; a send failure would mean the
        // receiver was dropped, which only happens after this function returned.
        let _ = sender.send(result);
    });
    wait_for(gpu, submission)?;
    match receiver.recv() {
        Ok(Ok(())) => {}
        Ok(Err(source)) => {
            return Err(RenderError::ReadbackFailed {
                detail: source.to_string(),
            });
        }
        Err(_) => {
            return Err(RenderError::DeviceLost {
                detail: "the map callback was dropped without running".into(),
            });
        }
    }
    Ok(())
}

/// Block until the device has executed `submission`, and run the callbacks it made ready.
pub(crate) fn wait_for(
    gpu: &wgpu::Device,
    submission: &wgpu::SubmissionIndex,
) -> Result<(), RenderError> {
    gpu.poll(wgpu::PollType::Wait {
        submission_index: Some(submission.clone()),
        timeout: None,
    })
    .map(|_| ())
    .map_err(|e| RenderError::DeviceLost {
        detail: e.to_string(),
    })
}

/// The rounding rule of `raster/doc/adr/0005`, for every (alpha, channel) pair there is:
/// `straight = round(255·c / a)`, computed in integers as `(c·255 + a/2) / a` and
/// clamped to 255 — the clamp covers the ≤1-ulp cases where unorm blending leaves a
/// channel a hair above its alpha.
///
/// 64 KiB, built at compile time, indexed `[alpha << 8 | channel]`. It exists because
/// the loop below ran three integer divisions per pixel — six million on a page — and a
/// division is the most expensive integer operation a CPU has. The table is not an
/// approximation of the rule: it *is* the rule, evaluated ahead of time, which
/// `demultiply_matches_the_documented_division` checks over all 65 536 pairs
/// (ADR 0022).
static STRAIGHT: [u8; 65_536] = build_straight_table();

// The 64 KiB array is the point of the function, and it is a `static`: the local is
// the initialiser, evaluated at compile time and never on a stack.
#[expect(clippy::cast_possible_truncation)] // every value is clamped to 255 first
#[expect(clippy::large_stack_arrays)]
// Both loop counters stop at 256 and `channel * 255 + alpha / 2` is at most 65 152, so
// nothing here can overflow; a const fn that panicked would fail the build rather than
// the frame, which is the strongest form this bound could be checked in.
#[expect(clippy::arithmetic_side_effects)]
const fn build_straight_table() -> [u8; 65_536] {
    let mut table = [0_u8; 65_536];
    let mut alpha = 1_usize; // alpha 0 keeps its row of zeros: a transparent pixel has
    while alpha < 256 {
        // no straight colour, and the loop below never reads that row.
        let mut channel = 0_usize;
        while channel < 256 {
            let straight = (channel * 255 + alpha / 2) / alpha;
            table[alpha << 8 | channel] = if straight > 255 { 255 } else { straight as u8 };
            channel += 1;
        }
        alpha += 1;
    }
    table
}

/// Premultiplied RGBA8 rows (with copy padding) to straight-alpha RGBA8, tightly
/// packed. The conversion happens exactly once, at this boundary (section 3 of the brief).
///
/// Three shapes of pixel, in the order a real page has them: fully transparent (most
/// of a page of text), fully opaque (a filled background, and every pixel of a photo),
/// and partial. The first two are byte copies; only the third reaches the table.
// Bounds make the arithmetic infallible: `alpha << 8 | channel` is below 65 536 by
// construction, and row indexing is bounded by the buffer layout the copy just wrote.
// Stated here once rather than checked per pixel in a hot loop.
#[expect(clippy::arithmetic_side_effects)]
fn demultiply(padded: &[u8], width: u32, height: u32, bytes_per_row: u32) -> Vec<u8> {
    let width = width as usize;
    let height = height as usize;
    let bytes_per_row = bytes_per_row as usize;
    let row_bytes = width * 4;
    // Written through a slice rather than `push`ed: eight million bounds-checked pushes
    // is the shape of the loop, not the work in it.
    let mut pixels = vec![0_u8; row_bytes * height];
    for row in 0..height {
        let start = row * bytes_per_row;
        let source = &padded[start..start + row_bytes];
        let destination = &mut pixels[row * row_bytes..(row + 1) * row_bytes];
        for (pixel, out) in source.chunks_exact(4).zip(destination.chunks_exact_mut(4)) {
            let alpha = pixel[3];
            if alpha == 0 {
                continue; // the destination is already zero
            }
            if alpha == 255 {
                out.copy_from_slice(pixel);
                continue;
            }
            let row = usize::from(alpha) << 8;
            out[0] = STRAIGHT[row | usize::from(pixel[0])];
            out[1] = STRAIGHT[row | usize::from(pixel[1])];
            out[2] = STRAIGHT[row | usize::from(pixel[2])];
            out[3] = alpha;
        }
    }
    pixels
}

/// Padded RGBA8 rows to tightly packed ones, unchanged: [`demultiply`] for a copy whose pixels
/// are opaque, where premultiplied and straight alpha are the same bytes.
// As `demultiply`: row indexing is bounded by the buffer layout the copy just wrote.
#[expect(clippy::arithmetic_side_effects)]
fn unpad(padded: &[u8], width: u32, height: u32, bytes_per_row: u32) -> Vec<u8> {
    let row_bytes = width as usize * 4;
    let bytes_per_row = bytes_per_row as usize;
    let mut pixels = Vec::with_capacity(row_bytes * height as usize);
    for row in padded.chunks(bytes_per_row).take(height as usize) {
        pixels.extend_from_slice(&row[..row_bytes]);
    }
    pixels
}

#[cfg(test)]
mod tests {
    use super::{demultiply, onto_params, unpad};
    use crate::shaders;
    use crate::shaders::layout::{Lane, check};
    use crate::target::Backdrop;

    /// `onto.wgsl`'s `Params` is where the shader declares it: each of the eight words fed a
    /// value of its own and found at the offset the declaration gives it.
    #[test]
    fn the_backdrop_s_params_are_where_onto_declares_them() {
        let backdrop = Backdrop {
            colour: [0x11, 0x22, 0x33],
            keep: Some([1, 2, 3, 4]),
        };
        check(
            shaders::ONTO,
            "Params",
            &onto_params(&backdrop, 5, 6, 28),
            &[
                ("left", Lane::Word(1)),
                ("top", Lane::Word(2)),
                ("right", Lane::Word(3)),
                ("bottom", Lane::Word(4)),
                ("colour", Lane::Word(0xff33_2211)),
                ("width", Lane::Word(5)),
                ("height", Lane::Word(6)),
                ("words_per_row", Lane::Word(7)),
            ],
        );
    }

    /// No rectangle keeps the whole target.
    #[test]
    fn no_keep_is_the_whole_target() {
        let backdrop = Backdrop {
            colour: [0, 0, 0],
            keep: None,
        };
        let bytes = onto_params(&backdrop, 640, 480, 2560);
        assert_eq!(&bytes[8..12], &640_u32.to_le_bytes());
        assert_eq!(&bytes[12..16], &480_u32.to_le_bytes());
    }

    /// `onto.wgsl`'s first step: this file's demultiply followed by the caller's premultiply —
    /// `(s·a + 127) / 255`, `render_raster::premultiply` — is a clamp of each channel to its
    /// alpha, for every one of the 65 536 pairs, so the device can take the clamp in place of
    /// the round trip and leave the caller's bytes where they were (the caller's ADR 2150).
    #[test]
    fn the_round_trip_through_straight_alpha_is_a_clamp_to_alpha() {
        for alpha in 0..=255_u8 {
            for channel in 0..=255_u8 {
                let straight = demultiply(&[channel, channel, channel, alpha], 1, 1, 4);
                let again = if alpha == u8::MAX {
                    straight[0]
                } else {
                    let scaled = u16::from(straight[0]) * u16::from(alpha) + 127;
                    u8::try_from(scaled / 255).unwrap_or(u8::MAX)
                };
                assert_eq!(
                    again,
                    channel.min(alpha),
                    "alpha {alpha}, channel {channel}"
                );
            }
        }
    }

    /// An opaque copy leaves its padding behind and nothing else.
    #[test]
    fn unpad_skips_row_padding_and_keeps_the_bytes() {
        let mut padded = vec![0xee_u8; 512];
        padded[..4].copy_from_slice(&[1, 2, 3, 255]);
        padded[256..260].copy_from_slice(&[4, 5, 6, 255]);
        assert_eq!(unpad(&padded, 1, 2, 256), vec![1, 2, 3, 255, 4, 5, 6, 255]);
    }

    /// The documented rounding rule, checked at its edges: zero alpha zeroes the
    /// pixel, full alpha is the identity, and a half-covered premultiplied value
    /// rounds to nearest.
    #[test]
    fn demultiply_rounds_as_documented() {
        // One row, no padding: alpha 0, alpha 255, alpha 128.
        let padded = [
            9, 9, 9, 0, // fully transparent: everything zeroes
            10, 128, 255, 255, // opaque: unchanged
            64, 1, 128, 128, // half-covered: see the expected values below
        ];
        let out = demultiply(&padded, 3, 1, 12);
        assert_eq!(&out[0..4], &[0, 0, 0, 0]);
        assert_eq!(&out[4..8], &[10, 128, 255, 255]);
        // (64·255 + 64)/128 = 128; (1·255 + 64)/128 = 2; (128·255 + 64)/128 clamps to 255.
        assert_eq!(&out[8..12], &[128, 2, 255, 128]);
    }

    /// The table *is* the documented division, for every pair there is — the claim
    /// that makes replacing six million divisions a shortcut rather than a change.
    #[test]
    #[expect(clippy::cast_possible_truncation)] // both counters are bounded by 255
    fn demultiply_matches_the_documented_division() {
        for alpha in 1_u32..=255 {
            for channel in 0_u32..=255 {
                let divided = ((channel * 255 + alpha / 2) / alpha).min(255) as u8;
                let padded = [channel as u8, 0, 0, alpha as u8];
                let out = demultiply(&padded, 1, 1, 4);
                assert_eq!(
                    out[0], divided,
                    "alpha {alpha}, channel {channel}: table {} against division {divided}",
                    out[0]
                );
            }
        }
    }

    /// Copy padding between rows is dropped, not read into the raster.
    #[test]
    fn demultiply_skips_row_padding() {
        let mut padded = vec![0_u8; 512];
        // Two rows of one opaque white pixel each, 256-byte padded rows.
        padded[0..4].copy_from_slice(&[255, 255, 255, 255]);
        padded[256..260].copy_from_slice(&[255, 255, 255, 255]);
        let out = demultiply(&padded, 1, 2, 256);
        assert_eq!(out, vec![255, 255, 255, 255, 255, 255, 255, 255]);
    }
}
