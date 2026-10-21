//! Reading a colour space from a document: the family named, its dictionary, and the defaults a page overrides.

use std::sync::Arc;

use pdf_syntax::{Dictionary, Document, Name, Object};

use super::entries::{icc_range, narrow, numbers, white_point};
use super::{ColourSpace, MAX_DEPTH, Reading};

impl ColourSpace {
    /// Resolves a colour space object.
    ///
    /// Returns `None` for a space this crate does not implement, so the caller reports it
    /// rather than guessing a colour.
    #[must_use]
    pub fn parse(document: &Document, object: &Object, resources: &Dictionary) -> Option<Self> {
        Self::parse_with_output_intent(document, object, resources, None)
    }

    /// Resolves a colour space object on a page whose output intent says what its device
    /// colours mean.
    ///
    /// `output_intent` is the space §14.11.5's `/DestOutputProfile` describes — a profile
    /// "defining the transformation from the PDF document's source colours to output device
    /// colourants" — which §8.6.5.7 NOTE 3 names as the one thing in a PDF that can suggest
    /// the calibration a document's device colours were prepared for. Where a device space is
    /// selected and the resources name no §8.6.5.6 default for it, the intent with the same
    /// number of components stands in for the space; where they name one, the default wins,
    /// being the nearer statement and a `shall`. That ranking is `Self::device_family`'s,
    /// and it is reached from every place a device space can be selected — the family name,
    /// the array form, a `Pattern`'s underlying space, an `Indexed` base, a `Separation` or
    /// `DeviceN` alternate — because a meaning is a property of the *space* and not of the
    /// operator that named it (trap 6). [`Self::parse`] is this with no intent.
    #[must_use]
    pub fn parse_with_output_intent(
        document: &Document,
        object: &Object,
        resources: &Dictionary,
        output_intent: Option<&Self>,
    ) -> Option<Self> {
        Self::parse_under(document, object, resources, Reading::new(output_intent))
    }

    /// Resolves a colour space object under everything that decides what it means on this
    /// page: §14.11.5's output intent, and whether ISO 32000-2 §10.8.3's separation
    /// simulation is in force.
    ///
    /// The general form of [`Self::parse`] and [`Self::parse_with_output_intent`], which are
    /// this with [`Reading::new`] and no simulation. See [`super::Separations`] for why the second
    /// half of a [`Reading`] is the host's answer rather than the document's.
    #[must_use]
    pub fn parse_under(
        document: &Document,
        object: &Object,
        resources: &Dictionary,
        reading: Reading<'_>,
    ) -> Option<Self> {
        Self::parse_at(document, object, resources, reading, 0)
    }

    pub(super) fn parse_at(
        document: &Document,
        object: &Object,
        resources: &Dictionary,
        reading: Reading<'_>,
        depth: usize,
    ) -> Option<Self> {
        if depth > MAX_DEPTH {
            return None;
        }
        let resolved = document.resolve(object);

        if let Some(name) = resolved.as_name() {
            return Self::by_name(document, name, resources, reading, depth);
        }

        let items = resolved.as_array()?;
        let family = items
            .first()
            .map(|item| document.resolve(item))
            .and_then(|item| item.as_name().map(|n| n.as_bytes().to_vec()))?;

        match family.as_slice() {
            // §8.6.3 makes the array the general form — "[a] colour space shall be defined
            // by an array object whose first element is a name object identifying the colour
            // space family" — and the bare name its shorthand for a family with no
            // parameters, so the two select one space; and §8.6.5.6 remaps a device space
            // "[r]egardless of how the colour space is specified". The array form therefore
            // takes the name's route (ADR 1001).
            b"DeviceGray" | b"G" | b"DeviceRGB" | b"RGB" | b"DeviceCMYK" | b"CMYK" => {
                Self::device_family(document, &family, resources, reading, depth)
            }
            b"CalGray" => Some(Self::parse_cal_gray(document, items.get(1))),
            b"CalRGB" => Some(Self::parse_cal_rgb(document, items.get(1))),
            // §8.6.5.1: "A PDF reader shall ignore CalCMYK colour space attributes and
            // render colours specified in this family as if they had been specified using
            // DeviceCMYK." The family was withdrawn before it was ever completed — NOTE 1
            // says its definition "has been completely removed" — so there is nothing to
            // calibrate against and the clause states the whole of what a reader does with
            // one. No corpus document writes it; without this arm such a file would report
            // an unsupported colour space, which is a *refusal* where the standard states
            // an answer. "As if … DeviceCMYK" includes what a `/DefaultCMYK` or an output
            // intent says that space means.
            b"CalCMYK" => Self::device_family(document, b"DeviceCMYK", resources, reading, depth),
            b"Pattern" => Some(Self::Pattern {
                base: items
                    .get(1)
                    .and_then(|item| {
                        Self::parse_at(document, item, resources, reading, depth.saturating_add(1))
                    })
                    .map(Box::new),
            }),
            b"Lab" => {
                let dict = items.get(1).map(|item| document.resolve(item));
                let dict = dict.as_ref().and_then(Object::as_dict);
                let white = white_point(document, dict);
                let black = numbers(document, dict, "BlackPoint").unwrap_or([0.0, 0.0, 0.0]);
                let range = dict
                    .and_then(|dict| {
                        let array = document.get_key(dict, "Range");
                        let values: Vec<f32> = array
                            .as_array()?
                            .iter()
                            .filter_map(|item| document.resolve(item).as_number().map(narrow))
                            .collect();
                        <[f32; 4]>::try_from(values.as_slice()).ok()
                    })
                    .unwrap_or([-100.0, 100.0, -100.0, 100.0]);
                Some(Self::Lab {
                    white,
                    black,
                    range,
                })
            }
            b"ICCBased" => {
                Self::parse_icc_based(document, items.get(1)?, resources, reading, depth)
            }
            b"Indexed" | b"I" => Self::parse_indexed(document, items, resources, reading, depth),
            b"Separation" | b"DeviceN" => {
                Self::parse_separation(document, &family, items, resources, reading, depth)
            }
            _ => None,
        }
    }

    /// Reads an `ICCBased` space: ISO 32000-2 §8.6.5.5, Table 65.
    ///
    /// Three answers in the order the clause ranks them — the profile, the `/Alternate`, and the
    /// device space `/N` implies — which is what makes the first one's condition worth stating
    /// here rather than inside the profile parser.
    fn parse_icc_based(
        document: &Document,
        object: &Object,
        resources: &Dictionary,
        reading: Reading<'_>,
        depth: usize,
    ) -> Option<Self> {
        let stream = document.resolve(object);
        let stream = stream.as_stream()?;

        // The profile itself is the document's own statement of what its numbers mean, so it
        // wins over everything below.
        //
        // **Except where the stream decoded only as far as its damage** (§7.4.1, and
        // [`pdf_syntax::Decoded::damage`]), which is ADR 0343's argument one clause over: a
        // profile is a tag table whose offsets point forward, so a prefix of one is a directory
        // describing bytes that are not there — an `A2B1` tag that fell off the end leaves the
        // curve-and-matrix branch reading `rTRC` as no curve at all, which is a tone response
        // nobody wrote standing in place of the producer's. Table 65 states where to go instead,
        // and it states the whole of it, which is why a damaged profile needs no report of its
        // own: `/Alternate` "shall be used in case the one specified in the stream data is not
        // supported", and where that entry is absent "the colour space that shall be used is
        // DeviceGray , DeviceRGB , or DeviceCMYK , depending on whether the value of N is 1 , 3 ,
        // or 4". Both are below.
        //
        // **[`pdf_syntax::Damage::CheckValue`] is refused here too, and deliberately rather than
        // by the sentence above.** ADR 0836 separated a `FlateDecode` stream that is whole by
        // RFC 1951 and wrong by RFC 1950's check value from one that broke, and the prefix
        // argument does not reach it — so this is a second decision and it turns on what a
        // refusal costs. For a font program it costs the page its text, which is why ADR 0836
        // admits one there and lets the program's own grammar judge it; here Table 65 states the
        // producer's own alternative, so declining a profile whose bytes are not the ones that
        // were compressed costs a *stated* space rather than a missing one. The population is
        // `pdf-model --example damaged_stream_census`, which counts the profiles this reaches.
        if let Ok(decoded) = document.decoded_stream_data_reported(stream)
            && decoded.damage.is_none()
            && let Some(profile) = crate::icc::Profile::parse(&decoded.data)
        {
            // Table 65's `/Range`, "[a]n array of 2 × N numbers … that shall specify the
            // minimum and maximum valid values of the corresponding colour components".
            // `crate::icc::Profile::with_range` says what it does with one and why an absent
            // entry leaves the profile's own range standing — that entry's default is
            // Table 68's row for the profile's data colour space, which is what the profile
            // already holds.
            let range = icc_range(document, &stream.dict);
            return Some(Self::Icc {
                profile: Arc::new(profile.with_range(&range)),
            });
        }

        // `/Alternate` is the producer's own statement of what to use when the profile cannot
        // be applied. Preferring it over a guess from `/N` is what the specification asks for
        // and is free: a document saying its profile stands in for `Lab` or a `Separation` gets
        // that, rather than whichever device space happens to have the same component count.
        if let Some(alternate) = stream.dict.get("Alternate")
            && let Some(space) = Self::parse_at(
                document,
                alternate,
                resources,
                reading,
                depth.saturating_add(1),
            )
        {
            return Some(space);
        }

        // Failing that, the device space with the same component count — which is the fallback
        // the specification itself describes.
        match document.get_key(&stream.dict, "N").as_integer() {
            Some(1) => Some(Self::Gray),
            Some(4) => Some(Self::Cmyk),
            // Three is the common case, and an absent or nonsensical `/N` is far more likely to
            // be RGB than anything else.
            _ => Some(Self::Rgb),
        }
    }

    /// Reads a `CalGray` dictionary: ISO 32000-2 §8.6.5.2, Table 62.
    fn parse_cal_gray(document: &Document, dictionary: Option<&Object>) -> Self {
        let dict = dictionary.map(|item| document.resolve(item));
        let dict = dict.as_ref().and_then(Object::as_dict);
        Self::CalGray {
            white: white_point(document, dict),
            black: numbers(document, dict, "BlackPoint").unwrap_or([0.0, 0.0, 0.0]),
            // Table 62: "G shall be positive". A non-positive exponent is not a gamma, and
            // `powf` would answer with an infinity rather than a colour.
            gamma: dict
                .map(|dict| document.get_key(dict, "Gamma"))
                .and_then(|value| value.as_number())
                .map(narrow)
                .filter(|gamma| *gamma > 0.0)
                .unwrap_or(1.0),
        }
    }

    /// Reads a `CalRGB` dictionary: ISO 32000-2 §8.6.5.3, Table 63.
    fn parse_cal_rgb(document: &Document, dictionary: Option<&Object>) -> Self {
        let dict = dictionary.map(|item| document.resolve(item));
        let dict = dict.as_ref().and_then(Object::as_dict);
        let gamma = numbers(document, dict, "Gamma").unwrap_or([1.0, 1.0, 1.0]);
        Self::CalRgb {
            white: white_point(document, dict),
            black: numbers(document, dict, "BlackPoint").unwrap_or([0.0, 0.0, 0.0]),
            gamma: gamma.map(|value| if value > 0.0 { value } else { 1.0 }),
            matrix: numbers(document, dict, "Matrix")
                .unwrap_or([1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]),
        }
    }

    /// Reads an `Indexed` space: a base space, a maximum index, and a palette.
    fn parse_indexed(
        document: &Document,
        items: &[Object],
        resources: &Dictionary,
        reading: Reading<'_>,
        depth: usize,
    ) -> Option<Self> {
        let base = Self::parse_at(
            document,
            items.get(1)?,
            resources,
            reading,
            depth.saturating_add(1),
        )?;
        let high = usize::try_from(
            items
                .get(2)
                .map(|item| document.resolve(item))
                .and_then(|item| item.as_integer())?,
        )
        .ok()?;
        let table = items.get(3).map(|item| document.resolve(item))?;
        let bytes = match &table {
            Object::String(bytes) => bytes.clone(),
            Object::Stream(stream) => document.decoded_stream_data(stream)?,
            _ => return None,
        };
        // §8.6.6.3: "Each byte shall be an unsigned integer in the range 0 to 255 that shall
        // be scaled to the range of the corresponding colour component in the base colour
        // space; that is, 0 corresponds to the minimum value in the range for that
        // component, and 255 corresponds to the maximum."
        //
        // Dividing by 255 is that scaling only where the base's components run 0 to 1, which
        // is every space but `Lab`. `issue2761.pdf` indexes into a `Lab` base and drew a
        // black square where four renderers draw a pale grey gradient: its lightest entry is
        // L = 253, which is 0.99 as a fraction and 99 out of 100 as a lightness.
        let components = base.components().max(1);
        let scaled: Vec<f32> = bytes
            .chunks(components)
            .flat_map(|entry| {
                entry.iter().enumerate().map(|(component, byte)| {
                    let (low, high) = base.component_range(component);
                    low + f32::from(*byte) * (high - low) / 255.0
                })
            })
            .collect();
        Some(Self::Indexed {
            base: Box::new(base),
            lookup: scaled,
            high,
        })
    }

    /// Resolves a space named directly, looking it up in the resources if need be.
    fn by_name(
        document: &Document,
        name: &Name,
        resources: &Dictionary,
        reading: Reading<'_>,
        depth: usize,
    ) -> Option<Self> {
        if let Some(space) =
            Self::device_family(document, name.as_bytes(), resources, reading, depth)
        {
            return Some(space);
        }
        // A bare `/Pattern` names no underlying space; the caller falls back on the
        // operand count when one is needed.
        if name.as_bytes() == b"Pattern" {
            return Some(Self::Pattern { base: None });
        }
        // Anything else is a name in the page's `/ColorSpace` resource dictionary — the
        // document's own name, so §7.3.5's exact binary match decides it (ADR 0439).
        let table = document.get_key(resources, "ColorSpace");
        let table = table.as_dict()?;
        let entry = table.get_by_name(name)?;
        Self::parse_at(document, entry, resources, reading, depth.saturating_add(1))
    }

    /// What a device colour space family means on this page: four sources, in the order
    /// the standard puts them, and `None` for a name that is not a family.
    ///
    /// Choosing a device space is a request to use the *default* space standing in for it,
    /// where the resources name one. ISO 32000-2 §8.6.5.6: "If such an entry is present, its
    /// value shall be used as the colour space for the operation currently being performed."
    /// This is how a producer says "my `DeviceCMYK` means this press", and ignoring it renders
    /// those documents in the wrong colours entirely. Failing that, the page's output intent
    /// describes the device the document's colours were prepared for, which §8.6.5.7 NOTE 3
    /// names as the only thing in a PDF that can, and it stands in for the family with the same
    /// number of components; §14.11.5 makes the intent informational and this tree chooses to
    /// respect it, so a `/DefaultCMYK` — a `shall` about the current operation — outranks it.
    /// Between the two, for `DeviceCMYK` alone, §11.7.2's redefinition inside an isolated group
    /// or page whose blending space is an `ICCBased` CMYK one ([`Reading::within_cmyk_group`]),
    /// which is a `shall` about the group and so outranks the intent too (ADR 2012).
    /// Failing all three, the device space itself — where §8.6.4.4 states no conversion,
    /// §10.4.2.5 states one and §10.4.2.1 ranks it below §10.3's ICC route, so what happens
    /// then is this processor's own choice between two answers the standard has already
    /// ordered, documented at [`super::device_cmyk::CMYK_CORNERS`].
    ///
    /// The families are ISO 32000-2's own names, so they are compared against literals — as
    /// bytes, which is §7.3.5's rule by the shorter route. A bare `/CalGray` or `/CalRGB` is
    /// not a colour space name §8.6.3 admits — those families take a dictionary — and is read
    /// as the device family it would otherwise stand for, which is a recovery rather than a
    /// reading.
    fn device_family(
        document: &Document,
        family: &[u8],
        resources: &Dictionary,
        reading: Reading<'_>,
        depth: usize,
    ) -> Option<Self> {
        let (default, device) = Self::family_named(family)?;
        if let Some(space) = Self::named_default(document, default, resources, reading, depth) {
            return Some(space);
        }
        // §11.7.2's redefinition, a `shall` about the group the operation is in, before the
        // intent's informational description of the device (ADR 2012).
        if matches!(device, Self::Cmyk)
            && let Some(group) = reading.cmyk_group
        {
            return Some(group.clone());
        }
        if let Some(intent) = reading.intent
            && intent.components() == device.components()
        {
            return Some(intent.clone());
        }
        Some(device)
    }

    /// Resolves a `/DefaultGray`, `/DefaultRGB` or `/DefaultCMYK` entry, if present.
    ///
    /// A default that resolves back to the device space it replaces would recurse for
    /// ever, so the lookup is bounded by the same depth limit as everything else here.
    /// The device family a name selects, and the resource key of the default standing in for
    /// it — the one table [`Self::device_family`] and [`Self::selected_device_family`] share,
    /// so that the space a selection resolves to and the family it was selected as cannot be
    /// read from two lists.
    fn family_named(family: &[u8]) -> Option<(&'static str, Self)> {
        match family {
            b"DeviceGray" | b"G" | b"CalGray" => Some(("DefaultGray", Self::Gray)),
            b"DeviceRGB" | b"RGB" | b"CalRGB" => Some(("DefaultRGB", Self::Rgb)),
            b"DeviceCMYK" | b"CMYK" => Some(("DefaultCMYK", Self::Cmyk)),
            _ => None,
        }
    }

    /// The device family `object` selects, where it selects one, before §8.6.5.6's default or
    /// §14.11.5's output intent stands in for it: the bare name, a resource name resolving to
    /// one, the array form ADR 1001 gives the same route, and `CalCMYK`, which §8.6.5.1 has a
    /// reader render as though the colours had been given in `DeviceCMYK`.
    ///
    /// `None` for every other space, which is its own family whatever the page says.
    pub(super) fn selected_device_family(
        document: &Document,
        object: &Object,
        resources: &Dictionary,
        depth: usize,
    ) -> Option<Self> {
        if depth > MAX_DEPTH {
            return None;
        }
        let resolved = document.resolve(object);
        if let Some(name) = resolved.as_name() {
            if let Some((_, family)) = Self::family_named(name.as_bytes()) {
                return Some(family);
            }
            if name.as_bytes() == b"Pattern" {
                return None;
            }
            let table = document.get_key(resources, "ColorSpace");
            let entry = table.as_dict()?.get_by_name(name)?;
            return Self::selected_device_family(
                document,
                entry,
                resources,
                depth.saturating_add(1),
            );
        }
        let items = resolved.as_array()?;
        let family = items.first().map(|item| document.resolve(item))?;
        match family.as_name()?.as_bytes() {
            b"CalCMYK" => Some(Self::Cmyk),
            // The array form of a Cal family takes a dictionary and is its own space; only the
            // device families' names select a device space from inside an array.
            b"CalGray" | b"CalRGB" => None,
            name => Self::family_named(name).map(|(_, family)| family),
        }
    }
}
