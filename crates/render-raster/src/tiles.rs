//! A target wider or taller than the adapter renders, drawn as tiles and stitched (ADR 1472).
//!
//! An adapter states a largest texture side — 16 384 on the machines this tree measures — and a
//! frame is one texture, so a target past it is refused by the device as a capability. The
//! window never asks for one: it draws a viewport, and a viewport is at most the window. What
//! does ask is [`pdf_render::Rasterizer::rasterize`]'s own contract, a raster of the whole target,
//! which the correctness gate asks of every page at the page's own scale — and a page of
//! 12 608 × 16 806 points (`issue19517.pdf`) is that at 1×.
//!
//! **A tile is the same page under a whole-pixel translation**, so it is the same picture: every
//! placement this backend makes is the target's transform followed by the tile's offset, and an
//! offset of whole device pixels moves no edge, sample or glyph phase relative to the pixel grid.
//! The post-passes [`crate::QuorraRasterizer`]'s `rasterize` runs over a readback — the four-
//! component pair, the curve and the cube, the crop, the medium, the transfer functions — run over
//! the stitched raster against the whole target, exactly as they do over one frame.
//!
//! **What the host holds is the target's raster and one tile.** Each tile is drawn, read back,
//! copied into its place and dropped before the next is drawn, so the peak over the device's own
//! memory is `width × height × 4` bytes plus one tile's readback — 0.99 GB for `issue19517.pdf`'s
//! 847 MB raster, measured. The raster is reserved before the first tile, and a target whose raster
//! cannot be reserved is refused as [`QuorraRasterError::Allocation`] with its size, before any
//! tile is drawn; how large a target a caller may ask for is that caller's pixel budget
//! ([`pdf_render::TargetSpec::for_page`]), which the correctness gate sets at 2^28 pixels (ADR 1481).

use pdf_render::{DisplayList, TargetSpec, Transform};

use crate::{FrameCost, QuorraRasterError, QuorraRasterizer};

/// The smallest tile side considered, so that a device stating a tiny budget is still asked for
/// tiles worth a frame each.
const MIN_TILE_SIDE: u32 = 256;

/// `target` through `rasterizer`'s device in tiles no side of which exceeds what the device
/// renders, stitched into one straight-alpha RGBA8 raster of the whole target.
///
/// # Errors
///
/// The first tile's refusal, as [`QuorraRasterizer::render_whole`] gives it, or
/// [`QuorraRasterError::Allocation`] where the stitched raster cannot be held.
pub(crate) fn render_in_tiles(
    rasterizer: &mut QuorraRasterizer,
    list: &DisplayList,
    target: TargetSpec,
    cost: &mut FrameCost,
) -> Result<Vec<u8>, QuorraRasterError> {
    let limits = rasterizer.device.limits();
    let side = tile_side(limits.max_target_size, limits.max_frame_bytes);
    let allocation = || QuorraRasterError::Allocation {
        width: target.width,
        height: target.height,
    };
    let row_bytes = usize::try_from(target.width)
        .ok()
        .and_then(|width| width.checked_mul(4))
        .ok_or_else(allocation)?;
    let bytes = usize::try_from(target.height)
        .ok()
        .and_then(|height| height.checked_mul(row_bytes))
        .ok_or_else(allocation)?;
    let mut stitched: Vec<u8> = Vec::new();
    stitched
        .try_reserve_exact(bytes)
        .map_err(|_| allocation())?;
    stitched.resize(bytes, 0);

    let mut top = 0;
    while top < target.height {
        let height = side.min(target.height.saturating_sub(top));
        let mut left = 0;
        while left < target.width {
            let width = side.min(target.width.saturating_sub(left));
            let tile = TargetSpec {
                width,
                height,
                transform: target
                    .transform
                    .then(Transform::translate(-device(left), -device(top))),
            };
            let mut tile_cost = FrameCost::default();
            let drawn = rasterizer.render_whole(list, tile, &mut tile_cost);
            cost.add(tile_cost);
            copy_tile(&mut stitched, row_bytes, &drawn?, (left, top), width);
            left = left.saturating_add(width);
        }
        top = top.saturating_add(height);
    }
    Ok(stitched)
}

/// The side of a square tile: the largest power of two no greater than the adapter's side
/// limit whose RGBA target takes at most a quarter of the frame budget, leaving the rest to the
/// layers and coverage the page's own marks ask for. 4 096 on a 16 384-texel adapter with a
/// 256 MiB budget.
fn tile_side(max_side: u32, max_frame_bytes: u64) -> u32 {
    let mut side = MIN_TILE_SIDE;
    while let Some(next) = side.checked_mul(2) {
        let target_bytes = u64::from(next)
            .saturating_mul(u64::from(next))
            .saturating_mul(4);
        if next > max_side || target_bytes > max_frame_bytes / 4 {
            break;
        }
        side = next;
    }
    side.min(max_side.max(1))
}

/// Copies one tile's rows into the stitched raster at `(left, top)`.
///
/// Every index is checked: a tile is placed inside the target by construction, so a row that
/// would land outside it is a stitching defect, and it is dropped rather than written past the end.
fn copy_tile(
    stitched: &mut [u8],
    row_bytes: usize,
    tile: &[u8],
    (left, top): (u32, u32),
    width: u32,
) {
    let bytes = |pixels: u32| (pixels as usize).saturating_mul(4);
    let tile_row = bytes(width);
    if tile_row == 0 {
        return;
    }
    let start = bytes(left);
    for (row, pixels) in tile.chunks_exact(tile_row).enumerate() {
        let at = (top as usize)
            .saturating_add(row)
            .saturating_mul(row_bytes)
            .saturating_add(start);
        if let Some(slot) = stitched.get_mut(at..at.saturating_add(tile_row)) {
            slot.copy_from_slice(pixels);
        }
    }
}

/// A pixel offset as a device coordinate.
#[expect(
    clippy::cast_precision_loss,
    reason = "an offset inside a target, below `pdf_render`'s extent bound of 2^24 and so exact \
              in f32"
)]
fn device(pixels: u32) -> f32 {
    pixels as f32
}

#[cfg(test)]
mod tests {
    use super::{copy_tile, tile_side};

    /// The adapter this tree measures on, with raster's default frame budget, tiles at 4 096; a
    /// limit below that is the tile.
    #[test]
    fn a_tile_is_a_quarter_of_the_budget_within_the_limit() {
        assert_eq!(tile_side(16_384, 1 << 28), 4_096);
        assert_eq!(tile_side(2_048, 1 << 28), 2_048);
        assert_eq!(tile_side(16_384, 1 << 20), 256);
    }

    /// Two tiles side by side land at their own columns of one raster.
    #[test]
    fn tiles_are_stitched_at_their_offsets() {
        let mut stitched = vec![0_u8; 3 * 4 * 2];
        copy_tile(&mut stitched, 12, &[1; 2 * 4 * 2], (0, 0), 2);
        copy_tile(&mut stitched, 12, &[2; 4 * 2], (2, 0), 1);
        assert_eq!(&stitched[..12], &[1, 1, 1, 1, 1, 1, 1, 1, 2, 2, 2, 2]);
        assert_eq!(&stitched[12..], &[1, 1, 1, 1, 1, 1, 1, 1, 2, 2, 2, 2]);
    }
}
