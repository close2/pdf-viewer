//! What a frame answer carries as its picture: raster, marks or window, and the lists a host holds.
//!
//! The bytes are unchanged by the split that made this module (ADR 2256).

use pdf_render::{Raster, RasterFormat};
use viewer_core::Command;

use super::display_list;
use super::error::ProtocolError;
use super::wire::Reader;

/// Reads one page's pixels off a frame.
pub(super) fn decode_raster_payload(reader: &mut Reader<'_>) -> Result<Raster, ProtocolError> {
    let width = reader.u32("a raster width")?;
    let height = reader.u32("a raster height")?;
    let format = match reader.u8("a raster format")? {
        0 => RasterFormat::Rgba8,
        value => {
            return Err(ProtocolError::Unrecognised {
                what: "a raster format",
                value: u32::from(value),
            });
        }
    };
    let data = reader.owned_bytes("a raster")?;
    // The worker is the untrusted side, so its dimensions are checked against the bytes it
    // actually sent rather than believed — the same rule `pdf_sandbox`'s parent applies to a
    // decoded image, and for the same reason.
    let expected = usize::try_from(width)
        .ok()
        .zip(usize::try_from(height).ok())
        .and_then(|(width, height)| width.checked_mul(height))
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or(ProtocolError::Overlong {
            what: "a raster",
            claimed: usize::MAX,
            available: data.len(),
        })?;
    if data.len() != expected {
        return Err(ProtocolError::Overlong {
            what: "a raster",
            claimed: expected,
            available: data.len(),
        });
    }
    Ok(Raster {
        width,
        height,
        format,
        data,
    })
}

/// The decoded lists a host was last handed, one per page, so the same bytes come back as the
/// same `Arc` (ADR 0725).
///
/// **This is what makes [`crate::Payload::List`]'s sharing promise true across two frames.** The
/// worker holds one *encoded* list per page and re-sends those same bytes on every
/// [`Query::Frame`](viewer_core::Query::Frame) until a re-render replaces them — but a decode that allocated a fresh
/// `Arc<pdf_render::DisplayList>` per reply gave a host a new identity for an unchanged page on every scroll,
/// so everything keyed by `Arc` identity on the host side — the confined screen's
/// same-drawing-moved reuse, `render-raster`'s retained scenes — missed on every frame. The
/// promise was written down twice and kept only inside unit tests that shared the `Arc` by hand.
///
/// The key is the page and the test is the encoded bytes themselves, compared whole: the worker
/// is the untrusted side, so nothing short of byte equality may stand in for "the same list", and
/// a hit skips the decode as well as the allocation. What is held is bounded by the pages of the
/// frame on hand — a page that leaves the frame, or crosses as pixels, is forgotten — and it
/// lives on the *host* side of the pipe, outside the worker's address-space ceiling.
#[derive(Debug, Default)]
pub(crate) struct HeldLists {
    /// Per page: the encoded bytes as they crossed, and what they decoded to. The target is
    /// deliberately not held — a zoom re-encodes the *same* marks under a new target, and the
    /// target's own decode is a few fields where the list's is the page.
    by_page: std::collections::BTreeMap<usize, (Vec<u8>, std::sync::Arc<pdf_render::DisplayList>)>,
    /// Whether the host this is held for said it draws a page past [`viewer_core::MAX_PIXELS`] from
    /// its marks, at the size of its window ([`Command::Oversize`]), which is what admits a list
    /// whose target is past the bound ([`decode_list_payload`], ADR 1944) and a window the worker
    /// drew of any page ([`decode_window_payload`], ADR 2016).
    past_the_bound: bool,
}

impl HeldLists {
    /// Takes in what a command the host sends says about the targets it draws: a
    /// [`Command::Oversize`] decides whether a list's target may be past the bound.
    pub(crate) fn told(&mut self, command: &Command) {
        if let Command::Oversize(oversize) = command {
            self.past_the_bound = matches!(oversize, viewer_core::Oversize::Listed);
        }
    }

    /// The held list for `page`, if `bytes` are exactly what it decoded from.
    fn same(&self, page: usize, bytes: &[u8]) -> Option<std::sync::Arc<pdf_render::DisplayList>> {
        self.by_page
            .get(&page)
            .filter(|(held, _)| held == bytes)
            .map(|(_, list)| std::sync::Arc::clone(list))
    }

    /// Records what `page` decoded from and to, replacing whatever was held.
    fn keep(&mut self, page: usize, bytes: &[u8], list: &std::sync::Arc<pdf_render::DisplayList>) {
        self.by_page
            .insert(page, (bytes.to_vec(), std::sync::Arc::clone(list)));
    }

    /// Forgets every page not in `kept` — the frame on hand is the bound on what is held.
    pub(super) fn retain(&mut self, kept: &[usize]) {
        self.by_page.retain(|page, _| kept.contains(page));
    }
}

/// Reads one page's marks off a frame, and the target the host is to draw them at (ADR 0607).
///
/// **The target is where this arm's own allocation lives, and it is eight bytes.** Every other
/// length on this boundary is checked against bytes the sender actually wrote — a raster's
/// samples, a display list's tables — so a claim costs the sender what it costs the reader. A
/// target does not: two `u32`s become however many pixels the *host* asks its allocator for when
/// it draws, which is 719's finding in a new place, and the frame carrying them can be nine
/// bytes long. So they are refused here against the bounds that produced them.
///
/// [`viewer_core::MAX_PIXELS`] is the bound a tier-1 host's render request is held to, and the
/// confined worker is one — so an honest worker cannot have produced a larger target, and this
/// refuses exactly what it cannot have sent. **Unless the host said it draws such a page from its
/// marks** ([`Command::Oversize`], ADR 1944): then the worker asks for it at its whole target and
/// the host allocates a window of it, never the target, so the target's pixels are no allocation
/// of this side's and only its extent is checked. [`pdf_render::MAX_EXTENT`] is the separate `f32`
/// precision limit `TargetSpec::for_page` applies to each dimension whatever the tier, and a
/// dimension of zero is a target that cannot exist.
///
/// The transform's six numbers are *not* checked for finiteness, deliberately and for ADR 0626
/// section 7's reason: the three places in this tree that ask that question are in `pdf-render`, where a
/// device decision belongs, and a fourth answer in a codec would be a fourth place the confined
/// path and the in-process path could differ.
pub(super) fn decode_list_payload(
    reader: &mut Reader<'_>,
    page: usize,
    held: &mut HeldLists,
) -> Result<crate::Payload, ProtocolError> {
    let target = read_target(reader, held)?;
    let bytes = reader.bytes("a display list")?;
    // The same bytes hand back the same `Arc` — see [`HeldLists`]. Byte equality first, because
    // the worker is the untrusted side and nothing weaker may stand in for "the same list"; on a
    // hit the decode is skipped too, which is most of this arm's cost on an unchanged page.
    let list = if let Some(list) = held.same(page, bytes) {
        list
    } else {
        let list = std::sync::Arc::new(display_list::decode(bytes)?);
        held.keep(page, bytes, &list);
        list
    };
    Ok(crate::Payload::List { list, target })
}

/// Reads the target a page's marks or window crossed under, refused against the bounds that
/// produced it — [`decode_list_payload`]'s reasoning, which is this function's.
fn read_target(
    reader: &mut Reader<'_>,
    held: &HeldLists,
) -> Result<pdf_render::TargetSpec, ProtocolError> {
    let width = reader.u32("a target width")?;
    let height = reader.u32("a target height")?;
    if width == 0 || height == 0 {
        return Err(ProtocolError::Unbuildable {
            what: "a render target",
            why: "a dimension of zero is a target nothing can be drawn into",
        });
    }
    if width > pdf_render::MAX_EXTENT || height > pdf_render::MAX_EXTENT {
        return Err(ProtocolError::Unbuildable {
            what: "a render target",
            why: "a dimension past what an f32 resolves to a fraction of a pixel",
        });
    }
    if u64::from(width).saturating_mul(u64::from(height)) > viewer_core::MAX_PIXELS
        && !held.past_the_bound
    {
        return Err(ProtocolError::Unbuildable {
            what: "a render target",
            why: "more pixels than a render request is held to, so no worker asked for it",
        });
    }
    let transform = display_list::read_transform(reader, "a target transform")?;
    Ok(pdf_render::TargetSpec {
        width,
        height,
        transform,
    })
}

/// Reads a window of a page, drawn by the worker (ADRs 1986, 2016).
///
/// **Only for a host that said it places one** ([`Command::Oversize`]): the worker draws a window
/// of a page past the bound, and of a page under it where the toolkit windows' rule held, only for
/// such a host, so a window sent to any other is nothing an honest worker sends. The rule's share
/// of commands is a question of the list, which only the worker holds of a page that crosses as
/// pixels, so it is not asked again here; what a window could cost this side is its pixels, and
/// those are checked against the bytes sent as every raster's are, so the allocation is the
/// sender's to pay for. The window must be a part of its target and hold a pixel.
pub(super) fn decode_window_payload(
    reader: &mut Reader<'_>,
    held: &HeldLists,
) -> Result<crate::Payload, ProtocolError> {
    let target = read_target(reader, held)?;
    if !held.past_the_bound {
        return Err(ProtocolError::Unbuildable {
            what: "a window of a page",
            why: "a window crosses only to a host that said it places one",
        });
    }
    let corner = (
        reader.u32("a window's first column")?,
        reader.u32("a window's first row")?,
    );
    let raster = decode_raster_payload(reader)?;
    let inside = |first: u32, length: u32, extent: u32| {
        length > 0 && u64::from(first).saturating_add(u64::from(length)) <= u64::from(extent)
    };
    if !inside(corner.0, raster.width, target.width)
        || !inside(corner.1, raster.height, target.height)
    {
        return Err(ProtocolError::Unbuildable {
            what: "a window of a page",
            why: "a window holds at least one pixel and lies inside its page",
        });
    }
    Ok(crate::Payload::Window {
        target,
        corner,
        raster,
    })
}
