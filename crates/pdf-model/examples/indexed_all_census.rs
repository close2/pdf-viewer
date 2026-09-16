//! How the corpus states §8.6.6.3's `Indexed` spaces and §8.6.6.4's `/All` colourant.
//!
//! Two of §8.6.6's families are decided by a value this census reports: an `Indexed` space's
//! base and `hival` fix what an index selects (§8.6.6.3), and a `Separation` whose colourant is
//! `/All` is the registration-mark black that reverts to a complemented tint on a display
//! (§8.6.6.4). ISO 32000-2 §8.6.3 gives the array form both share:
//!
//! > A colour space shall be defined by an array object whose first element is a name object
//! > identifying the colour space family.
//!
//! so this walks every object, and every image `XObject`'s inline `/ColorSpace`, for an array
//! whose first element is `/Indexed` (or `/I`) or `/Separation`. It reads the arrays with
//! `pdf_syntax` alone rather than through [`pdf_model::ColourSpace`], because a census whose
//! predicate is the code under test measures the code rather than the corpus (`doc/HANDOVER.md`
//! trap 8): the population here is the standard's array shapes, not this reader's classification
//! of them.
//!
//! It is a lower bound in one direction the walk cannot close: a colour space written *inline*
//! inside a `/ColorSpace` resource subdictionary — neither its own indirect object nor an
//! image's own entry — is reached only where a producer made it an object. That is the same
//! nesting tradeoff `colour_key_mask_census` documents, and it undercounts rather than
//! miscounts.
//!
//! ```sh
//! cargo run --release -p pdf-model --example indexed_all_census -- <file.pdf>…
//! ```

#![expect(
    clippy::print_stdout,
    reason = "an example whose entire output is a measurement"
)]

use std::collections::BTreeMap;

use pdf_syntax::{Document, Object, ObjectId};

fn main() {
    let mut opened = 0_usize;
    let mut with_indexed = 0_usize;
    let mut with_all = 0_usize;
    // `Indexed` spaces tallied by "<base> hival=<n>", and `/All` spaces counted outright.
    let mut indexed: BTreeMap<String, usize> = BTreeMap::new();
    let mut all_colourant = 0_usize;
    let mut lines: Vec<String> = Vec::new();

    for path in std::env::args().skip(1) {
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        opened = opened.saturating_add(1);

        let mut doc_indexed: BTreeMap<String, usize> = BTreeMap::new();
        let mut doc_all = 0_usize;
        for space in colour_space_arrays(&document) {
            match classify(&document, &space) {
                Some(Space::Indexed(descriptor)) => {
                    let seen = doc_indexed.entry(descriptor).or_default();
                    *seen = seen.saturating_add(1);
                }
                Some(Space::All) => doc_all = doc_all.saturating_add(1),
                None => {}
            }
        }

        if doc_indexed.is_empty() && doc_all == 0 {
            continue;
        }
        if !doc_indexed.is_empty() {
            with_indexed = with_indexed.saturating_add(1);
        }
        if doc_all > 0 {
            with_all = with_all.saturating_add(1);
        }
        all_colourant = all_colourant.saturating_add(doc_all);
        let mut named: Vec<String> = Vec::new();
        for (descriptor, count) in &doc_indexed {
            let total = indexed.entry(descriptor.clone()).or_default();
            *total = total.saturating_add(*count);
            named.push(format!("{count}×[{descriptor}]"));
        }
        if doc_all > 0 {
            named.push(format!("{doc_all}×/All"));
        }
        lines.push(format!("  {path}: {}", named.join(", ")));
    }

    println!("{opened} document(s) opened");
    println!("  {with_indexed} state an Indexed space, {with_all} state a Separation /All");
    println!("  Indexed spaces by base and hival:");
    for (descriptor, count) in &indexed {
        println!("      {count}× [{descriptor}]");
    }
    println!("  {all_colourant} Separation /All space(s) in all");
    for line in &lines {
        println!("{line}");
    }
}

/// One §8.6.6 space this census reports.
enum Space {
    /// An `Indexed` space, described by its base family and `hival`.
    Indexed(String),
    /// A `Separation` whose colourant name is `/All`.
    All,
}

/// Every colour-space array reachable as an indirect object or an image's own `/ColorSpace`.
///
/// The two routes are disjoint enough to matter: a palette shared between images is one indirect
/// array object, while a one-off `Indexed` space is written inline in the image's entry, and a
/// walk of only one route would miss half the corpus.
fn colour_space_arrays(document: &Document) -> Vec<Object> {
    let mut spaces = Vec::new();
    for number in document.xref().object_numbers() {
        let object = document.get(ObjectId {
            number,
            generation: 0,
        });
        if object.as_array().is_some() {
            spaces.push(object.clone());
        }
        if let Some(stream) = object.as_stream() {
            let space = document.get_key(&stream.dict, "ColorSpace");
            if space.as_array().is_some() {
                spaces.push(space);
            }
        }
    }
    spaces
}

/// Reads the family name and, for the two families this census reports, the value that fixes it.
fn classify(document: &Document, space: &Object) -> Option<Space> {
    let items = space.as_array()?;
    let family = document.resolve(items.first()?);
    match family.as_name()?.as_bytes() {
        b"Indexed" | b"I" => {
            let base = items.get(1).map_or_else(
                || "?".to_owned(),
                |item| base_descriptor(&document.resolve(item)),
            );
            let hival = items
                .get(2)
                .and_then(|item| document.resolve(item).as_integer());
            let hival = hival.map_or_else(|| "?".to_owned(), |value| value.to_string());
            Some(Space::Indexed(format!("{base} hival={hival}")))
        }
        b"Separation" => {
            let colourant = items.get(1).map(|item| document.resolve(item));
            let name = colourant.as_ref().and_then(Object::as_name);
            match name.map(pdf_syntax::Name::as_bytes) {
                Some(b"All") => Some(Space::All),
                _ => None,
            }
        }
        _ => None,
    }
}

/// Names an `Indexed` base by its family, one level deep — a bare name, or the first element of a
/// nested array (`[/ICCBased …]`, `[/CalRGB …]`), or the shape where it is neither.
fn base_descriptor(base: &Object) -> String {
    if let Some(name) = base.as_name() {
        return String::from_utf8_lossy(name.as_bytes()).into_owned();
    }
    if let Some(items) = base.as_array() {
        if let Some(family) = items.first().and_then(Object::as_name) {
            return String::from_utf8_lossy(family.as_bytes()).into_owned();
        }
        return "array".to_owned();
    }
    "?".to_owned()
}
