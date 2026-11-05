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
//! # Why only a PostScript calculator function is restricted
//!
//! §7.10 gives a PDF function no way to call another, so fixing inputs of a function is only
//! *expressible* where the function is a program: a §7.10.5 Type 4 function is a stack program
//! whose inputs are the operands it starts with, so pushing the fixed ones before it runs is the
//! restriction, exactly, and the producer's program follows byte for byte. A §7.10.2 sampled or
//! §7.10.3 exponential function would have to be resampled into a new one, which is an
//! approximation written into an archive as though it were the producer's definition; ADR 2490
//! leaves those refused until that loss has a name, and each refusal below says which type it met.
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

/// Why a tint transform that is a §7.10.2 sampled function is not restricted.
pub(super) const SAMPLED: &str = "a DeviceN colour space here uses a spot colourant its \
     Colorants dictionary does not define, this file states no Separation array of its own for \
     it, and the space's tint transform is a sampled function (ISO 32000-2 §7.10.2, FunctionType \
     0). Restricting it to that colourant alone means resampling it into a new one-input \
     function, an approximation of the producer's transform the archive would carry as their \
     definition; ADR 2490 keeps it refused until that loss is named";

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

/// The producer's tint transform with every input but `source.index` fixed at zero, as a
/// §7.10.5 Type 4 function stream.
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
        Some(0) => return Err(SAMPLED),
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
    for (at, bounds) in domain.chunks_exact(2).enumerate() {
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
                .chunks_exact(2)
                .nth(source.index)
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
