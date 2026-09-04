//! Reading a graphics state's `/SMask` (ISO 32000-2 §11.6.5.1, Table 142).
//!
//! This module answers *what the dictionary says*; running the group it names is
//! `crate::content`'s job, because evaluating a transparency group means running the
//! interpreter — the same reason a Type 3 font lives there rather than in `pdf-font`.
//!
//! The two halves are kept apart for the reason `crate::image`'s `soft_mask_entry` is: a
//! key is read once, and what it means is decided in one place, so a report cannot outlive
//! the gap it describes.

use std::sync::Arc;

use pdf_render::{Color, SoftMaskKind, Transfer};
use pdf_syntax::{Dictionary, Document, Object, Stream};

use crate::colour::{ColourSpace, Compositing, GreyRoute, Half, InkScale, Presses};
use crate::function::Function;

/// What a `gs` dictionary's `/SMask` entry holds, once read.
#[derive(Debug)]
pub(crate) enum SoftMaskEntry {
    /// Absent, or the name `/None`.
    ///
    /// §11.6.4.3: "The name None may be specified in place of a soft-mask dictionary,
    /// denoting the absence of a soft mask. It shall also mean that any existing mask shall
    /// be removed from the current graphics state."
    None,
    /// A soft-mask dictionary this crate can evaluate.
    Mask(Box<SoftMaskRequest>),
    /// Present, and something about it is not usable, with what.
    Unusable(String),
}

/// A soft-mask dictionary, resolved as far as it can be without running its group.
#[derive(Debug)]
pub(crate) struct SoftMaskRequest {
    /// Table 142's `/G`: "A transparency group `XObject` … that shall be used as the source
    /// of alpha or colour values for deriving the mask."
    pub group: Arc<Stream>,
    /// Which of §11.5's two derivations applies, with `/BC` already resolved for the
    /// luminosity one.
    pub kind: SoftMaskKind,
    /// Everything between the group's rendered channel and the mask value: §11.5.3's
    /// remaining arithmetic composed with Table 142's `/TR`, or `None` where that
    /// composition is the identity.
    pub transfer: Option<Transfer>,
    /// What the group's elements are painted in: the ink [`ink_scale`] decides for a device
    /// space, the component of a one-component CIE-based space (`GreyRoute::of`), the three
    /// components of a three-component one (`RgbRoute::of`), or the device's three channels.
    pub compositing: Compositing,
    /// §11.5.3's `Y` of a group painted in a CIE-based space's components — the three curves
    /// the backend sums, or a grid it interpolates over three components or four; `None`
    /// where the channels hold a grey or a device colour and the luminosity is the clause's
    /// device branch.
    pub luminance: Option<pdf_render::Luminance>,
    /// The backdrop §11.4.7's *second* raster composites onto, where the group's blending
    /// colour space has four components; `None` for every other mask.
    ///
    /// Its presence is what asks `crate::content` to interpret the group a second time under
    /// [`Half::Black`] — the commands themselves cannot be resolved here, because evaluating
    /// a transparency group means running the interpreter. `pdf_render::BlackHalf` is where
    /// the two meet.
    pub black_backdrop: Option<Color>,
    /// Everything about this mask §11.5.3 asks for and does not get, for the caller to report.
    pub departures: Vec<String>,
}

/// Reads `/SMask` from a graphics state parameter dictionary.
///
/// `presses` is the interpretation's own route cache, and it is a parameter rather than a
/// local for a reason worth stating: the route into a three-component *table* profile is
/// 36 000 profile evaluations for the conversion out and as many again for §11.5.3's `Y`
/// (ADR 0851), while a page can name one mask per `gs` and `6081357.pdf` names 912. Built
/// here, that would be a profile sampled once per soft-mask dictionary; asked of
/// [`Presses::rgb_route`], it is sampled once per interpretation per space.
pub(crate) fn entry(document: &Document, dict: &Dictionary, presses: &Presses) -> SoftMaskEntry {
    match document.get_key(dict, "SMask") {
        Object::Null => SoftMaskEntry::None,
        // Every other name is undefined here; `/None` is the only one Table 57 gives.
        Object::Name(name) if name.as_bytes() == b"None" => SoftMaskEntry::None,
        Object::Name(name) => SoftMaskEntry::Unusable(format!(
            "/SMask /{}, where Table 57 defines only /None",
            String::from_utf8_lossy(name.as_bytes())
        )),
        Object::Dictionary(mask) => read(document, &mask, presses),
        other => SoftMaskEntry::Unusable(format!("/SMask is {}", type_of(&other))),
    }
}

/// Reads a soft-mask dictionary's four entries.
fn read(document: &Document, mask: &Dictionary, presses: &Presses) -> SoftMaskEntry {
    // Table 142's `/G` is required, and is a stream: a form XObject with a `/Group`.
    let group = document.get_key(mask, "G");
    let Some(group) = group.as_stream().cloned() else {
        return SoftMaskEntry::Unusable("/SMask has no /G transparency group".to_owned());
    };

    // The group's own attributes decide what `/BC` means and what compositing the mask
    // happens in. §11.6.5.1: "If the subtype S is Luminosity , the group attributes
    // dictionary shall contain a CS entry defining the colour space in which the
    // compositing computation is to be performed."
    let attributes = document.get_key(&group.dict, "Group");
    let space = attributes.as_dict().map_or(Object::Null, |attributes| {
        document.get_key(attributes, "CS")
    });

    // Table 142's `/S` is required and takes exactly two values.
    let subtype = document.get_key(mask, "S");
    let Some(subtype) = subtype.as_name().map(|name| name.as_bytes().to_vec()) else {
        return SoftMaskEntry::Unusable("/SMask has no /S subtype".to_owned());
    };
    let transfer = match transfer(document, mask) {
        Ok(transfer) => transfer,
        Err(detail) => return SoftMaskEntry::Unusable(detail),
    };

    let (kind, compositing, transfer, luminance, black_backdrop, departures) =
        match subtype.as_slice() {
            b"Alpha" => (
                SoftMaskKind::Alpha,
                Compositing::Device,
                transfer,
                None,
                None,
                Vec::new(),
            ),
            b"Luminosity" => luminosity(document, mask, &space, presses, transfer.as_ref()),
            other => {
                return SoftMaskEntry::Unusable(format!(
                    "/SMask /S /{}, where Table 142 defines /Alpha and /Luminosity",
                    String::from_utf8_lossy(other)
                ));
            }
        };

    SoftMaskEntry::Mask(Box::new(SoftMaskRequest {
        group,
        kind,
        transfer,
        compositing,
        luminance,
        black_backdrop,
        departures,
    }))
}

/// Reads everything §11.5.3's derivation needs of a `/Luminosity` mask, once its `/CS` and
/// its `/TR` are in hand.
///
/// Split out of [`read`] because it is the whole of the clause and [`read`] is the whole of
/// Table 142: the five values it answers with are one decision — which of §11.5.3's two
/// branches the group's colour space takes, and in which quantity its elements are therefore
/// painted — taken once and threaded into the compositing, the backdrop, the transfer table
/// and the report together.
fn luminosity(
    document: &Document,
    mask: &Dictionary,
    space: &Object,
    presses: &Presses,
    transfer: Option<&Transfer>,
) -> (
    SoftMaskKind,
    Compositing,
    Option<Transfer>,
    Option<pdf_render::Luminance>,
    Option<Color>,
    Vec<String>,
) {
    let space = ColourSpace::parse(document, space, &Dictionary::new());
    let scale = space.as_ref().and_then(ink_scale);
    let route = match (&space, scale) {
        (Some(space), None) => GreyRoute::of(space).map(Arc::new),
        _ => None,
    };
    // A three-component CIE-based space, whose `Y` §11.5.3 asks for and
    // `RgbRoute::luminance` states: three curves summed for a `CalRGB` or a matrix
    // profile, a sampled grid for a table profile. Asked of the interpretation's
    // cache rather than built here, for the reason [`entry`] gives.
    let additive = match (&space, scale, &route) {
        (Some(space), None, None) => presses
            .rgb_route(space)
            .and_then(|route| route.luminance().map(|luminance| (route, luminance))),
        _ => None,
    };
    // And a space of **four** components, which §11.3.4 lists as a blending space and
    // §8.6.5.1 makes CIE-based, so §11.5.3 asks the same `Y` of it. The group is
    // painted in the press's own components — §11.4.7's pair, one scope over — and
    // the `Y` is the press's own grid over the four (`Press::luminance`, ADR 0857).
    // `None` where the interpretation may name no further press (§11.7.2's budget) or
    // the press is not a profile's, both of which `luminosity_departure` then names.
    let ink = match (&space, scale, &route, &additive) {
        (Some(ColourSpace::Icc { profile }), None, None, None) if profile.channels() == 4 => {
            presses
                .press_for_profile(profile)
                .and_then(|press| press.luminance().map(|luminance| (press, luminance)))
        }
        _ => None,
    };
    let weighed = backdrop(document, mask, space.as_ref());
    // The black half's backdrop, where there is one: §11.6.5.1's `/BC` has four
    // components and each raster composites onto the ones it carries.
    let black_backdrop = ink.as_ref().zip(space.as_ref()).map(|((press, _), space)| {
        let values = backdrop_values(document, mask, space);
        Color {
            a: 1.0,
            ..Compositing::Subtractive(Half::Black, Arc::clone(press)).paint(space, &values, true)
        }
    });
    let backdrop = match (scale, &route, &additive, &ink, &space) {
        // The group's elements are painted in `1 − ink ÷ scale`, so its backdrop is
        // too: a backdrop and the elements composited onto it have to be the same
        // quantity, or the compositing is not the clause's.
        (Some(scale), _, _, _, _) => Color::grey(1.0 - (weighed / scale.factor()).min(1.0)),
        // And a one-component CIE-based group's elements are painted in the space's
        // own component, so `/BC` — "n numbers, where n is the number of components
        // in the colour space specified by the CS entry" — is that component.
        (None, Some(route), _, _, Some(space)) => {
            Color::grey(route.component_of(space, &backdrop_values(document, mask, space)))
        }
        // And a three-component one's in its three components, which `/BC` states
        // as three numbers in that space.
        (None, None, Some((route, _)), _, Some(space)) => {
            let [a, b, c] =
                route.components_of(space, &backdrop_values(document, mask, space), true);
            Color::rgb(a, b, c)
        }
        // And a four-component group's chromatic half: §11.3.4's additive complements
        // of cyan, magenta and yellow, which is what that raster carries.
        (None, None, None, Some((press, _)), Some(space)) => {
            let values = backdrop_values(document, mask, space);
            Color {
                a: 1.0,
                ..Compositing::Subtractive(Half::Chromatic, Arc::clone(press))
                    .paint(space, &values, true)
            }
        }
        _ => Color {
            a: 1.0,
            ..space.as_ref().map_or(Color::BLACK, |space| {
                space.to_rgb(&backdrop_values(document, mask, space))
            })
        },
    };
    let compositing = match (scale, &route, &additive, &ink) {
        (Some(scale), _, _, _) => Compositing::Luminosity(scale),
        (None, Some(route), _, _) => Compositing::Calibrated(Arc::clone(route)),
        (None, None, Some((route, _)), _) => Compositing::Additive(Arc::clone(route)),
        (None, None, None, Some((press, _))) => {
            Compositing::Subtractive(Half::Chromatic, Arc::clone(press))
        }
        (None, None, None, None) => Compositing::Device,
    };
    // Asked of the compositing that was chosen rather than of the space alone, which
    // is what makes the condition §11.5.3's own: the clause branches on whether the
    // space is CIE-based, so the departure is a CIE-based space this tree did *not*
    // send down the colorimetric branch, whatever the reason.
    let colorimetric = matches!(
        compositing,
        Compositing::Calibrated(_) | Compositing::Additive(_) | Compositing::Subtractive(..)
    );
    let departures = space
        .as_ref()
        .and_then(|space| luminosity_departure(space, colorimetric))
        .into_iter()
        .collect();
    let transfer = match (scale, &route, &space) {
        (Some(scale), _, _) => derivation(scale, transfer),
        (None, Some(_), Some(space)) => Some(luminance_derivation(space, transfer)),
        // A three- or four-component group's `Y` is read off the space's own grid or
        // curves *before* this table, so the table is Table 142's `/TR` alone.
        _ => transfer.cloned(),
    };
    let luminance = match (additive, ink) {
        (Some((_, luminance)), _) | (None, Some((_, luminance))) => Some(luminance),
        (None, None) => None,
    };
    (
        SoftMaskKind::Luminosity { backdrop },
        compositing,
        transfer,
        luminance,
        black_backdrop,
        departures,
    )
}

/// Everything left between a scaled mask channel and the value §11.5.3 derives from it.
///
/// The group is painted in `1 − ink ÷ scale` so that the compositing happens on the ink
/// itself; what remains is [`InkScale::mask_value`] — §10.4.2.3's `1 − min(1, ink)` with the
/// `min` where §11.5.3 puts it, *after* the group has been composited — and then Table 142's
/// `/TR`, which §11.5.3 puts last of all:
///
/// > Following this conversion, the result shall be passed through a separately specified
/// > transfer function, allowing the masking effect to be customised.
///
/// The two are composed into one table rather than applied in turn, and the reason is not
/// economy. A backend that can express a luminosity mask *natively* — `render-quorra` does —
/// takes a backdrop colour and a 256-entry table and computes the luminosity in a shader of
/// its own, so a second arithmetic step outside the table would be a step the CPU oracle
/// takes and the graphics device does not. Composed here, both backends are handed the same
/// 256 bytes and cannot disagree by construction.
///
/// **What that costs is one rounding, and it is worth writing down.** The channel is eight
/// bits, so at [`InkScale::Double`] a mask value is recovered in steps of `2 ÷ 255` rather
/// than `1 ÷ 255`: at most one more level of 255 than the raster already rounds away, against
/// a departure the same clause puts at up to `(1 − α) · e` — half the mask's whole range at a
/// half-covered pixel over registration black. At [`InkScale::Unit`] the map is the identity
/// and this returns the `/TR` it was handed, so a `DeviceGray` group pays nothing at all.
fn derivation(scale: InkScale, transfer: Option<&Transfer>) -> Option<Transfer> {
    if scale == InkScale::Unit {
        return transfer.cloned();
    }
    let mut table = [0_u8; 256];
    for (index, entry) in table.iter_mut().enumerate() {
        #[expect(
            clippy::cast_precision_loss,
            reason = "index of a 256-entry table is exactly representable"
        )]
        let channel = index as f32 / 255.0;
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "clamped to 0..=255 before the cast, and NaN clamps to the low bound"
        )]
        let derived = (scale.mask_value(channel) * 255.0)
            .clamp(0.0, 255.0)
            .round() as u8;
        *entry = transfer.map_or(derived, |transfer| transfer.apply(derived));
    }
    Some(Transfer::from_samples(table))
}

/// Everything left between a composited *component* and the value §11.5.3 derives from it,
/// for a mask group whose `/CS` is `CalGray` or a one-component `ICCBased` space.
///
/// Such a group is painted in the space's own component (`Compositing::Calibrated`, ADR
/// 0792's route for a page or an isolated group, and since ADR 0796 a mask's), so what the
/// backend composites is §11.6.6's "resulting colour at each point … interpreted in the group
/// colour space", and what §11.5.3 asks of it is the colorimetric branch:
///
/// > For CIE-based spaces, convert to the CIE 1931 XYZ space and use the Y component as the
/// > luminosity. This produces a colorimetrically correct luminosity.
///
/// That `Y` is a function of one component — §8.6.5.2's gamma, or a profile's tone curve —
/// so it is sampled onto the same 256-entry table [`derivation`] uses, with Table 142's `/TR`
/// composed after it, for the same reason: a backend that computes the mask natively takes a
/// backdrop and a table and nothing else, and both backends are handed the same bytes.
///
/// **This is exact where the device branch's `InkScale::Double` is a rounding**: the channel
/// holds the component itself, so nothing is halved to fit.
fn luminance_derivation(space: &ColourSpace, transfer: Option<&Transfer>) -> Transfer {
    let mut table = [0_u8; 256];
    for (index, entry) in table.iter_mut().enumerate() {
        #[expect(
            clippy::cast_precision_loss,
            reason = "index of a 256-entry table is exactly representable"
        )]
        let component = index as f32 / 255.0;
        let luminance = space.cie_luminance(&[component]).unwrap_or(component);
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "clamped to 0..=255 before the cast, and NaN clamps to the low bound"
        )]
        let derived = (luminance * 255.0).clamp(0.0, 255.0).round() as u8;
        *entry = transfer.map_or(derived, |transfer| transfer.apply(derived));
    }
    Transfer::from_samples(table)
}

/// How much ink a group blending in this space can weigh, or `None` if §11.5.3 sends it to
/// `DeviceGray` through RGB rather than through §10.4.2.3.
///
/// §11.6.5.1 makes the group's `/CS` "the colour space in which the compositing computation
/// is to be performed", and §11.5.3 converts the composited colour to luminosity afterwards.
/// This tree composites on the three components of the device raster, so the two agree only
/// where the space's luminosity is a function of that RGB — and §10.4.2 says exactly when
/// that is:
///
/// - **`DeviceRGB`** is the space §10.4.2.2's `gray = 0.3 R + 0.59 G + 0.11 B` is written
///   for, so compositing there and converting afterwards is the clause. `None`.
/// - **A CIE-based space** — `CalRGB`, `CalGray`, `Lab`, `ICCBased` — asks for §11.5.3's
///   *colorimetric* branch, the `Y` of the colour in CIE 1931 XYZ, which is no ink at all:
///   `None`, and [`read`] answers it with the space's own route where there is one — a
///   component for one, three components and three curves for three — as
///   [`luminosity_departure`] records.
/// - **A subtractive space** — `DeviceCMYK`, `DeviceGray`, and a `Separation`, `DeviceN` or
///   `Indexed` space resting on one — is the case this answers with a scale. §10.4.2.3
///   converts it to a grey level without going near RGB, so a group composited in the
///   device's three components is a different arithmetic and this tree's `DeviceCMYK` → RGB
///   table (ADR 0009) has no business standing in the way of a formula the clause prints in
///   full.
///
/// `DeviceGray` counts as subtractive here, which is worth the sentence: §10.4.2.3 calls a
/// grey level "the complement of the black component of CMYK", and a `k` operator inside a
/// `/DeviceGray` mask group is the same conversion one component narrower. It is also why
/// the two spaces get *different* scales rather than one: §11.6.6's `/CS` is "[t]he colour
/// space into which colours shall be converted when painted into the group", and converting
/// into `DeviceGray` applies §10.4.2.3's `min` on the way in, so nothing in such a group can
/// weigh more than one unit. A `DeviceCMYK` group keeps four components until §11.5.3 and can
/// hold two. [`InkScale`] has the arithmetic.
///
/// A `Separation`, `DeviceN` or `Indexed` group colour space is one §11.6.6 excludes outright:
/// the restrictions there "exclude `Lab` and lightness-chromaticity `ICCBased` colour spaces,
/// as well as the special colour spaces `Pattern` , `Indexed` , `Separation` , and `DeviceN`",
/// so the recursion here is about a malformed file rather than a valid one, and it answers
/// with the scale of the space the file would have had to mean.
fn ink_scale(space: &ColourSpace) -> Option<InkScale> {
    match space {
        ColourSpace::Cmyk => Some(InkScale::Double),
        ColourSpace::Gray => Some(InkScale::Unit),
        ColourSpace::Separation { alternate, .. } => ink_scale(alternate),
        ColourSpace::Indexed { base, .. } => ink_scale(base),
        _ => None,
    }
}

/// Resolves Table 142's `/BC` into the components the group is composited onto.
///
/// §11.6.5.1 Table 142, of the entry:
///
/// > An array of component values specifying the colour that shall be used as the backdrop
/// > against which to composite the transparency group XObject G . This entry shall be
/// > consulted only if the subtype S is Luminosity . The array shall consist of n numbers,
/// > where n is the number of components in the colour space specified by the CS entry in
/// > the group attributes dictionary (see 11.6.6, "Transparency group XObjects" ). Default
/// > value: the colour space's initial value, representing black.
///
/// The count is what makes the group's `/CS` load-bearing here rather than decorative: the
/// same three numbers mean different colours in `DeviceRGB` and in a `Lab` space, and a
/// `/DeviceCMYK` backdrop is four. Where `/BC` is absent the default is "the colour space's
/// initial value, representing black", which `ColourSpace::initial_colour` already holds
/// for every space this crate reads (§8.6.8's five cases).
fn backdrop_values(document: &Document, mask: &Dictionary, space: &ColourSpace) -> Vec<f32> {
    document
        .get_key(mask, "BC")
        .as_array()
        .map(|items| {
            items
                .iter()
                .map(|item| document.resolve(item))
                .filter_map(|item| item.as_number())
                .map(|value| {
                    #[expect(
                        clippy::cast_possible_truncation,
                        reason = "a colour component outside f32's range is not one"
                    )]
                    {
                        value as f32
                    }
                })
                .collect()
        })
        .filter(|values: &Vec<f32>| values.len() == space.components())
        .unwrap_or_else(|| space.initial_colour())
}

/// The ink §10.4.2.3 weighs in Table 142's `/BC`.
///
/// A `/CS` this crate cannot parse leaves one whole unit, which is what every space's initial
/// value weighs and what §11.5.3's NOTE 2 calls the useful backdrop.
///
/// No alpha is taken from the conversion, and that is the clause rather than a simplification:
/// §11.5.3 composites the group onto "a fully opaque backdrop of a specified colour", so an
/// alpha a conversion happens to produce — a `/None` colourant's zero, say — is not this
/// colour's.
fn backdrop(document: &Document, mask: &Dictionary, space: Option<&ColourSpace>) -> f32 {
    space.map_or(1.0, |space| {
        space.ink(&backdrop_values(document, mask, space))
    })
}

/// Names a mask group's `/CS` where §11.5.3's colorimetric branch was not taken for it.
///
/// §11.5.3 converts a mask group's composited colour to luminosity "in one of the following
/// ways, depending on the group's colour space", and there are two ways, chosen by whether the
/// space is CIE-based. This tree carries out the device branch exactly and the colorimetric
/// branch for every CIE-based space it has a route into; what is left over is this, and the
/// condition is `colorimetric` rather than a list of space names for the reason trap 11 gives
/// — the clause's own branch is the discriminator, so a space that stops having a route stops
/// being silent by construction.
///
/// **The device branch** — "For device colour spaces, convert the colour to `DeviceGray` by
/// implementation-defined means and use the resulting gray value as the luminosity, with no
/// compensation for gamma or other colour calibration" — is carried out exactly, and since the
/// three-hundred-and-eightieth session in the space the group names rather than in the device's:
///
/// - **`DeviceRGB`** is the space EXAMPLE 2's `Y = 0.30 R + 0.59 G + 0.11 B` is written for,
///   and `pdf_render::SoftMask::value` computes it on the rendered pixel.
/// - **`DeviceCMYK` and `DeviceGray`** send a colour to grey by §10.4.2.3 without passing
///   through RGB, so the group's elements are painted in the ink that clause weighs instead
///   of in colour — [`ink_scale`], and `crate::colour`'s `Compositing::Luminosity`. Since the
///   three-hundred-and-eighty-third session that reaches an image's samples and a shading's
///   ramp as well as an operator's colour, and the `min` waits for the compositing the way
///   §11.5.3 states (ADR 0220).
///
/// **The colorimetric branch** — "For CIE-based spaces, convert to the CIE 1931 XYZ space and
/// use the Y component as the luminosity" — is taken for a space of **one** component since
/// ADR 0796 (`Compositing::Calibrated`, with [`luminance_derivation`] composing the clause's
/// `Y` of the composited component into the mask's table), for **three** since ADRs 0797 and
/// 0851 (`Compositing::Additive`, with `RgbRoute::luminance` read per pixel), and for **four**
/// since ADR 0857 (`Compositing::Subtractive`, §11.4.7's pair of rasters with
/// `Press::luminance` read off the pair).
///
/// What is left, and named here, is three shapes:
///
/// - **A `Lab` group.** §11.3.4 rules the space out of a blending space outright, and neither
///   route is the clause's: compositing `L*a*b*` on the device's RGB is a different arithmetic
///   *and* `0.3 R + 0.59 G + 0.11 B` of the sRGB is not the `Y` of the XYZ the clause asks for.
/// - **A four-component CIE-based group this interpretation has no press for.** The space
///   itself is drawn since ADR 0857 — §11.4.7's pair of rasters inside the mask, with the
///   `Y` read off the press's own grid over the four components — so what is left here is a
///   file whose page has already spent `crate::colour::MAX_PRESSES` on other spaces, or a
///   profile that yields no grid at all. Both are the budget or the file rather than the
///   clause, and both are named.
/// - **A CIE-based group this tree has no route into**: a profile that states no "from CIE"
///   half, which §11.3.4 requires of a blending space — "the ICC profile shall be capable of
///   both device to PCS and PCS to device transformations" — or a one-component curve with no
///   inverse. §11.6.5.1 makes the `/CS` "the colour space in which the compositing computation
///   is to be performed", and without a conversion in there is nothing to composite in it.
///
/// Each is drawn on the device's three components with EXAMPLE 2's weights, which is the
/// picture that was there before the report and is stated rather than left to be discovered.
fn luminosity_departure(space: &ColourSpace, colorimetric: bool) -> Option<String> {
    if colorimetric {
        return None;
    }
    match space {
        ColourSpace::Lab { .. } => Some(
            "a soft mask's group is composited in device RGB and its luminosity taken there, \
             rather than in the blending colour space its /CS names: §11.3.4 forbids Lab as a \
             blending colour space, so §11.5.3 has no CIE-based route to take for it"
                .to_owned(),
        ),
        ColourSpace::Icc { profile } if profile.channels() == 4 => Some(
            "a soft mask's group states a four-component ICCBased /CS this reader has no \
             press for — §11.3.4 requires a blending space's profile to be \"capable of both \
             device to PCS and PCS to device transformations\", and a page may name only so \
             many distinct presses — so its luminosity is taken as §11.5.3's device branch on \
             device RGB rather than as the Y of the CIE 1931 XYZ"
                .to_owned(),
        ),
        ColourSpace::CalGray { .. } | ColourSpace::CalRgb { .. } | ColourSpace::Icc { .. } => Some(
            "a soft mask's group states a CIE-based /CS this reader has no route into — \
                 §11.3.4 requires a blending space's profile to be \"capable of both device to \
                 PCS and PCS to device transformations\", and a curve with no inverse is not — \
                 so its luminosity is taken as §11.5.3's device branch on device RGB rather \
                 than as the Y of the CIE 1931 XYZ"
                .to_owned(),
        ),
        _ => None,
    }
}

/// Samples Table 142's `/TR` onto the 256 values an eight-bit mask value can take.
///
/// This is the document's function alone; [`derivation`] is what composes it with the rest of
/// §11.5.3's arithmetic for a group painted in a scaled channel.
///
/// `Ok(None)` is the identity, which is both the default and what the name `/Identity`
/// selects. An unreadable function is an error rather than a silently identity one: a
/// transfer function is how a producer inverts or shapes a mask, and ignoring it draws the
/// mask the file did not ask for.
fn transfer(document: &Document, mask: &Dictionary) -> Result<Option<Transfer>, String> {
    let entry = document.get_key(mask, "TR");
    match &entry {
        Object::Null => return Ok(None),
        Object::Name(name) if name.as_bytes() == b"Identity" => return Ok(None),
        _ => {}
    }
    let function = Function::parse(document, &entry)
        .map_err(|error| format!("/SMask has a /TR and {error}"))?;

    let mut table = [0_u8; 256];
    for (index, entry) in table.iter_mut().enumerate() {
        #[expect(
            clippy::cast_precision_loss,
            reason = "index of a 256-entry table is exactly representable"
        )]
        let input = index as f32 / 255.0;
        let outputs = function.eval(&[input]);
        let Some(value) = outputs.first() else {
            return Err("/SMask has a /TR returning no value".to_owned());
        };
        // §11.6.5.1: "The computed output shall be in the range 0.0 to 1.0; if it falls
        // outside this range, it shall be forced to the nearest valid value."
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "clamped to 0..=255 before the cast, and NaN clamps to the low bound"
        )]
        {
            *entry = (value * 255.0).clamp(0.0, 255.0).round() as u8;
        }
    }
    Ok(Some(Transfer::from_samples(table)))
}

/// Names an object's kind for a report.
fn type_of(object: &Object) -> &'static str {
    match object {
        Object::Null => "absent",
        Object::Boolean(_) => "a boolean",
        Object::Integer(_) | Object::Real(_) => "a number",
        Object::String(_) => "a string",
        Object::Name(_) => "a name",
        Object::Array(_) => "an array",
        Object::Dictionary(_) => "a dictionary",
        Object::Stream(_) => "a stream",
        Object::Reference(_) => "an unresolved reference",
    }
}
