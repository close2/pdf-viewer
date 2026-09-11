//! The colour specification boxes of a `JPXDecode` image, reduced to the one that is used.
//!
//! ISO 19005-2 section 6.2.8.3 and ISO 19005-4 section 6.2.7.3 say two things about a JPEG 2000
//! image's colour that a file can fail: where the data states more than one colour space
//! specification, exactly one shall be marked as the best available; and the specification
//! method a colour box states shall be one of the three the part admits. Both are fields of the
//! **JP2 wrapper** rather than of the codestream, which is why the rewrite below touches not one
//! wavelet coefficient.
//!
//! # Why removing a box is the remedy, and writing one is not
//!
//! `doc/pdf-a-mitigations.md` section 4.5 read these two rows twice and stopped in the same place
//! both times: every *value* this converter could write into a colour box is a choice. That is
//! still true — a specification method outside the three the part admits describes the image's
//! colour in a way the part does not read, so writing one of the three in its place states a
//! colour space the box did not. What neither reading asked is whether a box can **go**, and the
//! sentence that answers it is the one immediately after the method's, in both parts: a
//! conforming processor shall use only the selected colour space and shall ignore all the others.
//!
//! So the specifications this rewrite removes are the ones the target's own subclause requires a
//! processor to ignore. Which box is *selected* is not this converter's judgement either — two
//! sentences decide it and neither is ours:
//!
//! - Where exactly one specification carries an `APPROX` of `0x01`, that is the one. Both parts'
//!   NOTE 2 says the value marks the colour space with the best colour fidelity available, and
//!   ISO 32000-2 §7.4.9 sends a processor to the same box:
//!
//!   > If multiple colour space specifications are given in the JPEG 2000 data, a PDF processor
//!   > should attempt to use the one with the highest precedence and best approximation value.
//!
//! - Where **every** specification states an `APPROX` of zero, no box is marked better than
//!   another: ISO/IEC 15444-1:2000 I.5.3.3 reserves that field, requires it to be zero and has a
//!   conforming reader ignore its value — and the same clause says a conforming JP2 reader
//!   ignores every colour specification box after the first. The first box is then the used one
//!   by the core part's own rule.
//!
//! Any other shape — two boxes marked best, or a mixture of marked and unmarked ones — is
//! refused, because ranking the producer's specifications against each other is the choice
//! section 4.5 refused and it stays refused.
//!
//! # What it costs, and why it is authorised rather than mechanical
//!
//! §7.4.9 does not only recommend a box, it states a fallback:
//!
//! > If the colour space is given by an unsupported ICC profile, the next lower colour space, in
//! > terms of precedence and approximation value, shall be used. If no supported colour space is
//! > found, the colour space used shall be DeviceGray , DeviceRGB , or DeviceCMYK , depending on
//! > the whether the number of ordinary channels in the JPEG 2000 data is 1, 3, or 4.
//!
//! The removed specifications are that chain. A processor that can use the kept one sees no
//! difference whatever; one that cannot now falls back to a device space instead of to the
//! producer's second choice. ISO/IEC 15444-1:2000 I.5.3 says as much about why a file states
//! several in the first place — they are the producer's compatibility and optimisation options.
//! That is a real loss, small and nameable, so it is [`Loss::Jpeg2000ColourFallback`] and a
//! caller authorises it.
//!
//! # The structures this declines to edit
//!
//! Removing bytes from inside the `jp2h` superbox moves every byte after it. ISO/IEC
//! 15444-1:2000 Annex I defines no box that states a byte offset into the file, so a file built
//! only from part 1's boxes can take the edit; ISO/IEC 15444-2 — which defines the fragment
//! tables that do state such offsets — **is not held by this project**, so a file carrying any
//! box part 1 does not define is refused rather than guessed at. The same conservatism applies
//! to the stream: the rewrite replaces the object's own bytes, so a `/Filter` that is anything
//! other than `JPXDecode` alone is refused, because then the stream's bytes are not the JPEG
//! 2000 data.
//!
//! Every edit is proved before it is promised, which is `doc/adr/0973`'s rule: the rewritten
//! bytes are parsed again, and the document is refused unless exactly one colour specification
//! survives, its method is one the part admits, and every other header the data states — the
//! image header, the per-component depths, the channel definitions, the codestream's `SIZ`
//! marker — is the one it stated before.

use std::collections::BTreeMap;

use pdf_archive::Outcome;
use pdf_model::jpeg2000::Headers;
use pdf_syntax::Document;
use pdf_syntax::object::{Dictionary, Name, Object, ObjectId, Stream};

use super::decision::Because;

/// The two requirement identifiers this rewrite answers.
const ANSWERED: [&str; 2] = [
    "graphics/jpeg2000-one-best-colour-space-specification",
    "graphics/jpeg2000-colour-specification-method",
];

/// The colour specification methods both parts admit, ISO 19005-2 6.2.8.3 and ISO 19005-4 6.2.7.3.
///
/// The same three `pdf_archive` judges against, restated rather than shared because the two
/// crates read the same sentence for different purposes: one decides whether a file fails, the
/// other whether the box that survives a removal would.
const ADMITTED_METHODS: [u8; 3] = [0x01, 0x02, 0x03];

/// The `APPROX` value that marks the specification with the best colour fidelity available.
///
/// Both parts' NOTE 2. ISO/IEC 15444-1:2000 I.5.3.3 gives the field no such meaning — it
/// reserves it and sets it to zero — which is why [`selected`] has a second rule for a file that
/// states only zeros.
const BEST_APPROXIMATION: u8 = 0x01;

/// The `jp2h` superbox, ISO/IEC 15444-1:2000 I.5.3.
const JP2_HEADER: [u8; 4] = *b"jp2h";
/// The `colr` box, I.5.3.3.
const COLOUR_SPECIFICATION: [u8; 4] = *b"colr";

/// Every box type ISO/IEC 15444-1:2000 Table I-2 defines at the top level of a JP2 file.
///
/// A file stating a box outside this list is refused by [`box_ranges`]: part 2 defines the
/// fragment table, whose contents are byte offsets into the file, and this project does not hold
/// part 2 — so a converter that shifted a file's bytes past an unknown box would be asserting
/// that the box does not point at them.
const TOP_LEVEL_BOXES: [[u8; 4]; 8] = [
    *b"jP  ", *b"ftyp", *b"jp2h", *b"jp2c", *b"jp2i", *b"xml ", *b"uuid", *b"uinf",
];

/// Every box type Table I-2 defines inside the `jp2h` superbox.
const HEADER_BOXES: [[u8; 4]; 7] = [
    *b"ihdr", *b"bpcc", *b"colr", *b"pclr", *b"cmap", *b"cdef", *b"res ",
];

/// A box header is a four-byte `LBox` and a four-byte `TBox`, I.4 Table I-1.
const BOX_HEADER: usize = 8;
/// An `LBox` of 1 puts an eight-byte `XLBox` after `TBox`, I.4.
const EXTENDED_BOX_HEADER: usize = 16;

/// The replacement stream for each `JPXDecode` image whose colour specifications are reduced.
#[derive(Debug, Default)]
pub(super) struct Specifications {
    /// The new stream object, by the image `XObject` the finding named.
    pub(super) at: BTreeMap<ObjectId, Object>,
    /// How many colour specification boxes were removed, in total.
    pub(super) removed: usize,
}

/// Why no JPEG 2000 image's colour specifications were reduced.
const NOTHING_TO_REDUCE: &str = "no failed requirement named a JPXDecode image whose colour \
     specification boxes this rewrite could reduce";

/// Why a finding naming no object cannot be acted on.
const NO_IMAGE_OBJECT: &str = "a JPEG 2000 colour specification failure was reported without the \
     image XObject it is about, so there is no stream for this rewrite to replace. Writing into \
     an object no finding named would be this converter reading the file a second way";

/// Why data this rewrite cannot read is left alone.
const DATA_NOT_READ: &str = "a JPXDecode image whose colour specifications fail ISO 19005 states \
     bytes this converter cannot read as the JP2 file structure ISO 32000-2 \u{a7}7.4.9 requires \
     of them, or carries them under a filter chain that is not JPXDecode alone. Either way the \
     object's own bytes are not the JPEG 2000 data, and rewriting them would be writing over \
     something else";

/// Why a box part 1 does not define stops the edit.
const BOX_NOT_IN_PART_ONE: &str = "a JPXDecode image whose colour specifications fail ISO 19005 \
     carries a box ISO/IEC 15444-1:2000 Annex I does not define. Removing a colour specification \
     box moves every byte after it, and part 1 defines no box that states a byte offset into the \
     file — but ISO/IEC 15444-2, which defines the fragment tables that do, is not held by this \
     project. So a box this converter cannot name is a box it will not shift bytes past";

/// Why a file whose specifications rank themselves ambiguously is refused.
const NO_SELECTED_SPECIFICATION: &str = "a JPXDecode image states several colour space \
     specifications and none of ISO 19005's or the base standard's sentences selects one of \
     them: either more than one is marked as the best available, or some are marked and some are \
     not. ISO 19005 section 6.2.8.3 and section 6.2.7.3 mark the used specification with an \
     APPROX of 0x01, and ISO/IEC 15444-1:2000 I.5.3.3 makes the first box the used one in a file \
     whose APPROX fields are all zero as that clause requires; a file that is neither ranks its \
     producer's specifications in a way only its producer can settle, and choosing between them \
     here would be doc/pdf-a-mitigations.md section 4.5's refusal made silently";

/// Why the specification that would survive is itself the failing one.
const SELECTED_METHOD_NOT_ADMITTED: &str = "the colour space specification a JPXDecode image \
     marks as used states a specification method outside the three ISO 19005 admits. Removing \
     the other boxes would leave the file failing the same requirement, and writing one of the \
     three admitted methods in its place would state a colour space the box did not — which is \
     doc/pdf-a-mitigations.md section 4.5's finding and is unchanged";

/// Why a file with one specification has nothing to remove.
const ONE_SPECIFICATION_ONLY: &str = "a JPXDecode image states a single colour space \
     specification whose method is outside the three ISO 19005 admits. There is no other box for \
     a processor to fall back to and none to remove, so the only route left writes a method the \
     box did not state, which states a colour space the file does not";

/// Why the edit was not carried out on the copy it was proved against.
const REWRITE_NOT_PROVED: &str = "the colour specification boxes of a JPXDecode image were \
     removed on a copy and the result did not read back as the same image with one specification \
     left, so the edit is refused rather than written. doc/adr/0973's rule: a rewrite promises \
     only what it has placed";

impl Specifications {
    /// Whether anything was reduced.
    fn is_empty(&self) -> bool {
        self.at.is_empty()
    }
}

/// Every `JPXDecode` image a failed requirement named, with its colour specifications reduced.
///
/// The population is the validator's findings rather than a walk of this converter's: ISO 19005's
/// two colour-box rules have conditions — more than one specification, a method outside three —
/// and reading them is `pdf_archive`'s job, so the images this touches are the ones it named.
///
/// # Errors
///
/// [`Because`] naming the shape that stopped the edit; the module comment has each of them.
pub(super) fn reduce_specifications(
    document: &Document,
    input: &pdf_archive::Report,
) -> Result<Specifications, Because> {
    let mut out = Specifications::default();
    for judgement in input.failures() {
        if !ANSWERED.contains(&judgement.id) {
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
            let (stream, data) = jpx_data(document, id)?;
            let removed = reduce(&data)?;
            out.removed = out.removed.saturating_add(removed.removed);
            out.at.insert(id, replacement(&stream.dict, &removed.data)?);
        }
    }
    if out.is_empty() {
        return Err(Because::NotBuiltYet(NOTHING_TO_REDUCE));
    }
    Ok(out)
}

/// The stream an image `XObject` is, and the JPEG 2000 data that *is* its bytes.
///
/// The second half is the condition: ISO 32000-2 Table 5 lets `/Filter` be a chain, and this
/// rewrite writes the object's own bytes back — so it is refused unless `JPXDecode` is the whole
/// chain and the stream's data is therefore the JPX file structure itself.
fn jpx_data(
    document: &Document,
    id: ObjectId,
) -> Result<(std::sync::Arc<Stream>, Vec<u8>), Because> {
    let Object::Stream(stream) = document.get(id) else {
        return Err(Because::NotBuiltYet(NO_IMAGE_OBJECT));
    };
    // §7.3.8.2's `/F` makes the stream's data live in another file, which §7.4.9 then decodes
    // instead of these bytes; there is nothing here to rewrite.
    if stream.decryption_failed || !document.get_key(&stream.dict, "F").is_null() {
        return Err(Because::NotBuiltYet(DATA_NOT_READ));
    }
    if !only_jpx(document, &stream.dict) {
        return Err(Because::NotBuiltYet(DATA_NOT_READ));
    }
    let data = stream.data.to_vec();
    Ok((stream, data))
}

/// Whether `/Filter` is `JPXDecode` and nothing else.
///
/// §7.3.8.2 lets the entry be a name or an array, and Table 5 lets the array hold a chain. Only
/// the two shapes that make the stream's stored bytes the JPEG 2000 data are admitted here.
fn only_jpx(document: &Document, dict: &Dictionary) -> bool {
    let filter = document.get_key(dict, "Filter");
    match document.resolve(&filter) {
        Object::Name(name) => name.as_bytes() == b"JPXDecode",
        Object::Array(items) => match items.as_slice() {
            [only] => document
                .resolve(only)
                .as_name()
                .is_some_and(|name| name.as_bytes() == b"JPXDecode"),
            _ => false,
        },
        _ => false,
    }
}

/// The JPEG 2000 data with every colour specification but the used one removed.
struct Reduced {
    /// The new bytes.
    data: Vec<u8>,
    /// How many boxes went.
    removed: usize,
}

/// Removes the colour specification boxes ISO 19005 requires a processor to ignore.
fn reduce(data: &[u8]) -> Result<Reduced, Because> {
    let before = Headers::parse(data).map_err(|_| Because::NotBuiltYet(DATA_NOT_READ))?;
    let keep = selected(&before)?;
    if !ADMITTED_METHODS.contains(&before.colour[keep].method) {
        return Err(Because::NotBuiltYet(if before.colour.len() == 1 {
            ONE_SPECIFICATION_ONLY
        } else {
            SELECTED_METHOD_NOT_ADMITTED
        }));
    }
    if before.colour.len() == 1 {
        // The selected box is admitted and it is the only one, so nothing this rewrite removes
        // would change the file: the failure the finding named is the second sentence of the
        // one-best rule — the selected specification's own ICC profile — and that is the
        // profile-replacement case rather than a box removal.
        return Err(Because::NotBuiltYet(ONE_SPECIFICATION_ONLY));
    }
    let top = box_ranges(data, 0..data.len(), &TOP_LEVEL_BOXES)?;
    let header = top
        .iter()
        .find(|found| found.kind == JP2_HEADER)
        .ok_or(Because::NotBuiltYet(DATA_NOT_READ))?;
    let inner = box_ranges(data, header.payload.clone(), &HEADER_BOXES)?;

    let mut payload: Vec<u8> = Vec::with_capacity(header.payload.len());
    let mut seen = 0usize;
    let mut removed = 0usize;
    for found in &inner {
        if found.kind == COLOUR_SPECIFICATION {
            let index = seen;
            seen = seen.saturating_add(1);
            if index != keep {
                removed = removed.saturating_add(1);
                continue;
            }
        }
        payload.extend_from_slice(&data[found.whole.clone()]);
    }
    let mut out = Vec::with_capacity(data.len());
    out.extend_from_slice(&data[..header.whole.start]);
    out.extend_from_slice(&rebuilt_header(header, &payload)?);
    out.extend_from_slice(&data[header.whole.end..]);
    prove(&before, &out, keep)?;
    Ok(Reduced { data: out, removed })
}

/// Which colour specification the standards' own sentences make the used one.
///
/// The module comment has both rules and the clause each comes from. A file the two do not
/// settle is [`NO_SELECTED_SPECIFICATION`], which is the refusal this rewrite inherits rather
/// than overturns.
fn selected(headers: &Headers<'_>) -> Result<usize, Because> {
    if headers.colour.is_empty() {
        return Err(Because::NotBuiltYet(DATA_NOT_READ));
    }
    let best: Vec<usize> = headers
        .colour
        .iter()
        .enumerate()
        .filter(|(_, colour)| colour.approximation == BEST_APPROXIMATION)
        .map(|(index, _)| index)
        .collect();
    match best.as_slice() {
        [only] => Ok(*only),
        [] if headers
            .colour
            .iter()
            .all(|colour| colour.approximation == 0) =>
        {
            Ok(0)
        }
        _ => Err(Because::NotBuiltYet(NO_SELECTED_SPECIFICATION)),
    }
}

/// One box, as bytes.
#[derive(Debug)]
struct Found {
    /// `TBox`, the four characters.
    kind: [u8; 4],
    /// The whole box, header included.
    whole: std::ops::Range<usize>,
    /// `DBox`, the payload alone.
    payload: std::ops::Range<usize>,
    /// Whether `LBox` was 1, so the header carries an `XLBox`.
    extended: bool,
}

/// Every box in one range, refusing a type the admitted list does not hold.
///
/// ISO/IEC 15444-1:2000 I.4 gives `LBox` three readings — a length, 1 meaning the length is in
/// `XLBox`, and 0 meaning the box runs to the end of the file — and reserves 2 to 7. The
/// last-box reading is admitted only where the box really is last, which is the only shape it
/// can have without the boxes after it being unreachable.
fn box_ranges(
    data: &[u8],
    within: std::ops::Range<usize>,
    admitted: &[[u8; 4]],
) -> Result<Vec<Found>, Because> {
    let mut found = Vec::new();
    let mut at = within.start;
    while at < within.end {
        let rest = within.end.saturating_sub(at);
        if rest < BOX_HEADER {
            return Err(Because::NotBuiltYet(DATA_NOT_READ));
        }
        let stated = u32::from_be_bytes(
            data[at..at.saturating_add(4)]
                .try_into()
                .map_err(|_| Because::NotBuiltYet(DATA_NOT_READ))?,
        );
        let mut kind = [0u8; 4];
        kind.copy_from_slice(&data[at.saturating_add(4)..at.saturating_add(BOX_HEADER)]);
        let (header, length) = match stated {
            0 => (BOX_HEADER, rest),
            1 => {
                if rest < EXTENDED_BOX_HEADER {
                    return Err(Because::NotBuiltYet(DATA_NOT_READ));
                }
                let extended = u64::from_be_bytes(
                    data[at.saturating_add(BOX_HEADER)..at.saturating_add(EXTENDED_BOX_HEADER)]
                        .try_into()
                        .map_err(|_| Because::NotBuiltYet(DATA_NOT_READ))?,
                );
                (
                    EXTENDED_BOX_HEADER,
                    usize::try_from(extended).map_err(|_| Because::NotBuiltYet(DATA_NOT_READ))?,
                )
            }
            2..=7 => return Err(Because::NotBuiltYet(DATA_NOT_READ)),
            other => (
                BOX_HEADER,
                usize::try_from(other).map_err(|_| Because::NotBuiltYet(DATA_NOT_READ))?,
            ),
        };
        if length < header || length > rest {
            return Err(Because::NotBuiltYet(DATA_NOT_READ));
        }
        if !admitted.contains(&kind) {
            return Err(Because::NotBuiltYet(BOX_NOT_IN_PART_ONE));
        }
        let end = at.saturating_add(length);
        found.push(Found {
            kind,
            whole: at..end,
            payload: at.saturating_add(header)..end,
            extended: stated == 1,
        });
        at = end;
    }
    Ok(found)
}

/// The `jp2h` superbox rebuilt around a shorter payload, in the length form it already used.
fn rebuilt_header(header: &Found, payload: &[u8]) -> Result<Vec<u8>, Because> {
    let head = if header.extended {
        EXTENDED_BOX_HEADER
    } else {
        BOX_HEADER
    };
    let total = payload.len().saturating_add(head);
    let mut out = Vec::with_capacity(total);
    if header.extended {
        out.extend_from_slice(&1u32.to_be_bytes());
        out.extend_from_slice(&header.kind);
        out.extend_from_slice(&(total as u64).to_be_bytes());
    } else {
        out.extend_from_slice(
            &u32::try_from(total)
                .map_err(|_| Because::NotBuiltYet(REWRITE_NOT_PROVED))?
                .to_be_bytes(),
        );
        out.extend_from_slice(&header.kind);
    }
    out.extend_from_slice(payload);
    Ok(out)
}

/// The proof: the edited bytes read back as the same image with one specification left.
///
/// Everything the headers state except the colour specifications has to be the value it was, and
/// the one specification that survives has to be the one selected, byte field for byte field.
/// `doc/adr/0973`'s rule, applied to bytes rather than to a dictionary entry.
fn prove(before: &Headers<'_>, data: &[u8], keep: usize) -> Result<(), Because> {
    let after = Headers::parse(data).map_err(|_| Because::NotBuiltYet(REWRITE_NOT_PROVED))?;
    let same = after.image == before.image
        && after.bits_per_component == before.bits_per_component
        && after.channels == before.channels
        && after.component_mapping == before.component_mapping
        && after.codestream == before.codestream;
    if !same || after.colour.len() != 1 || after.colour[0] != before.colour[keep] {
        return Err(Because::NotBuiltYet(REWRITE_NOT_PROVED));
    }
    if !ADMITTED_METHODS.contains(&after.colour[0].method) {
        return Err(Because::NotBuiltYet(REWRITE_NOT_PROVED));
    }
    Ok(())
}

/// The image stream with its new bytes and the `/Length` they need.
///
/// `/Filter` is left exactly as it was, which [`jpx_data`] has already established is `JPXDecode`
/// and nothing else — so the data these bytes replace is decoded by the same filter it was.
fn replacement(from: &Dictionary, data: &[u8]) -> Result<Object, Because> {
    let mut dict = from.clone();
    dict.insert(
        Name::new(&b"Length"[..]),
        Object::Integer(
            i64::try_from(data.len()).map_err(|_| Because::NotBuiltYet(REWRITE_NOT_PROVED))?,
        ),
    );
    Ok(Object::Stream(std::sync::Arc::new(Stream {
        dict,
        data: data.to_vec().into(),
        decryption_failed: false,
    })))
}
