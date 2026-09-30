//! `preserve` for a JPEG 2000 image outside the JPX baseline: its samples, written again under
//! `FlateDecode` in the colour space its own dictionary states.
//!
//! ISO 19005-2 section 6.2.8.3 and ISO 19005-4 section 6.2.7.3 restrict *JPEG 2000 data* to the
//! JPX baseline set of features, which ITU-T T.801 M.9.2 defines (ADR 1383). Every other
//! sentence of those subclauses is about JPEG 2000 data too, so an image whose samples are
//! carried by another filter is outside all of them; what binds it then is what bound it before —
//! section 6.1.7.2's filters, which admit `FlateDecode`, and section 6.2.4's colour space rules,
//! which bind the same `/ColorSpace` entry the image already states. ADR 1400 is the argument.
//!
//! **Only where the dictionary states `/ColorSpace`.** §7.4.9 has a processor ignore the data's
//! own colour specifications then — "If present, it shall determine how the image samples are
//! interpreted, and the colour space specifications in the JPEG 2000 data shall be ignored" — so
//! the samples are already in the dictionary's domain, and ADR 1371's rule that a codec's output
//! *is* the image's samples makes writing them again a change of encoding and of nothing else.
//! Where the dictionary states none, the data's specification is what the samples mean, and any
//! `/ColorSpace` written beside a Flate copy would be this converter choosing one; that half keeps
//! its refusal ([`super::decision::SHAPES`]). An image mask has no colour space to state and one
//! meaning for its samples, §7.4.9's "single colour channel with 1-bit samples", so it is
//! transcoded as well.
//!
//! **An opacity channel travels as the soft-mask image Table 87 names.** A non-zero `/SMaskInData`
//! says the codestream carries one, and Table 87 has a processor "create a soft-mask image from the
//! information"; a Flate copy has no channel for it, so the transcode creates that image as an
//! object of its own — `DeviceGray` as Table 143 requires, at the opacity channel's own depth — and
//! names it in `/SMask`. Code 2's colour was multiplied by the opacity, which §11.6.5.2's
//! pre-blending states as a `/Matte` of the space's zero, the redaction writer's construction
//! (ADR 1277, ADR 1412).
//!
//! The depth is the codestream's: where every component has one depth that Table 87 can state,
//! the samples are written at it and the dictionary's `/Decode` is carried as it stood; otherwise
//! they are written in the field of the widest, eight bits or sixteen, with each component's
//! `/Decode` pair widened from its own depth so that §8.9.5.2's map gives every integer the value
//! it had (the redaction writer's construction, ADR 1371).

use std::collections::BTreeMap;
use std::sync::Arc;

use pdf_archive::Outcome;
use pdf_model::image::{JpxSamples, ORDINARY_JPX_SAMPLES};
use pdf_syntax::Document;
use pdf_syntax::object::{Dictionary, Name, Object, ObjectId, Stream};

use crate::json::Value;

use super::COMPRESSION_LEVEL;
use super::decision::Because;

/// The requirement whose `preserve` transcodes the image.
pub(super) const SITE: &str = "graphics/jpeg2000-uses-the-baseline-feature-set";

/// Every requirement the same transcode answers, [`SITE`] first.
///
/// ISO 19005-2 section 6.2.8.3 and ISO 19005-4 section 6.2.7.3 state the channel count, the bit
/// depth and the enumerated `CIEJab` colour space of *JPEG 2000 data*, exactly as they state its
/// baseline, so an image whose samples are carried under `FlateDecode` is outside those three
/// sentences for the reason it is outside the first. The shape is the same too: only where the
/// dictionary states `ColorSpace` are the samples already in a domain a copy can keep (ADR 1412).
pub(super) const SITES: [&str; 4] = [
    SITE,
    "graphics/jpeg2000-bit-depth",
    "graphics/jpeg2000-channel-count",
    "graphics/jpeg2000-no-ciejab-colour-space",
];

/// What an operator answering the site with `preserve` is agreeing to.
pub(super) const TRANSCODED: &str = "each JPEG 2000 image outside the JPX baseline whose \
     dictionary states its own ColorSpace is decoded to its samples, at the depth its codestream \
     gives each component, and written again under FlateDecode in that same ColorSpace. Two costs \
     come with it: the file grows, often by a great deal, and this program's JPEG 2000 decoder's \
     output becomes the archive's copy of the picture, so a fault in that decoder is kept rather \
     than left re-decodable. The report names every image with its size before and after \
     (doc/adr/1400)";

/// Why a finding naming no object cannot be acted on.
const NO_IMAGE_OBJECT: &str = "a JPX baseline failure was reported without the image XObject it \
     is about, so there is no stream to transcode. Writing into an object no finding named would \
     be this converter reading the file a second way";

/// Why an image whose data carries its colour space keeps the refusal.
const THE_DATA_STATES_THE_COLOUR: &str = "a JPEG 2000 image outside the JPX baseline states no \
     ColorSpace, so ISO 32000-2 \u{a7}7.4.9 has a reader take the colour space from the JPEG 2000 \
     data, and that specification is what the samples mean. A Flate copy needs a ColorSpace of \
     its own, and writing one would be this converter deciding what the producer's samples are \
     (doc/adr/1400)";

/// Why a stream whose bytes are not the JPEG 2000 data is left alone.
const NOT_THE_DATA: &str = "a JPEG 2000 image outside the JPX baseline keeps its data in \
     another file, could not be decrypted, or carries it under a filter chain that is not \
     JPXDecode alone, so the stream's own bytes are not the JPEG 2000 data this remedy would \
     decode";

/// Why an image stating both kinds of soft mask is not transcoded.
const TWO_SOFT_MASKS: &str = "a JPEG 2000 image outside the JPX baseline states a non-zero \
     SMaskInData beside an SMask entry, which ISO 32000-2 Table 87 forbids (\"[i]f this entry has \
     a non-zero value, SMask shall not be specified\"), so there is no one soft mask a Flate copy \
     could carry";

/// Why a premultiplied image in an `Indexed` space is not transcoded.
const PREMULTIPLIED_INDICES: &str = "a JPEG 2000 image outside the JPX baseline states \
     SMaskInData 2 over an Indexed space or one whose components do not reach zero, so \
     \u{a7}11.6.5.2's Matte, which states the pre-blending in the parent's own components, has no \
     value that says what the multiplication did";

/// Why an image mask whose data carries opacity is not transcoded.
const OPACITY_OVER_A_STENCIL: &str = "a JPEG 2000 image mask outside the JPX baseline states a \
     non-zero SMaskInData, so its stencil and its opacity are two planes, and ISO 32000-2 Table \
     87 gives an image mask no SMask entry a Flate copy could carry the second in";

/// Why no object number could be found for a soft-mask image.
const NO_NUMBER_FOR_THE_MASK: &str = "a JPEG 2000 image's opacity channel needs a soft-mask image \
     object of its own and no unused object number was found for it";

/// Why an image the codec does not return whole is not transcoded.
const NOT_DECODED_WHOLE: &str = "a JPEG 2000 image outside the JPX baseline was not decoded to \
     its own samples at full resolution within the ordinary decoder's budget, or its codestream \
     requires a feature this tree's decoder does not provide. A reduced resolution level is not \
     the image, and nothing else is a copy of it";

/// Why a component deeper than sixteen bits is not transcoded.
const DEEPER_THAN_SIXTEEN: &str = "a JPEG 2000 image outside the JPX baseline has a component \
     deeper than sixteen bits, which ISO 32000-2 \u{a7}7.4.9 permits a codestream and Table 87 \
     permits no other image: its integers have no field a Flate copy could hold them in";

/// Why an image whose component count contradicts its colour space is not transcoded.
const COMPONENTS_DISAGREE: &str = "a JPEG 2000 image outside the JPX baseline decodes to a \
     number of colour channels its stated ColorSpace does not have, which ISO 32000-2 \
     \u{a7}7.4.9 requires to match; a Flate copy would state samples the space cannot read";

/// Why the transcoded stream was not written.
const NOT_PROVED: &str = "a JPEG 2000 image was transcoded on a copy and the copy did not decode \
     back to the samples it was built from, so it is refused rather than written. \
     doc/adr/0973's rule: a rewrite promises only what it has placed";

/// Why no image was transcoded.
const NOTHING_TO_TRANSCODE: &str = "no failed requirement named a JPXDecode image this remedy \
     could transcode";

/// One image this conversion transcoded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranscodedImage {
    /// The image `XObject`.
    pub at: ObjectId,
    /// Each component's depth in the codestream.
    pub depths: Vec<u8>,
    /// The `/BitsPerComponent` written.
    pub bits: u8,
    /// The stream's length under `JPXDecode`.
    pub before: usize,
    /// The stream's length under `FlateDecode`.
    pub after: usize,
    /// The soft-mask image created from the codestream's opacity channel, where Table 87's
    /// `/SMaskInData` said it carried one (ADR 1412).
    pub soft_mask: Option<ObjectId>,
}

impl TranscodedImage {
    /// One transcoded image as JSON.
    pub(super) fn to_json(&self) -> Value {
        Value::Object(vec![
            (
                "object".to_owned(),
                Value::text(format!("{} {}", self.at.number, self.at.generation)),
            ),
            (
                "depths".to_owned(),
                Value::Array(
                    self.depths
                        .iter()
                        .map(|depth| Value::count(usize::from(*depth)))
                        .collect(),
                ),
            ),
            (
                "bits_per_component".to_owned(),
                Value::count(usize::from(self.bits)),
            ),
            ("bytes_before".to_owned(), Value::count(self.before)),
            ("bytes_after".to_owned(), Value::count(self.after)),
            (
                "soft_mask".to_owned(),
                self.soft_mask.map_or(Value::Null, |mask| {
                    Value::text(format!("{} {}", mask.number, mask.generation))
                }),
            ),
        ])
    }
}

/// The images whose stream is replaced, and what each became.
#[derive(Debug, Default)]
pub(super) struct Transcodes {
    /// The new stream object, by the image `XObject` the finding named.
    pub(super) at: BTreeMap<ObjectId, Object>,
    /// One row per image, for the report.
    pub(super) rows: Vec<TranscodedImage>,
    /// The soft-mask images the opacity channels became, by the number each was given.
    pub(super) written: BTreeMap<ObjectId, Object>,
}

/// Every `JPXDecode` image one of the requirements in `answered` named, transcoded.
///
/// `answered` is the members of [`SITES`] that failed and that the plan answers with `preserve`.
/// The population is the validator's findings, so the images this touches are the ones those
/// requirements reported; one that cannot be transcoded refuses the site with its own sentence,
/// because a conversion that transcoded the others would still fail the requirement.
///
/// # Errors
///
/// [`Because`] naming the shape that stopped it.
pub(super) fn prepare(
    document: &Document,
    input: &pdf_archive::Report,
    spare: &mut super::prepare::Spare,
    answered: &[&str],
) -> Result<Transcodes, Because> {
    let mut out = Transcodes::default();
    for judgement in input.failures() {
        if !answered.contains(&judgement.id) {
            continue;
        }
        let Outcome::Failed { places, .. } = &judgement.outcome else {
            continue;
        };
        for finding in places {
            let id = finding
                .place
                .object
                .ok_or(Because::NotBuiltYet(NO_IMAGE_OBJECT))?;
            if out.at.contains_key(&id) {
                continue;
            }
            let transcoded = transcode(document, id, spare)?;
            out.at.insert(id, transcoded.stream);
            out.rows.push(transcoded.row);
            if let Some((at, mask)) = transcoded.soft_mask {
                out.written.insert(at, mask);
            }
        }
    }
    if out.at.is_empty() {
        return Err(Because::NotBuiltYet(NOTHING_TO_TRANSCODE));
    }
    Ok(out)
}

/// What one image became: its replacement stream, its report row, and the soft-mask image its
/// opacity channel became, with the number it was given, where it carried one.
struct Transcoded {
    /// The replacement image stream, proved.
    stream: Object,
    /// The report row.
    row: TranscodedImage,
    /// The soft-mask image object and its number.
    soft_mask: Option<(ObjectId, Object)>,
}

/// One image's replacement stream, proved, its report row, and the soft-mask image its opacity
/// channel became where it carried one.
fn transcode(
    document: &Document,
    id: ObjectId,
    spare: &mut super::prepare::Spare,
) -> Result<Transcoded, Because> {
    let Object::Stream(stream) = document.get(id) else {
        return Err(Because::NotBuiltYet(NO_IMAGE_OBJECT));
    };
    let dict = &stream.dict;
    let samples = admitted_samples(document, &stream)?;
    let soft_mask = soft_mask_for(document, dict, &samples, spare)?;
    let (bits, data, decode) = laid_out(&samples);
    let encoded = pdf_syntax::serialize::flate_encode(&data, COMPRESSION_LEVEL)
        .ok_or(Because::NotBuiltYet(NOT_PROVED))?;
    let mut written = dict.clone();
    written.insert(
        Name::new(&b"Filter"[..]),
        Object::Name(Name::new(&b"FlateDecode"[..])),
    );
    written.remove("DecodeParms");
    // Table 87 makes the entry meaningless for any filter but `JPXDecode`; what it said is now
    // the soft-mask image below, where it said anything.
    written.remove("SMaskInData");
    if let Some((at, _)) = &soft_mask {
        written.insert(Name::new(&b"SMask"[..]), Object::Reference(*at));
    }
    written.insert(
        Name::new(&b"BitsPerComponent"[..]),
        Object::Integer(i64::from(bits)),
    );
    if let Some(decode) = decode {
        written.insert(
            Name::new(&b"Decode"[..]),
            Object::Array(
                decode
                    .into_iter()
                    .map(|value| Object::Real(f64::from(value)))
                    .collect(),
            ),
        );
    }
    written.insert(
        Name::new(&b"Length"[..]),
        Object::Integer(i64::try_from(encoded.len()).unwrap_or(i64::MAX)),
    );
    let replacement = Stream {
        dict: written,
        data: encoded.into(),
        decryption_failed: false,
    };
    if document
        .decoded_stream_data(&replacement)
        .is_none_or(|back| *back != *data)
    {
        return Err(Because::NotBuiltYet(NOT_PROVED));
    }
    let row = TranscodedImage {
        at: id,
        depths: samples.depths.clone(),
        bits,
        before: stream.data.len(),
        after: replacement.data.len(),
        soft_mask: soft_mask.as_ref().map(|(at, _)| *at),
    };
    Ok(Transcoded {
        stream: Object::Stream(Arc::new(replacement)),
        row,
        soft_mask,
    })
}

/// The image's samples, decoded whole, where every refusal this remedy states has been asked of
/// its dictionary and of what the codec returned.
fn admitted_samples(document: &Document, stream: &Stream) -> Result<JpxSamples, Because> {
    let dict = &stream.dict;
    if stream.decryption_failed
        || !document.get_key(dict, "F").is_null()
        || !super::jpeg2000::only_jpx(document, dict)
    {
        return Err(Because::NotBuiltYet(NOT_THE_DATA));
    }
    let mask = document.get_key(dict, "ImageMask") == Object::Boolean(true);
    let stated = !document.get_key(dict, "ColorSpace").is_null();
    if !stated && !mask {
        return Err(Because::NotBuiltYet(THE_DATA_STATES_THE_COLOUR));
    }
    let opacity_stated = document
        .get_key(dict, "SMaskInData")
        .as_integer()
        .is_some_and(|code| code != 0);
    if opacity_stated && !document.get_key(dict, "SMask").is_null() {
        return Err(Because::NotBuiltYet(TWO_SOFT_MASKS));
    }
    if opacity_stated && mask {
        return Err(Because::NotBuiltYet(OPACITY_OVER_A_STENCIL));
    }
    let samples = pdf_model::image::jpx_samples(
        document,
        dict,
        &Dictionary::new(),
        (&stream.data, ORDINARY_JPX_SAMPLES),
    )
    .map_err(|_| Because::NotBuiltYet(NOT_DECODED_WHOLE))?;
    if samples.depths.iter().any(|depth| *depth > 16) {
        return Err(Because::NotBuiltYet(DEEPER_THAN_SIXTEEN));
    }
    let expected = if mask {
        Some(1)
    } else {
        pdf_model::colour::ColourSpace::parse(
            document,
            &document.get_key(dict, "ColorSpace"),
            &Dictionary::new(),
        )
        .map(|space| space.components())
    };
    if expected != Some(samples.components) || (mask && samples.depths.as_slice() != [1]) {
        return Err(Because::NotBuiltYet(COMPONENTS_DISAGREE));
    }
    Ok(samples)
}

/// The soft-mask image Table 87 has a processor create from the codestream's opacity channel,
/// with the object number it takes, or `None` where the samples carry no opacity (ADR 1412).
fn soft_mask_for(
    document: &Document,
    dict: &Dictionary,
    samples: &JpxSamples,
    spare: &mut super::prepare::Spare,
) -> Result<Option<(ObjectId, Object)>, Because> {
    Ok(match samples.opacity.as_deref() {
        Some(opacity) => {
            let matte = if samples.premultiplied {
                Some(zero_matte(document, dict, samples.components)?)
            } else {
                None
            };
            let at = spare
                .take(document)
                .ok_or(Because::NotBuiltYet(NO_NUMBER_FOR_THE_MASK))?;
            Some((at, soft_mask_image(document, samples, opacity, matte)?))
        }
        None => None,
    })
}

/// §11.6.5.2's `/Matte` for Table 87's code 2: the parent space's zero in every component.
///
/// The pre-blending formula is `c′ = m + α × (c − m)`, computed "using actual colour component
/// values, with the effects of the Filter and Decode transformations already performed", so a
/// multiplication by the opacity alone is the matte whose every component is zero — which this
/// tree's own reading of code 2 undoes in those same values. Table 144 requires the matte's numbers
/// to be "valid colour components in that colour space", so a space one of whose components does
/// not reach zero, and an `Indexed` space whose indices no opacity can have multiplied, are refused.
fn zero_matte(
    document: &Document,
    dict: &Dictionary,
    components: usize,
) -> Result<Vec<f32>, Because> {
    let space = pdf_model::colour::ColourSpace::parse(
        document,
        &document.get_key(dict, "ColorSpace"),
        &Dictionary::new(),
    )
    .ok_or(Because::NotBuiltYet(PREMULTIPLIED_INDICES))?;
    let reaches_zero = (0..components).all(|component| {
        let (low, high) = space.component_range(component);
        low <= 0.0 && high >= 0.0
    });
    if matches!(space, pdf_model::colour::ColourSpace::Indexed { .. }) || !reaches_zero {
        return Err(Because::NotBuiltYet(PREMULTIPLIED_INDICES));
    }
    Ok(vec![0.0; components])
}

/// The soft-mask image Table 87 has a processor create from the codestream's opacity channel,
/// written under `FlateDecode` and proved.
///
/// Table 143's restrictions are what it states: `/Subtype /Image`, `/ColorSpace /DeviceGray`,
/// and a `/BitsPerComponent` — the opacity channel's own depth where Table 87 can state it, the
/// field of the widest channel otherwise with `/Decode` widened, as [`laid_out`] decides for the
/// colour. `/Width` and `/Height` are the parent's, which Table 143 requires where a `/Matte` is
/// present and which the channel has in any case.
fn soft_mask_image(
    document: &Document,
    samples: &JpxSamples,
    opacity: &[u8],
    matte: Option<Vec<f32>>,
) -> Result<Object, Because> {
    let plane = JpxSamples {
        components: 1,
        depths: vec![samples.opacity_depth],
        colour: opacity.to_vec(),
        opacity: None,
        premultiplied: false,
        decode: vec![(0.0, 1.0)],
        ..samples.clone()
    };
    let (bits, data, decode) = laid_out(&plane);
    let encoded = pdf_syntax::serialize::flate_encode(&data, COMPRESSION_LEVEL)
        .ok_or(Because::NotBuiltYet(NOT_PROVED))?;
    let mut dict = Dictionary::new();
    let name = |text: &[u8]| Object::Name(Name::new(text));
    dict.insert(Name::new(&b"Type"[..]), name(b"XObject"));
    dict.insert(Name::new(&b"Subtype"[..]), name(b"Image"));
    dict.insert(
        Name::new(&b"Width"[..]),
        Object::Integer(i64::from(samples.width)),
    );
    dict.insert(
        Name::new(&b"Height"[..]),
        Object::Integer(i64::from(samples.height)),
    );
    dict.insert(Name::new(&b"ColorSpace"[..]), name(b"DeviceGray"));
    dict.insert(
        Name::new(&b"BitsPerComponent"[..]),
        Object::Integer(i64::from(bits)),
    );
    dict.insert(Name::new(&b"Filter"[..]), name(b"FlateDecode"));
    let reals = |values: Vec<f32>| {
        Object::Array(
            values
                .into_iter()
                .map(|value| Object::Real(f64::from(value)))
                .collect(),
        )
    };
    if let Some(decode) = decode {
        dict.insert(Name::new(&b"Decode"[..]), reals(decode));
    }
    if let Some(matte) = matte {
        dict.insert(Name::new(&b"Matte"[..]), reals(matte));
    }
    dict.insert(
        Name::new(&b"Length"[..]),
        Object::Integer(i64::try_from(encoded.len()).unwrap_or(i64::MAX)),
    );
    let written = Stream {
        dict,
        data: encoded.into(),
        decryption_failed: false,
    };
    if document
        .decoded_stream_data(&written)
        .is_none_or(|back| *back != *data)
    {
        return Err(Because::NotBuiltYet(NOT_PROVED));
    }
    Ok(Object::Stream(Arc::new(written)))
}

/// The depth to write, the samples laid out at it, and the `/Decode` array to state where the
/// dictionary's own no longer describes them.
///
/// One depth Table 87 can state — 1, 2, 4, 8 or 16 — is written as it is, each row packed to a
/// byte boundary as §8.9.5.2 lays samples out, and the dictionary's `/Decode` still describes
/// them. Anything else goes in the field [`JpxSamples::precision`] sizes, eight bits or sixteen,
/// with each component's pair widened from its own depth.
fn laid_out(samples: &JpxSamples) -> (u8, Vec<u8>, Option<Vec<f32>>) {
    let depth = samples.depths.first().copied().unwrap_or(8);
    let uniform = samples.depths.iter().all(|each| *each == depth);
    if uniform && matches!(depth, 8 | 16) {
        return (depth, samples.colour.clone(), None);
    }
    if uniform && matches!(depth, 1 | 2 | 4) {
        return (depth, packed(samples, depth), None);
    }
    let bits: u8 = if samples.precision > 8 { 16 } else { 8 };
    let decode = samples
        .decode
        .iter()
        .zip(&samples.depths)
        .flat_map(|(pair, own)| crate::redact::widened_decode(&[*pair], *own, usize::from(bits)))
        .collect();
    (bits, samples.colour.clone(), Some(decode))
}

/// One-byte samples of `depth` bits packed high bit first, each row to a byte boundary.
fn packed(samples: &JpxSamples, depth: u8) -> Vec<u8> {
    let per_row = usize::try_from(samples.width)
        .unwrap_or(0)
        .saturating_mul(samples.components);
    let row_bytes = per_row.saturating_mul(usize::from(depth)).div_ceil(8);
    let rows = usize::try_from(samples.height).unwrap_or(0);
    let mut out = vec![0u8; row_bytes.saturating_mul(rows)];
    if per_row == 0 {
        return out;
    }
    let mask = u8::MAX >> 8u8.saturating_sub(depth);
    for (row, line) in samples.colour.chunks_exact(per_row).enumerate() {
        for (index, sample) in line.iter().enumerate() {
            let bit = index.saturating_mul(usize::from(depth));
            let shift = 8usize
                .saturating_sub(usize::from(depth))
                .saturating_sub(bit % 8);
            if let Some(cell) = out.get_mut(row.saturating_mul(row_bytes).saturating_add(bit / 8)) {
                *cell |= (sample & mask) << shift;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdf_model::image::JpxSpace;

    /// Samples as the decoder delivers them, one byte each at eight bits and fewer.
    fn samples(width: u32, depths: &[u8], colour: Vec<u8>) -> JpxSamples {
        JpxSamples {
            width,
            height: 1,
            components: depths.len(),
            precision: depths.iter().copied().max().unwrap_or(8),
            depths: depths.to_vec(),
            opacity_depth: 0,
            colour,
            opacity: None,
            premultiplied: false,
            space: JpxSpace::Stated,
            decode: vec![(0.0, 1.0); depths.len()],
        }
    }

    /// One depth Table 87 states is written at that depth, packed high bit first, and the
    /// dictionary's `/Decode` still describes it.
    #[test]
    fn a_depth_table_87_states_is_packed_as_it_is() {
        let (bits, data, decode) = laid_out(&samples(10, &[1], vec![1, 0, 1, 1, 0, 0, 0, 0, 1, 1]));
        assert_eq!((bits, decode), (1, None));
        assert_eq!(data, vec![0b1011_0000, 0b1100_0000]);

        let (bits, data, _) = laid_out(&samples(3, &[4], vec![0xF, 0x1, 0x8]));
        assert_eq!(bits, 4);
        assert_eq!(data, vec![0xF1, 0x80]);
    }

    /// Depths that differ, or one Table 87 cannot state, go in the widest's field, each pair
    /// widened from its own depth so that every integer keeps its value.
    #[test]
    fn other_depths_widen_each_decode_pair() {
        let (bits, data, decode) = laid_out(&samples(1, &[5, 8], vec![31, 255]));
        assert_eq!(bits, 8);
        assert_eq!(data, vec![31, 255]);
        let decode = decode.expect("the dictionary's pairs no longer describe the field");
        assert_eq!(decode.len(), 4);
        assert!((decode[1] - 255.0 / 31.0).abs() < 1e-4, "{decode:?}");
        assert!((decode[3] - 1.0).abs() < 1e-6, "{decode:?}");
    }
}
