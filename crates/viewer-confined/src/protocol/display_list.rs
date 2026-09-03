//! One page's resolved marks, as bytes.
//!
//! ADR 0607 settled what crosses this boundary when the window is on it: **display lists from
//! the confined process to a host that keeps the graphics device**, with the raster payload kept
//! and chosen per page by size. This module is that codec. Nothing here is a clause of ISO
//! 32000-2 — a [`DisplayList`] is what is left *after* the standard has been read, which is why
//! `pdf-render` contains no PDF semantics — so what this file owes is a transport's obligations
//! rather than a renderer's.
//!
//! # The direction of trust, which decides every rule below
//!
//! The encoder runs in the **confined** process, which is the one holding a hostile document.
//! The decoder runs in the **host**, which is not confined. So this is a parser in the
//! privileged process over bytes an attacker who had already subverted the renderer would
//! choose, and every count it reads is a claim rather than a fact. Four rules follow, and each
//! is a place a length could otherwise become an allocation or an index:
//!
//! - **A count is checked against the bytes that could hold it, before anything is reserved.**
//!   [`count`] takes the smallest number of bytes one element of that table can occupy and
//!   refuses a count the message is too short for. That is tighter than the parent module's
//!   [`Reader::list`], which can only assume one byte an element, and it costs one
//!   multiplication.
//! - **Every identifier is checked against a table that has already been read whole.** The
//!   tables come first in the byte stream for exactly this reason: a clip identifier, a soft
//!   mask identifier, a path index and a shading index are all bounds-checked against a length
//!   the decoder has already established, never against a length the message asserts.
//! - **Nesting is bounded by [`pdf_render::MAX_GROUP_DEPTH`]**, which is the bound every backend
//!   in this tree already refuses past. A decoder that followed a group as deep as it was told
//!   to would let a few hundred bytes exhaust the host's stack.
//! - **A structural invariant a backend depends on is checked here rather than assumed there.**
//!   An [`Image`] whose dimensions and sample count disagree is refused by name, which is the
//!   same check `fuzz/fuzz_targets/confined_wire.rs` already asserts of a [`crate::Raster`].
//!
//! # Sharing is preserved, and that is a requirement rather than an optimisation
//!
//! ADR 0607 measured what a flattening encoder costs: over 958 first pages of `doc/pdf.js` the
//! same corpus goes from **0.37 to 0.91** of its raster when each occurrence of a shared
//! `Arc<Path>`, `Arc<[u8]>` or `Arc<Shading>` is written out again — 30× worse at the extreme,
//! for the reason [`pdf_render::Command`]'s own documentation gives: "3005 fill commands on a
//! dense specification page carried 101 320 path segments between them". An encoder that did not
//! preserve the sharing would buy nothing at all, so the four tables below are the format's
//! subject and the command stream is a list of indices into them.
//!
//! `Arc<ShadingKind>` is interned beside `Arc<Shading>` and the two are separate tables, because
//! they are separately shared: `pdf_model::shading::Cache` hands one kind to many shadings that
//! differ only in their transform, which is `bug1721218_reduced.pdf`'s 3576 paints from three
//! function objects.
//!
//! # What does not cross, refused by name
//!
//! [`ImageSource::AtDeviceScale`] and [`ShadingKind::Sampled`] carry `Arc<dyn ImageAtDeviceScale>`
//! and `Arc<dyn ColoursAtDeviceScale>` — *producers* the backend invokes once it knows how many
//! device pixels the mark covers (ADR 0210), not data. Encoding them means putting §7.10's four
//! function types and `pdf-model`'s colour conversion on the wire, and ADR 0607 deferred that
//! with a reason: **the raster arm covers exactly those pages**, and for a page whose colours are
//! a function of position at the device's own resolution, pixels are what this boundary is for.
//! So [`encode`] refuses, naming the variant, and [`crossing`] turns that refusal into the
//! payload choice rather than into a failure.
//!
//! # Why the whole clip and soft-mask tables cross, not only the reachable part
//!
//! A [`pdf_render::ClipId`] is an index, so dropping an unreferenced entry would renumber every
//! identifier after it. The tables are written whole and the decoder checks that rebuilding them
//! reproduces the message's own numbering — see [`Unbuildable`](ProtocolError::Unbuildable),
//! which is what refuses a message stating one clip region twice.

use std::collections::HashMap;
use std::sync::Arc;

use pdf_render::{
    BlendMode, BlendingSpace, Clip, ClipId, Color, ColourCube, Command, Corners, DisplayList,
    FillRule, GreyCurve, GroupBlending, Image, ImageSource, LineCap, LineJoin, Luminance,
    MAX_GROUP_DEPTH, Paint, Path, PathCommand, Point, Ramp, Rect, Shading, ShadingKind, Size,
    SoftMask, SoftMaskId, SoftMaskKind, Stop, Stroke, Transfer, Transform, Triangle,
};

use super::{ProtocolError, Reader, Writer};

/// Why a display list cannot be written down, naming what stopped it.
///
/// Every variant is a *refusal*, not a failure: the caller's answer to all of them is ADR 0607's
/// raster arm, which is why [`crossing`] returns a payload rather than a `Result`. Naming the
/// variant that stopped it is `doc/traps/parsers-and-streams.md` trap 5 — unsupported input stays
/// loud — and it is what makes the deferred producers a documented boundary rather than a page
/// that quietly went blank.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum Uncodable {
    /// `ImageSource::AtDeviceScale` names a producer of samples rather than samples.
    #[error(
        "`ImageSource::AtDeviceScale` carries a producer invoked at the device's own scale, \
         which this format does not carry"
    )]
    DeferredImage,
    /// `ShadingKind::Sampled` names a producer of colours rather than colours.
    #[error(
        "`ShadingKind::Sampled` carries a producer invoked at the device's own scale, \
         which this format does not carry"
    )]
    DeferredColours,
    /// The list nests deeper than any backend in this tree composites.
    #[error("a display list nesting {depth} deep is past the {MAX_GROUP_DEPTH} a backend draws")]
    TooDeep {
        /// The depth reached.
        depth: usize,
    },
    /// More distinct shared objects of one kind than an index can address.
    #[error("more distinct {what} than a 32-bit index can address")]
    TooMany {
        /// Which table.
        what: &'static str,
    },
    /// The list passed the size its own raster would be, so the encoder stopped.
    ///
    /// **A cost this format used to pay once per corpus run and, on the frame path, would pay
    /// once per frame.** A scanned page is one `Command::Image` over tens of megabytes of
    /// samples: finishing its encoding to learn that pixels are the smaller payload costs the
    /// confined process that whole allocation and the milliseconds to make it — 33.7 MB and 7.7
    /// ms on `scan-bad.pdf`, against a page that takes 49 ms to draw. So [`crossing`] hands the
    /// encoder the number it is about to compare against and the encoder stops at it.
    ///
    /// `written` is a **lower bound** rather than the list's price: what the encoder had
    /// accounted for when it stopped. `examples/list_over_the_wire` is where an exact figure
    /// comes from, and it gets one because [`encode`] passes no budget and never returns this.
    #[error("a display list of at least {written} bytes is past the {budget} its raster costs")]
    TooLarge {
        /// What had been written or accounted for when the encoder stopped.
        written: usize,
        /// What it was being compared against.
        budget: usize,
    },
    /// A variant of one of `pdf-render`'s `#[non_exhaustive]` enums this format does not know.
    ///
    /// Four of the types crossing here are open — [`Command`], [`Paint`], [`ImageSource`] and
    /// [`ShadingKind`] — so the compiler requires a wildcard arm and cannot make an addition a
    /// build failure the way it does for the closed ones. This is that arm, and it refuses
    /// rather than drawing the variant as something else.
    #[error("{what} is a variant this format does not carry")]
    Unknown {
        /// Which enumeration reached its wildcard arm.
        what: &'static str,
    },
}

/// Which of ADR 0607's two payloads one page crosses as.
///
/// The choice is per page and by size, which is the whole of the decision: a display list is
/// scale-invariant and a raster is quadratic in the scale, so a list is smaller for almost every
/// page and larger for exactly one population — a scanned page, whose decoded samples *are* its
/// display list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Crossing {
    /// The encoded list, which was smaller than the pixels it produces.
    List(Vec<u8>),
    /// The page crosses as pixels, and this says why.
    Raster(RasterReason),
}

/// Why a page crosses as pixels rather than as a list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum RasterReason {
    /// The list would be at least as large as the raster.
    ///
    /// The scanned page: one `Command::Image` whose samples are most of the file.
    #[error("the encoded list is at least {list} bytes against a raster of {raster}")]
    Larger {
        /// What the encoded list came to, or a **lower bound** where the encoder stopped early.
        ///
        /// See [`Uncodable::TooLarge`]: the encoder is handed the raster's size and stops when it
        /// passes it, so on a scanned page this is what it had accounted for rather than what the
        /// whole list would have cost. The two are the same number for every page that crosses as
        /// a list, because that encoder ran to the end.
        list: usize,
        /// What the target's pixels come to.
        raster: u64,
    },
    /// The list holds something this format refuses, named by [`Uncodable`].
    #[error("{0}")]
    Uncodable(#[from] Uncodable),
}

/// The payload one page crosses as, given what its raster would cost.
///
/// `raster_bytes` is the target's own pixel count times four, which the confined process computes
/// from [`pdf_render::TargetSpec`] without rasterising anything — so both sides of the comparison
/// are known before either payload is produced, which is what ADR 0607's decision asks for.
///
/// **The list is encoded before it is compared, and that is deliberate.** The alternative is a
/// second implementation that predicts an encoded size, and two statements of one format is how
/// they drift; the cost of being wrong is one buffer, on the four percent of pages where the
/// answer is pixels.
#[must_use]
pub(crate) fn crossing(list: &DisplayList, raster_bytes: u64) -> Crossing {
    let budget = usize::try_from(raster_bytes).unwrap_or(usize::MAX);
    match write_down(list, budget) {
        Err(Uncodable::TooLarge { written, .. }) => Crossing::Raster(RasterReason::Larger {
            list: written,
            raster: raster_bytes,
        }),
        Err(refusal) => Crossing::Raster(RasterReason::Uncodable(refusal)),
        Ok(bytes) if super::as_u64(bytes.len()) >= raster_bytes => {
            Crossing::Raster(RasterReason::Larger {
                list: bytes.len(),
                raster: raster_bytes,
            })
        }
        Ok(bytes) => Crossing::List(bytes),
    }
}

/// Writes one display list down.
///
/// # Errors
///
/// [`Uncodable`] where the list holds a deferred producer, nests past
/// [`pdf_render::MAX_GROUP_DEPTH`], or reaches the wildcard arm of one of `pdf-render`'s open
/// enumerations. Every one of those is ADR 0607's raster arm rather than a failure; [`crossing`]
/// is the caller that says so.
pub(crate) fn encode(list: &DisplayList) -> Result<Vec<u8>, Uncodable> {
    write_down(list, usize::MAX)
}

/// The encoder, with the number it may stop at.
///
/// `budget` is what the caller is going to compare the result against, and handing it in is what
/// turns "encode it and see" into "stop when the answer is known". [`Uncodable::TooLarge`] has
/// the measurement that made it worth doing; [`encode`] passes [`usize::MAX`], where no check can
/// fire, and is therefore the exact price.
fn write_down(list: &DisplayList, budget: usize) -> Result<Vec<u8>, Uncodable> {
    let mut writer = Writer::new();
    write_list(&mut writer, list, true, budget)?;
    Ok(writer.finish())
}

/// Reads one display list back.
///
/// # Errors
///
/// [`ProtocolError`] where a field is truncated, a count is larger than the message that states
/// it, an identifier is past the table it indexes, a discriminant is not one this build defines,
/// the list nests too deep, or bytes are left over.
pub(crate) fn decode(bytes: &[u8]) -> Result<DisplayList, ProtocolError> {
    let mut reader = Reader::new(bytes);
    let list = read_list(&mut reader, true)?;
    reader.end("a display list")?;
    Ok(list)
}

/// A table's element count, refused where the message is too short to hold it.
///
/// `least` is the smallest number of bytes one element of this table occupies. Multiplying is
/// what makes the check tighter than [`Reader::list`]'s, which can only assume one byte an
/// element: a count of 2^28 clips in a nine-byte message is refused here before a reservation
/// rather than after 2^28 truncation errors.
///
/// The reservation is bounded separately, by [`super::RESERVE`], for the reason that constant
/// gives — a checked count is still a count the other side chose.
fn count(
    reader: &mut Reader<'_>,
    what: &'static str,
    least: usize,
) -> Result<usize, ProtocolError> {
    let claimed = reader.usize(what)?;
    let needed = claimed.saturating_mul(least.max(1));
    if needed > reader.remaining() {
        return Err(ProtocolError::Overlong {
            what,
            claimed: needed,
            available: reader.remaining(),
        });
    }
    Ok(claimed)
}

/// Reads `count` elements, reserving a bounded prefix and growing into the rest.
fn table<T>(
    reader: &mut Reader<'_>,
    what: &'static str,
    least: usize,
    mut element: impl FnMut(&mut Reader<'_>) -> Result<T, ProtocolError>,
) -> Result<Vec<T>, ProtocolError> {
    let claimed = count(reader, what, least)?;
    let mut out = Vec::new();
    out.try_reserve(claimed.min(super::RESERVE))
        .map_err(|_| ProtocolError::NoRoom {
            what,
            bytes: claimed.min(super::RESERVE),
        })?;
    for _ in 0..claimed {
        out.push(element(reader)?);
    }
    Ok(out)
}

/// An index into a table the decoder has already read whole.
///
/// **This is the codec's central check.** Everything a display list is made of is reached
/// through one of these, and `held` is always a length this decoder established by reading the
/// table rather than a length the message stated — which is what the table-before-body layout
/// buys.
fn index(reader: &mut Reader<'_>, what: &'static str, held: usize) -> Result<u32, ProtocolError> {
    let at = reader.u32(what)?;
    if at_least(at) >= held {
        return Err(ProtocolError::OutOfTable {
            what,
            index: at_least(at),
            held,
        });
    }
    Ok(at)
}

/// An optional index into a table the decoder has already read whole.
fn option_index(
    reader: &mut Reader<'_>,
    what: &'static str,
    held: usize,
) -> Result<Option<u32>, ProtocolError> {
    if reader.bool(what)? {
        Ok(Some(index(reader, what, held)?))
    } else {
        Ok(None)
    }
}

/// An index as a machine word, never smaller than it was.
///
/// A `u32` fits a `usize` on every platform this runs on; `usize::MAX` on one where it did not
/// would fail every bounds check rather than pass one, which is the direction a conversion in a
/// decoder has to err in.
fn at_least(index: u32) -> usize {
    usize::try_from(index).unwrap_or(usize::MAX)
}

/// An identifier's own index, as the `u32` it is inside.
///
/// # Errors
///
/// [`Uncodable::TooMany`] where a table has grown past what an identifier can address, which
/// [`DisplayList::add_clip`] and [`DisplayList::add_soft_mask`] both already refuse to do.
fn identifier(what: &'static str, index: usize) -> Result<u32, Uncodable> {
    u32::try_from(index).map_err(|_| Uncodable::TooMany { what })
}

// ---------------------------------------------------------------------------------------------
// The shared tables
// ---------------------------------------------------------------------------------------------

/// The distinct shared objects one list holds, in the order the command stream first names them.
///
/// Identity is the `Arc`'s address, which is what "the same image referenced twice" means and
/// what [`pdf_render::DeferredImage`]'s own `PartialEq` already uses. Two structurally equal
/// paths held behind different `Arc`s stay two entries: this preserves the sharing the
/// interpreter produced rather than looking for sharing it did not.
#[derive(Default)]
struct Tables {
    paths: Interned<Path>,
    samples: Interned<[u8]>,
    kinds: Interned<ShadingKind>,
    shadings: Interned<Shading>,
}

/// One table of shared objects, and where each of them went.
struct Interned<T: ?Sized> {
    order: Vec<Arc<T>>,
    at: HashMap<usize, u32>,
}

impl<T: ?Sized> Default for Interned<T> {
    fn default() -> Self {
        Self {
            order: Vec::new(),
            at: HashMap::new(),
        }
    }
}

impl<T: ?Sized> Interned<T> {
    /// This object's index, adding it to the table the first time it is seen.
    fn intern(&mut self, what: &'static str, value: &Arc<T>) -> Result<u32, Uncodable> {
        let identity = Arc::as_ptr(value).cast::<u8>() as usize;
        if let Some(existing) = self.at.get(&identity) {
            return Ok(*existing);
        }
        let next = u32::try_from(self.order.len()).map_err(|_| Uncodable::TooMany { what })?;
        self.order.push(Arc::clone(value));
        self.at.insert(identity, next);
        Ok(next)
    }
}

// ---------------------------------------------------------------------------------------------
// Encoding
// ---------------------------------------------------------------------------------------------

/// One whole list: its tables, then everything that indexes into them.
///
/// `page` is false for the companion list [`DisplayList::black`] holds, which may not carry a
/// blending space of its own — §11.4.7's space is the *page's*, and refusing a second one bounds
/// this recursion at one level rather than at whatever a message asserts.
fn write_list(
    writer: &mut Writer,
    list: &DisplayList,
    page: bool,
    budget: usize,
) -> Result<(), Uncodable> {
    let mut tables = Tables::default();
    let mut body = Writer::new();

    // The body is written first and appended last, because interning happens while it is
    // written and the tables have to reach the decoder in front of the indices that name them.
    write_clips(&mut body, list)?;
    write_soft_masks(&mut body, list, &mut tables)?;
    write_commands(&mut body, list.commands(), &mut tables, 0)?;

    // **The two places the budget can be known before the bytes exist**, and between them they
    // cover both shapes of an oversized list. The body is written, so its length is a fact. The
    // samples table is not written yet, and interning holds the `Arc`s rather than copying them
    // — so summing the slices it is about to write is a lower bound on this format's output
    // whatever the format later decides to write *around* them, which is what keeps this from
    // being a second statement of the format. A scanned page is refused here, before its
    // thirty-three megabytes are copied anywhere.
    let interned = tables
        .samples
        .order
        .iter()
        .fold(0usize, |sum, samples| sum.saturating_add(samples.len()));
    let accounted = body.len().saturating_add(interned);
    if accounted >= budget {
        return Err(Uncodable::TooLarge {
            written: accounted,
            budget,
        });
    }

    writer.f32(list.page_size.width).f32(list.page_size.height);
    match list.content_clip() {
        Some(region) => {
            writer.u8(1);
            write_rect(writer, region);
        }
        None => {
            writer.u8(0);
        }
    }
    write_tables(writer, &tables)?;
    writer.append(&body);

    match (
        page,
        list.blending(),
        list.black(),
        list.grey_curve(),
        list.colour_cube(),
    ) {
        (true, Some(space), Some(black), _, _) => {
            writer.u8(1);
            write_blending_space(writer, space);
            write_list(writer, black, false, budget.saturating_sub(writer.len()))?;
        }
        // §11.4.7's one-component form: the curve the composited component leaves by.
        (true, None, None, Some(curve), _) => {
            writer.u8(2);
            write_grey_curve(writer, curve);
        }
        // And its three-component form: the cube the composited components leave by.
        (true, None, None, None, Some(cube)) => {
            writer.u8(3);
            write_colour_cube(writer, cube);
        }
        // A list with a space and no companion, or a companion and no space, cannot be built
        // through `set_blending`, which takes both. Writing the absent case is the honest
        // encoding of a value that says nothing.
        _ => {
            writer.u8(0);
        }
    }
    Ok(())
}

/// The four shared tables, in the order the decoder needs them: nothing here names anything
/// later than itself except a shading, which names its kind.
fn write_tables(writer: &mut Writer, tables: &Tables) -> Result<(), Uncodable> {
    writer.usize(tables.paths.order.len());
    for path in &tables.paths.order {
        write_path(writer, path);
    }

    writer.usize(tables.samples.order.len());
    for samples in &tables.samples.order {
        writer.bytes(samples);
    }

    writer.usize(tables.kinds.order.len());
    for kind in &tables.kinds.order {
        write_shading_kind(writer, kind)?;
    }

    writer.usize(tables.shadings.order.len());
    for shading in &tables.shadings.order {
        // The kind is already interned: `intern_shading` interns both together, so this lookup
        // cannot miss. It is written as a lookup rather than as an assumption because the
        // alternative is an index nobody checked.
        let at = tables
            .kinds
            .at
            .get(&(Arc::as_ptr(&shading.kind).cast::<u8>() as usize))
            .copied()
            .ok_or(Uncodable::Unknown {
                what: "a shading whose kind was never interned",
            })?;
        writer.u32(at);
        write_transform(writer, shading.transform);
        match shading.background {
            Some(colour) => {
                writer.u8(1);
                write_colour(writer, colour);
            }
            None => {
                writer.u8(0);
            }
        }
    }
    Ok(())
}

fn write_clips(writer: &mut Writer, list: &DisplayList) -> Result<(), Uncodable> {
    writer.usize(list.clip_count());
    for at in 0..list.clip_count() {
        let at = identifier("clips", at)?;
        // `clip_count` is the table's own length, so this cannot miss; the `else` is the
        // refusal a caller can read rather than an `unwrap` with a comment.
        let Some(clip) = list.clip(ClipId::new(at)) else {
            return Err(Uncodable::Unknown {
                what: "a clip identifier the list's own table does not hold",
            });
        };
        let Clip {
            path,
            transform,
            fill_rule,
            parent,
        } = clip;
        // A clip's path is not shared through an `Arc`, so it is written in place rather than
        // interned: `Clip` owns its geometry and no two clips in a table are equal at all.
        write_path(writer, path);
        write_transform(writer, *transform);
        writer.u8(fill_rule_tag(*fill_rule));
        match parent {
            Some(parent) => {
                writer.u8(1).u32(identifier("clips", parent.index())?);
            }
            None => {
                writer.u8(0);
            }
        }
    }
    Ok(())
}

fn write_soft_masks(
    writer: &mut Writer,
    list: &DisplayList,
    tables: &mut Tables,
) -> Result<(), Uncodable> {
    writer.usize(list.soft_mask_count());
    for at in 0..list.soft_mask_count() {
        let at = identifier("soft masks", at)?;
        let Some(mask) = list.soft_mask(SoftMaskId::new(at)) else {
            return Err(Uncodable::Unknown {
                what: "a soft mask identifier the list's own table does not hold",
            });
        };
        let SoftMask {
            commands,
            kind,
            transfer,
            luminance,
        } = mask;
        match kind {
            SoftMaskKind::Alpha => {
                writer.u8(1);
            }
            SoftMaskKind::Luminosity { backdrop } => {
                writer.u8(2);
                write_colour(writer, *backdrop);
            }
        }
        match transfer {
            Some(transfer) => {
                writer.u8(1);
                // `Transfer` hands out no table, and `apply` is a lookup in it, so the 256
                // values it can be asked about *are* the table — exactly, not approximately.
                for value in 0..=u8::MAX {
                    writer.u8(transfer.apply(value));
                }
            }
            None => {
                writer.u8(0);
            }
        }
        // §11.5.3's `Y`, where the group composited a CIE-based space's components: three
        // curves the reader sums, or a grid it interpolates. The tag tells the two apart, so
        // a list stating one cannot be read back as the other.
        match luminance {
            Some(luminance) => {
                if let Some(curves) = luminance.as_curves() {
                    writer.u8(1);
                    for sample in curves {
                        for component in sample {
                            writer.f32(*component);
                        }
                    }
                } else if let Some((side, samples)) = luminance.as_grid() {
                    writer.u8(2);
                    writer.usize(side);
                    for sample in samples {
                        writer.f32(*sample);
                    }
                } else {
                    return Err(Uncodable::Unknown {
                        what: "a soft mask's luminance in a shape this build cannot write",
                    });
                }
            }
            None => {
                writer.u8(0);
            }
        }
        write_commands(writer, commands, tables, 0)?;
    }
    Ok(())
}

fn write_commands(
    writer: &mut Writer,
    commands: &[Command],
    tables: &mut Tables,
    depth: usize,
) -> Result<(), Uncodable> {
    if depth > MAX_GROUP_DEPTH {
        return Err(Uncodable::TooDeep { depth });
    }
    writer.usize(commands.len());
    for command in commands {
        write_command(writer, command, tables, depth)?;
    }
    Ok(())
}

fn write_command(
    writer: &mut Writer,
    command: &Command,
    tables: &mut Tables,
    depth: usize,
) -> Result<(), Uncodable> {
    match command {
        Command::Fill {
            path,
            transform,
            fill_rule,
            paint,
            clip,
            mask,
            blend,
        } => {
            writer.u8(TAG_FILL);
            writer.u32(tables.paths.intern("paths", path)?);
            write_transform(writer, *transform);
            writer.u8(fill_rule_tag(*fill_rule));
            write_paint(writer, paint, tables)?;
            write_mark_state(writer, *clip, *mask, *blend)?;
        }
        Command::Stroke {
            path,
            transform,
            stroke,
            paint,
            clip,
            mask,
            blend,
        } => {
            writer.u8(TAG_STROKE);
            writer.u32(tables.paths.intern("paths", path)?);
            write_transform(writer, *transform);
            write_stroke(writer, stroke);
            write_paint(writer, paint, tables)?;
            write_mark_state(writer, *clip, *mask, *blend)?;
        }
        Command::Image {
            image,
            transform,
            alpha,
            clip,
            mask,
            blend,
        } => {
            writer.u8(TAG_IMAGE);
            match image {
                ImageSource::Decoded(decoded) => {
                    writer.u8(1);
                    writer.u32(decoded.width).u32(decoded.height);
                    writer.bool(decoded.interpolate);
                    writer.u32(tables.samples.intern("image samples", &decoded.data)?);
                }
                ImageSource::AtDeviceScale(_) => return Err(Uncodable::DeferredImage),
                _ => {
                    return Err(Uncodable::Unknown {
                        what: "an `ImageSource`",
                    });
                }
            }
            write_transform(writer, *transform);
            writer.f32(*alpha);
            write_mark_state(writer, *clip, *mask, *blend)?;
        }
        Command::Group { .. } => write_group(writer, command, tables, depth)?,
        Command::Shaped { object, shape } => {
            writer.u8(TAG_SHAPED);
            let deeper = depth.saturating_add(1);
            if deeper > MAX_GROUP_DEPTH {
                return Err(Uncodable::TooDeep { depth: deeper });
            }
            write_command(writer, object, tables, deeper)?;
            write_command(writer, shape, tables, deeper)?;
        }
        // `Command` is `#[non_exhaustive]`, so the compiler requires this arm and cannot make a
        // variant added in `pdf-render` a build failure here. Refusing by name is what keeps
        // such a variant from being drawn as something else.
        _ => {
            return Err(Uncodable::Unknown {
                what: "a `Command`",
            });
        }
    }
    Ok(())
}

/// §11.4's group, which is the one command whose body is other commands.
///
/// Its own function rather than an arm, because the arm carried a fifth of this module: a
/// group states eight fields, one of which is §11.7.2's four-component pair and two of which
/// are command lists.
fn write_group(
    writer: &mut Writer,
    command: &Command,
    tables: &mut Tables,
    depth: usize,
) -> Result<(), Uncodable> {
    let Command::Group {
        commands,
        alpha,
        clip,
        mask,
        blend,
        isolated,
        knockout,
        alpha_is_shape,
        blending,
    } = command
    else {
        return Err(Uncodable::Unknown {
            what: "a command routed to the group encoder",
        });
    };
    writer.u8(TAG_GROUP);
    writer.f32(*alpha);
    write_mark_state(writer, *clip, *mask, *blend)?;
    writer.bool(*isolated).bool(*knockout).bool(*alpha_is_shape);
    match blending.as_deref() {
        Some(GroupBlending::FourComponents { space, black }) => {
            writer.u8(1);
            write_blending_space(writer, space);
            write_commands(writer, black, tables, depth.saturating_add(1))?;
        }
        Some(GroupBlending::OneComponent { curve }) => {
            writer.u8(2);
            write_grey_curve(writer, curve);
        }
        Some(GroupBlending::ThreeComponents { cube }) => {
            writer.u8(3);
            write_colour_cube(writer, cube);
        }
        None => {
            writer.u8(0);
        }
    }
    write_commands(writer, commands, tables, depth.saturating_add(1))
}

fn write_paint(writer: &mut Writer, paint: &Paint, tables: &mut Tables) -> Result<(), Uncodable> {
    match paint {
        Paint::Solid(colour) => {
            writer.u8(1);
            write_colour(writer, *colour);
        }
        Paint::Shading(shading) => {
            writer.u8(2);
            // The kind is interned with the shading, so that the table written for the one is
            // complete when the table written for the other is walked.
            tables.kinds.intern("shading kinds", &shading.kind)?;
            let at = tables.shadings.intern("shadings", shading)?;
            // A shading interned earlier already interned its kind; doing it again is a
            // dictionary hit and keeps the two tables in step without a second walk.
            writer.u32(at);
        }
        _ => {
            return Err(Uncodable::Unknown { what: "a `Paint`" });
        }
    }
    Ok(())
}

fn write_shading_kind(writer: &mut Writer, kind: &ShadingKind) -> Result<(), Uncodable> {
    match kind {
        ShadingKind::Axial {
            start,
            end,
            ramp,
            extend,
        } => {
            writer.u8(1);
            write_point(writer, *start);
            write_point(writer, *end);
            write_ramp(writer, ramp);
            writer.bool(extend.0).bool(extend.1);
        }
        ShadingKind::Radial {
            start,
            start_radius,
            end,
            end_radius,
            ramp,
            extend,
        } => {
            writer.u8(2);
            write_point(writer, *start);
            writer.f32(*start_radius);
            write_point(writer, *end);
            writer.f32(*end_radius);
            write_ramp(writer, ramp);
            writer.bool(extend.0).bool(extend.1);
        }
        ShadingKind::Mesh { triangles, ramp } => {
            writer.u8(3);
            writer.usize(triangles.len());
            for triangle in triangles.iter() {
                let Triangle { points, corners } = triangle;
                for point in points {
                    write_point(writer, *point);
                }
                match corners {
                    Corners::Colours(colours) => {
                        writer.u8(1);
                        for colour in colours {
                            write_colour(writer, *colour);
                        }
                    }
                    Corners::Parameters(parameters) => {
                        writer.u8(2);
                        for parameter in parameters {
                            writer.f32(*parameter);
                        }
                    }
                }
            }
            match ramp {
                Some(ramp) => {
                    writer.u8(1);
                    write_ramp(writer, ramp);
                }
                None => {
                    writer.u8(0);
                }
            }
        }
        ShadingKind::Sampled { .. } => return Err(Uncodable::DeferredColours),
        _ => {
            return Err(Uncodable::Unknown {
                what: "a `ShadingKind`",
            });
        }
    }
    Ok(())
}

fn write_ramp(writer: &mut Writer, ramp: &Ramp) {
    writer.usize(ramp.stops.len());
    for stop in ramp.stops.iter() {
        writer.f32(stop.at);
        write_colour(writer, stop.colour);
    }
}

fn write_stroke(writer: &mut Writer, stroke: &Stroke) {
    let Stroke {
        width,
        adjust,
        cap,
        join,
        miter_limit,
        dash_array,
        dash_phase,
    } = stroke;
    writer.f32(*width);
    writer.bool(*adjust);
    writer.u8(match cap {
        LineCap::Butt => 0,
        LineCap::Round => 1,
        LineCap::Square => 2,
    });
    writer.u8(match join {
        LineJoin::Miter => 0,
        LineJoin::Round => 1,
        LineJoin::Bevel => 2,
    });
    writer.f32(*miter_limit);
    writer.usize(dash_array.len());
    for length in dash_array {
        writer.f32(*length);
    }
    writer.f32(*dash_phase);
}

fn write_mark_state(
    writer: &mut Writer,
    clip: Option<ClipId>,
    mask: Option<SoftMaskId>,
    blend: BlendMode,
) -> Result<(), Uncodable> {
    match clip {
        Some(clip) => {
            writer.u8(1).u32(identifier("clips", clip.index())?);
        }
        None => {
            writer.u8(0);
        }
    }
    match mask {
        Some(mask) => {
            writer.u8(1).u32(identifier("soft masks", mask.index())?);
        }
        None => {
            writer.u8(0);
        }
    }
    writer.u8(blend_tag(blend));
    Ok(())
}

fn write_blending_space(writer: &mut Writer, space: &BlendingSpace) {
    writer.usize(space.side());
    writer.usize(space.grid().len());
    for sample in space.grid() {
        for component in sample {
            writer.f32(*component);
        }
    }
}

fn write_grey_curve(writer: &mut Writer, curve: &GreyCurve) {
    writer.usize(curve.samples().len());
    for sample in curve.samples() {
        for component in sample {
            writer.f32(*component);
        }
    }
}

fn write_colour_cube(writer: &mut Writer, cube: &ColourCube) {
    writer.usize(cube.input().len());
    for sample in cube.input() {
        for component in sample {
            writer.f32(*component);
        }
    }
    writer.usize(cube.side());
    writer.usize(cube.grid().len());
    for sample in cube.grid() {
        for component in sample {
            writer.f32(*component);
        }
    }
    writer.usize(cube.output().len());
    for sample in cube.output() {
        writer.f32(*sample);
    }
}

fn write_path(writer: &mut Writer, path: &Path) {
    writer.usize(path.commands().len());
    for step in path.commands() {
        match *step {
            PathCommand::MoveTo(point) => {
                writer.u8(1);
                write_point(writer, point);
            }
            PathCommand::LineTo(point) => {
                writer.u8(2);
                write_point(writer, point);
            }
            PathCommand::CurveTo(a, b, c) => {
                writer.u8(3);
                write_point(writer, a);
                write_point(writer, b);
                write_point(writer, c);
            }
            PathCommand::Close => {
                writer.u8(4);
            }
        }
    }
}

fn write_point(writer: &mut Writer, point: Point) {
    writer.f32(point.x).f32(point.y);
}

fn write_rect(writer: &mut Writer, rect: Rect) {
    write_point(writer, rect.min);
    write_point(writer, rect.max);
}

fn write_colour(writer: &mut Writer, colour: Color) {
    writer
        .f32(colour.r)
        .f32(colour.g)
        .f32(colour.b)
        .f32(colour.a);
}

/// The six numbers of a matrix, in `pdf-render`'s own order.
///
/// `pub(super)` because a frame's list arm carries the target the host is to draw at, and a
/// target is two dimensions and one of these. One statement of the layout rather than two, for
/// the reason this module gives about every other field it writes.
pub(super) fn write_transform(writer: &mut Writer, transform: Transform) {
    writer
        .f32(transform.a)
        .f32(transform.b)
        .f32(transform.c)
        .f32(transform.d)
        .f32(transform.e)
        .f32(transform.f);
}

/// Command tags. One byte each, and the numbering is this format's rather than the enum's
/// declaration order, so that reordering `Command`'s variants cannot silently change the wire.
const TAG_FILL: u8 = 1;
const TAG_IMAGE: u8 = 2;
const TAG_GROUP: u8 = 3;
const TAG_SHAPED: u8 = 4;
const TAG_STROKE: u8 = 5;

fn fill_rule_tag(rule: FillRule) -> u8 {
    match rule {
        FillRule::NonZero => 0,
        FillRule::EvenOdd => 1,
    }
}

/// The sixteen modes of §11.3.5, in the order the standard's own table lists them.
///
/// Written out both ways rather than derived from the discriminant: `BlendMode` is a closed
/// enumeration, so naming every variant here is what makes an addition to it a build failure in
/// this file instead of a mode that crossed as a different one.
fn blend_tag(blend: BlendMode) -> u8 {
    match blend {
        BlendMode::Normal => 0,
        BlendMode::Multiply => 1,
        BlendMode::Screen => 2,
        BlendMode::Overlay => 3,
        BlendMode::Darken => 4,
        BlendMode::Lighten => 5,
        BlendMode::ColorDodge => 6,
        BlendMode::ColorBurn => 7,
        BlendMode::HardLight => 8,
        BlendMode::SoftLight => 9,
        BlendMode::Difference => 10,
        BlendMode::Exclusion => 11,
        BlendMode::Hue => 12,
        BlendMode::Saturation => 13,
        BlendMode::Color => 14,
        BlendMode::Luminosity => 15,
    }
}

// ---------------------------------------------------------------------------------------------
// Decoding
// ---------------------------------------------------------------------------------------------

/// The smallest number of bytes each table's element can occupy, for [`count`].
mod least {
    /// A path: its own step count.
    pub(super) const PATH: usize = 8;
    /// A length-prefixed run of samples.
    pub(super) const SAMPLES: usize = 8;
    /// A shading kind: a tag and the smallest body any of them has.
    pub(super) const KIND: usize = 1 + 8;
    /// A shading: a kind index, a transform and a presence byte.
    pub(super) const SHADING: usize = 4 + 24 + 1;
    /// A clip: a step count, a transform, a fill rule and a presence byte.
    pub(super) const CLIP: usize = 8 + 24 + 1 + 1;
    /// A soft mask: a kind tag, a transfer presence byte and a command count.
    pub(super) const SOFT_MASK: usize = 1 + 1 + 8;
    /// A command: its tag.
    pub(super) const COMMAND: usize = 1;
    /// A path step: its tag.
    pub(super) const STEP: usize = 1;
    /// A ramp stop: a position and a colour.
    pub(super) const STOP: usize = 4 + 16;
    /// A mesh triangle: three points, a corner tag and three parameters.
    pub(super) const TRIANGLE: usize = 24 + 1 + 12;
    /// A dash length.
    pub(super) const DASH: usize = 4;
    /// A blending grid sample: three components.
    pub(super) const SAMPLE: usize = 12;
}

/// Everything one table-and-body pair decodes to. `page` mirrors [`write_list`]'s.
fn read_list(reader: &mut Reader<'_>, page: bool) -> Result<DisplayList, ProtocolError> {
    let size = Size::new(
        reader.f32("a page's width")?,
        reader.f32("a page's height")?,
    );
    let mut list = DisplayList::new(size);
    if reader.bool("a content clip")? {
        list.set_content_clip(read_rect(reader, "a content clip")?);
    }

    let paths = table(reader, "a display list's paths", least::PATH, |reader| {
        Ok(Arc::new(read_path(reader, "a path")?))
    })?;
    let samples = table(
        reader,
        "a display list's image samples",
        least::SAMPLES,
        |reader| {
            // The one allocation on this boundary whose size the other side chose, and therefore
            // the one that is a `try_reserve` — `Reader::owned_bytes`'s own rule. The `Arc`
            // copies it a second time, which is what stable Rust can do without `unsafe`.
            Ok(Arc::<[u8]>::from(reader.owned_bytes("an image's samples")?))
        },
    )?;
    let kinds = table(
        reader,
        "a display list's shading kinds",
        least::KIND,
        |reader| Ok(Arc::new(read_shading_kind(reader)?)),
    )?;
    let held = kinds.len();
    let shadings = table(
        reader,
        "a display list's shadings",
        least::SHADING,
        |reader| {
            let at = at_least(index(reader, "a shading's kind", held)?);
            let kind = kinds.get(at).ok_or(ProtocolError::OutOfTable {
                what: "a shading's kind",
                index: at,
                held,
            })?;
            Ok(Arc::new(Shading {
                kind: Arc::clone(kind),
                transform: read_transform(reader, "a shading's transform")?,
                background: if reader.bool("a shading's background")? {
                    Some(read_colour(reader, "a shading's background")?)
                } else {
                    None
                },
            }))
        },
    )?;

    read_clips(reader, &mut list)?;
    let clips = list.clip_count();
    let masks = count(reader, "a display list's soft masks", least::SOFT_MASK)?;
    for _ in 0..masks {
        let mask = read_soft_mask(reader, &paths, &samples, &shadings, clips, masks)?;
        list.add_soft_mask(mask)
            .map_err(|_| ProtocolError::Unbuildable {
                what: "a display list's soft masks",
                why: "more of them than an identifier can address",
            })?;
    }

    let commands = read_commands(reader, &paths, &samples, &shadings, clips, masks, 0)?;
    for command in commands {
        list.push(command);
    }

    match reader.u8("a page's blending space")? {
        0 => {}
        // The pair, the curve and the cube are three shapes of one statement — §11.4.7's
        // space is the *page's* — so the companion list may carry none of them.
        1..=3 if !page => {
            return Err(ProtocolError::Unbuildable {
                what: "a blending space",
                why: "the companion list carrying the black component states one of its own",
            });
        }
        1 => {
            let space = read_blending_space(reader)?;
            let black = read_list(reader, false)?;
            list.set_blending(space, black);
        }
        2 => list.set_grey_curve(read_grey_curve(reader)?),
        3 => list.set_colour_cube(read_colour_cube(reader)?),
        value => {
            return Err(ProtocolError::Unrecognised {
                what: "a page's blending space",
                value: u32::from(value),
            });
        }
    }
    Ok(list)
}

/// The clip table, rebuilt in the message's own order.
///
/// Two checks make this a decode rather than a suggestion, and both are refusals a hostile
/// message would otherwise turn into a wrong picture or a spin:
///
/// - **a parent must sit at a lower index than its child**, which is what
///   `DisplayList::add_clip` guarantees of any table it built and which is what keeps
///   `DisplayList::clip_bounds` from walking a cycle;
/// - **rebuilding must reproduce the numbering the message states.** `add_clip` hands back the
///   identifier of a region already in the table, so a message stating one region twice would
///   silently renumber every identifier after it and every command naming one would clip by the
///   wrong region.
fn read_clips(reader: &mut Reader<'_>, list: &mut DisplayList) -> Result<(), ProtocolError> {
    let claimed = count(reader, "a display list's clips", least::CLIP)?;
    for at in 0..claimed {
        let path = read_path(reader, "a clip's path")?;
        let transform = read_transform(reader, "a clip's transform")?;
        let fill_rule = read_fill_rule(reader, "a clip's fill rule")?;
        let parent = option_index(reader, "a clip's parent", at)?;
        let clip = Clip {
            path,
            transform,
            fill_rule,
            parent: parent.map(ClipId::new),
        };
        let rebuilt = list
            .add_clip(clip)
            .map_err(|_| ProtocolError::Unbuildable {
                what: "a display list's clips",
                why: "more of them than an identifier can address",
            })?;
        if rebuilt.index() != at {
            return Err(ProtocolError::Unbuildable {
                what: "a display list's clips",
                why: "the message states one clip region twice, which renumbers every \
                      identifier after it",
            });
        }
    }
    Ok(())
}

fn read_commands(
    reader: &mut Reader<'_>,
    paths: &[Arc<Path>],
    samples: &[Arc<[u8]>],
    shadings: &[Arc<Shading>],
    clips: usize,
    masks: usize,
    depth: usize,
) -> Result<Vec<Command>, ProtocolError> {
    if depth > MAX_GROUP_DEPTH {
        return Err(ProtocolError::TooDeep {
            what: "a display list's groups",
            limit: MAX_GROUP_DEPTH,
        });
    }
    let claimed = count(reader, "a display list's commands", least::COMMAND)?;
    let mut out = Vec::new();
    out.try_reserve(claimed.min(super::RESERVE))
        .map_err(|_| ProtocolError::NoRoom {
            what: "a display list's commands",
            bytes: claimed.min(super::RESERVE),
        })?;
    for _ in 0..claimed {
        out.push(read_command(
            reader, paths, samples, shadings, clips, masks, depth,
        )?);
    }
    Ok(out)
}

fn read_command(
    reader: &mut Reader<'_>,
    paths: &[Arc<Path>],
    samples: &[Arc<[u8]>],
    shadings: &[Arc<Shading>],
    clips: usize,
    masks: usize,
    depth: usize,
) -> Result<Command, ProtocolError> {
    let tag = reader.u8("a command")?;
    match tag {
        TAG_FILL => {
            let path = read_shared(reader, "a fill's path", paths)?;
            let transform = read_transform(reader, "a fill's transform")?;
            let fill_rule = read_fill_rule(reader, "a fill's fill rule")?;
            let paint = read_paint(reader, shadings)?;
            let (clip, mask, blend) = read_mark_state(reader, clips, masks)?;
            Ok(Command::Fill {
                path,
                transform,
                fill_rule,
                paint,
                clip,
                mask,
                blend,
            })
        }
        TAG_STROKE => {
            let path = read_shared(reader, "a stroke's path", paths)?;
            let transform = read_transform(reader, "a stroke's transform")?;
            let stroke = read_stroke(reader)?;
            let paint = read_paint(reader, shadings)?;
            let (clip, mask, blend) = read_mark_state(reader, clips, masks)?;
            Ok(Command::Stroke {
                path,
                transform,
                stroke,
                paint,
                clip,
                mask,
                blend,
            })
        }
        TAG_IMAGE => {
            let image = read_image(reader, samples)?;
            let transform = read_transform(reader, "an image's transform")?;
            let alpha = reader.f32("an image's alpha")?;
            let (clip, mask, blend) = read_mark_state(reader, clips, masks)?;
            Ok(Command::Image {
                image,
                transform,
                alpha,
                clip,
                mask,
                blend,
            })
        }
        TAG_GROUP => {
            let alpha = reader.f32("a group's alpha")?;
            let (clip, mask, blend) = read_mark_state(reader, clips, masks)?;
            let isolated = reader.bool("a group's isolation")?;
            let knockout = reader.bool("a group's knockout")?;
            let alpha_is_shape = reader.bool("a group's shape")?;
            let deeper = depth.saturating_add(1);
            let blending =
                read_group_blending(reader, paths, samples, shadings, clips, masks, deeper)?;
            let commands = read_commands(reader, paths, samples, shadings, clips, masks, deeper)?;
            Ok(Command::Group {
                commands,
                alpha,
                clip,
                mask,
                blend,
                isolated,
                knockout,
                alpha_is_shape,
                blending,
            })
        }
        TAG_SHAPED => {
            let deeper = depth.saturating_add(1);
            if deeper > MAX_GROUP_DEPTH {
                return Err(ProtocolError::TooDeep {
                    what: "a display list's shaped elements",
                    limit: MAX_GROUP_DEPTH,
                });
            }
            let object = read_command(reader, paths, samples, shadings, clips, masks, deeper)?;
            let shape = read_command(reader, paths, samples, shadings, clips, masks, deeper)?;
            Ok(Command::Shaped {
                object: Box::new(object),
                shape: Box::new(shape),
            })
        }
        value => Err(ProtocolError::Unrecognised {
            what: "a command",
            value: u32::from(value),
        }),
    }
}

fn read_shared<T: ?Sized>(
    reader: &mut Reader<'_>,
    what: &'static str,
    table: &[Arc<T>],
) -> Result<Arc<T>, ProtocolError> {
    let at = at_least(index(reader, what, table.len())?);
    table
        .get(at)
        .map(Arc::clone)
        .ok_or(ProtocolError::OutOfTable {
            what,
            index: at,
            held: table.len(),
        })
}

fn read_image(
    reader: &mut Reader<'_>,
    samples: &[Arc<[u8]>],
) -> Result<ImageSource, ProtocolError> {
    match reader.u8("an image's source")? {
        1 => {
            let width = reader.u32("an image's width")?;
            let height = reader.u32("an image's height")?;
            let interpolate = reader.bool("an image's interpolation")?;
            let data = read_shared(reader, "an image's samples", samples)?;
            let image = Image {
                width,
                height,
                data,
                interpolate,
            };
            // The invariant every backend indexes by, checked here rather than assumed there:
            // `fuzz/fuzz_targets/confined_wire.rs` already asserts the same of a `Raster`, and
            // the reason is the same — a stated dimension is a claim about a buffer.
            if !image.is_consistent() {
                return Err(ProtocolError::Unbuildable {
                    what: "an image",
                    why: "its stated dimensions and its sample count disagree",
                });
            }
            Ok(ImageSource::Decoded(image))
        }
        value => Err(ProtocolError::Unrecognised {
            what: "an image's source",
            value: u32::from(value),
        }),
    }
}

fn read_paint(reader: &mut Reader<'_>, shadings: &[Arc<Shading>]) -> Result<Paint, ProtocolError> {
    match reader.u8("a paint")? {
        1 => Ok(Paint::Solid(read_colour(reader, "a solid colour")?)),
        2 => Ok(Paint::Shading(read_shared(
            reader,
            "a paint's shading",
            shadings,
        )?)),
        value => Err(ProtocolError::Unrecognised {
            what: "a paint",
            value: u32::from(value),
        }),
    }
}

fn read_shading_kind(reader: &mut Reader<'_>) -> Result<ShadingKind, ProtocolError> {
    match reader.u8("a shading kind")? {
        1 => Ok(ShadingKind::Axial {
            start: read_point(reader, "an axial shading's start")?,
            end: read_point(reader, "an axial shading's end")?,
            ramp: read_ramp(reader)?,
            extend: (
                reader.bool("an axial shading's extend")?,
                reader.bool("an axial shading's extend")?,
            ),
        }),
        2 => Ok(ShadingKind::Radial {
            start: read_point(reader, "a radial shading's start")?,
            start_radius: reader.f32("a radial shading's start radius")?,
            end: read_point(reader, "a radial shading's end")?,
            end_radius: reader.f32("a radial shading's end radius")?,
            ramp: read_ramp(reader)?,
            extend: (
                reader.bool("a radial shading's extend")?,
                reader.bool("a radial shading's extend")?,
            ),
        }),
        3 => {
            let triangles = table(reader, "a mesh's triangles", least::TRIANGLE, |reader| {
                let mut points = [Point::new(0.0, 0.0); 3];
                for point in &mut points {
                    *point = read_point(reader, "a mesh triangle's corner")?;
                }
                let corners = match reader.u8("a mesh triangle's corners")? {
                    1 => {
                        let mut colours = [Color::TRANSPARENT; 3];
                        for colour in &mut colours {
                            *colour = read_colour(reader, "a mesh corner's colour")?;
                        }
                        Corners::Colours(colours)
                    }
                    2 => {
                        let mut parameters = [0.0_f32; 3];
                        for parameter in &mut parameters {
                            *parameter = reader.f32("a mesh corner's parameter")?;
                        }
                        Corners::Parameters(parameters)
                    }
                    value => {
                        return Err(ProtocolError::Unrecognised {
                            what: "a mesh triangle's corners",
                            value: u32::from(value),
                        });
                    }
                };
                Ok(Triangle { points, corners })
            })?;
            let ramp = if reader.bool("a mesh's ramp")? {
                Some(read_ramp(reader)?)
            } else {
                None
            };
            Ok(ShadingKind::Mesh {
                triangles: Arc::from(triangles),
                ramp,
            })
        }
        value => Err(ProtocolError::Unrecognised {
            what: "a shading kind",
            value: u32::from(value),
        }),
    }
}

fn read_ramp(reader: &mut Reader<'_>) -> Result<Ramp, ProtocolError> {
    let stops = table(reader, "a shading's ramp", least::STOP, |reader| {
        Ok(Stop {
            at: reader.f32("a ramp stop's position")?,
            colour: read_colour(reader, "a ramp stop's colour")?,
        })
    })?;
    // `Ramp`'s own documentation says its stops are "never empty", and every reader of one
    // depends on it. A message that states none is refused rather than handed to a rasteriser.
    if stops.is_empty() {
        return Err(ProtocolError::Unbuildable {
            what: "a shading's ramp",
            why: "a ramp with no stops states no colour",
        });
    }
    Ok(Ramp {
        stops: Arc::from(stops),
    })
}

fn read_stroke(reader: &mut Reader<'_>) -> Result<Stroke, ProtocolError> {
    let width = reader.f32("a stroke's width")?;
    let adjust = reader.bool("a stroke's adjustment")?;
    let cap = match reader.u8("a stroke's cap")? {
        0 => LineCap::Butt,
        1 => LineCap::Round,
        2 => LineCap::Square,
        value => {
            return Err(ProtocolError::Unrecognised {
                what: "a stroke's cap",
                value: u32::from(value),
            });
        }
    };
    let join = match reader.u8("a stroke's join")? {
        0 => LineJoin::Miter,
        1 => LineJoin::Round,
        2 => LineJoin::Bevel,
        value => {
            return Err(ProtocolError::Unrecognised {
                what: "a stroke's join",
                value: u32::from(value),
            });
        }
    };
    let miter_limit = reader.f32("a stroke's mitre limit")?;
    let dash_array = table(reader, "a stroke's dash array", least::DASH, |reader| {
        reader.f32("a dash length")
    })?;
    let dash_phase = reader.f32("a stroke's dash phase")?;
    Ok(Stroke {
        width,
        adjust,
        cap,
        join,
        miter_limit,
        dash_array,
        dash_phase,
    })
}

fn read_soft_mask(
    reader: &mut Reader<'_>,
    paths: &[Arc<Path>],
    samples: &[Arc<[u8]>],
    shadings: &[Arc<Shading>],
    clips: usize,
    masks: usize,
) -> Result<SoftMask, ProtocolError> {
    let kind = match reader.u8("a soft mask's kind")? {
        1 => SoftMaskKind::Alpha,
        2 => SoftMaskKind::Luminosity {
            backdrop: read_colour(reader, "a soft mask's backdrop")?,
        },
        value => {
            return Err(ProtocolError::Unrecognised {
                what: "a soft mask's kind",
                value: u32::from(value),
            });
        }
    };
    let transfer = if reader.bool("a soft mask's transfer")? {
        let mut curve = [0_u8; 256];
        for value in &mut curve {
            *value = reader.u8("a soft mask's transfer")?;
        }
        Some(Transfer::from_samples(curve))
    } else {
        None
    };
    let luminance = match reader.u8("a soft mask's luminance")? {
        0 => None,
        1 => {
            let mut curves = [[0.0_f32; 3]; 256];
            for sample in &mut curves {
                for component in sample {
                    *component = reader.f32("a soft mask's luminance")?;
                }
            }
            Some(Luminance::curves(Arc::new(curves)))
        }
        2 => {
            let side = reader.usize("a soft mask's luminance grid")?;
            // Not `with_capacity`: the side is a length field on the wire, so a corrupt one
            // would reserve whatever it says. Pushing lets the reader run out of bytes first,
            // which is the refusal this branch wants.
            let wanted = side.checked_pow(3).ok_or(ProtocolError::Unrecognised {
                what: "a soft mask's luminance grid side",
                value: u32::try_from(side).unwrap_or(u32::MAX),
            })?;
            let mut samples = Vec::new();
            for _ in 0..wanted {
                samples.push(reader.f32("a soft mask's luminance grid")?);
            }
            Some(
                Luminance::grid(side, Arc::from(samples)).ok_or(ProtocolError::Unrecognised {
                    what: "a soft mask's luminance grid side",
                    value: u32::try_from(side).unwrap_or(u32::MAX),
                })?,
            )
        }
        value => {
            return Err(ProtocolError::Unrecognised {
                what: "a soft mask's luminance",
                value: u32::from(value),
            });
        }
    };
    let commands = read_commands(reader, paths, samples, shadings, clips, masks, 0)?;
    Ok(SoftMask {
        commands,
        kind,
        transfer,
        luminance,
    })
}

fn read_mark_state(
    reader: &mut Reader<'_>,
    clips: usize,
    masks: usize,
) -> Result<(Option<ClipId>, Option<SoftMaskId>, BlendMode), ProtocolError> {
    let clip = option_index(reader, "a command's clip", clips)?.map(ClipId::new);
    let mask = option_index(reader, "a command's soft mask", masks)?.map(SoftMaskId::new);
    let blend = match reader.u8("a command's blend mode")? {
        0 => BlendMode::Normal,
        1 => BlendMode::Multiply,
        2 => BlendMode::Screen,
        3 => BlendMode::Overlay,
        4 => BlendMode::Darken,
        5 => BlendMode::Lighten,
        6 => BlendMode::ColorDodge,
        7 => BlendMode::ColorBurn,
        8 => BlendMode::HardLight,
        9 => BlendMode::SoftLight,
        10 => BlendMode::Difference,
        11 => BlendMode::Exclusion,
        12 => BlendMode::Hue,
        13 => BlendMode::Saturation,
        14 => BlendMode::Color,
        15 => BlendMode::Luminosity,
        value => {
            return Err(ProtocolError::Unrecognised {
                what: "a command's blend mode",
                value: u32::from(value),
            });
        }
    };
    Ok((clip, mask, blend))
}

fn read_blending_space(reader: &mut Reader<'_>) -> Result<BlendingSpace, ProtocolError> {
    let side = reader.usize("a blending space's side")?;
    let grid = table(reader, "a blending space's grid", least::SAMPLE, |reader| {
        let mut sample = [0.0_f32; 3];
        for component in &mut sample {
            *component = reader.f32("a blending space's sample")?;
        }
        Ok(sample)
    })?;
    // `BlendingSpace::new` is where the two conditions every reader of one depends on live —
    // at least two samples an axis, and exactly `side⁴` of them — so the refusal is its answer
    // rather than a second statement of the same arithmetic here.
    BlendingSpace::new(side, Arc::from(grid)).ok_or(ProtocolError::Unbuildable {
        what: "a blending space",
        why: "its stated side and its sample count are not a grid",
    })
}

/// A group's own blending colour space, in whichever of its two shapes the writer stated.
fn read_group_blending(
    reader: &mut Reader<'_>,
    paths: &[Arc<Path>],
    samples: &[Arc<[u8]>],
    shadings: &[Arc<Shading>],
    clips: usize,
    masks: usize,
    depth: usize,
) -> Result<Option<Box<GroupBlending>>, ProtocolError> {
    match reader.u8("a group's blending space")? {
        0 => Ok(None),
        1 => {
            let space = read_blending_space(reader)?;
            let black = read_commands(reader, paths, samples, shadings, clips, masks, depth)?;
            Ok(Some(Box::new(GroupBlending::FourComponents {
                space,
                black,
            })))
        }
        2 => Ok(Some(Box::new(GroupBlending::OneComponent {
            curve: read_grey_curve(reader)?,
        }))),
        3 => Ok(Some(Box::new(GroupBlending::ThreeComponents {
            cube: read_colour_cube(reader)?,
        }))),
        value => Err(ProtocolError::Unrecognised {
            what: "a group's blending space",
            value: u32::from(value),
        }),
    }
}

fn read_colour_cube(reader: &mut Reader<'_>) -> Result<ColourCube, ProtocolError> {
    let triples = |reader: &mut Reader<'_>, what: &'static str| {
        table(reader, what, least::SAMPLE, |reader| {
            let mut sample = [0.0_f32; 3];
            for component in &mut sample {
                *component = reader.f32(what)?;
            }
            Ok(sample)
        })
    };
    let input = triples(reader, "a colour cube's input curves")?;
    let side = reader.usize("a colour cube's side")?;
    let grid = triples(reader, "a colour cube's grid")?;
    let output = table(reader, "a colour cube's output curve", 4, |reader| {
        reader.f32("a colour cube's output curve")
    })?;
    // `ColourCube::new` owns the conditions — two samples a curve and an axis, `side³` of
    // them in the grid — so the refusal is its answer, as the pair's and the curve's are.
    ColourCube::new(Arc::from(input), side, Arc::from(grid), Arc::from(output)).ok_or(
        ProtocolError::Unbuildable {
            what: "a colour cube",
            why: "its curves, its stated side and its sample count are not a cube",
        },
    )
}

fn read_grey_curve(reader: &mut Reader<'_>) -> Result<GreyCurve, ProtocolError> {
    let samples = table(reader, "a grey curve's samples", least::SAMPLE, |reader| {
        let mut sample = [0.0_f32; 3];
        for component in &mut sample {
            *component = reader.f32("a grey curve's sample")?;
        }
        Ok(sample)
    })?;
    // `GreyCurve::new` owns the one condition — two samples at least — so the refusal is its
    // answer, as `read_blending_space`'s is `BlendingSpace::new`'s.
    GreyCurve::new(Arc::from(samples)).ok_or(ProtocolError::Unbuildable {
        what: "a grey curve",
        why: "fewer than two samples is not a curve",
    })
}

fn read_path(reader: &mut Reader<'_>, what: &'static str) -> Result<Path, ProtocolError> {
    let steps = table(reader, what, least::STEP, |reader| {
        match reader.u8(what)? {
            1 => Ok(PathCommand::MoveTo(read_point(reader, what)?)),
            2 => Ok(PathCommand::LineTo(read_point(reader, what)?)),
            3 => Ok(PathCommand::CurveTo(
                read_point(reader, what)?,
                read_point(reader, what)?,
                read_point(reader, what)?,
            )),
            4 => Ok(PathCommand::Close),
            value => Err(ProtocolError::Unrecognised {
                what,
                value: u32::from(value),
            }),
        }
    })?;
    let mut path = Path::new();
    path.extend(&steps);
    Ok(path)
}

fn read_fill_rule(reader: &mut Reader<'_>, what: &'static str) -> Result<FillRule, ProtocolError> {
    match reader.u8(what)? {
        0 => Ok(FillRule::NonZero),
        1 => Ok(FillRule::EvenOdd),
        value => Err(ProtocolError::Unrecognised {
            what,
            value: u32::from(value),
        }),
    }
}

fn read_point(reader: &mut Reader<'_>, what: &'static str) -> Result<Point, ProtocolError> {
    Ok(Point::new(reader.f32(what)?, reader.f32(what)?))
}

fn read_rect(reader: &mut Reader<'_>, what: &'static str) -> Result<Rect, ProtocolError> {
    let min = read_point(reader, what)?;
    let max = read_point(reader, what)?;
    // Not `Rect::from_corners`, which sorts: a rectangle that made a round trip through a
    // constructor would come back a different value from the one that was written, and this
    // codec's whole test is that it does not.
    Ok(Rect { min, max })
}

fn read_colour(reader: &mut Reader<'_>, what: &'static str) -> Result<Color, ProtocolError> {
    Ok(Color::rgba(
        reader.f32(what)?,
        reader.f32(what)?,
        reader.f32(what)?,
        reader.f32(what)?,
    ))
}

/// The six numbers back. `pub(super)` for [`write_transform`]'s reason.
pub(super) fn read_transform(
    reader: &mut Reader<'_>,
    what: &'static str,
) -> Result<Transform, ProtocolError> {
    Ok(Transform::new(
        reader.f32(what)?,
        reader.f32(what)?,
        reader.f32(what)?,
        reader.f32(what)?,
        reader.f32(what)?,
        reader.f32(what)?,
    ))
}

#[cfg(test)]
#[expect(
    clippy::arithmetic_side_effects,
    clippy::cast_precision_loss,
    clippy::too_many_lines,
    reason = "test code: every count here is a literal under a hundred, and the one fixture is \
              deliberately a whole page rather than a case per variant, because the shared tables \
              are what the format is about and a page is where they are shared"
)]
mod tests {
    use pdf_render::{
        ColourGrid, ColoursAtDeviceScale, DeferredColours, DeferredImage, Grid, ImageAtDeviceScale,
        Patch,
    };

    use super::*;

    /// The header every hand-built message starts with: a page size and no content clip.
    fn head() -> Writer {
        let mut writer = Writer::new();
        writer.f32(10.0).f32(20.0).u8(0);
        writer
    }

    /// Four empty shared tables.
    fn no_tables(writer: &mut Writer) {
        writer.usize(0).usize(0).usize(0).usize(0);
    }

    /// A path with one straight segment, so that geometry is present rather than degenerate.
    fn a_path() -> Arc<Path> {
        let mut path = Path::new();
        path.extend(&[
            PathCommand::MoveTo(Point::new(1.0, 2.0)),
            PathCommand::LineTo(Point::new(3.0, 4.0)),
            PathCommand::CurveTo(
                Point::new(5.0, 6.0),
                Point::new(7.0, 8.0),
                Point::new(9.0, 10.0),
            ),
            PathCommand::Close,
        ]);
        Arc::new(path)
    }

    fn an_image(size: u32) -> Image {
        Image {
            width: size,
            height: size,
            data: vec![0x7F; (size as usize) * (size as usize) * 4].into(),
            interpolate: true,
        }
    }

    fn a_ramp() -> Ramp {
        Ramp {
            stops: Arc::from(vec![
                Stop {
                    at: 0.0,
                    colour: Color::BLACK,
                },
                Stop {
                    at: 1.0,
                    colour: Color::rgba(0.25, 0.5, 0.75, 0.5),
                },
            ]),
        }
    }

    /// A four-component blending space at the smallest side [`BlendingSpace::new`] admits.
    fn a_blending_space() -> BlendingSpace {
        let grid: Vec<[f32; 3]> = (0..16).map(|at| [at as f32 / 16.0, 0.5, 0.25]).collect();
        BlendingSpace::new(2, Arc::from(grid)).expect("2^4 samples is a grid of side two")
    }

    /// A producer of samples, which is what ADR 0607 leaves to the raster arm.
    #[derive(Debug)]
    struct Deferred;

    impl ImageAtDeviceScale for Deferred {
        fn samples(&self, _grid: Grid) -> Image {
            an_image(1)
        }
    }

    impl ColoursAtDeviceScale for Deferred {
        fn colours(&self, _patch: Patch) -> ColourGrid {
            ColourGrid {
                width: 1,
                height: 1,
                pixels: Arc::from(vec![Color::BLACK]),
                covers: [0.0, 0.0, 1.0, 1.0],
            }
        }

        fn is_opaque(&self) -> bool {
            true
        }
    }

    /// Everything this format carries, on one page, so that a round trip exercises every arm.
    ///
    /// Written as one list rather than as a case per variant because the tables are the subject:
    /// a shared path, a shared image and a shared shading each appear twice, and the assertions
    /// below are about what that costs on the wire as well as about what comes back.
    fn a_whole_page() -> DisplayList {
        let mut list = DisplayList::new(Size::new(612.0, 792.0));
        list.set_content_clip(Rect {
            min: Point::new(0.0, 0.0),
            max: Point::new(612.0, 792.0),
        });

        let outer = list
            .add_clip(Clip {
                path: (*a_path()).clone(),
                transform: Transform::scale(2.0, 3.0),
                fill_rule: FillRule::EvenOdd,
                parent: None,
            })
            .expect("a first clip");
        let inner = list
            .add_clip(Clip {
                path: Path::new(),
                transform: Transform::IDENTITY,
                fill_rule: FillRule::NonZero,
                parent: Some(outer),
            })
            .expect("a nested clip");

        let mask = list
            .add_soft_mask(SoftMask {
                commands: vec![Command::Fill {
                    path: a_path(),
                    transform: Transform::IDENTITY,
                    fill_rule: FillRule::NonZero,
                    paint: Paint::Solid(Color::WHITE),
                    clip: Some(outer),
                    mask: None,
                    blend: BlendMode::Normal,
                }],
                kind: SoftMaskKind::Luminosity {
                    backdrop: Color::rgb(0.1, 0.2, 0.3),
                },
                transfer: Some(Transfer::from_samples([9; 256])),
                luminance: None,
            })
            .expect("a first soft mask");
        list.add_soft_mask(SoftMask {
            commands: Vec::new(),
            kind: SoftMaskKind::Alpha,
            transfer: None,
            luminance: None,
        })
        .expect("a second soft mask");

        // One kind, two shadings: `pdf_model::shading::Cache`'s own shape, and the reason the
        // kinds are a table of their own.
        let kind = Arc::new(ShadingKind::Axial {
            start: Point::new(0.0, 0.0),
            end: Point::new(1.0, 1.0),
            ramp: a_ramp(),
            extend: (true, false),
        });
        let first = Arc::new(Shading {
            kind: Arc::clone(&kind),
            transform: Transform::translate(3.0, 4.0),
            background: Some(Color::rgba(1.0, 0.0, 0.0, 0.25)),
        });
        let second = Arc::new(Shading {
            kind,
            transform: Transform::IDENTITY,
            background: None,
        });
        let mesh = Arc::new(Shading {
            kind: Arc::new(ShadingKind::Mesh {
                triangles: Arc::from(vec![
                    Triangle {
                        points: [
                            Point::new(0.0, 0.0),
                            Point::new(1.0, 0.0),
                            Point::new(0.0, 1.0),
                        ],
                        corners: Corners::Colours([Color::BLACK, Color::WHITE, Color::TRANSPARENT]),
                    },
                    Triangle {
                        points: [
                            Point::new(1.0, 1.0),
                            Point::new(1.0, 0.0),
                            Point::new(0.0, 1.0),
                        ],
                        corners: Corners::Parameters([0.0, 0.5, 1.0]),
                    },
                ]),
                ramp: Some(a_ramp()),
            }),
            transform: Transform::IDENTITY,
            background: None,
        });
        let radial = Arc::new(Shading {
            kind: Arc::new(ShadingKind::Radial {
                start: Point::new(1.0, 2.0),
                start_radius: 0.5,
                end: Point::new(3.0, 4.0),
                end_radius: 9.0,
                ramp: a_ramp(),
                extend: (false, true),
            }),
            transform: Transform::scale(0.5, 0.5),
            background: None,
        });

        let path = a_path();
        let samples = an_image(4);
        list.push(Command::Fill {
            path: Arc::clone(&path),
            transform: Transform::IDENTITY,
            fill_rule: FillRule::NonZero,
            paint: Paint::Shading(Arc::clone(&first)),
            clip: Some(inner),
            mask: Some(mask),
            blend: BlendMode::Multiply,
        });
        list.push(Command::Stroke {
            path,
            transform: Transform::new(1.0, 0.0, 0.0, 1.0, 5.0, 6.0),
            stroke: Stroke {
                width: 2.5,
                adjust: true,
                cap: LineCap::Round,
                join: LineJoin::Bevel,
                miter_limit: 4.0,
                dash_array: vec![1.0, 2.0, 3.0],
                dash_phase: 0.5,
            },
            paint: Paint::Shading(second),
            clip: None,
            mask: None,
            blend: BlendMode::Luminosity,
        });
        list.push(Command::Image {
            image: ImageSource::Decoded(samples.clone()),
            transform: Transform::scale(10.0, 10.0),
            alpha: 0.75,
            clip: Some(outer),
            mask: None,
            blend: BlendMode::Screen,
        });
        list.push(Command::Group {
            commands: vec![
                Command::Shaped {
                    object: Box::new(Command::Fill {
                        path: a_path(),
                        transform: Transform::IDENTITY,
                        fill_rule: FillRule::EvenOdd,
                        paint: Paint::Shading(mesh),
                        clip: None,
                        mask: None,
                        blend: BlendMode::Normal,
                    }),
                    shape: Box::new(Command::Fill {
                        path: a_path(),
                        transform: Transform::IDENTITY,
                        fill_rule: FillRule::EvenOdd,
                        paint: Paint::Solid(Color::WHITE),
                        clip: None,
                        mask: None,
                        blend: BlendMode::Normal,
                    }),
                },
                // The same samples again, which is what the sharing requirement is about.
                Command::Image {
                    image: ImageSource::Decoded(samples),
                    transform: Transform::IDENTITY,
                    alpha: 1.0,
                    clip: None,
                    mask: None,
                    blend: BlendMode::Normal,
                },
                Command::Fill {
                    path: a_path(),
                    transform: Transform::IDENTITY,
                    fill_rule: FillRule::NonZero,
                    paint: Paint::Shading(radial),
                    clip: None,
                    mask: None,
                    blend: BlendMode::Darken,
                },
            ],
            alpha: 0.5,
            clip: Some(inner),
            mask: Some(mask),
            blend: BlendMode::Overlay,
            isolated: true,
            knockout: true,
            alpha_is_shape: true,
            blending: Some(Box::new(GroupBlending::FourComponents {
                space: a_blending_space(),
                black: vec![Command::Fill {
                    path: a_path(),
                    transform: Transform::IDENTITY,
                    fill_rule: FillRule::NonZero,
                    paint: Paint::Solid(Color::grey(0.5)),
                    clip: None,
                    mask: None,
                    blend: BlendMode::Normal,
                }],
            })),
        });

        let mut black = DisplayList::new(Size::new(612.0, 792.0));
        black.push(Command::Fill {
            path: a_path(),
            transform: Transform::IDENTITY,
            fill_rule: FillRule::NonZero,
            paint: Paint::Solid(Color::BLACK),
            clip: None,
            mask: None,
            blend: BlendMode::Normal,
        });
        list.set_blending(a_blending_space(), black);
        list
    }

    /// The whole vocabulary, written down and read back to an equal value.
    ///
    /// `DisplayList`'s own `PartialEq` compares its commands, both tables, the clip index, the
    /// blending pair and the content clip, and `Path`'s compares its steps — so this is an exact
    /// assertion rather than a resemblance. It is the test the whole codec exists to pass.
    #[test]
    fn a_whole_page_round_trips_to_an_equal_list() {
        let list = a_whole_page();
        let bytes = encode(&list).expect("a list with no deferred producer");
        let back = decode(&bytes).expect("what this encoder wrote");
        assert_eq!(back, list);
    }

    /// The one-component shapes of §11.4.7 and §11.7.2 — a page curve, and a group carrying
    /// one — round-trip too. Their own list, because a page carries the pair or the curve
    /// and never both.
    #[test]
    fn a_page_in_one_component_round_trips_to_an_equal_list() {
        let curve = |scale: f32| {
            let samples: Vec<[f32; 3]> = (0..3)
                .map(|index| [index as f32 * scale / 2.0; 3])
                .collect();
            GreyCurve::new(Arc::from(samples)).expect("three samples is a curve")
        };
        let mut list = DisplayList::new(Size::new(200.0, 100.0));
        list.push(Command::Group {
            commands: vec![Command::Fill {
                path: a_path(),
                transform: Transform::IDENTITY,
                fill_rule: FillRule::NonZero,
                paint: Paint::Solid(Color::grey(0.25)),
                clip: None,
                mask: None,
                blend: BlendMode::Normal,
            }],
            alpha: 0.75,
            clip: None,
            mask: None,
            blend: BlendMode::Normal,
            isolated: true,
            knockout: false,
            alpha_is_shape: false,
            blending: Some(Box::new(GroupBlending::OneComponent { curve: curve(0.5) })),
        });
        list.set_grey_curve(curve(1.0));
        let bytes = encode(&list).expect("a list with no deferred producer");
        let back = decode(&bytes).expect("what this encoder wrote");
        assert_eq!(back, list);
    }

    /// The three-component shapes of §11.4.7, §11.7.2 and §11.5.3 — a page cube, a group
    /// carrying one, and a luminosity mask carrying three curves — round-trip too, on a list
    /// of their own for the reason the one-component test gives.
    #[test]
    fn a_page_in_three_components_round_trips_to_an_equal_list() {
        let cube = |scale: f32| {
            let input: Vec<[f32; 3]> = vec![[0.0; 3], [scale; 3]];
            let grid: Vec<[f32; 3]> = (0..8)
                .map(|corner| [corner as f32 / 7.0 * scale; 3])
                .collect();
            let output: Vec<f32> = vec![0.0, 0.5 * scale, scale];
            ColourCube::new(Arc::from(input), 2, Arc::from(grid), Arc::from(output))
                .expect("two curves and eight corners is a cube")
        };
        let curves: [[f32; 3]; 256] =
            std::array::from_fn(|index| [index as f32 / 255.0, 0.5, 0.25]);
        let mut list = DisplayList::new(Size::new(200.0, 100.0));
        let mask = list
            .add_soft_mask(SoftMask {
                commands: Vec::new(),
                kind: SoftMaskKind::Luminosity {
                    backdrop: Color::rgb(0.1, 0.2, 0.3),
                },
                transfer: None,
                luminance: Some(Luminance::curves(Arc::new(curves))),
            })
            .expect("one soft mask is under the table's bound");
        list.push(Command::Group {
            commands: vec![Command::Fill {
                path: a_path(),
                transform: Transform::IDENTITY,
                fill_rule: FillRule::NonZero,
                paint: Paint::Solid(Color::rgb(0.25, 0.5, 0.75)),
                clip: None,
                mask: Some(mask),
                blend: BlendMode::Normal,
            }],
            alpha: 0.75,
            clip: None,
            mask: None,
            blend: BlendMode::Normal,
            isolated: true,
            knockout: false,
            alpha_is_shape: false,
            blending: Some(Box::new(GroupBlending::ThreeComponents { cube: cube(0.5) })),
        });
        list.set_colour_cube(cube(1.0));
        let bytes = encode(&list).expect("a list with no deferred producer");
        let back = decode(&bytes).expect("what this encoder wrote");
        assert_eq!(back, list);
    }

    /// §11.5.3's other shape — a luminosity mask whose `Y` is a *sampled grid* rather than
    /// three curves, which is what a three-component table profile states (ADR 0851) — crosses
    /// the boundary as itself.
    ///
    /// Its own test rather than a case in the one above, for the reason that one gives, and
    /// because the two shapes share a tag byte: a list stating the grid must not come back as
    /// curves, which equality here is what checks.
    #[test]
    fn a_luminosity_masks_sampled_y_round_trips_to_an_equal_list() {
        let samples: Vec<f32> = (0..27).map(|index| index as f32 / 26.0).collect();
        let mut list = DisplayList::new(Size::new(200.0, 100.0));
        let mask = list
            .add_soft_mask(SoftMask {
                commands: Vec::new(),
                kind: SoftMaskKind::Luminosity {
                    backdrop: Color::rgb(0.1, 0.2, 0.3),
                },
                transfer: None,
                luminance: Some(
                    Luminance::grid(3, Arc::from(samples)).expect("twenty-seven of a side of 3"),
                ),
            })
            .expect("one soft mask is under the table's bound");
        list.push(Command::Fill {
            path: a_path(),
            transform: Transform::IDENTITY,
            fill_rule: FillRule::NonZero,
            paint: Paint::Solid(Color::rgb(0.25, 0.5, 0.75)),
            clip: None,
            mask: Some(mask),
            blend: BlendMode::Normal,
        });
        let bytes = encode(&list).expect("a list with no deferred producer");
        let back = decode(&bytes).expect("what this encoder wrote");
        assert_eq!(back, list);
    }

    /// Whatever the decoder accepts, the encoder can write, and it writes it the same way.
    ///
    /// The property the fuzz target asserts over arbitrary bytes, stated here over the one
    /// message this build produces on purpose: the two halves agree, so a decoded list is not a
    /// value that could never be sent back.
    #[test]
    fn what_decodes_re_encodes_to_the_same_bytes() {
        let bytes = encode(&a_whole_page()).expect("a codable list");
        let again = encode(&decode(&bytes).expect("a valid message")).expect("a codable list");
        assert_eq!(again, bytes);
    }

    /// The same samples referenced twice cross once, and come back shared.
    ///
    /// ADR 0607's hard requirement, with the number that made it one: flattened, the corpus goes
    /// from 0.37 of its raster to 0.91. Asserted twice over — as *bytes*, so that an encoder that
    /// wrote the payload again would fail here rather than in a corpus measurement, and as
    /// `Arc::ptr_eq`, because a host that re-rasterises holds one copy or two.
    #[test]
    fn a_shared_image_crosses_once() {
        let samples = an_image(64);
        let payload = samples.data.len();
        let mut list = DisplayList::new(Size::new(10.0, 10.0));
        for _ in 0..4 {
            list.push(Command::Image {
                image: ImageSource::Decoded(samples.clone()),
                transform: Transform::IDENTITY,
                alpha: 1.0,
                clip: None,
                mask: None,
                blend: BlendMode::Normal,
            });
        }
        let bytes = encode(&list).expect("a codable list");
        assert!(
            bytes.len() < payload + payload / 2,
            "four references to {payload} bytes of samples came to {} on the wire",
            bytes.len()
        );

        let back = decode(&bytes).expect("a valid message");
        let mut data = Vec::new();
        for command in back.commands() {
            if let Command::Image {
                image: ImageSource::Decoded(image),
                ..
            } = command
            {
                data.push(Arc::clone(&image.data));
            }
        }
        assert_eq!(data.len(), 4);
        for later in data.iter().skip(1) {
            assert!(
                Arc::ptr_eq(&data[0], later),
                "the samples arrived as separate allocations"
            );
        }
    }

    /// A path shared by many commands crosses once, and a shading kind shared by many shadings
    /// crosses once — the two other tables ADR 0607's figure depends on.
    #[test]
    fn shared_geometry_and_a_shared_shading_kind_cross_once() {
        let path = a_path();
        let kind = Arc::new(ShadingKind::Mesh {
            triangles: Arc::from(vec![
                Triangle {
                    points: [
                        Point::new(0.0, 0.0),
                        Point::new(1.0, 0.0),
                        Point::new(0.0, 1.0),
                    ],
                    corners: Corners::Colours([Color::BLACK; 3]),
                };
                64
            ]),
            ramp: None,
        });
        let mut list = DisplayList::new(Size::new(10.0, 10.0));
        for at in 0..32 {
            list.push(Command::Fill {
                path: Arc::clone(&path),
                transform: Transform::translate(at as f32, 0.0),
                fill_rule: FillRule::NonZero,
                paint: Paint::Shading(Arc::new(Shading {
                    kind: Arc::clone(&kind),
                    transform: Transform::translate(at as f32, 0.0),
                    background: None,
                })),
                clip: None,
                mask: None,
                blend: BlendMode::Normal,
            });
        }
        let shared = encode(&list).expect("a codable list").len();

        // The same page with nothing shared, which is what a flattening encoder would write.
        let mut flat = DisplayList::new(Size::new(10.0, 10.0));
        for command in list.commands() {
            let Command::Fill {
                transform, paint, ..
            } = command
            else {
                unreachable!("the list above is fills")
            };
            let Paint::Shading(shading) = paint else {
                unreachable!("the fills above are shaded")
            };
            flat.push(Command::Fill {
                path: Arc::new((*a_path()).clone()),
                transform: *transform,
                fill_rule: FillRule::NonZero,
                paint: Paint::Shading(Arc::new(Shading {
                    kind: Arc::new((*shading.kind).clone()),
                    transform: shading.transform,
                    background: None,
                })),
                clip: None,
                mask: None,
                blend: BlendMode::Normal,
            });
        }
        let written_out = encode(&flat).expect("a codable list").len();
        assert!(
            written_out > shared * 8,
            "sharing bought {shared} against {written_out} bytes, which is not the gap ADR 0607 \
             measured"
        );
    }

    /// The two producers are refused by name rather than dropped.
    #[test]
    fn a_deferred_producer_is_refused_by_name() {
        let mut deferred_image = DisplayList::new(Size::new(10.0, 10.0));
        deferred_image.push(Command::Image {
            image: ImageSource::AtDeviceScale(DeferredImage::new(Arc::new(Deferred))),
            transform: Transform::IDENTITY,
            alpha: 1.0,
            clip: None,
            mask: None,
            blend: BlendMode::Normal,
        });
        assert_eq!(encode(&deferred_image), Err(Uncodable::DeferredImage));

        let mut deferred_colours = DisplayList::new(Size::new(10.0, 10.0));
        deferred_colours.push(Command::Fill {
            path: a_path(),
            transform: Transform::IDENTITY,
            fill_rule: FillRule::NonZero,
            paint: Paint::Shading(Arc::new(Shading {
                kind: Arc::new(ShadingKind::Sampled {
                    domain: [0.0, 1.0, 0.0, 1.0],
                    source: DeferredColours::new(Arc::new(Deferred)),
                    program: None,
                }),
                transform: Transform::IDENTITY,
                background: None,
            })),
            clip: None,
            mask: None,
            blend: BlendMode::Normal,
        });
        assert_eq!(encode(&deferred_colours), Err(Uncodable::DeferredColours));
    }

    /// The payload is chosen per page by size, which is the whole of ADR 0607's decision.
    #[test]
    fn the_payload_is_the_smaller_of_the_two() {
        let list = a_whole_page();
        let encoded = encode(&list).expect("a codable list").len();

        let Crossing::List(bytes) = crossing(&list, super::super::as_u64(encoded) + 1) else {
            panic!("a raster one byte larger than the list is the case a list is for")
        };
        assert_eq!(bytes.len(), encoded);

        // A scanned page: one image whose samples are the whole payload, so the pixels win.
        let mut scan = DisplayList::new(Size::new(10.0, 10.0));
        scan.push(Command::Image {
            image: ImageSource::Decoded(an_image(64)),
            transform: Transform::IDENTITY,
            alpha: 1.0,
            clip: None,
            mask: None,
            blend: BlendMode::Normal,
        });
        assert!(matches!(
            crossing(&scan, 64 * 64 * 4),
            Crossing::Raster(RasterReason::Larger { .. })
        ));

        // And a refusal is a payload choice rather than a failure.
        let mut deferred = DisplayList::new(Size::new(10.0, 10.0));
        deferred.push(Command::Image {
            image: ImageSource::AtDeviceScale(DeferredImage::new(Arc::new(Deferred))),
            transform: Transform::IDENTITY,
            alpha: 1.0,
            clip: None,
            mask: None,
            blend: BlendMode::Normal,
        });
        assert!(matches!(
            crossing(&deferred, u64::MAX),
            Crossing::Raster(RasterReason::Uncodable(Uncodable::DeferredImage))
        ));
    }

    /// Nesting past what a backend composites is refused at both ends.
    #[test]
    fn a_list_nesting_past_what_a_backend_draws_is_refused() {
        let mut command = Command::Fill {
            path: a_path(),
            transform: Transform::IDENTITY,
            fill_rule: FillRule::NonZero,
            paint: Paint::Solid(Color::BLACK),
            clip: None,
            mask: None,
            blend: BlendMode::Normal,
        };
        for _ in 0..=MAX_GROUP_DEPTH {
            command = Command::Group {
                commands: vec![command],
                alpha: 1.0,
                clip: None,
                mask: None,
                blend: BlendMode::Normal,
                isolated: true,
                knockout: false,
                alpha_is_shape: false,
                blending: None,
            };
        }
        let mut list = DisplayList::new(Size::new(10.0, 10.0));
        list.push(command);
        assert!(matches!(encode(&list), Err(Uncodable::TooDeep { .. })));

        // And the decoder refuses the same shape without having to be handed one: a message can
        // state a nesting no encoder here would write, which is the case a stack depends on.
        let mut writer = head();
        no_tables(&mut writer);
        writer.usize(0).usize(0);
        for _ in 0..=MAX_GROUP_DEPTH {
            writer.usize(1).u8(TAG_GROUP).f32(1.0);
            writer.u8(0).u8(0).u8(0);
            writer.bool(true).bool(false).bool(false).u8(0);
        }
        writer.usize(0).u8(0);
        assert!(matches!(
            decode(&writer.finish()),
            Err(ProtocolError::TooDeep { .. })
        ));
    }

    /// An identifier past the table it names is refused, not clamped.
    #[test]
    fn an_identifier_past_its_table_is_refused() {
        let mut writer = head();
        no_tables(&mut writer);
        writer.usize(0).usize(0);
        // One fill naming path 0 of an empty table.
        writer.usize(1).u8(TAG_FILL).u32(0);
        assert!(matches!(
            decode(&writer.finish()),
            Err(ProtocolError::OutOfTable { held: 0, .. })
        ));

        // And a clip identifier past the clip table, on a message whose image is otherwise
        // whole — the case that would silently clip a mark by somebody else's region.
        let mut writer = head();
        writer.usize(0);
        writer.usize(1).bytes(&[0, 0, 0, 0]);
        writer.usize(0).usize(0);
        writer.usize(0).usize(0);
        writer
            .usize(1)
            .u8(TAG_IMAGE)
            .u8(1)
            .u32(1)
            .u32(1)
            .bool(false);
        writer.u32(0);
        write_transform(&mut writer, Transform::IDENTITY);
        writer.f32(1.0).u8(1).u32(0);
        assert!(matches!(
            decode(&writer.finish()),
            Err(ProtocolError::OutOfTable {
                what: "a command's clip",
                held: 0,
                ..
            })
        ));
    }

    /// An image whose samples do not fill its stated dimensions is refused.
    ///
    /// The invariant every backend indexes by, and the same one `confined_wire` asserts of a
    /// raster. A decoder that let this through would hand a rasteriser a buffer shorter than the
    /// grid it says it is.
    #[test]
    fn an_image_whose_samples_do_not_fill_it_is_refused() {
        let mut writer = head();
        writer.usize(0);
        // One run of samples, four bytes long, described below as a 4x4 image.
        writer.usize(1).bytes(&[0, 0, 0, 0]);
        writer.usize(0).usize(0);
        writer.usize(0).usize(0);
        writer
            .usize(1)
            .u8(TAG_IMAGE)
            .u8(1)
            .u32(4)
            .u32(4)
            .bool(false);
        writer.u32(0);
        write_transform(&mut writer, Transform::IDENTITY);
        writer.f32(1.0).u8(0).u8(0).u8(0);
        writer.u8(0);
        assert!(matches!(
            decode(&writer.finish()),
            Err(ProtocolError::Unbuildable {
                what: "an image",
                ..
            })
        ));
    }

    /// A clip table that cannot be rebuilt as the message numbers it is refused.
    ///
    /// `DisplayList::add_clip` hands back the identifier of a region already in the table, so one
    /// region stated twice renumbers every identifier after it — and every command naming one
    /// would then clip by the wrong region, silently and on the host's side of the boundary.
    #[test]
    fn a_clip_region_stated_twice_is_refused() {
        let mut writer = head();
        no_tables(&mut writer);
        writer.usize(2);
        for _ in 0..2 {
            write_path(&mut writer, &Path::new());
            write_transform(&mut writer, Transform::IDENTITY);
            writer.u8(0).u8(0);
        }
        writer.usize(0).usize(0).u8(0);
        assert!(matches!(
            decode(&writer.finish()),
            Err(ProtocolError::Unbuildable {
                what: "a display list's clips",
                ..
            })
        ));
    }

    /// A clip naming itself or a later clip as its parent is refused, which is what keeps
    /// `DisplayList::clip_bounds` from walking a cycle.
    #[test]
    fn a_clip_naming_a_parent_at_or_after_itself_is_refused() {
        let mut writer = head();
        no_tables(&mut writer);
        writer.usize(1);
        write_path(&mut writer, &Path::new());
        write_transform(&mut writer, Transform::IDENTITY);
        writer.u8(0).u8(1).u32(0);
        writer.usize(0).usize(0).u8(0);
        assert!(matches!(
            decode(&writer.finish()),
            Err(ProtocolError::OutOfTable {
                what: "a clip's parent",
                ..
            })
        ));
    }

    /// A ramp with no stops states no colour, and `Ramp`'s own documentation says it never has
    /// none.
    #[test]
    fn a_ramp_with_no_stops_is_refused() {
        let mut writer = head();
        writer.usize(0).usize(0);
        writer.usize(1).u8(1);
        write_point(&mut writer, Point::new(0.0, 0.0));
        write_point(&mut writer, Point::new(1.0, 1.0));
        writer.usize(0);
        assert!(matches!(
            decode(&writer.finish()),
            Err(ProtocolError::Unbuildable {
                what: "a shading's ramp",
                ..
            })
        ));
    }

    /// A blending grid that is not `side⁴` samples is refused by the constructor that owns the
    /// rule, rather than by a second statement of it here.
    #[test]
    fn a_blending_grid_that_is_not_a_grid_is_refused() {
        let mut writer = head();
        no_tables(&mut writer);
        writer.usize(0).usize(0).usize(0);
        writer.u8(1).usize(3).usize(2);
        for _ in 0..2 {
            writer.f32(0.0).f32(0.0).f32(0.0);
        }
        assert!(matches!(
            decode(&writer.finish()),
            Err(ProtocolError::Unbuildable {
                what: "a blending space",
                ..
            })
        ));
    }

    /// A number no arithmetic produced is carried across rather than refused, and the codec's own
    /// equality is what that costs.
    ///
    /// **The seven-hundred-and-thirty-second session's fuzz target found this at 750 executions**,
    /// as `one message decoded two ways` — which it was not. `DisplayList`'s `PartialEq` is
    /// ultimately `f32`'s and `f32`'s is not reflexive, so a list holding NaN is equal to nothing
    /// including itself, and an `assert_eq!` over two perfect decodes fails.
    ///
    /// **The decoder does not refuse one, deliberately** (ADR 0626 section 7): the whole premise
    /// of this boundary is that the confined path draws the page the in-process path draws, so a
    /// value the interpreter can produce may not be refused here. `Transform::invert`,
    /// `thinnest_line` and `Grid::for_placement` all test `is_finite` already, which is where the
    /// question belongs. What is asserted instead is that the *encoding* is a canonical form,
    /// which is total where equality is not.
    #[test]
    fn a_non_finite_number_crosses_and_the_encoding_is_what_compares() {
        let mut list = DisplayList::new(Size::new(f32::NAN, 20.0));
        list.push(Command::Fill {
            path: a_path(),
            transform: Transform::new(f32::INFINITY, 0.0, 0.0, f32::NAN, 0.0, 0.0),
            fill_rule: FillRule::NonZero,
            paint: Paint::Solid(Color::rgba(f32::NAN, 0.0, 0.0, 1.0)),
            clip: None,
            mask: None,
            blend: BlendMode::Normal,
        });
        let bytes = encode(&list).expect("a codable list");
        let back = decode(&bytes).expect("a valid message");
        assert_ne!(
            back, list,
            "f32's equality is not reflexive, and this is why"
        );
        assert_eq!(
            encode(&back).expect("a list this decoder accepted"),
            bytes,
            "the encoding is a canonical form even where equality is not"
        );
    }

    /// A count larger than the message that states it is refused before anything is reserved.
    ///
    /// The bound is per *table*, taking the smallest record each can hold, which is what makes it
    /// tighter than the parent module's one-byte-an-element assumption: a claim of 2^28 clips in
    /// a message of a few bytes is a claim rather than a length.
    #[test]
    fn a_count_larger_than_the_message_is_refused() {
        let mut writer = head();
        writer.usize(1 << 28);
        assert!(matches!(
            decode(&writer.finish()),
            Err(ProtocolError::Overlong { .. })
        ));
    }

    /// Every truncation and every single-byte change of a real message is a refusal or a value,
    /// never a panic.
    ///
    /// The deterministic half of what `fuzz/fuzz_targets/display_list.rs` does continuously; it
    /// is here so that a change to this format is checked by `cargo nextest` rather than only by
    /// somebody remembering to fuzz.
    #[test]
    fn a_truncated_or_altered_message_is_never_a_panic() {
        let message = encode(&a_whole_page()).expect("a codable list");
        for cut in 0..=message.len().min(4096) {
            let Some(prefix) = message.get(..cut) else {
                continue;
            };
            let _ = decode(prefix);
        }
        for at in (0..message.len()).step_by(7) {
            for value in [0x00_u8, 0x01, 0x7F, 0x80, 0xFF] {
                let mut changed = message.clone();
                if let Some(byte) = changed.get_mut(at) {
                    *byte = value;
                }
                if let Ok(list) = decode(&changed) {
                    // Anything that decodes can be written again, which is the property the
                    // fuzz target asserts and the one that says the two halves agree.
                    assert!(encode(&list).is_ok());
                }
            }
        }
    }
}
