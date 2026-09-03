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

use crate::colour::{ColourSpace, Compositing, GreyRoute, InkScale};
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
    /// space, the component of a one-component CIE-based space (`GreyRoute::of`), or the
    /// device's three channels.
    pub compositing: Compositing,
    /// Everything about this mask §11.5.3 asks for and does not get, for the caller to report.
    pub departures: Vec<String>,
}

/// Reads `/SMask` from a graphics state parameter dictionary.
pub(crate) fn entry(document: &Document, dict: &Dictionary) -> SoftMaskEntry {
    match document.get_key(dict, "SMask") {
        Object::Null => SoftMaskEntry::None,
        // Every other name is undefined here; `/None` is the only one Table 57 gives.
        Object::Name(name) if name.as_bytes() == b"None" => SoftMaskEntry::None,
        Object::Name(name) => SoftMaskEntry::Unusable(format!(
            "/SMask /{}, where Table 57 defines only /None",
            String::from_utf8_lossy(name.as_bytes())
        )),
        Object::Dictionary(mask) => read(document, &mask),
        other => SoftMaskEntry::Unusable(format!("/SMask is {}", type_of(&other))),
    }
}

/// Reads a soft-mask dictionary's four entries.
fn read(document: &Document, mask: &Dictionary) -> SoftMaskEntry {
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

    let (kind, compositing, transfer, departures) = match subtype.as_slice() {
        b"Alpha" => (
            SoftMaskKind::Alpha,
            Compositing::Device,
            transfer,
            Vec::new(),
        ),
        b"Luminosity" => {
            let space = ColourSpace::parse(document, &space, &Dictionary::new());
            let scale = space.as_ref().and_then(ink_scale);
            let route = match (&space, scale) {
                (Some(space), None) => GreyRoute::of(space).map(Arc::new),
                _ => None,
            };
            let ink = backdrop(document, mask, space.as_ref());
            let backdrop = match (scale, &route, &space) {
                // The group's elements are painted in `1 − ink ÷ scale`, so its backdrop is
                // too: a backdrop and the elements composited onto it have to be the same
                // quantity, or the compositing is not the clause's.
                (Some(scale), _, _) => Color::grey(1.0 - (ink / scale.factor()).min(1.0)),
                // And a one-component CIE-based group's elements are painted in the space's
                // own component, so `/BC` — "n numbers, where n is the number of components
                // in the colour space specified by the CS entry" — is that component.
                (None, Some(route), Some(space)) => {
                    Color::grey(route.component_of(space, &backdrop_values(document, mask, space)))
                }
                _ => Color {
                    a: 1.0,
                    ..space.as_ref().map_or(Color::BLACK, |space| {
                        space.to_rgb(&backdrop_values(document, mask, space))
                    })
                },
            };
            let departures = space
                .as_ref()
                .and_then(luminosity_departure)
                .into_iter()
                .collect();
            let compositing = match (scale, &route) {
                (Some(scale), _) => Compositing::Luminosity(scale),
                (None, Some(route)) => Compositing::Calibrated(Arc::clone(route)),
                (None, None) => Compositing::Device,
            };
            let transfer = match (scale, &route, &space) {
                (Some(scale), _, _) => derivation(scale, transfer.as_ref()),
                (None, Some(_), Some(space)) => {
                    Some(luminance_derivation(space, transfer.as_ref()))
                }
                _ => transfer.clone(),
            };
            (
                SoftMaskKind::Luminosity { backdrop },
                compositing,
                transfer,
                departures,
            )
        }
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
        departures,
    }))
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
///   *colorimetric* branch, the `Y` of the colour in CIE 1931 XYZ, and this tree answers with
///   the grey of the sRGB it converts every such space to. That is the same page-wide choice
///   recorded in [`luminosity_departure`] rather than a second view of it, and it does not
///   become truer by being computed one component at a time. `None`.
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

/// Names a mask group's `/CS` if neither of this tree's two routes is §11.5.3's.
///
/// §11.5.3 converts a mask group's composited colour to luminosity "in one of the following
/// ways, depending on the group's colour space", and there are two ways. This tree has two
/// routes to answer them with, and this function is what is left over.
///
/// **The device branch** — "For device colour spaces, convert the colour to `DeviceGray` by
/// implementation-defined means and use the resulting gray value as the luminosity" — is
/// carried out exactly, and since the three-hundred-and-eightieth session in the space the
/// group names rather than in the device's:
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
/// ADR 0796: `CalGray` and an `ICCBased` 'GRAY' group are painted in their own component
/// (`Compositing::Calibrated`, the route ADR 0792 built for a page) and
/// [`luminance_derivation`] takes the clause's `Y` of the composited component, exactly. For
/// **three** components — `CalRGB` and an RGB profile — it is still answered with the grey of
/// the sRGB this tree converts every such space to, and that is a *choice*: §11.6.6 already
/// treats a three-component CIE space as the device's own, page-wide, and a mask is not the
/// place to take a different view of the same question; taking the clause's `Y` there would
/// mean a backend computing a three-curve sum per pixel rather than reading a table.
/// `examples/luminosity_mask_census` counts both populations over a corpus.
///
/// What is left, and reported, is a `Lab` group. Its three components are not a linear map of
/// the device's, so neither route is the clause's: compositing `L*a*b*` on the device's RGB
/// is a different arithmetic *and* `0.3 R + 0.59 G + 0.11 B` of the sRGB is not the `Y` of the
/// XYZ the clause asks for. No corpus document states one, which is said out loud rather than
/// around — this is a report with no members, kept because the alternative is a silence.
fn luminosity_departure(space: &ColourSpace) -> Option<String> {
    matches!(space, ColourSpace::Lab { .. }).then(|| {
        "a soft mask's group is composited in device RGB and its luminosity taken there, \
         rather than in the blending colour space its /CS names"
            .to_owned()
    })
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
