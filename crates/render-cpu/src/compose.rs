//! How an element is composited onto what is under it: over, knockout, or §11.4.6's two-stage
//! shaped form — ISO 32000-2 §11.3 and §11.4.
//!
//! [`Compose`] is the choice a group hands each of its elements, and the two functions beside it
//! are the coverage-as-alpha question every kernel asks before it draws (ADR 0226) and the
//! premultiplication of a straight-alpha sample. The separable and non-separable blend functions
//! themselves are `crate::blend`'s.

use crate::{blend, convert};

/// How an element of a group combines with the elements drawn before it.
///
/// ISO 32000-2 §11.4.6's two answers. The ordinary one is over, and a knockout group's is
/// that "each individual element shall be composited with the group's initial backdrop
/// rather than with the stack of preceding elements in the group" — which, onto the
/// transparent backdrop [`CpuRasterizer::draw_group`] builds, is the element itself
/// replacing what is under it within its own coverage. That is Porter-Duff Source, which
/// `tiny-skia` applies through the same coverage and clip mask as any other mode, so a
/// knockout element needs no second raster and no shape channel.
///
/// An element's *own* blend mode disappears under knockout, and that is the clause rather
/// than a shortcut: its backdrop is transparent, and §11.3.6's formula with αb = 0 is the
/// source colour whatever the blend function is.
///
/// # The two halves of a [`Command::Shaped`](pdf_render::Command::Shaped) element
///
/// Where the shape is *not* the coverage the display list states it separately, and the
/// clause's two stages become two draws: [`Self::Erase`] scales the accumulated result by
/// `1 − shape`, then [`Self::Add`] adds the element. In premultiplied form that is
/// `P' = (1 − f) × P + S` exactly — see [`Command::Shaped`] — where the one-step
/// [`Self::Knockout`] form is the same line with `f` read off the coverage.
///
/// [`CpuRasterizer::draw_group`]: crate::CpuRasterizer::draw_group
/// [`Command::Shaped`]: pdf_render::Command::Shaped
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Compose {
    /// §11.4.5: an element paints over the group's accumulated result.
    Over,
    /// §11.4.6: an element replaces it within its own coverage.
    Knockout,
    /// §11.4.6's second stage, weighting the immediate backdrop by `1 − shape`.
    Erase,
    /// §11.4.6's second stage, adding the object the first stage composited.
    ///
    /// Addition rather than source-over, and the difference is the whole point of the pair:
    /// source-over would weight the backdrop by `1 − shape × opacity` a second time, which
    /// is right only where the object is opaque or its shape is 0 or 1. What the two draws
    /// leave behind is disjoint — `1 − f` of the backdrop and `f × opacity` of the object —
    /// so the sum cannot exceed 1 and `Plus`'s saturation never engages.
    Add,
}

impl Compose {
    /// The `tiny-skia` mode an element painted under `blend` is drawn with.
    pub(crate) fn mode(self, blend: pdf_render::BlendMode) -> tiny_skia::BlendMode {
        match self {
            Self::Over => convert::blend_mode(blend),
            Self::Knockout => tiny_skia::BlendMode::Source,
            Self::Erase => tiny_skia::BlendMode::DestinationOut,
            Self::Add => tiny_skia::BlendMode::Plus,
        }
    }

    /// Whether the mode is one this backend composites itself — §11.3.5.3's four, or
    /// §11.7.4.3's special overprinting blend mode.
    ///
    /// Never under knockout: the element has no backdrop to blend with.
    pub(crate) fn computed(self, blend: pdf_render::BlendMode) -> Option<blend::Computed> {
        match self {
            Self::Over => blend::Computed::of(blend),
            Self::Knockout | Self::Erase | Self::Add => None,
        }
    }
}

/// Whether painting a shape at alpha `c` says the same thing as painting it at coverage `c`.
///
/// **The first condition is that this rasteriser is anti-aliasing at all.** The substitution's
/// whole warrant is the departure §10.7.4's row records — the clause's "paint the pixel" replaced
/// by coverage proportional to area — so where [`CpuRasterizer::with_anti_alias`] has turned that
/// departure off, the clause's own answer for a sub-pixel shape is a *whole* covered pixel and a
/// fractional alpha is neither that nor what the rasteriser was asked for. The aliased mode is
/// therefore left exactly as it was. It is one test's knob today and this costs it nothing; what
/// it buys is that the rule below is true as stated rather than true of one configuration.
///
/// [`pdf_render::sub_pixel_bands`] hands back a coverage this rasteriser cannot express as a
/// shape, and the only place left to put it is the paint's alpha. The standard says when the two
/// are one quantity, ISO 32000-2 §11.3.7.1:
///
/// > As stated earlier, the alpha values that control the compositing process shall be defined as
/// > the product of shape and opacity
///
/// and §11.3.7.2's NOTE 1 says that anti-aliased coverage is the first of the two factors:
///
/// > Mathematically, elementary objects have "hard" edges, with a shape value of either 0.0 or
/// > 1.0 at every point. However, when such objects are rasterized to device pixels, the shape
/// > values along the boundaries can be anti-aliased, taking on fractional values representing
/// > fractional coverage of those pixels. When such anti-aliasing is performed, it is important
/// > to treat the fractional coverage as shape rather than opacity.
///
/// A raster that carries one alpha channel per pixel carries exactly that product, so folding a
/// band's coverage into it is the clause's own arithmetic rather than a trick — every blend
/// function reads `αs` and none of them can tell which factor it came from.
///
/// **The NOTE's warning is the exception, and it is why this returns `false` for one mode.**
/// Where shape has to stay distinguishable from opacity the substitution is wrong, and this
/// backend has one such place: §11.4.6's knockout group, where an element replaces its backdrop
/// *within its own shape* and `tiny-skia` states that as Porter-Duff Source. There a scaled alpha
/// leaves a partly transparent pixel where a partly covered one is meant. A sub-pixel shape inside
/// a knockout group therefore keeps whatever the rasteriser already gave it. ADR 0226.
///
/// Asked of the two fields rather than of a paint, because §8.5.3.2's marks are decided before
/// there is a paint to ask: `draw_stroke` states a dot's *diameter* while building the geometry,
/// and the alpha it will be filled at is the same question one step later.
///
/// [`CpuRasterizer::with_anti_alias`]: crate::CpuRasterizer::with_anti_alias
pub(crate) fn carries_coverage_as_alpha(anti_alias: bool, blend: tiny_skia::BlendMode) -> bool {
    anti_alias && blend != tiny_skia::BlendMode::Source
}

/// Multiplies a straight-alpha channel by its alpha.
pub(crate) fn premultiply(value: u8, alpha: u8) -> u8 {
    // Rounded rather than truncated, so a fully opaque pixel round-trips exactly.
    let scaled = u16::from(value)
        .saturating_mul(u16::from(alpha))
        .saturating_add(127);
    u8::try_from(scaled / 255).unwrap_or(u8::MAX)
}
