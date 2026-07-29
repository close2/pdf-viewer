//! Reading a graphics state's `/SMask` (ISO 32000-2 §11.6.5.1, Table 142).
//!
//! This module answers *what the dictionary says*; running the group it names is
//! `crate::content`'s job, because evaluating a transparency group means running the
//! interpreter — the same reason a Type 3 font lives there rather than in `pdf-font`.
//!
//! The two halves are kept apart for the reason `crate::image`'s `soft_mask_entry` is: a
//! key is read once, and what it means is decided in one place, so a report cannot outlive
//! the gap it describes.

use pdf_render::{Color, SoftMaskKind, Transfer};
use pdf_syntax::{Dictionary, Document, Object, Stream};

use crate::colour::ColourSpace;
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
    pub group: std::sync::Arc<Stream>,
    /// Which of §11.5's two derivations applies, with `/BC` already resolved into the
    /// device's components for the luminosity one.
    pub kind: SoftMaskKind,
    /// Table 142's `/TR`, sampled, or `None` for the `/Identity` it defaults to.
    pub transfer: Option<Transfer>,
    /// The group's `/CS` if compositing the mask in it would differ from compositing in the
    /// device's RGB, for the caller to report.
    pub colour_space_departure: Option<String>,
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
    let (kind, departure) = match subtype.as_slice() {
        b"Alpha" => (SoftMaskKind::Alpha, None),
        b"Luminosity" => {
            let space = ColourSpace::parse(document, &space, &Dictionary::new());
            (
                SoftMaskKind::Luminosity {
                    backdrop: backdrop(document, mask, space.as_ref()),
                },
                space.as_ref().and_then(luminosity_departure),
            )
        }
        other => {
            return SoftMaskEntry::Unusable(format!(
                "/SMask /S /{}, where Table 142 defines /Alpha and /Luminosity",
                String::from_utf8_lossy(other)
            ));
        }
    };

    let transfer = match transfer(document, mask) {
        Ok(transfer) => transfer,
        Err(detail) => return SoftMaskEntry::Unusable(detail),
    };

    SoftMaskEntry::Mask(Box::new(SoftMaskRequest {
        group,
        kind,
        transfer,
        colour_space_departure: departure,
    }))
}

/// Resolves Table 142's `/BC` into the opaque device colour the group is composited onto.
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
///
/// A `/CS` this crate cannot parse leaves opaque black, which is what every space's initial
/// value converts to and what §11.5.3's NOTE 2 calls the useful backdrop.
fn backdrop(document: &Document, mask: &Dictionary, space: Option<&ColourSpace>) -> Color {
    let Some(space) = space else {
        return Color::BLACK;
    };
    let values: Vec<f32> = document
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
        .unwrap_or_else(|| space.initial_colour());

    // Opaque by construction: §11.5.3 composites the group onto "a fully opaque backdrop of
    // a specified colour", so whatever alpha a conversion produces is not this colour's.
    Color {
        a: 1.0,
        ..space.to_rgb(&values)
    }
}

/// Names a mask group's `/CS` if compositing it in device RGB changes the mask's values.
///
/// A mask group is composited here on the three components of the device raster and its
/// luminosity taken there, which is §11.5.3's device branch: "For device colour spaces,
/// convert the colour to `DeviceGray` by implementation-defined means and use the resulting
/// gray value as the luminosity" — the means being EXAMPLE 2's
/// `Y = 0.30 R + 0.59 G + 0.11 B`, which `pdf_render::SoftMask::value` computes. Four
/// answers ask for nothing else, and are silent:
///
/// - **`DeviceRGB`** is the space that formula is written for.
/// - **A three-component `CalRGB` or ICC space** is what §11.6.6 already treats as the
///   device's own, page-wide and as a documented choice; a mask is not the place to take a
///   different view of the same question. What it costs is §11.5.3's colorimetric branch —
///   `Y` of the colour converted to CIE XYZ — against the device luminosity of the sRGB
///   this tree converts everything to. That is a difference in the same class as compositing
///   in device RGB at all, and it is recorded as such rather than reported per mask.
/// - **`DeviceGray`, `CalGray` and a one-component ICC space** are *exact*, which is worth
///   showing rather than assuming: each converts to an `R = G = B` triple, and the three
///   coefficients sum to 1, so the luminosity of a grey is that grey whatever route it took.
///   That is 64 of the corpus's 134 mask dictionaries.
///
/// What is left is a space whose components are not those — `DeviceCMYK` above all, which is
/// 45 more of the 134. Compositing four inks as three has no reason to agree, and neither
/// does the luminosity: EXAMPLE 2's second formula is `Y = 1 − min(1, 0.3 C + 0.59 M +
/// 0.11 Y + K)`, where this takes the luminosity of whatever RGB `ColourSpace::to_rgb`
/// produced — and that conversion is a documented choice of ours (ADR 0009) rather than
/// anything the standard defines. Process black alone is the visible case: `K = 1` is a
/// mask value of 0 under the clause's formula and 32 under this route, so content the
/// producer masked away is faintly there.
fn luminosity_departure(space: &ColourSpace) -> Option<String> {
    let exact = match space {
        ColourSpace::Rgb
        | ColourSpace::Gray
        | ColourSpace::CalRgb { .. }
        | ColourSpace::CalGray { .. } => true,
        ColourSpace::Icc { profile } => matches!(profile.channels(), 1 | 3),
        _ => false,
    };
    (!exact).then(|| {
        "a soft mask's group is composited in device RGB and its luminosity taken there, \
         rather than in the blending colour space its /CS names"
            .to_owned()
    })
}

/// Samples Table 142's `/TR` onto the 256 values an eight-bit mask value can take.
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
