//! Phase 2 of a frame: the buffers and textures it stages, sized by phase 1's counts
//! and scheduled before anything is recorded.
//!
//! **Counting precedes allocation** (brief section 5), and the counting is not here: `encode.rs`
//! did it on a walk that ran before any allocation and regardless of the target's
//! size. What is left for this module is to allocate exactly what was counted and to
//! charge exactly what it wrote — a lane with nothing to draw gets no buffer rather
//! than an empty one, because wgpu is never handed a zero-length slice.
//!
//! Three things reach the device here, and they are three because phase 1 produced
//! them in three places: the rectangle and quad instance streams, the frame's scratch
//! coverage sheet — one sheet with two producers, which is the whole of the GPU
//! coverage lane's integration (ADR 0016) — and the glyph tiles the atlas packed this
//! frame into a texture that outlives it.

use std::cell::OnceCell;
use std::time::{Duration, Instant};

use super::Device;
use super::residency::Residency;
use crate::encode::Encoded;
use crate::error::RenderError;
use crate::frame::UploadSplit;

/// Phase 2's product: the frame's buffers and textures, scheduled for upload.
pub(super) struct Upload {
    /// `None` for a lane with nothing to draw — wgpu is never handed a zero-length
    /// buffer (brief section 5: the `debug_layers` lesson).
    pub(super) rect_instances: Option<wgpu::Buffer>,
    /// The line lane's instances (ADR 0097), or `None` where the frame drew no line there.
    pub(super) line_instances: Option<wgpu::Buffer>,
    pub(super) quad_instances: Option<wgpu::Buffer>,
    /// The frame's scratch coverage texture, kept alive until the submit.
    pub(super) scratch_view: Option<(wgpu::Texture, wgpu::TextureView)>,
    /// What the frame scheduled, stream by stream; its total is the frame's
    /// `bytes_uploaded`.
    pub(super) split: UploadSplit,
    pub(super) time: Duration,
    /// Host-side spans the upload wants named in the frame's phases — the compute
    /// lane's residency walk and its count stall — so "upload" stops being one number
    /// over four different things.
    pub(super) spans: Vec<(&'static str, Duration)>,
    /// Whether the compute lane stamped its pass queries this frame, which is what
    /// says the query buffers hold this frame's numbers rather than an older one's.
    pub(super) compute_stamped: bool,
    /// The compute chain in flight (ADR 0095): the eight-byte readback the frame
    /// checks at its own end-wait, before anything is presented.
    pub(super) compute_run: Option<crate::compute::ComputeRun>,
}

/// The device's copies of one encode's three instance streams, kept beside a retained
/// encode so that a replay of it sends none of them again (viewer ADR 1993).
///
/// **The streams are the encode's bytes and nothing else**: they are written once, by the
/// first frame that draws the encode, and no pass writes an instance buffer, so every replay
/// reads what a fresh upload of the same encode would have written. They are held by the
/// retained encode itself (`retained::Held`), which is what keeps them from ever being read
/// for another encode — a new encode is a new cell — and what gives them back when the
/// encode is dropped, replaced or forgotten. The price is device memory between frames: the
/// encode's instance bytes, which the frame that made them had already allocated and the
/// budget had admitted, now held while the page is shown rather than for one frame — and
/// charged, for that, against the device's residency budget (`residency`, viewer ADR 2030).
#[derive(Debug, Clone)]
pub(crate) struct KeptInstances {
    rect: Option<wgpu::Buffer>,
    quad: Option<wgpu::Buffer>,
    line: Option<wgpu::Buffer>,
}

impl KeptInstances {
    /// The line lane's stream, which the next encode of the same scene draws from where its
    /// runs are this encode's (viewer ADR 2022).
    pub(crate) fn line(&self) -> Option<&wgpu::Buffer> {
        self.line.as_ref()
    }

    /// Device bytes the three streams hold.
    pub(crate) fn bytes(&self) -> u64 {
        [&self.rect, &self.quad, &self.line]
            .into_iter()
            .flatten()
            .map(wgpu::Buffer::size)
            .fold(0_u64, u64::saturating_add)
    }
}

impl Device {
    /// Phase 2: create the frame's buffers and textures, sized by phase 1's counts,
    /// and schedule their uploads.
    ///
    /// `kept` is the retained encode's cell for its instance streams, `None` for an encode
    /// no later frame can replay: the first frame that draws a retained encode fills it, and
    /// a replay reads the streams from it and schedules none of their bytes (ADR 1993) —
    /// unless the residency budget refused them, when the cell says so and the streams are
    /// this frame's (ADR 2030). `this_frame` holds the streams across the frame's own
    /// attempts (`submit_encoded`'s loop) wherever the encode's cell does not.
    ///
    /// # Errors
    ///
    /// Whatever the GPU coverage lane refuses: its sheet is a texture like any other
    /// and is bounded by the adapter's dimension.
    pub(super) fn upload(
        &mut self,
        encoded: &Encoded,
        kept: Option<&OnceCell<Residency>>,
        this_frame: &OnceCell<KeptInstances>,
    ) -> Result<Upload, RenderError> {
        let started = Instant::now();
        let mut spans: Vec<(&'static str, Duration)> = Vec::new();
        // The globals are per plan now (ADR 0036) and made where the pass is recorded,
        // so nothing is charged here for them.
        let mut split = UploadSplit::default();
        // The compute lane's whole chain is submitted *first* — count, scan, emit,
        // deposit, one submission with no mid-frame readback (ADR 0095) — so the
        // device works through it while the host stages the instance buffers and
        // flushes the atlas. What used to be a stall on the count's map is eight
        // bytes the frame reads at its own end-wait, before anything is presented.
        let scratch_view = self.create_scratch_sheet(encoded, &mut split.scratch_sheet)?;
        let compute_run = if encoded.compute.is_empty() {
            None
        } else {
            split.compute_records = encoded.compute.record_bytes();
            split.compute_device_only = encoded
                .compute
                .device_bytes()
                .saturating_sub(split.compute_records)
                .saturating_add(self.compute_persist.capacity_bytes());
            // The arena's growth is this frame's staging too: the first frame that
            // draws an outline pays its residency once, and the counter says so.
            let arena_before = self.segment_arena.device_bytes();
            let run = match scratch_view.as_ref() {
                Some((texture, _)) => crate::compute::dispatch_chain(
                    &self.gpu,
                    &self.queue,
                    &mut self.compute_pipelines,
                    &mut self.segment_arena,
                    &mut self.compute_persist,
                    &self.resources,
                    self.limits.max_frame_bytes,
                    texture,
                    &encoded.compute,
                    self.compute_queries.as_ref(),
                    &mut spans,
                )?,
                None => None,
            };
            split.arena_growth = self
                .segment_arena
                .device_bytes()
                .saturating_sub(arena_before);
            run
        };
        let KeptInstances {
            rect: rect_instances,
            quad: quad_instances,
            line: line_instances,
        } = self.instance_streams(encoded, kept, this_frame, &mut split);
        self.flush_atlas_tiles(&mut split.atlas_rows);
        if !encoded.winding.is_empty()
            && let Some((_, view)) = scratch_view.as_ref()
        {
            split.winding = encoded.winding.device_bytes();
            crate::winding::render_into(
                &self.gpu,
                &self.queue,
                &self.pipelines,
                &mut self.winding_texture,
                view,
                &encoded.winding,
                self.coverage_samples,
                if encoded.scratch.as_ref().is_none_or(|s| s.data.is_empty()) {
                    crate::winding::SheetUse::LaneAlone
                } else {
                    crate::winding::SheetUse::BesideCpuBytes
                },
                self.limits.max_target_size,
            )?;
        }

        let compute_stamped = compute_run.is_some() && self.compute_queries.is_some();
        Ok(Upload {
            compute_run,
            rect_instances,
            quad_instances,
            line_instances,
            scratch_view,
            split,
            time: started.elapsed(),
            spans,
            compute_stamped,
        })
    }

    /// The frame's three instance streams on the device: made from the encode's bytes, or —
    /// for a replay of a retained encode — the ones its first frame made, read from `kept`
    /// (ADR 1993), or — for a later attempt of this frame — the ones its first attempt made,
    /// read from `this_frame`. Streams read from either schedule none of their bytes, so
    /// their shares stay zero, which is what crossed.
    ///
    /// A retained encode's first frame charges the streams against the residency budget
    /// before it makes them (ADR 2030): admitted, they are kept with their charge; refused,
    /// the cell keeps the refusal, so that no later frame of the encode asks again, and the
    /// streams are this frame's alone.
    fn instance_streams(
        &self,
        encoded: &Encoded,
        kept: Option<&OnceCell<Residency>>,
        this_frame: &OnceCell<KeptInstances>,
        split: &mut UploadSplit,
    ) -> KeptInstances {
        if let Some(streams) = this_frame.get() {
            return streams.clone();
        }
        let make_instances = |label, data: &[u8]| {
            if data.is_empty() {
                None
            } else {
                let buffer = self.gpu.create_buffer(&wgpu::BufferDescriptor {
                    label: Some(label),
                    size: data.len() as u64,
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
                self.queue.write_buffer(&buffer, 0, data);
                Some(buffer)
            }
        };
        let mut make_streams = || {
            let [rect, quad] = [
                (
                    "raster rect instances",
                    &encoded.rect_instances,
                    &mut split.rect_instances,
                ),
                (
                    "raster quad instances",
                    &encoded.quad_instances,
                    &mut split.quad_instances,
                ),
            ]
            .map(|(label, data, share)| {
                *share = data.len() as u64;
                make_instances(label, data)
            });
            // The line stream the encode before this one left on the device, where it is this
            // encode's to the byte (viewer ADR 2022): drawn from where it is, so none of its
            // bytes cross again.
            let line = if let Some(adopted) = encoded.adopted_line.as_ref() {
                Some(adopted.clone())
            } else {
                split.line_instances = encoded.line_instances.len() as u64;
                make_instances("raster line instances", &encoded.line_instances)
            };
            KeptInstances { rect, quad, line }
        };
        let Some(cell) = kept else {
            return this_frame.get_or_init(make_streams).clone();
        };
        // Decided once, by the encode's first frame: counted from the bytes the streams will
        // hold, charged before a buffer is made, and kept with the streams or refused for the
        // encode's life.
        let residency = cell.get_or_init(|| {
            let bytes = [
                &encoded.rect_instances,
                &encoded.quad_instances,
                &encoded.line_instances,
            ]
            .into_iter()
            .map(|data| data.len() as u64)
            .fold(0_u64, u64::saturating_add);
            match self.residency.charge(bytes) {
                Ok(charge) => Residency::Kept {
                    streams: make_streams(),
                    charge,
                },
                Err(refused) => Residency::Refused(refused),
            }
        });
        match residency {
            Residency::Kept { streams, .. } => streams.clone(),
            Residency::Refused(_) => this_frame.get_or_init(make_streams).clone(),
        }
    }

    /// The frame's scratch coverage sheet: the CPU lane's bytes uploaded, the GPU
    /// lane's tiles drawn, into one texture.
    ///
    /// One sheet with two producers is the whole of the GPU lane's integration (ADR
    /// 0016): downstream, a coverage quad names a rectangle of *this* texture and has
    /// no idea which lane put the bytes there. The upload goes first because the draw
    /// loads what it finds — a tile the GPU draws covers only its own rectangle, so
    /// the CPU lane's bytes elsewhere on the sheet survive it.
    ///
    /// **Borrowed, not taken.** Both of these used to be moved out of the `Encoded`,
    /// which is what a frame that owns its encode can afford; a retained encode is
    /// replayed by later frames, so the sheet it names has to survive its first upload
    /// (ADR 0048, and ADR 0045's invalidation list named this as the one plumbing
    /// change the design needs).
    // The `Result` is the seam's shape: the two lane dispatches that used to live
    // here kept their refusals, and the caller threads one `?` through all three
    // steps of the sheet's life.
    #[expect(clippy::unnecessary_wraps)]
    fn create_scratch_sheet(
        &mut self,
        encoded: &Encoded,
        bytes: &mut u64,
    ) -> Result<Option<(wgpu::Texture, wgpu::TextureView)>, RenderError> {
        let Some(scratch) = encoded.scratch.as_ref() else {
            return Ok(None);
        };
        let winding = &encoded.winding;
        let mut usage = wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST;
        if !winding.is_empty() {
            // COPY_SRC with it, so a test can read the sheet back and hold the two
            // lanes to a stated difference rather than to a hope.
            usage |= wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC;
        }
        if !encoded.compute.is_empty() {
            // The compute lane's image travels texture → buffer → dispatch → texture
            // (ADR 0080), so the sheet is a copy source as well as a destination.
            usage |= wgpu::TextureUsages::COPY_SRC;
        }
        let texture = self.gpu.create_texture(&wgpu::TextureDescriptor {
            label: Some("raster scratch coverage"),
            size: wgpu::Extent3d {
                width: scratch.width,
                height: scratch.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage,
            view_formats: &[],
        });
        if !scratch.data.is_empty() {
            self.queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &scratch.data,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(scratch.width),
                    rows_per_image: None,
                },
                wgpu::Extent3d {
                    width: scratch.width,
                    height: scratch.height,
                    depth_or_array_layers: 1,
                },
            );
        }
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        *bytes = bytes.saturating_add(scratch.data.len() as u64);
        Ok(Some((texture, view)))
    }

    /// New glyph tiles into the persistent atlas texture (created on first need —
    /// the startup path never pays for it, brief section 7).
    ///
    /// **One `write_texture` per dirty row span, never one per tile** (ADR 0078). A
    /// `write_texture` costs a fixed price before its first byte — validation, a
    /// staging allocation, a scheduled copy — and on one adapter's DX12 path that
    /// price was measured at ~110 µs a call: a cold frame of the caller's 58 003-tile
    /// drawing spent 6.4 s here moving 4.9 MB. The atlas keeps a CPU sheet of its own
    /// texels, so a span of it uploads as one borrowed full-width slice; the same
    /// frame's shelves coalesce to a single span, and the loop below runs once.
    fn flush_atlas_tiles(&mut self, bytes: &mut u64) {
        let spans = self.atlas.take_dirty();
        if spans.is_empty() {
            return;
        }
        let (atlas_w, atlas_h) = self.atlas.dimensions();
        let (texture, _) = self.atlas_texture.get_or_insert_with(|| {
            let texture = self.gpu.create_texture(&wgpu::TextureDescriptor {
                label: Some("raster glyph atlas"),
                size: wgpu::Extent3d {
                    width: atlas_w,
                    height: atlas_h,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::R8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            (texture, view)
        });
        for span in spans {
            let rows = self.atlas.rows(span);
            self.queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: 0,
                        y: span.start,
                        z: 0,
                    },
                    aspect: wgpu::TextureAspect::All,
                },
                rows,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(atlas_w),
                    rows_per_image: None,
                },
                wgpu::Extent3d {
                    width: atlas_w,
                    height: span.end.saturating_sub(span.start),
                    depth_or_array_layers: 1,
                },
            );
            *bytes = bytes.saturating_add(rows.len() as u64);
        }
    }
}
