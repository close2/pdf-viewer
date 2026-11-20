//! A spot colourant's `Separation`, derived from the `DeviceN` space that uses it (`doc/adr/2690`).
//!
//! ISO 19005-2 section 6.2.4.4 and ISO 19005-4 section 6.2.4.4 require every spot colourant a
//! `DeviceN` space uses to have an entry in that space's `Colorants` dictionary. Where the file
//! states a `Separation` array for the colourant elsewhere, `super::sites::colorant_entries` copies
//! it, because the same subclause requires every `Separation` of one name to agree. This module is
//! the other case: the colourant is defined nowhere but inside the `DeviceN` space, and ISO 32000-2
//! §8.6.6.5 says what that space already states about it:
//!
//! > In particular, the alternate colour space and tint transformation function of a Separation
//! > colour space describe the appearance of that colourant alone, whereas those of a DeviceN
//! > colour space describe only the appearance of its colourants in combination.
//!
//! The appearance of one colourant alone is the combination in which every other component is
//! absent, and a `DeviceN` component's value is a tint, zero being none of it. So the `Separation`
//! written names the same colourant, states the same alternate space, and has as its tint
//! transform the producer's own function with the other inputs fixed at zero.
//!
//! # Which functions are restricted, and how
//!
//! §7.10 gives a PDF function no way to call another, so fixing inputs of a function is only
//! *expressible* where the function itself can carry it. A §7.10.5 Type 4 function is a stack
//! program whose inputs are the operands it starts with, so pushing the fixed ones before it runs
//! is the restriction, exactly, and the producer's program follows byte for byte (ADR 2690). A
//! §7.10.2 sampled function is a table: where each absent input lands on one of its samples, the
//! restriction is the row of the table it names, copied bit for bit (ADR 3337); where one lands
//! between two, the restriction would be a blend of rows, an approximation written into an archive
//! as though it were the producer's definition, which ADR 2490 leaves refused until that loss has
//! a name. A §7.10.3 exponential or §7.10.4 stitching function has one input and cannot serve a
//! space of more than one component; each refusal below says which case it met.
//!
//! # Why an `NChannel` space is refused
//!
//! §8.6.6.5 makes the entry informative for a plain `DeviceN` ("may be useful to some PDF
//! processors"), so writing it changes nothing a reader draws. Under the `NChannel` subtype it is
//! not: §8.6.6.5 has an `NChannel` space's components evaluated individually, each through its
//! own `Colorants` entry, so an entry written here would replace the producer's
//! combination with this converter's per-component evaluation of it — a change to the page,
//! which no loss word names yet.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::sync::Arc;

use pdf_syntax::Document;
use pdf_syntax::object::{Dictionary, Name, Object, ObjectId, Stream};

use super::prepare::Spare;
use super::sites::MAX_ENTRY_DEPTH;

/// Why a §7.10.2 sampled tint transform whose absent inputs fall between its samples is not
/// restricted.
pub(super) const SAMPLED_BETWEEN: &str = "a DeviceN colour space here uses a spot colourant its \
     Colorants dictionary does not define, this file states no Separation array of its own for \
     it, and the space's tint transform is a sampled function (ISO 32000-2 §7.10.2, FunctionType \
     0) whose other inputs, at zero, fall between two of its samples rather than on one. \
     Restricting it to that colourant alone means blending those samples into new ones, which \
     BitsPerSample may not hold exactly: an approximation of the producer's transform the archive \
     would carry as their definition; ADR 2490 keeps it refused until that loss is named";

/// Why a tint transform that is a §7.10.3 exponential function is not restricted.
pub(super) const EXPONENTIAL: &str = "a DeviceN colour space here uses a spot colourant its \
     Colorants dictionary does not define, this file states no Separation array of its own for \
     it, and the space's tint transform is an exponential interpolation function (ISO 32000-2 \
     §7.10.3, FunctionType 2), which has one input and so cannot serve a space of more than one \
     component; the space is malformed and no restriction of it is the producer's";

/// Why a tint transform that is a §7.10.4 stitching function is not restricted.
pub(super) const STITCHING: &str = "a DeviceN colour space here uses a spot colourant its \
     Colorants dictionary does not define, this file states no Separation array of its own for \
     it, and the space's tint transform is a stitching function (ISO 32000-2 §7.10.4, \
     FunctionType 3), which has one input and so cannot serve a space of more than one component; \
     the space is malformed and no restriction of it is the producer's";

/// Why a tint transform this module cannot read as a function is not restricted.
pub(super) const UNREADABLE: &str = "a DeviceN colour space here uses a spot colourant its \
     Colorants dictionary does not define, this file states no Separation array of its own for \
     it, and the space's tint transform is not a function stream this converter can read (no \
     FunctionType ISO 32000-2 §7.10 defines, a Domain or Range that is not an array of numbers of \
     the space's size, or a program that does not decode or open with its brace), so there is no \
     producer's program to restrict";

/// Why an `NChannel` space's spot colourant is not given an entry derived from its transform.
pub(super) const UNDER_NCHANNEL: &str = "an NChannel colour space here uses a spot colourant its \
     Colorants dictionary does not define, and this file states no Separation array of its own \
     for it. ISO 32000-2 §8.6.6.5 evaluates an NChannel space's components individually through \
     their Colorants entries, so an entry derived from the space's tint transform would change \
     what the page draws from the producer's combination to this converter's per-component \
     evaluation of it, and no loss word names that change yet";

/// Why two `DeviceN` spaces that would derive different arrays for one colourant are refused.
pub(super) const DERIVED_TWO_WAYS: &str = "two DeviceN colour spaces in this file use one spot \
     colourant with different alternate spaces, tint transforms or positions, and neither the \
     file nor a Colorants dictionary defines it. ISO 19005 section 6.2.4.4 requires every \
     Separation of one name to agree, so the two restrictions cannot both be written, and \
     choosing one would be this converter's choice of the ink's appearance, not the producer's";

/// What one restriction is derived from: two spaces that agree on all of it derive the same
/// `Separation`, and two that do not are [`DERIVED_TWO_WAYS`].
#[derive(Debug, Clone, PartialEq)]
struct Source {
    /// The `DeviceN` space's alternate space, as the file wrote it.
    alternate: Object,
    /// The `DeviceN` space's tint transform, as the file wrote it.
    transform: Object,
    /// The colourant's position among the space's components.
    index: usize,
    /// How many components the space has.
    count: usize,
}

/// The `Separation` arrays derived so far, one per colourant across the file, and the function
/// streams they refer to.
#[derive(Debug, Default)]
pub(super) struct Derived {
    /// Per colourant, what it was derived from and the array written.
    by_name: BTreeMap<Vec<u8>, (Source, Object)>,
    /// The restricted tint transforms, at the spare numbers they took.
    pub(super) written: BTreeMap<ObjectId, Object>,
}

impl Derived {
    /// The `Separation` array for `colourant`, derived from every `DeviceN` space inside
    /// `object` that uses it.
    ///
    /// Every such space must derive the same one, so a colourant met in a second space — in
    /// this object or an earlier finding's — is held to the first.
    pub(super) fn separation(
        &mut self,
        document: &Document,
        object: &Object,
        colourant: &[u8],
        spare: &mut Spare,
    ) -> Result<Object, &'static str> {
        let mut spaces = Vec::new();
        spaces_using(document, object, colourant, 0, &mut spaces);
        let mut array = None;
        for items in &spaces {
            let source = source_of(document, items, colourant)?;
            if let Some((held, written)) = self.by_name.get(colourant) {
                if held != &source {
                    return Err(DERIVED_TWO_WAYS);
                }
                array = Some(written.clone());
                continue;
            }
            // A space of one component describes that colourant alone already, so its own
            // transform is the entry whatever its type, and nothing is derived.
            let transform = if source.count == 1 {
                source.transform.clone()
            } else {
                let function = restricted(document, &source)?;
                let id = spare.take(document).ok_or(UNREADABLE)?;
                self.written.insert(id, function);
                Object::Reference(id)
            };
            let written = Object::Array(vec![
                Object::Name(Name::new(&b"Separation"[..])),
                Object::Name(Name::new(colourant.to_vec())),
                source.alternate.clone(),
                transform,
            ]);
            self.by_name
                .insert(colourant.to_vec(), (source, written.clone()));
            array = Some(written);
        }
        array.ok_or(UNREADABLE)
    }
}

/// Every §8.6.6.5 `DeviceN` array inside `object` whose names include `colourant`.
///
/// References are not followed, as `super::sites::place_colorants` does not follow them: the
/// spaces read are the ones the rewrite will write into.
fn spaces_using(
    document: &Document,
    object: &Object,
    colourant: &[u8],
    depth: usize,
    found: &mut Vec<Vec<Object>>,
) {
    if depth >= MAX_ENTRY_DEPTH {
        return;
    }
    let deeper = depth.saturating_add(1);
    match object {
        Object::Array(items) => {
            if is_devicen(document, items)
                && names_of(document, items).iter().any(|n| n == colourant)
            {
                found.push(items.clone());
            }
            for item in items {
                spaces_using(document, item, colourant, deeper, found);
            }
        }
        Object::Dictionary(dict) => {
            for (_, value) in dict.iter() {
                spaces_using(document, value, colourant, deeper, found);
            }
        }
        Object::Stream(stream) => {
            for (_, value) in stream.dict.iter() {
                spaces_using(document, value, colourant, deeper, found);
            }
        }
        _ => {}
    }
}

/// Whether `items` is a `DeviceN` colour space array, §8.6.6.5's four or five elements.
fn is_devicen(document: &Document, items: &[Object]) -> bool {
    (4..=5).contains(&items.len())
        && items.first().is_some_and(|family| {
            document
                .resolve(family)
                .as_name()
                .is_some_and(|name| name.as_bytes() == b"DeviceN")
        })
}

/// The space's component names, in order.
fn names_of(document: &Document, items: &[Object]) -> Vec<Vec<u8>> {
    items
        .get(1)
        .map(|names| document.resolve(names))
        .and_then(|names| {
            names.as_array().map(|names| {
                names
                    .iter()
                    .filter_map(|entry| {
                        document
                            .resolve(entry)
                            .as_name()
                            .map(|name| name.as_bytes().to_vec())
                    })
                    .collect()
            })
        })
        .unwrap_or_default()
}

/// What the restriction of `items` to `colourant` is derived from, or why it is refused.
fn source_of(
    document: &Document,
    items: &[Object],
    colourant: &[u8],
) -> Result<Source, &'static str> {
    if let Some(attributes) = items.get(4) {
        let attributes = document.resolve(attributes);
        if attributes.as_dict().is_some_and(|dict| {
            document
                .get_key(dict, "Subtype")
                .as_name()
                .is_some_and(|subtype| subtype.as_bytes() == b"NChannel")
        }) {
            return Err(UNDER_NCHANNEL);
        }
    }
    let names = names_of(document, items);
    let index = names
        .iter()
        .position(|name| name == colourant)
        .ok_or(UNREADABLE)?;
    let (Some(alternate), Some(transform)) = (items.get(2), items.get(3)) else {
        return Err(UNREADABLE);
    };
    Ok(Source {
        alternate: alternate.clone(),
        transform: transform.clone(),
        index,
        count: names.len(),
    })
}

/// The producer's tint transform with every input but `source.index` fixed at zero: for a §7.10.2
/// sampled function, [`sampled_slice`]; for a §7.10.5 Type 4 function, a Type 4 function stream
/// built as below.
///
/// # The construction
///
/// §7.10.5.1 has a Type 4 function start with its inputs on the operand stack, the first
/// deepest, run its program, and take its outputs from what is left. The program written is the
/// producer's, with the operands pushed after its opening brace that turn the one input `t` into
/// the producer's inputs: the zeros before `t`'s position, `t` rolled above them, and the zeros
/// after. Nothing of the producer's program is read or changed.
///
/// Two details keep it exact rather than close:
///
/// - **Each fixed input is the value the producer's function evaluates zero as.** §7.10.1 clips
///   an input to the function's `Domain` before the function runs, so a domain that excludes zero
///   evaluates the bound nearest it; that bound is what is pushed, and the domain of the one input
///   left is the producer's for that input, so `t` is clipped the same way.
/// - **Each fixed input is written as a real.** A tint is a real number, and §7.10.5.1's
///   operators distinguish integers from reals (`cvi`, `idiv` and `mod` among them), so a zero
///   pushed as the integer `0` could run a program down a path the producer's never took.
fn restricted(document: &Document, source: &Source) -> Result<Object, &'static str> {
    let transform = document.resolve(&source.transform);
    let Some(stream) = transform.as_stream() else {
        return Err(UNREADABLE);
    };
    match document.get_key(&stream.dict, "FunctionType").as_integer() {
        Some(4) => {}
        Some(0) => return sampled_slice(document, stream, source),
        Some(2) => return Err(EXPONENTIAL),
        Some(3) => return Err(STITCHING),
        _ => return Err(UNREADABLE),
    }
    let domain = numbers(document, &stream.dict, "Domain").ok_or(UNREADABLE)?;
    let range = document.get_key(&stream.dict, "Range");
    if domain.len() != source.count.saturating_mul(2) || range.as_array().is_none() {
        return Err(UNREADABLE);
    }
    let program = document.decoded_stream_data(stream).ok_or(UNREADABLE)?;
    let opening = program
        .iter()
        .position(|byte| !matches!(byte, 0 | b'\t' | b'\n' | 0x0c | b'\r' | b' '))
        .filter(|&at| program.get(at) == Some(&b'{'))
        .ok_or(UNREADABLE)?;

    let mut prefix = String::from(" ");
    if domain.iter().any(|(bound, _)| !bound.is_finite()) {
        return Err(UNREADABLE);
    }
    for (at, bounds) in domain.as_chunks::<2>().0.iter().enumerate() {
        if at == source.index {
            continue;
        }
        let (low, high) = (bounds[0].0.min(bounds[1].0), bounds[0].0.max(bounds[1].0));
        let absent = 0.0_f64.clamp(low, high);
        write_real(&mut prefix, absent);
        let below = at.saturating_add(1);
        if below == source.index {
            // `t` is under the `below` values just pushed, and `n -1 roll` over those `n`
            // operands carries it to the top, so the ones pushed next sit above it.
            let _ = write!(prefix, "{} -1 roll ", below.saturating_add(1));
        }
    }

    let (opened, rest) = program.split_at(opening.saturating_add(1));
    let mut written = Vec::with_capacity(program.len().saturating_add(prefix.len()));
    written.extend_from_slice(opened);
    written.extend_from_slice(prefix.as_bytes());
    written.extend_from_slice(rest);

    let mut dict = Dictionary::new();
    dict.insert(Name::new(&b"FunctionType"[..]), Object::Integer(4));
    dict.insert(
        Name::new(&b"Domain"[..]),
        Object::Array(
            domain
                .as_chunks::<2>()
                .0
                .get(source.index)
                .ok_or(UNREADABLE)?
                .iter()
                .map(|(_, written)| written.clone())
                .collect(),
        ),
    );
    dict.insert(Name::new(&b"Range"[..]), range.clone());
    dict.insert(
        Name::new(&b"Length"[..]),
        Object::Integer(i64::try_from(written.len()).map_err(|_| UNREADABLE)?),
    );
    Ok(Object::Stream(Arc::new(Stream {
        dict,
        data: written.into(),
        decryption_failed: false,
    })))
}

/// A §7.10.2 sampled tint transform restricted to the input at `source.index`, as the slice of
/// its sample table where every other input sits at the sample zero evaluates to (ADR 3337).
///
/// # Why the slice is the restriction, exactly
///
/// §7.10.2 evaluates a sampled function by clipping each input to `Domain`, mapping it through
/// `Encode` onto a position in that dimension's samples,
///
/// > ei = Interpolate (x i ′ , Domain2i , Domain 2i+1 , Encode 2i , Encode 2i+1 )
///
/// clipping that position to `0` to `Size − 1`, and interpolating between the samples around it.
/// Where an absent input's position is a whole number it names one row of samples, and an
/// interpolation that is a product of one interpolation per dimension — Table 39's linear one, and
/// the tensor-product spline this tree reads `/Order 3` as (ADR 1636) — reproduces a dimension's
/// samples at its own sample positions. So the producer's function along the one input left is
/// that input's own interpolation over the samples of that row, which is a one-input sampled
/// function with the producer's `Domain`, `Encode`, `Range`, `Decode` and `BitsPerSample`, and
/// its samples copied bit for bit. No sample is computed and none is rounded.
///
/// A position between two rows would make each new sample a blend of the two, which
/// `BitsPerSample` may not hold exactly; that is the resampling ADR 2490 leaves refused until its
/// loss has a name, and [`SAMPLED_BETWEEN`] says so.
///
/// `/Order 3` is kept only where the producer's function is a spline: Table 39 ignores it where any
/// `/Size` is below four, and the row may be four or longer when another dimension was not.
fn sampled_slice(
    document: &Document,
    stream: &Stream,
    source: &Source,
) -> Result<Object, &'static str> {
    const BITS: [u64; 8] = [1, 2, 4, 8, 12, 16, 24, 32];
    let dict = &stream.dict;
    let domain = numbers(document, dict, "Domain").ok_or(UNREADABLE)?;
    let range = numbers(document, dict, "Range").ok_or(UNREADABLE)?;
    let size = numbers(document, dict, "Size").ok_or(UNREADABLE)?;
    let bits = document
        .get_key(dict, "BitsPerSample")
        .as_integer()
        .and_then(|bits| u64::try_from(bits).ok())
        .filter(|bits| BITS.contains(bits))
        .ok_or(UNREADABLE)?;
    let count = source.count;
    if domain.len() != count.saturating_mul(2) || size.len() != count || range.len() < 2 {
        return Err(UNREADABLE);
    }
    let outputs = range.len() / 2;
    let size: Vec<u32> = size
        .iter()
        .map(|(value, _)| whole(*value).filter(|samples| *samples >= 1))
        .collect::<Option<_>>()
        .ok_or(UNREADABLE)?;
    let encode = match document.get_key(dict, "Encode") {
        Object::Null => None,
        _ => Some(numbers(document, dict, "Encode").ok_or(UNREADABLE)?),
    };
    if encode
        .as_ref()
        .is_some_and(|encode| encode.len() != count.saturating_mul(2))
    {
        return Err(UNREADABLE);
    }
    let order = match document.get_key(dict, "Order") {
        Object::Null => 1,
        stated => stated.as_integer().ok_or(UNREADABLE)?,
    };
    if order != 1 && order != 3 {
        return Err(UNREADABLE);
    }
    let spline = order == 3 && size.iter().all(|n| *n >= 4);

    let row = row(&domain, &size, encode.as_deref(), source.index)?;
    let data = document.decoded_stream_data(stream).ok_or(UNREADABLE)?;
    let kept = size.get(source.index).copied().ok_or(UNREADABLE)?;
    let written = sliced(&data, &row, kept, outputs, bits)?;

    let pair = |entries: &[(f64, Object)]| -> Result<Object, &'static str> {
        Ok(Object::Array(
            entries
                .as_chunks::<2>()
                .0
                .get(source.index)
                .ok_or(UNREADABLE)?
                .iter()
                .map(|(_, written)| written.clone())
                .collect(),
        ))
    };
    let mut function = Dictionary::new();
    function.insert(Name::new(&b"FunctionType"[..]), Object::Integer(0));
    function.insert(Name::new(&b"Domain"[..]), pair(&domain)?);
    function.insert(
        Name::new(&b"Range"[..]),
        Object::Array(range.iter().map(|(_, written)| written.clone()).collect()),
    );
    function.insert(
        Name::new(&b"Size"[..]),
        Object::Array(vec![Object::Integer(i64::from(kept))]),
    );
    function.insert(
        Name::new(&b"BitsPerSample"[..]),
        Object::Integer(i64::try_from(bits).map_err(|_| UNREADABLE)?),
    );
    if let Some(encode) = &encode {
        function.insert(Name::new(&b"Encode"[..]), pair(encode)?);
    }
    let decode = document.get_key(dict, "Decode");
    if !matches!(decode, Object::Null) {
        function.insert(Name::new(&b"Decode"[..]), decode.clone());
    }
    if spline {
        function.insert(Name::new(&b"Order"[..]), Object::Integer(3));
    }
    function.insert(
        Name::new(&b"Length"[..]),
        Object::Integer(i64::try_from(written.len()).map_err(|_| UNREADABLE)?),
    );
    Ok(Object::Stream(Arc::new(Stream {
        dict: function,
        data: written.into(),
        decryption_failed: false,
    })))
}

/// Where the restriction's samples sit in a sampled function's table, which [`row`] finds.
struct Row {
    /// The index of the first sample of the row, the kept input at its first sample.
    first: u64,
    /// How many samples one step along the kept input skips.
    stride: u64,
    /// How many samples the whole table holds.
    total: u64,
}

/// The row of a §7.10.2 sample table every input but `index` picks out at zero: per dimension,
/// the sample an absent input lands on, and how many samples one step along it skips (the first
/// dimension varies fastest).
fn row(
    domain: &[(f64, Object)],
    size: &[u32],
    encode: Option<&[(f64, Object)]>,
    index: usize,
) -> Result<Row, &'static str> {
    let mut first = 0_u64;
    let mut stride = 1_u64;
    let mut kept_stride = 1_u64;
    for (at, (bounds, &samples)) in domain.as_chunks::<2>().0.iter().zip(size).enumerate() {
        let (low, high) = (bounds[0].0, bounds[1].0);
        if !(low.is_finite() && high.is_finite() && low < high) {
            return Err(UNREADABLE);
        }
        let last = f64::from(samples.saturating_sub(1));
        if at == index {
            kept_stride = stride;
        } else {
            let (from, to) = encode.map_or((0.0, last), |encode| {
                (
                    encode[at.saturating_mul(2)].0,
                    encode[at.saturating_mul(2).saturating_add(1)].0,
                )
            });
            let x = 0.0_f64.clamp(low, high);
            let position = (from + (x - low) * (to - from) / (high - low)).clamp(0.0, last);
            if !position.is_finite() {
                return Err(UNREADABLE);
            }
            let position = whole(position).ok_or(SAMPLED_BETWEEN)?;
            first = u64::from(position)
                .checked_mul(stride)
                .and_then(|offset| first.checked_add(offset))
                .ok_or(UNREADABLE)?;
        }
        stride = stride.checked_mul(u64::from(samples)).ok_or(UNREADABLE)?;
    }

    Ok(Row {
        first,
        stride: kept_stride,
        total: stride,
    })
}

/// The samples of `row`, `kept` of them, each of `outputs` values `bits` wide, copied bit for bit
/// into a table of their own.
fn sliced(
    data: &[u8],
    row: &Row,
    kept: u32,
    outputs: usize,
    bits: u64,
) -> Result<Vec<u8>, &'static str> {
    let outputs = u64::try_from(outputs).map_err(|_| UNREADABLE)?;
    let need = row
        .total
        .checked_mul(outputs)
        .and_then(|values| values.checked_mul(bits))
        .ok_or(UNREADABLE)?;
    if u64::try_from(data.len())
        .map_err(|_| UNREADABLE)?
        .saturating_mul(8)
        < need
    {
        return Err(UNREADABLE);
    }
    let mut written = BitWriter::default();
    for step in 0..kept {
        let sample = row
            .first
            .saturating_add(u64::from(step).saturating_mul(row.stride));
        for output in 0..outputs {
            let at = sample
                .saturating_mul(outputs)
                .saturating_add(output)
                .saturating_mul(bits);
            written.push(read_bits(data, at, bits), bits);
        }
    }
    Ok(written.finish())
}

/// `value` as a whole number of `u32`'s range, or `None` where it is not one.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the value is checked to be a whole number from 0 to u32::MAX first, so the cast is exact"
)]
fn whole(value: f64) -> Option<u32> {
    (value.fract() == 0.0 && (0.0..=f64::from(u32::MAX)).contains(&value)).then_some(value as u32)
}

/// The `bits`-wide value starting `at` bits into `data`, high-order bit first, as §7.10.2 packs a
/// sample table. The caller has checked that `data` holds it.
fn read_bits(data: &[u8], at: u64, bits: u64) -> u64 {
    let mut value = 0_u64;
    for bit in at..at.saturating_add(bits) {
        let byte = usize::try_from(bit / 8)
            .ok()
            .and_then(|index| data.get(index))
            .copied()
            .unwrap_or(0);
        // `bit % 8` is below eight, so the shift stays inside the byte.
        let shift = 7_u64.saturating_sub(bit % 8);
        value = value.wrapping_shl(1)
            | u64::from(byte.wrapping_shr(u32::try_from(shift).unwrap_or(0)) & 1);
    }
    value
}

/// A sample table being written, high-order bit first, its last byte padded with zero bits.
#[derive(Default)]
struct BitWriter {
    bytes: Vec<u8>,
    used: u64,
}

impl BitWriter {
    fn push(&mut self, value: u64, bits: u64) {
        for shift in (0..bits).rev() {
            if self.used.is_multiple_of(8) {
                self.bytes.push(0);
            }
            let bit = u8::from(value.wrapping_shr(u32::try_from(shift).unwrap_or(0)) & 1 == 1);
            // `used % 8` is below eight, so the shift stays inside the byte.
            let place = 7_u64.saturating_sub(self.used % 8);
            if let Some(last) = self.bytes.last_mut() {
                *last |= bit.wrapping_shl(u32::try_from(place).unwrap_or(0));
            }
            self.used = self.used.saturating_add(1);
        }
    }

    fn finish(self) -> Vec<u8> {
        self.bytes
    }
}

/// The numbers of an array entry, each with the object the file wrote for it, which is what is
/// copied into the function written.
fn numbers(document: &Document, dict: &Dictionary, key: &str) -> Option<Vec<(f64, Object)>> {
    let value = document.get_key(dict, key);
    value
        .as_array()?
        .iter()
        .map(|entry| {
            let entry = document.resolve(entry);
            entry.as_number().map(|number| (number, entry.clone()))
        })
        .collect()
}

/// Writes `value` as a §7.3.3 real followed by a space: a decimal point always, and never an
/// exponent, which Rust's `Display` for `f64` does not use and the PostScript calculator does
/// not need.
fn write_real(out: &mut String, value: f64) {
    let text = format!("{value}");
    out.push_str(&text);
    if !text.contains('.') {
        out.push_str(".0");
    }
    out.push(' ');
}
