//! An image's colour space and its `/Decode` map, as the image dictionary states them.
//!
//! ISO 32000-2 §8.9.5 says what the two entries are and §8.9.5.2 how `/Decode` maps a sample
//! onto its colour space's range; an image mask (§8.9.6.2) has no colour space and its own
//! one-bit map. [`colour_space`] reads `/ColorSpace` against the page's resources, and
//! [`Decode`] tabulates the map once per image so that a sample is a lookup.

use pdf_syntax::{Dictionary, Document, Object};

use crate::colour::{Compositing, Conversion};

use super::ImageError;
use super::convert::channel;

/// How an image's samples are interpreted.
#[derive(Debug, Clone)]
pub(super) enum ColourSpace {
    /// One component per sample.
    Gray,
    /// Three components per sample.
    Rgb,
    /// Four components per sample, subtractive.
    Cmyk,
    /// A stencil mask: one bit per sample, painted in the current fill colour.
    Mask,
    /// Any other space, converted sample by sample through the colour module.
    ///
    /// A CIE-based space is *not* its device equivalent. ISO 32000-2 §8.6.5.2 and §8.6.5.3
    /// define `CalGray` and `CalRGB` in CIE terms, so a `Gamma 1` grey is a linear luminance
    /// and writing it into an sRGB raster unchanged renders the image far too dark.
    /// `Indexed`, `Separation` and `DeviceN` are not device spaces either: each is a function
    /// of its sample rather than a colour. Converting per sample costs what the `DeviceCMYK`
    /// arm already costs and buys the same thing: one answer for a colour, whether it reached
    /// the page as a fill or as an image.
    Resolved(crate::colour::ColourSpace),
}

impl ColourSpace {
    /// A reduced source still knows whether §11.7.5.3's `DeviceRGB` conversion bypassed the
    /// stated pair. Fast RGB can only arise where the target needs no such conversion.
    pub(super) fn bypasses_black_generation(&self, into: &Conversion) -> bool {
        match self {
            Self::Resolved(space) => into.bypasses_black_generation(space),
            _ => false,
        }
    }

    /// Reduces a resolved space to the arm the unpacker can take fastest.
    ///
    /// The three device families are eight-bit channels and nothing else, so they get an arm
    /// that reads bytes; every other space is a *function* of its samples and takes the one
    /// that converts. This is a choice about speed and never about meaning — the two routes
    /// agree by construction, because `crate::colour::ColourSpace::to_rgb` is the identity on
    /// a device space's components (§8.6.4, and the test that pins it).
    ///
    /// **They stop agreeing inside a `/Luminosity` mask group**, which is why the reduction
    /// asks what the samples are being composited into. There a sample becomes §10.4.2.3's
    /// ink rather than a colour (`crate::colour::Compositing`), and that is the identity on
    /// no device space at all: a grey `g` painted into a `DeviceCMYK` group is `(1 + g) ÷ 2`
    /// on the channel. So the fast arms are not taken and every sample goes through the one
    /// conversion, memoised by [`SampleMemo`] or tabulated by [`palette`] as it would be for
    /// any other space. That costs a mask group's images the fast path and nothing else pays
    /// (ADR 0220).
    ///
    /// A page §11.4.7 composites in `DeviceCMYK` is the same argument again: a sample becomes
    /// one half of that space's four components, which is the identity on no device space
    /// either — a `DeviceRGB` sample goes through §10.4.2.4 and a `DeviceCMYK` one is split
    /// between the two rasters. So those images take the converting arm as a mask group's do,
    /// and only a page that states such a space pays for it (ADR 0262).
    ///
    /// [`SampleMemo`]: super::convert::SampleMemo
    /// [`palette`]: super::unpack::palette
    fn reduced(space: crate::colour::ColourSpace, into: Compositing) -> Self {
        match (space, into) {
            // A calibrated one-component page is the argument once more, and it takes the
            // converting arm before the `DeviceGray` exception below: a `DeviceGray` sample
            // is *not* its own component there, because the component is the curve's
            // inverse of that grey (ADR 0792).
            // A three-component CIE-based page keeps the `DeviceRGB` fast arm and no other:
            // §11.7.2 makes a device colour of three components the group's space "for
            // compositing purposes only", so a `DeviceRGB` sample *is* its own components
            // there, where a grey or a CMYK one is converted in through the route (ADR 0797).
            (crate::colour::ColourSpace::Rgb, Compositing::Additive(_)) => Self::Rgb,
            (
                space,
                Compositing::Luminosity(_)
                | Compositing::Subtractive(..)
                | Compositing::Calibrated(_)
                | Compositing::Additive(_),
            ) => Self::Resolved(space),
            // A page composited in `DeviceGray` is the same argument once more, with one
            // exception the arm order states: a `DeviceGray` sample *is* its own grey, so the
            // fast arm is exact there, and every other space's sample becomes §10.4.2.2's or
            // §10.4.2.3's grey through the one conversion.
            (crate::colour::ColourSpace::Gray, _) => Self::Gray,
            (space, Compositing::Grey) => Self::Resolved(space),
            (crate::colour::ColourSpace::Rgb, _) => Self::Rgb,
            (crate::colour::ColourSpace::Cmyk, _) => Self::Cmyk,
            (other, _) => Self::Resolved(other),
        }
    }

    pub(super) fn components(&self) -> usize {
        match self {
            Self::Gray | Self::Mask => 1,
            Self::Rgb => 3,
            Self::Cmyk => 4,
            Self::Resolved(space) => space.components(),
        }
    }

    /// Table 88's default `/Decode` pair for one component, at this bit depth.
    ///
    /// Every device space's components run from 0.0 to 1.0, so their default pair is that
    /// range; the two spaces where it is not — `Lab`, whose lightness is a percentage and
    /// whose chromatic axes take the space's own `/Range`, and `Indexed`, whose default
    /// passes an index through unchanged — answer for themselves.
    fn default_decode(&self, component: usize, bits: u32) -> (f32, f32) {
        match self {
            Self::Resolved(space) => space.default_decode(component, bits),
            _ => (0.0, 1.0),
        }
    }

    /// The range a decoded value of one component is permitted to take.
    ///
    /// §8.9.5.2's closing sentence: "If an output value is not permitted for a component, it
    /// shall be adjusted to the nearest allowed value." A `/Decode` array may state any two
    /// numbers — the clause says so one paragraph earlier — so the map's output is clamped
    /// here rather than trusted.
    pub(super) fn permitted(&self, component: usize) -> (f32, f32) {
        match self {
            Self::Resolved(space) => space.component_range(component),
            _ => (0.0, 1.0),
        }
    }
}

/// §8.9.5.2's `/Decode` array: what an integer sample means as a colour component.
///
/// > Samples with a value of 0 shall be mapped to D min … those with intermediate values
/// > shall be mapped linearly between D min and D max
///
/// Held as one table per component rather than as the clause's formula. Every input the
/// formula takes — the pair, the bit depth, the component's permitted range — is fixed by
/// the image dictionary before a sample is read, and its domain has at most 2^n points,
/// where n is 1 or 8 here. So the map is evaluated at most 256 times per component per
/// image instead of once per component per *pixel*, and the unpacker's arms become a lookup
/// that no longer has to know what a `/Decode` array is.
pub(super) struct Decode {
    /// `values[component][sample]`, in the colour space's own component values.
    values: Vec<Vec<f32>>,
    /// [`Self::values`] quantised, because a device space's components *are* eight-bit
    /// channels and the arms that produce one would otherwise clamp, scale and round per
    /// pixel. 256 entries per component against a photograph's millions, and the benchmark
    /// that says it earns its place: interpreting `issue19971.pdf`'s 2500x1364 `DeviceRGB`
    /// photograph costs 162.40 G instructions with this table and 166.35 G by quantising per
    /// pixel, against 161.54 G before `/Decode` was applied at all. So it takes the whole
    /// clause's cost on the corpus's largest image from +2.98% to +0.54%, and to nothing
    /// measurable on a page carrying no image.
    pub(super) channels: Vec<Vec<u8>>,
}

impl Decode {
    /// Reads `/Decode`, or Table 88's default for `space` where the dictionary states none.
    pub(super) fn read(
        document: &Document,
        dict: &Dictionary,
        space: &ColourSpace,
        bits: u32,
    ) -> Self {
        let stated: Vec<f32> = document
            .get_key(dict, "Decode")
            .as_array()
            .map(|items| {
                items
                    .iter()
                    .map(|item| document.resolve(item))
                    .filter_map(|item| item.as_number())
                    .map(|value| {
                        #[expect(
                            clippy::cast_possible_truncation,
                            reason = "a component value beyond f32 is clamped to its \
                                      permitted range either way"
                        )]
                        {
                            value as f32
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();
        Self::from_pairs(&stated, space, bits)
    }

    /// The same map from an already-read `/Decode` array, empty for Table 88's defaults.
    ///
    /// Split from [`Self::read`] so that a caller with no document — a unit test — can build
    /// the map a real image would get.
    pub(super) fn from_pairs(stated: &[f32], space: &ColourSpace, bits: u32) -> Self {
        // 2^n − 1, the largest sample the depth can carry, and the formula's x max.
        let max = (1u32 << bits.min(16)).saturating_sub(1);
        let span = f32::from(u16::try_from(max.max(1)).unwrap_or(u16::MAX));

        let mut values = Vec::with_capacity(space.components());
        for component in 0..space.components() {
            let at = component.saturating_mul(2);
            // A `/Decode` array "shall contain one pair of numbers for each component"; one
            // that does not is honoured as far as it goes, since the alternative is
            // discarding a pair the producer did state.
            let (low, high) = match (stated.get(at), stated.get(at.saturating_add(1))) {
                (Some(low), Some(high)) => (*low, *high),
                _ => space.default_decode(component, bits),
            };
            let (floor, ceiling) = space.permitted(component);
            let table = (0..=max)
                .map(|raw| {
                    let sample = f32::from(u16::try_from(raw).unwrap_or(u16::MAX));
                    // D min + x × (D max − D min) ÷ (2^n − 1), with the multiplication
                    // before the division: an `Indexed` space's default pair is
                    // `[0 2^n − 1]`, which the clause's NOTE 2 says "ensures that component
                    // values … are passed through unchanged", and `raw ÷ 255 × 255` is
                    // 253.99998 for a sample of 254 while `raw × 255 ÷ 255` is 254.
                    let value = (high - low).mul_add(sample, low * span) / span;
                    value.clamp(floor.min(ceiling), ceiling.max(floor))
                })
                .collect::<Vec<f32>>();
            values.push(table);
        }

        let channels = values
            .iter()
            .map(|table| table.iter().copied().map(channel).collect())
            .collect();
        Self { values, channels }
    }

    /// What sample `raw` of component `component` means.
    pub(super) fn value(&self, component: usize, raw: usize) -> f32 {
        self.values
            .get(component)
            .and_then(|table| table.get(raw))
            .copied()
            .unwrap_or(0.0)
    }

    /// The same, quantised to an eight-bit channel.
    pub(super) fn channel(&self, component: usize, raw: usize) -> u8 {
        self.channels
            .get(component)
            .and_then(|table| table.get(raw))
            .copied()
            .unwrap_or(0)
    }

    /// Whether the map is the identity on eight-bit device channels.
    ///
    /// The one question the `DCTDecode` route asks, because that route has already turned
    /// the codestream into eight-bit channels and only has to know whether to remap them.
    pub(super) fn is_identity(&self) -> bool {
        self.channels.iter().all(|table| {
            table.len() == 256
                && table
                    .iter()
                    .enumerate()
                    .all(|(raw, value)| u8::try_from(raw).is_ok_and(|raw| raw == *value))
        })
    }
}

/// An image dictionary, the document holding it, and the resources it was named from.
///
/// One argument rather than three because the three are never apart. Table 87's entries are in the
/// dictionary, the objects they reference are in the document, and §8.6.5.6's `/DefaultGray`,
/// `/DefaultRGB` and `/DefaultCMYK` are in the resource dictionary the image was *drawn* from,
/// which is a property of the `Do` rather than of the image (ADR 0034).
#[derive(Clone, Copy)]
pub(super) struct Dictionaries<'a> {
    /// The document, for resolving anything indirect.
    pub(super) document: &'a Document,
    /// The image's own dictionary.
    pub(super) dict: &'a Dictionary,
    /// The resources in force where the image was drawn.
    pub(super) resources: &'a Dictionary,
}

/// Determines the colour space, reduced to what the sample unpacker handles.
pub(super) fn colour_space(
    document: &Document,
    dict: &Dictionary,
    resources: &Dictionary,
    into: &Conversion,
) -> Result<ColourSpace, ImageError> {
    let space = document.get_key(dict, "ColorSpace");
    // §8.9.5.1 Table 87, of `/ColorSpace`: "it can be any type of colour space except
    // Pattern". A pattern carries no colour of its own, so a sample in one names nothing
    // that could be unpacked, and the refusal has to come before the parse because a
    // `/Pattern` array *is* a colour space the colour module reads.
    let family = match &space {
        Object::Name(name) => name.as_bytes().to_vec(),
        Object::Array(items) => items
            .first()
            .map(|item| document.resolve(item))
            .and_then(|item| item.as_name().map(|name| name.as_bytes().to_vec()))
            .unwrap_or_default(),
        _ => {
            return Err(ImageError::UnsupportedColourSpace {
                space: "absent".to_owned(),
            });
        }
    };
    if family == b"Pattern" || family == b"P" {
        return Err(ImageError::UnsupportedColourSpace {
            space: "Pattern, which Table 87 excludes".to_owned(),
        });
    }

    // Everything else goes through the one function that decides what a colour space *is*,
    // against the resources the image is drawn from. Two clauses want it that way and this
    // route was reaching neither. §8.6.5.6: the remapping through `/DefaultGray`,
    // `/DefaultRGB` or `/DefaultCMYK` applies to "a colour space given as an entry in an
    // image XObject, inline image, or shading dictionary", and an empty resource dictionary
    // used to be passed here, so a device space named by an image was never remapped.
    // §8.6.5.5: an `ICCBased` space's profile is the document's own statement of what its
    // numbers mean, and this function used to reduce such a space to a device one by its
    // `/N` — so the same colour rendered differently depending on whether it reached the
    // page as a fill or as an image, which is exactly trap 6's defect one level up.
    //
    // And §14.11.5's output intent, the third source `ColourSpace::device_family` ranks: a space
    // parsed with no intent would make an image in `DeviceCMYK`, on a page carrying one, the
    // assumed press's cyan beside a fill that was the intent's (ADR 1001, ADR 1008). The intent
    // travels in `into` because that is what the interpreter already hands every route that
    // converts after it, and so does §11.7.2's redefinition of `DeviceCMYK` inside a group
    // (ADR 2012).
    let resolved = into
        .parse_space(document, &space, resources)
        .ok_or_else(|| ImageError::UnsupportedColourSpace {
            space: String::from_utf8_lossy(&family).into_owned(),
        })?;
    Ok(ColourSpace::reduced(resolved, into.target().clone()))
}

/// §11.6.5.2's soft-mask image's own colour space, read against no resource dictionary.
///
/// The one difference from [`colour_space`] is the resources, and it is the whole of this
/// function: a soft-mask image's samples are §11.6.4.3's mask values rather than colours, so
/// §8.6.5.6's default colour space mechanism has nothing here to remap. The clause says so
/// itself — "\[c]olour values in the original device colour space shall be passed unchanged to
/// the default colour space" — so the remapping never changes a component's *value*, only
/// which colour that value denotes; and what §11.6.5.2 uses is the value. Table 143 fixes the
/// entry at one word, "Required; shall be `DeviceGray`", a name no resource dictionary is
/// consulted to resolve, so a conforming mask loses nothing by being read without one.
///
/// What a non-conforming mask loses is a report rather than a guess: a mask naming its space
/// by a resource key resolves to nothing here and [`soft_mask_entry`] names it unusable, where
/// before it resolved against the *parent's* resources and a `/DefaultGray` with a tone curve
/// bent the alpha through it (ADR 1008 found that and did not fix it; ADR 1054 is the fix).
///
/// [`soft_mask_entry`]: super::soft_mask::soft_mask_entry
pub(super) fn mask_colour_space(
    document: &Document,
    mask_dict: &Dictionary,
) -> Result<ColourSpace, ImageError> {
    colour_space(
        document,
        mask_dict,
        &Dictionary::new(),
        &Conversion::device(),
    )
}

/// Names a colour space object for a report.
///
/// Its family, not its contents: a report saying `Indexed` is what a reader needs, and one
/// that printed the whole lookup table would bury it.
pub(super) fn space_name(space: &Object) -> String {
    match space {
        Object::Name(name) => String::from_utf8_lossy(name.as_bytes()).into_owned(),
        Object::Array(items) => items.first().and_then(Object::as_name).map_or_else(
            || "an empty array".to_owned(),
            |name| String::from_utf8_lossy(name.as_bytes()).into_owned(),
        ),
        other => format!("{other:?}"),
    }
}
