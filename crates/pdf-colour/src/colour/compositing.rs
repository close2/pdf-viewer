//! What a colour is resolved *for*: the space a raster composites in, and the plane of it one raster carries.

use std::sync::Arc;

use pdf_render::Color;

use super::ColourSpace;
use super::conversion::InkScale;
use super::grey_route::{GreyIdentity, GreyRoute};
use super::press::{Press, PressIdentity};
use super::rgb_route::{RgbIdentity, RgbRoute};
use super::spots::DeviceSpots;
use crate::black_generation::BlackGeneration;
use crate::icc::Rendering;

/// What a colour is resolved *for*.
///
/// Every colour in a PDF becomes three device components on its way to a raster, and for a
/// page that is the whole story. §11.5.3's second derivation is where it stops being one: a
/// `/Luminosity` soft mask's group is composited "in the colour space in which the
/// compositing computation is to be performed" (§11.6.5.1) and only *then* converted to a
/// single value, so where that space is subtractive the group's own arithmetic is not the
/// device's.
///
/// A whole second raster format would be one answer, and it is not this one. §10.4.2.3's
/// conversion to grey is linear in the components except for one `min`, so the group can be
/// painted in that linear quantity — one channel, composited by the ordinary rasteriser —
/// and the `min` applied once at the end, where §11.5.3 puts it. That is ADR 0210's shape one
/// level up: the display list names a quantity a backend resolves rather than carrying the
/// components a backend would have to be taught.
///
/// It lives here rather than in `pdf_model::content` because a colour reaches a raster by three
/// routes and only one of them is an operator: an image's samples (`pdf_model::image`) and a
/// shading's ramp (`crate::shading`) are colours too, and a group composited in one quantity
/// cannot have two of its three sources painting in another (ADR 0220).
/// # Why this carries the press itself
///
/// It held a `Copy` index into a process-wide table until ADR 0417, and the table was the
/// budget as well as the store — so which document was refused a press depended on which
/// eight had been interpreted first. Scoping the budget to an interpretation means the press
/// has to be reachable from wherever a colour is resolved, and a colour is resolved *per
/// colour*: an index into a table behind a lock would put that lock on the hottest path in the
/// module. An `Arc` in here is a pointer already in hand, so the read stays what it was and
/// what is cloned is a refcount, once per structure that holds one.
#[derive(Debug, Clone)]
pub enum Compositing {
    /// The page, or any group this tree composites on the device's three components.
    Device,
    /// A `/Luminosity` mask group whose blending space §10.4.2.3 sends to grey without
    /// passing through RGB, painted in the ink that clause weighs rather than in colour.
    Luminosity(InkScale),
    /// A page or an isolated group whose blending colour space is `DeviceGray` (§11.3.4,
    /// §11.4.7, §11.6.6), painted in that one component on all three channels.
    ///
    /// §11.3.4 applies the compositing formula per component:
    ///
    /// > The i th component of the result colour 𝐶𝑟 shall be obtained by applying the
    /// > compositing formula to the i th components of the constituent colours
    ///
    /// so a space of one component composites one number per pixel, and a raster holding that
    /// number in each of its three channels runs the same arithmetic three times over.
    /// §11.3.5.3 says the same of the non-separable modes outright:
    ///
    /// > Blending in gray colour spaces ( DeviceGray , CalGray and ICCBased gray) shall be
    /// > done by conversion to RGB, blending in RGB, and then converting back to gray.
    ///
    /// and each of its four functions returns a grey for two greys, so the conversion back
    /// is the identity on what the channels hold. What §11.6.6 asks of a painting operator
    /// inside such a space — convert source colours to the group colour space before
    /// compositing — is then the whole construction: every colour becomes its §10.4.2.2 or
    /// §10.4.2.3 grey on the way in, and §10.4.2.2's conversion out, a grey level "equivalent
    /// to an RGB value with all three components the same", is what the raster already holds.
    ///
    /// **This is `DeviceGray` and not the clause's other two one-component spaces.** A
    /// `CalGray` or `ICCBased` grey component reaches the device through a gamma or a tone
    /// curve, which is not affine, so the space's own component is not the channel's and a
    /// group composited in it is a different picture from one composited in device grey. Those
    /// two are [`Compositing::Calibrated`].
    Grey,
    /// A page or an isolated group whose blending colour space is `CalGray` or an `ICCBased`
    /// 'GRAY' profile (§11.3.4, §8.6.5.2, §8.6.5.5), painted in that space's own component
    /// on all three channels, with the conversion out applied once at the end.
    ///
    /// The same per-component reading as [`Compositing::Grey`] — one number per pixel,
    /// three equal channels — with the one difference the clause makes: the number is the
    /// space's *component*, which reaches the device through §8.6.5.2's gamma or a profile's
    /// tone curve rather than being the channel itself. So the conversion in is
    /// [`GreyRoute::component_of`] and the conversion out is [`GreyRoute::curve`], applied by
    /// `pdf_render::blending::resolve_grey` where §11.4.7 puts it — "the entire result shall
    /// then, if the colour spaces are not equivalent, be converted to the native colour space
    /// of the output device". ADR 0792.
    Calibrated(Arc<GreyRoute>),
    /// A page or an isolated group whose blending colour space is `CalRGB` or an `ICCBased`
    /// 'RGB ' profile (§11.3.4, §8.6.5.3, §8.6.5.5), painted in that space's own three
    /// components, with the conversion out applied once at the end.
    ///
    /// The three-component form of [`Compositing::Calibrated`]: §11.3.4's per-component
    /// formula composites each of the three, so the raster's channels carry the components
    /// themselves — `A`, `B` and `C`, or the profile's device values — and §11.4.7's
    /// conversion to the device runs over the composited result through
    /// [`RgbRoute::cube`], applied by `pdf_render::blending::resolve_cube`. The conversion
    /// in is [`RgbRoute::components_of`], and §11.7.2 decides most of it: a device colour of
    /// three components "shall be the CIE-based space of the nearest such ancestor" for
    /// compositing purposes, so a `DeviceRGB` mark keeps its numbers. ADR 0797.
    Additive(Arc<RgbRoute>),
    /// A page §11.4.7 composites in four components, painted in the plane of them this raster
    /// carries. See [`Plane`] for which plane, [`Press`] for whose four, [`DeviceSpots`] for the
    /// spot colourants §10.8.3's simulated device has beside them — none, on every run a backend
    /// draws today — and `pdf_render::blending`.
    Subtractive(Plane, Arc<Press>, DeviceSpots),
}

/// What distinguishes one [`Compositing`] from another, for the caches keyed on one.
///
/// Written out rather than derived because the press is behind an `Arc` and two `Arc`s of one
/// profile are one press — [`SAMPLED`] evicts, so that case is reachable. Ordering and hashing
/// both go through here, which is what keeps them agreeing with equality. A subtractive run's
/// [`DeviceSpots`] is compared after the tuple, by its colourants, for the same reason.
type CompositingKey = (
    u8,
    Option<InkScale>,
    Option<Plane>,
    Option<PressIdentity>,
    Option<GreyIdentity>,
    Option<RgbIdentity>,
);

impl Compositing {
    /// This value as the tuple every derived trait below is defined on.
    fn key(&self) -> CompositingKey {
        match self {
            Self::Device => (0, None, None, None, None, None),
            Self::Luminosity(scale) => (1, Some(*scale), None, None, None, None),
            Self::Subtractive(plane, press, _) => {
                (2, None, Some(*plane), Some(press.identity), None, None)
            }
            Self::Grey => (3, None, None, None, None, None),
            Self::Calibrated(route) => (4, None, None, None, Some(route.identity), None),
            Self::Additive(route) => (5, None, None, None, None, Some(route.identity)),
        }
    }
}

impl Compositing {
    /// The spot colourants a subtractive run carries planes for, and none for every other run.
    #[inline]
    #[must_use]
    pub fn spots(&self) -> &DeviceSpots {
        match self {
            Self::Subtractive(_, _, spots) => spots,
            _ => &NO_SPOTS,
        }
    }
}

/// What [`Compositing::spots`] answers for a run that carries no spot plane.
static NO_SPOTS: DeviceSpots = DeviceSpots(None);

impl Compositing {
    /// [`Compositing::paint`] on a run whose device carries `spots` beside whatever this
    /// composites in.
    ///
    /// A subtractive run carries its spot colourants itself, and every other run with none is the
    /// ordinary paint; what is left is a run compositing in one or three components on a page §10.8.3
    /// separates, whose process components a spot colour does not reach (§11.7.3's first bullet,
    /// `DeviceSpots::paint_process`). One discriminant test on every run a backend draws.
    #[inline]
    #[must_use]
    pub fn paint_beside(
        &self,
        spots: &DeviceSpots,
        space: &ColourSpace,
        values: &[f32],
        rendering: Rendering,
        generation: Option<&BlackGeneration>,
    ) -> Color {
        if spots.is_empty() || matches!(self, Self::Subtractive(..) | Self::Luminosity(_)) {
            return self.paint(space, values, rendering, generation);
        }
        spots.paint_process(self, space, values, rendering, generation)
    }
}

impl PartialEq for Compositing {
    fn eq(&self, other: &Self) -> bool {
        self.key() == other.key() && self.spots() == other.spots()
    }
}

impl Eq for Compositing {}

impl PartialOrd for Compositing {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Compositing {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.key()
            .cmp(&other.key())
            .then_with(|| self.spots().cmp(other.spots()))
    }
}

impl std::hash::Hash for Compositing {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.key().hash(state);
        self.spots().hash(state);
    }
}

/// Which plane of a page's separations one raster carries: three of a `DeviceCMYK` blending
/// space's four components, or the fourth.
///
/// §11.3.3's compositing formula is a vector function applied **per component** — §11.3.4:
/// "[t]he i th component of the result colour 𝐶𝑟 shall be obtained by applying the
/// compositing formula to the i th components of the constituent colours" — and §11.3.5.2's
/// separable blend functions are per component too. So a rasteriser with three channels
/// composites four components by drawing the page once per plane, and this says which plane it
/// is drawing.
///
/// Both process planes are painted in §11.3.4's **additive** form, the complement of the ink,
/// because that clause requires the blend functions to see additive values:
///
/// > When performing blending operations in subtractive colour spaces ( DeviceCMYK , ICCBased
/// > 'CMYK', Separation , and DeviceN ), the colour component values shall be complemented
/// > (subtracted from 1.0) before the blend function is applied and the results of the
/// > function shall then be complemented back before being used.
///
/// Storing the complement is that requirement met by construction rather than by an arithmetic
/// step around every blend.
///
/// **A spot colourant has a plane only where §10.8.3's simulated device gives it one**
/// ([`Self::Spot`], [`DeviceSpots`]): `pdf_model::colourants` counts the page's spot colourants,
/// [`Self::COLOURANTS`] to a plane, and a run carrying them paints each on its own plane and
/// §11.7.3's "additive value of 1.0" everywhere else. On every other run a spot colourant reverts
/// to its alternate colour space as it is painted — §11.7.3's second bullet, "[t]he spot colour
/// shall be converted to its alternate colour space" — and lands on the two process planes.
/// ADRs 1281 and 1311.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Plane {
    /// Cyan, magenta and yellow, one per channel.
    Chromatic,
    /// The black component, in every channel, so a backend may read any of them.
    Black,
    /// Three of the simulated device's spot colourants, one per channel: colourant `3 × n + c`
    /// of [`DeviceSpots::names`] in channel `c` of spot plane `n`, stored as §11.3.4's additive
    /// complement like the process planes, and white in a channel no colourant fills.
    Spot(usize),
}

impl Plane {
    /// The planes §11.4.7's four process components take, in the order a page is interpreted
    /// into them.
    pub const PROCESS: [Self; 2] = [Self::Chromatic, Self::Black];

    /// How many colourants one plane carries: a raster's three channels, each composited on
    /// its own under §11.3.4.
    pub const COLOURANTS: usize = 3;
}

/// Four process tints on a process plane, as §11.3.4's additive complement: cyan, magenta and
/// yellow on the chromatic plane, black on the black one. A spot plane carries none of them.
pub(super) fn process_plane(plane: Plane, [cyan, magenta, yellow, black]: [f32; 4]) -> Color {
    match plane {
        Plane::Chromatic => Color::rgb(1.0 - cyan, 1.0 - magenta, 1.0 - yellow),
        Plane::Black => Color::grey(1.0 - black),
        Plane::Spot(_) => Color::rgb(1.0, 1.0, 1.0),
    }
}

impl Compositing {
    /// The colour `values` become for what is being composited into.
    ///
    /// The alpha comes from the ordinary conversion in both branches, because it is not a
    /// colour component: §8.6.6.4's `/None` colourant "shall have no effect on the current
    /// page", which this tree says with a transparent paint, and a mask group is not the
    /// place for it to start painting black instead.
    ///
    /// That costs the luminosity branch one conversion it does not use. It is paid per
    /// *distinct* colour rather than per sample — `pdf_model::image`'s memo and palette are keyed
    /// on the samples, not on what they convert to — and buying it back would mean a second
    /// function deciding which spaces mark the page, which is exactly the drift trap 6 exists
    /// for.
    ///
    /// `generation` is §10.4.2.4's pair of functions where the graphics state states one, and it
    /// reaches only the subtractive branch — §11.7.5.3 scopes the pair to "painting an elementary
    /// object with a DeviceRGB colour directly into a transparency group whose colour space is
    /// DeviceCMYK". A caller with no graphics state has no stated pair and passes `None`
    /// honestly: `/BC`'s backdrop colour in `pdf_model::soft_mask` is the standing case, and it
    /// is the one §11.7.5.3's second bullet already answers for the rendering parameters. ADR
    /// 1207.
    #[expect(
        clippy::doc_markdown,
        reason = "the paragraph quotes §11.7.5.3 verbatim, and a quotation may not gain backticks"
    )]
    #[must_use]
    pub fn paint(
        &self,
        space: &ColourSpace,
        values: &[f32],
        rendering: Rendering,
        generation: Option<&BlackGeneration>,
    ) -> Color {
        let colour = space.to_rgb_under(values, rendering);
        match self {
            Self::Device => colour,
            Self::Luminosity(scale) => Color {
                a: colour.a,
                ..Color::grey(scale.grey_of(space, values))
            },
            // The same conversion as a `DeviceGray` mask group's, because §11.6.6 gives both
            // the same instruction — convert into the group's space on the way in — and
            // §10.4.2.2 and §10.4.2.3 are the conversions, `min` included: a `k` operator's
            // ink can weigh more than one unit and its grey cannot go below black.
            Self::Grey => Color {
                a: colour.a,
                ..Color::grey(InkScale::Unit.grey_of(space, values))
            },
            Self::Calibrated(route) => Color {
                a: colour.a,
                ..Color::grey(route.component_of(space, values))
            },
            Self::Additive(route) => {
                let [a, b, c] = route.components_of(space, values, rendering);
                Color {
                    a: colour.a,
                    ..Color::rgb(a, b, c)
                }
            }
            Self::Subtractive(plane, press, spots) => {
                // One discriminant test on every run a backend draws: with no spot colourant
                // the device is §11.4.7's four process components and nothing else (ADR 1311).
                let painted = if spots.is_empty() {
                    process_plane(
                        *plane,
                        space.to_cmyk_under(values, rendering, press, generation),
                    )
                } else {
                    spots.paint(*plane, press, space, values, rendering, generation)
                };
                Color {
                    a: colour.a,
                    ..painted
                }
            }
        }
    }
}
