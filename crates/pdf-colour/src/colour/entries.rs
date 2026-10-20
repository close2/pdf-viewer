//! Reading a colour space dictionary's entries: colourant names, white point, ranges and number arrays.

use std::sync::Arc;

use pdf_syntax::{Dictionary, Document, Name, Object};

use super::cie::D50;

/// Reads the colourant names of a `Separation` or `DeviceN` space.
///
/// ISO 32000-2 §8.6.6.4 gives a `Separation` one `name` object; §8.6.6.5 gives a `DeviceN`
/// an array of them, whose length is also how many operands `scn` takes. The returned vector
/// therefore keeps the array's *arity*: an entry that is not a name becomes empty rather than
/// disappearing, so a malformed name cannot silently change the number of components.
/// The colourant names a space carries, as [`Name`]s shared between its clones.
pub(super) fn named(names: &[Vec<u8>]) -> Arc<[Name]> {
    names.iter().map(|name| Name::new(name.clone())).collect()
}

pub(super) fn colourant_names(
    document: &Document,
    object: Option<&Object>,
    single: bool,
) -> Vec<Vec<u8>> {
    let name_of = |item: &Object| {
        document
            .resolve(item)
            .as_name()
            .map(|name| name.as_bytes().to_vec())
            .unwrap_or_default()
    };
    let Some(object) = object else {
        return Vec::new();
    };
    if single {
        return vec![name_of(object)];
    }
    document
        .resolve(object)
        .as_array()
        .map(|items| items.iter().map(name_of).collect())
        .unwrap_or_default()
}

/// Reads a fixed-length array of numbers from a colour space dictionary.
pub(super) fn numbers<const N: usize>(
    document: &Document,
    dict: Option<&Dictionary>,
    key: &'static str,
) -> Option<[f32; N]> {
    let array = document.get_key(dict?, key);
    let values: Vec<f32> = array
        .as_array()?
        .iter()
        .filter_map(|item| document.resolve(item).as_number().map(narrow))
        .collect();
    <[f32; N]>::try_from(values.as_slice()).ok()
}

/// Reads a `WhitePoint` entry, which Tables 62, 63 and 64 all make required.
///
/// A dictionary without one is not a CIE-based space at all, and there is nothing in the
/// file to recover the intent from. D50 is the substitute because it is the connection
/// space's own white, which makes the adaptation stage vanish and leaves the space's
/// `Gamma` and `Matrix`, or a `Lab` space's `Range` — the parts the document *did* state —
/// doing exactly what they say.
pub(super) fn white_point(document: &Document, dict: Option<&Dictionary>) -> [f32; 3] {
    numbers(document, dict, "WhitePoint")
        // "The numbers X_W and Z_W shall be positive, and Y_W shall be equal to 1.0."
        // A white point violating that would divide the adaptation by zero or invert it.
        .filter(|white| white[0] > 0.0 && white[1] > 0.0 && white[2] > 0.0)
        .unwrap_or(D50)
}

/// ISO 32000-2 §8.6.5.5 Table 65's `/Range`, read in pairs.
///
/// > ( Optional ) An array of 2 × N numbers [𝑚𝑖𝑛 0 𝑚𝑎𝑥 0 𝑚𝑖𝑛 1 𝑚𝑎𝑥 1 …] that shall specify the
/// > minimum and maximum valid values of the corresponding colour components.
///
/// An odd trailing number is dropped rather than paired with a bound nobody wrote; whether the
/// pairs that remain are the profile's `N` of them is [`crate::icc::Profile::with_range`]'s
/// question, since the profile is what states how many components there are.
pub(super) fn icc_range(document: &Document, dict: &Dictionary) -> Vec<(f32, f32)> {
    let array = document.get_key(dict, "Range");
    let Some(items) = array.as_array() else {
        return Vec::new();
    };
    items
        .chunks_exact(2)
        .filter_map(|pair| {
            let low = document.resolve(pair.first()?).as_number().map(narrow)?;
            let high = document.resolve(pair.get(1)?).as_number().map(narrow)?;
            Some((low, high))
        })
        .collect()
}

pub(super) fn narrow(value: f64) -> f32 {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "a colour bound outside f32's range is not a bound"
    )]
    {
        value as f32
    }
}
