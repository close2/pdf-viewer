//! How many documents state §8.9.6.4's colour key `/Mask`, and how many of those are filtered.
//!
//! ISO 32000-2 §8.9.6.4 spells one of `/Mask`'s two mechanisms as an array of ranges:
//!
//! > For colour key masking, the value of the Mask entry shall be an array of 2 × 𝑛 integers,
//! > [min1max1 …min𝑛max𝑛] , where n is the number of colour components in the image's colour
//! > space.
//!
//! and it names the filters that make the test approximate in the same clause:
//!
//! > When colour key masking is specified, the use of a DCTDecode or lossy JPXDecode filter for
//! > the stream can produce unexpected results.
//!
//! which is a warning rather than an exclusion. So the population that decides how much a
//! reader's choice about those filters is worth is: how many images state a colour key at all,
//! and how many of those state it over a codestream rather than over packed samples.
//!
//! Counted per document and per image, by the **last** filter in the chain, which is the one that
//! decides what the samples are. `/Mask` beside an `/SMask` or a non-zero `/SMaskInData` is
//! counted separately, because §11.6.4.3 makes those override it and a reader owes nothing there.
//!
//! It reads the dictionaries with `pdf_syntax` alone rather than through [`pdf_model::image`],
//! because a census whose predicate is the code under test measures the code rather than the
//! corpus (`doc/HANDOVER.md` trap 8).
//!
//! ```sh
//! cargo run --release -p pdf-model --example colour_key_mask_census -- <file.pdf>…
//! ```

#![expect(
    clippy::print_stdout,
    reason = "an example whose entire output is a measurement"
)]

use std::collections::BTreeMap;

use pdf_syntax::{Dictionary, Document, Object, ObjectId};

/// What one document's image dictionaries say about the question.
#[derive(Default)]
struct Finding {
    /// Images stating a `/Mask` array, by the last filter in the chain (`"(none)"` where the
    /// chain leaves packed samples behind).
    by_filter: BTreeMap<String, usize>,
    /// How many of those state an `/SMask` or a non-zero `/SMaskInData` beside it, which
    /// §11.6.4.3 makes win.
    overridden: usize,
}

fn main() {
    let mut opened = 0_usize;
    let mut with_any = 0_usize;
    let mut with_codec = 0_usize;
    let mut totals: BTreeMap<String, usize> = BTreeMap::new();
    let mut overridden = 0_usize;
    let mut lines: Vec<String> = Vec::new();

    for path in std::env::args().skip(1) {
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        opened = opened.saturating_add(1);
        let finding = examine(&document);
        if finding.by_filter.is_empty() {
            continue;
        }
        with_any = with_any.saturating_add(1);
        if finding.by_filter.keys().any(|filter| is_codec(filter)) {
            with_codec = with_codec.saturating_add(1);
        }
        overridden = overridden.saturating_add(finding.overridden);
        let mut named: Vec<String> = Vec::new();
        for (filter, count) in &finding.by_filter {
            let total = totals.entry(filter.clone()).or_default();
            *total = total.saturating_add(*count);
            named.push(format!("{count} {filter}"));
        }
        lines.push(format!(
            "  {path}: {} with a colour-key /Mask, {} overridden by /SMask or /SMaskInData",
            named.join(", "),
            finding.overridden
        ));
    }

    println!("{opened} document(s) opened");
    println!("  {with_any} state an image with a colour-key /Mask");
    println!("    {with_codec} of those state one over a codestream filter");
    println!("    {overridden} such image(s) are overridden by /SMask or /SMaskInData");
    for (filter, count) in &totals {
        println!("      {count} behind /{filter}");
    }
    for line in &lines {
        println!("{line}");
    }
}

/// The four filters that hand this tree a codestream rather than packed samples.
fn is_codec(filter: &str) -> bool {
    matches!(
        filter,
        "DCTDecode" | "DCT" | "JPXDecode" | "JBIG2Decode" | "CCITTFaxDecode" | "CCF"
    )
}

/// Walks every object, counting the image dictionaries whose `/Mask` is a range array.
///
/// Every object rather than the page tree, because an image reached through a form `XObject`,
/// a tiling pattern's resources or a soft-mask group is the same image as far as §8.9.6.4 is
/// concerned, and a walk that only followed `/Resources` would undercount by whatever nesting
/// a producer chose.
fn examine(document: &Document) -> Finding {
    let mut finding = Finding::default();
    for number in document.xref().object_numbers() {
        let object = document.get(ObjectId {
            number,
            generation: 0,
        });
        let Some(stream) = object.as_stream() else {
            continue;
        };
        let dict = &stream.dict;
        if document
            .get_key(dict, "Subtype")
            .as_name()
            .is_none_or(|name| name.as_bytes() != b"Image")
        {
            continue;
        }
        // §8.9.6.1 spells two mechanisms with one key: a stream is §8.9.6.3's explicit mask
        // and an array is this clause's ranges.
        if document.get_key(dict, "Mask").as_array().is_none() {
            continue;
        }
        let filter = last_filter(document, dict);
        let seen = finding.by_filter.entry(filter).or_default();
        *seen = seen.saturating_add(1);
        if document.get_key(dict, "SMask").as_stream().is_some()
            || document
                .get_key(dict, "SMaskInData")
                .as_integer()
                .is_some_and(|value| value != 0)
        {
            finding.overridden = finding.overridden.saturating_add(1);
        }
    }
    finding
}

/// The last name in `/Filter`, which is the filter that decides what the samples are.
fn last_filter(document: &Document, dict: &Dictionary) -> String {
    let filter = document.get_key(dict, "Filter");
    let name = match &filter {
        Object::Name(name) => Some(name.as_bytes().to_vec()),
        Object::Array(items) => items
            .last()
            .map(|item| document.resolve(item))
            .and_then(|item| item.as_name().map(|name| name.as_bytes().to_vec())),
        _ => None,
    };
    name.map_or_else(
        || "(none)".to_owned(),
        |name| String::from_utf8_lossy(&name).into_owned(),
    )
}
