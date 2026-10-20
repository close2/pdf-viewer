//! `optimize --images` — an image's samples re-encoded as baseline JPEG where that is smaller,
//! RFC 0002 section 6.5's lossy half, and only where it was asked for.
//!
//! # Asked for, never inherited
//!
//! The owner's condition on the encoder is that nothing is re-encoded unless a format is
//! explicitly requested (`doc/questions/A01`), so this pass has no default and no profile turns
//! it on: an [`OptimizePlan`](crate::optimize::OptimizePlan) without [`ImageRecompression`] writes
//! every image's bytes as the lossless passes leave them. With it, an image whose samples this
//! module can state exactly is handed to `pdf-sandbox`'s confined worker, and the JPEG that comes
//! back replaces the stream **only where it is smaller than what the producer wrote** — qpdf's
//! rule for `--optimize-images`, ADR 0842 section 3's rule for every stream, and the reason a
//! second run over this verb's own output changes no image (ADR 2138).
//!
//! # What may be re-encoded, and why each refusal is the clause's
//!
//! §7.4.8's JPEG is "a stream of image samples, each consisting of one, three, or four colour
//! components" where "[e]ach component value shall occupy a byte", so the samples are taken
//! exactly as §8.9.5 states them and nothing is converted on the way:
//!
//! - **eight bits a component**, Table 87's `/BitsPerComponent` 8, because a JPEG component is a
//!   byte and widening or narrowing a sample would be a conversion this verb does not make;
//! - **one or three components in a space whose samples are colour values** — `DeviceGray`,
//!   `DeviceRGB`, `CalGray`, `CalRGB`, or `ICCBased` with `/N` 1 or 3. Not `Indexed`: §8.6.6.3's
//!   samples are indices into a table, and a lossy index is another colour altogether. Not four
//!   components: the encoder writes none;
//! - **not a stencil**, §8.9.6.2's `/ImageMask`, whose one-bit samples are paint and no paint;
//! - **no colour-key mask**, §8.9.6.4's `/Mask` array, which masks a sample that falls in its
//!   ranges *exactly*, so a sample moved by one level is unmasked or masked;
//! - **no codec in the chain** — an image already `DCTDecode`, `JPXDecode`, `JBIG2Decode` or
//!   `CCITTFaxDecode` is the producer's choice of codec, and re-encoding a lossy one loses twice;
//! - and the conditions ADR 0842 section 3 carries every stream under: no external data (§7.3.8.2's
//!   `/F`), no `/Crypt` filter, data the document could decrypt, a decode that stopped short of
//!   nothing, and a `/Filter` and `/DecodeParms` that name no other object — the re-encoded stream
//!   states neither, so an object stated only inside them would be written and referred to by
//!   nothing.
//!
//! Every other entry of the image dictionary crosses as the producer wrote it, `/Decode`,
//! `/SMask`, `/Intent` and `/Interpolate` among them: they say what the samples mean, and the
//! samples still mean it.

use pdf_sandbox::{DctComponents, DctParameters};
use pdf_syntax::Document;
use pdf_syntax::object::{Dictionary, Name, Object, ObjectId, Stream};
use pdf_syntax::serialize::{Assembly, AssemblyError, Streams};

/// `optimize --images`: the lossy pass, at the caller's quality.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImageRecompression {
    /// The encoder's quality, 1 to 100 — the caller's, since §7.4.8 leaves "the information loss"
    /// to parameters "entirely under the control of the encoder".
    pub quality: u8,
}

/// What the pass did, for the report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ImageSavings {
    /// Image `XObject`s whose samples this pass could state and sent to the encoder.
    pub encoded: u64,
    /// How many of those the JPEG replaced, being smaller than the producer's bytes.
    pub replaced: u64,
    /// How many bytes the replacements saved against the producer's.
    pub saved: u64,
    /// Image `XObject`s the pass declined to re-encode, by the rules in this module's
    /// documentation.
    pub declined: u64,
    /// Images the encoder refused — a grid past its budget, a worker that did not answer.
    pub refused: u64,
}

/// One image whose slot the walk reserved for its JPEG.
struct Pending {
    /// The slot, in the output's numbering.
    slot: ObjectId,
    /// The source image it stands in for.
    source: ObjectId,
    /// What the encoder wrote.
    jpeg: Vec<u8>,
}

/// The pass, carried through `optimize`'s closure walk.
///
/// The walk asks [`ImageRecompressor::take`] before it copies a source object; an image this pass
/// replaces gets a reserved slot instead, at the place in the walk's order a copy would have had —
/// so the output's numbering is the lossless run's and only the image's bytes differ — and its
/// references are walked as a copy's would be. [`ImageRecompressor::place`] fills the slots once
/// the walk has numbered everything those references name.
pub(crate) struct ImageRecompressor {
    /// The caller's statement.
    recompression: ImageRecompression,
    /// What the lossless pass does to every stream, which a replaced image's JPEG is put through
    /// too — so that the lossless pass of a second run finds nothing to change in it.
    streams: Streams,
    /// The slots reserved, in walk order.
    pending: Vec<Pending>,
    /// What happened, so far.
    tally: ImageSavings,
    /// The encoder's refusals, each a sentence for the report.
    refusals: Vec<String>,
}

impl ImageRecompressor {
    /// A pass at the caller's quality.
    pub(crate) fn new(recompression: ImageRecompression, streams: Streams) -> Self {
        Self {
            recompression,
            streams,
            pending: Vec::new(),
            tally: ImageSavings::default(),
            refusals: Vec::new(),
        }
    }

    /// Reserves a slot for `id` where it is an image this pass re-encodes smaller, answering the
    /// slot; `None` where the walk should copy it as it is.
    ///
    /// # Errors
    ///
    /// The assembly's, where the slot cannot be reserved.
    pub(crate) fn take(
        &mut self,
        assembly: &mut Assembly<'_>,
        document: &Document,
        id: ObjectId,
    ) -> Result<Option<ObjectId>, AssemblyError> {
        let object = document.get(id);
        let Some(stream) = object.as_stream() else {
            return Ok(None);
        };
        if !is_image(document, &stream.dict) {
            return Ok(None);
        }
        let Some((samples, components)) = samples_of(document, stream) else {
            self.tally.declined = self.tally.declined.saturating_add(1);
            return Ok(None);
        };
        let (Some(width), Some(height)) = (
            dimension(document, &stream.dict, "Width"),
            dimension(document, &stream.dict, "Height"),
        ) else {
            self.tally.declined = self.tally.declined.saturating_add(1);
            return Ok(None);
        };
        self.tally.encoded = self.tally.encoded.saturating_add(1);
        let jpeg = match pdf_sandbox::encode_dct(
            &samples,
            DctParameters {
                width,
                height,
                components,
                quality: self.recompression.quality,
            },
        ) {
            Ok(jpeg) => jpeg,
            Err(error) => {
                self.tally.refused = self.tally.refused.saturating_add(1);
                self.refusals.push(format!("object {}: {error}", id.number));
                return Ok(None);
            }
        };
        // The producer's bytes as stored, which is what a reader of the source pays for the image.
        if jpeg.len() >= stream.data.len() {
            return Ok(None);
        }
        self.tally.replaced = self.tally.replaced.saturating_add(1);
        let slot = assembly.replace(0, id)?;
        self.pending.push(Pending {
            slot,
            source: id,
            jpeg,
        });
        Ok(Some(slot))
    }

    /// Fills every reserved slot with its image's dictionary, references in the output's
    /// numbering, and its JPEG; answers the tally and the encoder's refusals.
    ///
    /// # Errors
    ///
    /// The assembly's, where a slot cannot be filled.
    pub(crate) fn place(
        mut self,
        assembly: &mut Assembly<'_>,
        document: &Document,
    ) -> Result<(ImageSavings, Vec<String>), AssemblyError> {
        for pending in std::mem::take(&mut self.pending) {
            let source = document.get(pending.source);
            let Some(stream) = source.as_stream() else {
                return Err(AssemblyError::NotReserved { id: pending.slot });
            };
            let mut dict = Dictionary::new();
            for (key, value) in stream.dict.iter() {
                if matches!(key.as_bytes(), b"Filter" | b"DecodeParms" | b"Length") {
                    continue;
                }
                dict.insert(key.clone(), in_output(assembly, value, 0));
            }
            let codec = Object::Name(Name::new(&b"DCTDecode"[..]));
            // ADR 0842 section 3's rule for an image codec, applied here rather than left to a
            // second run: the lossless pass deflates in front of the codec where that is
            // smaller, so a JPEG it would wrap is written wrapped, and optimising this verb's own
            // output finds nothing left to do.
            let wrapped = match self.streams {
                Streams::Recompress { level } => {
                    pdf_syntax::serialize::flate_encode(&pending.jpeg, level)
                        .filter(|deflated| deflated.len() < pending.jpeg.len())
                }
                Streams::Carry => None,
            };
            let (filter, data) = match wrapped {
                Some(deflated) => (
                    Object::Array(vec![Object::Name(Name::new(&b"FlateDecode"[..])), codec]),
                    deflated,
                ),
                None => (codec, pending.jpeg),
            };
            self.tally.saved = self.tally.saved.saturating_add(
                u64::try_from(stream.data.len().saturating_sub(data.len())).unwrap_or(u64::MAX),
            );
            dict.insert(Name::new(&b"Filter"[..]), filter);
            dict.insert(
                Name::new(&b"Length"[..]),
                Object::Integer(i64::try_from(data.len()).unwrap_or(i64::MAX)),
            );
            assembly.place(
                pending.slot,
                Object::Stream(std::sync::Arc::new(Stream {
                    dict,
                    data: data.into(),
                    decryption_failed: false,
                })),
            )?;
        }
        Ok((self.tally, self.refusals))
    }
}

/// The deepest a dictionary entry is renumbered, the walk's own depth.
const MAX_DEPTH: usize = 257;

/// One source value with every reference in the output's numbering: the walk has numbered every
/// object the image's dictionary names, and one it did not number is §7.3.10's null.
fn in_output(assembly: &Assembly<'_>, value: &Object, depth: usize) -> Object {
    if depth >= MAX_DEPTH {
        return Object::Null;
    }
    match value {
        Object::Reference(id) => assembly
            .copied(0, *id)
            .map_or(Object::Null, Object::Reference),
        Object::Array(items) => Object::Array(
            items
                .iter()
                .map(|item| in_output(assembly, item, depth.saturating_add(1)))
                .collect(),
        ),
        Object::Dictionary(dict) => {
            let mut mapped = Dictionary::new();
            for (key, item) in dict.iter() {
                mapped.insert(
                    key.clone(),
                    in_output(assembly, item, depth.saturating_add(1)),
                );
            }
            Object::Dictionary(mapped)
        }
        other => other.clone(),
    }
}

/// Whether a stream dictionary is §8.9.5's image `XObject`.
fn is_image(document: &Document, dict: &Dictionary) -> bool {
    dict.get("Subtype")
        .map(|value| document.resolve(value))
        .is_some_and(|subtype| {
            subtype
                .as_name()
                .is_some_and(|name| name.as_bytes() == b"Image")
        })
}

/// A positive integer entry of the image dictionary, as Table 87 states `/Width` and `/Height`.
fn dimension(document: &Document, dict: &Dictionary, key: &str) -> Option<u32> {
    dict.get(key)
        .map(|value| document.resolve(value))
        .and_then(|value| value.as_integer())
        .and_then(|value| u32::try_from(value).ok())
        .filter(|value| *value > 0)
}

/// The image's samples, exactly as many as its grid states, and what they are — or `None` where
/// any rule in this module's documentation declines it.
fn samples_of(document: &Document, stream: &Stream) -> Option<(Vec<u8>, DctComponents)> {
    let dict = &stream.dict;
    let resolved = |key: &str| dict.get(key).map(|value| document.resolve(value));
    if resolved("ImageMask") == Some(Object::Boolean(true)) {
        return None;
    }
    if resolved("BitsPerComponent").and_then(|value| value.as_integer()) != Some(8) {
        return None;
    }
    if resolved("Mask").is_some_and(|mask| mask.as_array().is_some()) {
        return None;
    }
    let components = components_of(document, &resolved("ColorSpace")?)?;
    if stream.decryption_failed || Document::is_external(stream) || stream.data.is_empty() {
        return None;
    }
    let decoded = lossless_chain(document, dict, &stream.data)?;
    let width = usize::try_from(dimension(document, dict, "Width")?).ok()?;
    let height = usize::try_from(dimension(document, dict, "Height")?).ok()?;
    let channels = match components {
        DctComponents::Gray => 1usize,
        DctComponents::Rgb => 3,
    };
    let wanted = width.checked_mul(height)?.checked_mul(channels)?;
    // §8.9.5 states the grid; a stream shorter than it is one this pass cannot state exactly, and
    // bytes past it are not samples.
    let samples = decoded.get(..wanted)?;
    Some((samples.to_vec(), components))
}

/// The components a JPEG of this colour space carries, or `None` for a space this pass declines.
fn components_of(document: &Document, space: &Object) -> Option<DctComponents> {
    let by_name = |name: &[u8]| match name {
        b"DeviceGray" | b"CalGray" => Some(DctComponents::Gray),
        b"DeviceRGB" | b"CalRGB" => Some(DctComponents::Rgb),
        _ => None,
    };
    if let Some(name) = space.as_name() {
        return by_name(name.as_bytes());
    }
    let family = space.as_array()?;
    let kind = family.first()?.as_name()?;
    match kind.as_bytes() {
        b"CalGray" | b"CalRGB" => by_name(kind.as_bytes()),
        // §8.6.5.5's `/N`, "[t]he number of colour components in the colour space described by
        // the ICC profile data".
        b"ICCBased" => {
            let profile = document.resolve(family.get(1)?);
            match profile
                .as_stream()?
                .dict
                .get("N")
                .map(|n| document.resolve(n))?
                .as_integer()?
            {
                1 => Some(DctComponents::Gray),
                3 => Some(DctComponents::Rgb),
                _ => None,
            }
        }
        _ => None,
    }
}

/// The stream's data through every filter of its chain, or `None` where a stage is a codec, a
/// `/Crypt`, a decode that refused or stopped short, or an entry that names another object.
fn lossless_chain(document: &Document, dict: &Dictionary, data: &[u8]) -> Option<Vec<u8>> {
    let filters: Vec<Name> = match dict.get("Filter") {
        None => Vec::new(),
        Some(Object::Name(name)) => vec![name.clone()],
        Some(Object::Array(items)) => items
            .iter()
            .map(|item| item.as_name().cloned())
            .collect::<Option<_>>()?,
        Some(_) => return None,
    };
    let parms: Vec<Option<Dictionary>> = match dict.get("DecodeParms") {
        None => vec![None; filters.len()],
        Some(Object::Dictionary(only)) if filters.len() == 1 => vec![Some(only.clone())],
        Some(Object::Array(items)) if items.len() == filters.len() => items
            .iter()
            .map(|item| match item {
                Object::Null => Some(None),
                Object::Dictionary(parms) => Some(Some(parms.clone())),
                _ => None,
            })
            .collect::<Option<_>>()?,
        Some(_) => return None,
    };
    if parms
        .iter()
        .flatten()
        .any(|parms| parms.iter().any(|(_, value)| holds_reference(value, 0)))
    {
        return None;
    }
    let mut bytes: std::sync::Arc<[u8]> = data.into();
    for (filter, parms) in filters.iter().zip(&parms) {
        let name = filter.as_bytes();
        if pdf_syntax::filter::is_image_codec(name) || name == b"Crypt" {
            return None;
        }
        let stage = pdf_syntax::filter::decode_with_parms_reported(
            name,
            &bytes,
            parms.as_ref(),
            document.limits(),
        )
        .ok()?;
        if stage.damage.is_some() {
            return None;
        }
        bytes = stage.data;
    }
    Some(bytes.to_vec())
}

/// Whether a value names another object anywhere inside it.
fn holds_reference(value: &Object, depth: usize) -> bool {
    if depth >= MAX_DEPTH {
        return true;
    }
    match value {
        Object::Reference(_) => true,
        Object::Array(items) => items
            .iter()
            .any(|item| holds_reference(item, depth.saturating_add(1))),
        Object::Dictionary(dict) => dict
            .iter()
            .any(|(_, item)| holds_reference(item, depth.saturating_add(1))),
        _ => false,
    }
}
