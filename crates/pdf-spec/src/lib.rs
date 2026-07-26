//! PDF object model validation, generated from the Arlington PDF Model.
//!
//! The Arlington PDF Model (`doc/arlington-pdf-model`, a pinned submodule) describes every
//! object type in ISO 32000-2 as tab-separated data: permitted keys, their types, whether
//! they are required, the version each appeared in, and how object types link together.
//!
//! `build.rs` turns that data into `static` Rust tables. Hand-writing conformance checks
//! for the whole object model would mean thousands of conditionals no reviewer could audit
//! against the specification; generated tables keep conformance reviewable as *data*. See
//! `doc/adr/0003-arlington-generated-validation.md`.
//!
//! # Startup cost is zero
//!
//! The tables are `static`, so they live in the binary's read-only data. Nothing is parsed
//! or allocated at startup, which `CLAUDE.md` principle 2 requires.
//!
//! # What is and is not interpreted
//!
//! Structure is fully modelled: keys, types, links, versions, inheritance, and
//! unconditional requirement. Arlington's `fn:` predicate language is **not** evaluated —
//! those cells are carried verbatim so a caller can report that it cannot decide, rather
//! than guessing. Guessing either way is a bug: assuming a conditionally-required key is
//! optional accepts invalid files, and assuming it is required rejects valid ones.
//!
//! Of 3973 key rows, predicates appear in `Required` for 192, in `IndirectReference` for
//! 113, in `PossibleValues` for 315, and in `SpecialCase` for 593. The remaining structure
//! — the great majority — is exact.
//!
//! ```
//! # use pdf_spec::{Requirement, object};
//! let catalog = object("Catalog").expect("the model defines Catalog");
//! let pages = catalog.key("Pages").expect("a catalogue has a page tree");
//! assert_eq!(pages.required, Requirement::Always);
//! ```

#![forbid(unsafe_code)]

mod model;

pub use model::{
    Availability, Indirectness, KeyPattern, KeySpec, ObjectSpec, PdfType, Requirement,
    TypeAlternative, TypeGate, Version,
};

// The generated tables. Kept in a private module so that only the curated API below is
// public, rather than whatever shape the generator happens to emit.
mod generated {
    use super::model::{
        Availability, Indirectness, KeyPattern, KeySpec, ObjectSpec, PdfType, Requirement,
        TypeAlternative, TypeGate, Version,
    };

    include!(concat!(env!("OUT_DIR"), "/arlington.rs"));
}

pub use generated::{KEY_COUNT, OBJECT_COUNT, OBJECTS};

/// Looks up an object definition by its Arlington name.
///
/// Names are Arlington's own, which are the TSV filenames: `Catalog`, `PageObject`,
/// `ArrayOfDecodeParams`. They are not always the PDF `/Type` value, because the model
/// distinguishes structures the specification describes in one table.
///
/// Binary search over a table sorted at generation time, so this is logarithmic and
/// allocation-free.
#[must_use]
pub fn object(name: &str) -> Option<&'static ObjectSpec> {
    OBJECTS
        .binary_search_by(|candidate| candidate.name.cmp(name))
        .ok()
        .and_then(|index| OBJECTS.get(index))
}

/// Returns every object definition whose name starts with `prefix`.
///
/// Useful for exploring the model: `prefix("ArrayOf")` enumerates the array types.
pub fn with_prefix(prefix: &'static str) -> impl Iterator<Item = &'static ObjectSpec> {
    OBJECTS
        .iter()
        .filter(move |spec| spec.name.starts_with(prefix))
}
