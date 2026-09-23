//! ISO 32000-2 §11.7.4.3's special overprinting blend mode where it keeps a proper subset of
//! the channels: the one fill that draws through a layer for a reason other than §11.3.5.
//!
//! [`Compose::DestOverIn`](raster_scene::Compose::DestOverIn) carries the derivation:
//! destination-over in the channels the mark leaves alone, source-over in the rest, and the
//! union alpha in both. No blend state states two operators at once, so the fill is drawn
//! alone into a layer — onto transparency, where it is its own premultiplied colour — and
//! `composite.wgsl` composites that layer once with §11.3.6's formula and the clause's
//! selected `B`. The two uniform selections never come here: they are
//! [`Compose::DestOver`](raster_scene::Compose::DestOver)'s blend state and
//! [`Compose::SrcOver`](raster_scene::Compose::SrcOver)'s (`doc/adr/1295`).

use raster_scene::{Affine, BlendMode, ClipId, Compose, FillRule, OutlineId, Paint};

use super::{ChildOp, Encoder, Op};
use crate::error::RenderError;

impl Encoder<'_> {
    /// One fill under §11.7.4.3's mode in the channels `kept` marks, through a layer of
    /// its own.
    ///
    /// The fill is re-encoded inside the layer with every other field it came with and
    /// without its soft mask, which weighs the finished layer once at the composite —
    /// §11.3.7.2 multiplies the mask into the source opacity either way, and a mask
    /// applied inside the layer and again at the composite would be applied twice. The
    /// replay road is abandoned at `plan_child`, as at every child layer: a record cannot
    /// rebuild the layer (`replay.rs`).
    #[expect(clippy::too_many_arguments)] // the fill's own parameters, forwarded once
    pub(super) fn fill_through_overprint_group(
        &mut self,
        outline: OutlineId,
        transform: Affine,
        rule: FillRule,
        paint: Paint,
        clip: Option<ClipId>,
        kept: [bool; 3],
        mask: Option<u32>,
    ) -> Result<(), RenderError> {
        let child = self.plan_child(|encoder| {
            encoder.encode_fill(
                outline,
                transform,
                rule,
                paint,
                clip,
                BlendMode::Normal,
                Compose::SrcOver,
                None,
            )
        })?;
        self.push_op(Op::Child(ChildOp::implicit_overprint_group(
            child, kept, mask,
        )))
    }
}
