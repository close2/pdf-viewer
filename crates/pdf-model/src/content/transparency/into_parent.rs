//! §11.6.6: the conversion a group drawn in its own space owes at its `Do`, sampled as a cube or a curve the
//! backends interpolate, and composed with the conversion out of the group's own space (ADR 2080).

use std::sync::Arc;

use pdf_render::Color;

use crate::black_generation::BlackGeneration;
use crate::colour::{ColourSpace, Compositing};
use crate::icc::Rendering;

use crate::content::Interpreter;

use super::blending_space::OwnSpaceRun;
use super::knockout::paired;

/// How many samples an axis of the conversion into a parent's components takes.
///
/// The conversion is sampled over device RGB with identity curves on both sides, which is the
/// shape `crate::colour` gives a table profile's cube and at the same side: a conversion with no
/// linear stage to separate is sampled whole, and thirty-three an axis is where that crate found
/// a profile's table reproduced to within its own interpolation.
const INTO_PARENT_SIDE: usize = 33;

/// How many samples an axis of a press's conversion out takes once it is composed with the
/// conversion into a parent's components.
///
/// The assumed inks' grid has two samples an axis and is multilinear between them
/// (`crate::colour::Press::assumed`); composed with a conversion that is not linear, two samples
/// would interpolate a different function from the composition, so the composed grid is resampled
/// at the side `crate::colour` samples a profile's press at.
const COMPOSED_PRESS_SIDE: usize = 17;

/// The channels a parent's raster holds for a colour a group's raster holds as device RGB, or
/// `None` where the parent composites on the device and no conversion is owed.
///
/// §11.6.6's final compositing: a group's result "shall then be painted into the parent group
/// or page", and where the parent composites in a space of its own that painting converts the
/// result into that space — the same conversion in every mark inside the parent takes
/// ([`Compositing::paint`]), asked of the device colour the group's own conversion out
/// produced. One route in per space, which is trap 6's rule.
///
/// The exception is a CIE-based parent of three components, and it is the clause's: §11.7.2
/// lets a `DeviceRGB` *graphics object* keep its components inside such a parent "for
/// compositing purposes only", and a group's result is not one — §11.6.6 interprets it "in the
/// group colour space", which differs from the parent's or this would not be called — so it goes
/// in from its XYZ ([`crate::colour::RgbRoute::components_of_srgb`]) as any other colour in a
/// space of its own does (§10.3.1).
pub(super) fn parent_channels(
    parent: &Compositing,
    rendering: Rendering,
    generation: Option<&BlackGeneration>,
    rgb: [f32; 3],
) -> Option<[f32; 3]> {
    match parent {
        Compositing::Device => None,
        Compositing::Additive(route) => {
            Some(route.components_of_srgb(Color::rgb(rgb[0], rgb[1], rgb[2]), rendering))
        }
        Compositing::Luminosity(_)
        | Compositing::Grey
        | Compositing::Calibrated(_)
        | Compositing::Subtractive(..) => {
            // §11.7.5.3's *second* bullet: "[w]hen painting a transparency group whose colour
            // space is DeviceRGB into a parent group whose colour space is DeviceCMYK , the
            // functions used shall be the ones in effect at the time the Do operator is applied
            // to the group". `generation` is that pair, read off the state at the `Do`, and it
            // reaches §10.4.2.4's arithmetic through `Compositing::Subtractive`'s arm of
            // `paint` — the only arm that takes it, which is the clause's own "shall be applied
            // only during conversion from DeviceRGB to DeviceCMYK colour spaces". ADR 1242,
            // amending ADR 1207.
            let painted = parent.paint(&ColourSpace::Rgb, &rgb, rendering, generation);
            Some([painted.r, painted.g, painted.b])
        }
    }
}

/// The conversion into a parent's components as the cube a backend interpolates — or `None`
/// where the parent composites on the device.
///
/// **A CIE-based parent's conversion is carried in stages and not as one grid**, which is
/// ADR 1267 and the whole of the first branch: `crate::colour`'s route decomposes the
/// conversion into the device's own decoding, a linear map and the space's encoding, and puts
/// each where `pdf_render::ColourCube` has a place for it. Sampling the whole of it here put
/// 4.66 levels of 255 between the cube and the conversion for a `CalRGB` with `/Gamma 2.2`
/// (ADR 1254), because a grid cannot resolve a stage whose slope at zero is unbounded; the
/// stages put 0.88.
///
/// Every other parent is sampled whole, and none of them has that stage: a `DeviceGray`
/// parent's §10.4.2.2 weights and a device parent's identity are affine, and §10.4.2.3's ink
/// and §10.4.2.4's generation are piecewise linear with a bounded slope, so a grid of
/// [`INTO_PARENT_SIDE`] resolves what they do between its samples.
fn into_parent_cube(
    parent: &Compositing,
    rendering: Rendering,
    generation: Option<&BlackGeneration>,
) -> Option<pdf_render::ColourCube> {
    match parent {
        Compositing::Additive(route) => {
            if let Some(cube) = route.into_cube(rendering) {
                return Some(cube);
            }
        }
        Compositing::Calibrated(route) => {
            if let Some(cube) = route.into_cube() {
                return Some(cube);
            }
        }
        // A profile whose conversion in is a lookup table has no stage to separate and comes
        // back from `into_cube` as `None`, which is the grid below — the same answer
        // `crate::colour` gives such a profile for the conversion *out*.
        Compositing::Device
        | Compositing::Luminosity(_)
        | Compositing::Grey
        | Compositing::Subtractive(..) => {}
    }
    let side = INTO_PARENT_SIDE;
    #[expect(clippy::cast_precision_loss, reason = "a grid index below the side")]
    let at = |index: usize| index as f32 / (side - 1) as f32;
    let mut grid = Vec::with_capacity(side.pow(3));
    for blue in 0..side {
        for green in 0..side {
            for red in 0..side {
                grid.push(parent_channels(
                    parent,
                    rendering,
                    generation,
                    [at(red), at(green), at(blue)],
                )?);
            }
        }
    }
    pdf_render::ColourCube::new(
        Arc::from([[0.0f32; 3], [1.0f32; 3]]),
        side,
        Arc::from(grid),
        Arc::from([0.0f32, 1.0]),
    )
}

/// A conversion out of a group's own space with the conversion into its parent's composed
/// onto it, as the data the display list carries.
///
/// What the parent's raster holds for a group's composited result is one function of that
/// result — the group's conversion out, then [`parent_channels`] — and the backends apply one
/// grid, curve or cube per group, so the two are composed here, where both are known. Each shape
/// is sampled where the group's own conversion already is: a curve at its own samples, a cube
/// and a press resampled at [`INTO_PARENT_SIDE`] and [`COMPOSED_PRESS_SIDE`] with identity
/// curves, because the composition has no linear stage left to separate. What that costs is one
/// interpolation's precision on top of the conversion out's own, which §11.7.2's NOTE 5 is about.
#[derive(Debug, Clone)]
pub(in crate::content) enum ComposedOut {
    /// Four components of a press, leaving into the parent's channels.
    Space(pdf_render::BlendingSpace),
    /// One component, leaving into the parent's channels.
    Curve(pdf_render::GreyCurve),
    /// Three components, or the device's own, leaving into the parent's channels.
    Cube(pdf_render::ColourCube),
}

/// The conversion out of `own` composed with `into`, or `None` where the shape cannot be sampled.
///
/// `out` is what the group carries where its parent composites on the device
/// ([`super::blending_space::own_space_conversion`], or a press's grid); `own` says which of the two `None`s a missing
/// one is — device grey, whose conversion out is the identity (§10.4.2.2), or the device's own
/// three components — because the composition is a different curve or cube for each.
fn composed_into_parent(
    own: &Compositing,
    out: Option<&pdf_render::GroupBlending>,
    into: &pdf_render::ColourCube,
) -> Option<ComposedOut> {
    match (own, out) {
        (_, Some(pdf_render::GroupBlending::FourComponents { space, .. })) => {
            let grid = sampled_grid(COMPOSED_PRESS_SIDE, |[cyan, magenta, yellow, black]| {
                into.convert(space.convert(cyan, magenta, yellow, black))
            });
            pdf_render::BlendingSpace::new(COMPOSED_PRESS_SIDE, Arc::from(grid))
                .map(ComposedOut::Space)
        }
        (_, Some(pdf_render::GroupBlending::OneComponent { curve })) => {
            let samples: Vec<[f32; 3]> = curve
                .samples()
                .iter()
                .map(|sample| into.convert(*sample))
                .collect();
            pdf_render::GreyCurve::new(Arc::from(samples)).map(ComposedOut::Curve)
        }
        (_, Some(pdf_render::GroupBlending::ThreeComponents { cube })) => {
            resampled_cube(|components| into.convert(cube.convert(components)))
                .map(ComposedOut::Cube)
        }
        (Compositing::Grey, None) => {
            let samples: Vec<[f32; 3]> = (0..GREY_SAMPLES)
                .map(|index| {
                    #[expect(clippy::cast_precision_loss, reason = "an index below 256")]
                    let grey = index as f32 / (GREY_SAMPLES - 1) as f32;
                    into.convert([grey; 3])
                })
                .collect();
            pdf_render::GreyCurve::new(Arc::from(samples)).map(ComposedOut::Curve)
        }
        (Compositing::Device, None) => Some(ComposedOut::Cube(into.clone())),
        (
            Compositing::Luminosity(_)
            | Compositing::Calibrated(_)
            | Compositing::Additive(_)
            | Compositing::Subtractive(..),
            None,
        ) => None,
    }
}

/// How many samples a device-grey group's composed curve takes: one per eight-bit level, so
/// that the curve is exact at every value the raster can hold.
const GREY_SAMPLES: usize = 256;

/// `convert` sampled over three components at [`INTO_PARENT_SIDE`] with identity curves.
fn resampled_cube(convert: impl Fn([f32; 3]) -> [f32; 3] + Sync) -> Option<pdf_render::ColourCube> {
    pdf_render::ColourCube::new(
        Arc::from([[0.0f32; 3], [1.0f32; 3]]),
        INTO_PARENT_SIDE,
        Arc::from(sampled_grid(INTO_PARENT_SIDE, convert)),
        Arc::from([0.0f32, 1.0]),
    )
}

/// `convert` at every point of a grid of `side` samples an axis, `i ÷ (side − 1)` on each, in
/// the order [`pdf_render::BlendingSpace`] and [`pdf_render::ColourCube`] index their grids —
/// the first axis fastest.
///
/// **Sampled across rayon's pool, and the clock is why.** A four-component group composed into a
/// parent of its own is 83 521 of these, each a press's sixteen corners and the parent's cube
/// behind them: on `bug1721218_reduced.pdf` 136 million instructions and about five milliseconds
/// of a 69-millisecond interpretation, all of it on the interpreter's thread (ADR 1632). Every
/// point is a function of its own coordinates and nothing else, and `collect` keeps the index
/// order, so which thread computes a point cannot change a sample — the grid is the one the
/// nested loops it replaces built. The cost is that the order is an index decomposed rather
/// than four loops read off the page.
fn sampled_grid<const AXES: usize>(
    side: usize,
    convert: impl Fn([f32; AXES]) -> [f32; 3] + Sync,
) -> Vec<[f32; 3]> {
    use rayon::prelude::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator};

    /// How many points one piece of the pool's work samples: enough that a piece is tens of
    /// microseconds of conversion rather than the pool's own bookkeeping.
    const POINTS_PER_PIECE: usize = 1024;

    let last = side.saturating_sub(1).max(1);
    #[expect(clippy::cast_precision_loss, reason = "a grid index below the side")]
    let at = |index: usize| index as f32 / last as f32;
    let points = side
        .checked_pow(u32::try_from(AXES).unwrap_or(u32::MAX))
        .unwrap_or(0);
    (0..points)
        .into_par_iter()
        .with_min_len(POINTS_PER_PIECE)
        .map(|index| {
            let mut point = [0.0f32; AXES];
            let mut rest = index;
            for coordinate in &mut point {
                *coordinate = at(rest.checked_rem(side).unwrap_or(0));
                rest = rest.checked_div(side).unwrap_or(0);
            }
            convert(point)
        })
        .collect()
}

/// Whether two groups' conversions out are the same construction.
///
/// [`paired`]'s question asked of a group's own space: the two halves of a pair are one content
/// stream interpreted twice, so a group inside states its space in both or in neither, and a
/// press's own pair inside must pair in turn. The grids differ by construction — each half's is
/// composed into that half — and are not compared.
pub(super) fn blending_paired(
    left: Option<&pdf_render::GroupBlending>,
    right: Option<&pdf_render::GroupBlending>,
) -> bool {
    match (left, right) {
        (
            Some(pdf_render::GroupBlending::FourComponents { black: ours, .. }),
            Some(pdf_render::GroupBlending::FourComponents { black: theirs, .. }),
        ) => paired(ours, theirs),
        (Some(ours), Some(theirs)) => {
            std::mem::discriminant(ours) == std::mem::discriminant(theirs)
        }
        (None, None) => true,
        (None, Some(_)) | (Some(_), None) => false,
    }
}

impl Interpreter<'_> {
    /// The conversion a group drawn in `own` carries out to the backend, composed with the
    /// conversion into what this run composites in — or `None` where that cannot be sampled.
    ///
    /// `out` is the group's own conversion out; where the parent composites on the device it
    /// is the whole answer. Elsewhere
    /// [`composed_into_parent`] is applied, so that what the backend resolves the group's
    /// raster through lands in the parent's channels — §11.7.2's "interpreted in the group's
    /// colour space when the group is subsequently composited with its backdrop", with the
    /// backdrop's own space read off `self.compositing`. Both samplings are memoised on this
    /// interpretation, because a page drawing one shape of group many times would otherwise
    /// sample the same conversion once per `Do`.
    ///
    /// `generation` is §11.7.5.3's second bullet, read off the state at the `Do` beside
    /// `rendering` and carried into [`parent_channels`]. It is in the memoisation key by
    /// *pointer* identity — `BlackGeneration::identity`, the same key `crate::colour::Conversion`
    /// uses — so two `/ExtGState` dictionaries stating one pair of functions sample the cube
    /// twice rather than once; that costs a sampling and cannot cost a colour. ADR 1242.
    pub(super) fn conversion_into_parent(
        &mut self,
        own: &Compositing,
        out: Option<pdf_render::GroupBlending>,
        rendering: Rendering,
        generation: Option<&Arc<BlackGeneration>>,
    ) -> OwnSpaceRun {
        if self.compositing == Compositing::Device {
            return OwnSpaceRun::Drawn(out);
        }
        let key = (
            own.clone(),
            self.compositing.clone(),
            rendering,
            generation.map(BlackGeneration::identity),
        );
        let composed = if let Some(composed) = self.into_parent.get(&key) {
            composed.clone()
        } else {
            let Some(composed) =
                into_parent_cube(&self.compositing, rendering, generation.map(AsRef::as_ref))
                    .and_then(|into| composed_into_parent(own, out.as_ref(), &into))
            else {
                return OwnSpaceRun::GivenUp;
            };
            self.into_parent.insert(key, composed.clone());
            composed
        };
        OwnSpaceRun::Drawn(Some(match (composed, out) {
            (
                ComposedOut::Space(space),
                Some(pdf_render::GroupBlending::FourComponents { black, .. }),
            ) => pdf_render::GroupBlending::FourComponents { space, black },
            (ComposedOut::Space(_), _) => return OwnSpaceRun::GivenUp,
            (ComposedOut::Curve(curve), _) => pdf_render::GroupBlending::OneComponent { curve },
            (ComposedOut::Cube(cube), _) => pdf_render::GroupBlending::ThreeComponents { cube },
        }))
    }
}
