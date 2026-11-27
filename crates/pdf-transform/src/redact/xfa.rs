//! The XFA resource of a redacted hybrid form: the value of every field whose widget the removal
//! took goes from the packets that hold it, or `/XFA` goes whole (ISO 32000-2 §12.5.6.23, §K.2;
//! ADR 3789).
//!
//! Which packets hold a field's value, and whether they can be found with certainty, is
//! [`pdf_model::xfa::redacted`]'s to answer; this module writes the answer. A spliced packet is a
//! new stream in the source stream's place, and a resource that goes whole is an interactive form
//! dictionary written without `/XFA` — and a catalog without Table 29's `/NeedsRendering`, which
//! asks for pages to be generated from the resource that is gone. Each is a
//! [`crate::Departure`] in the redaction's report, as the archive converter's `XfaRemoved` is in
//! its own, and none quotes the value.

use std::collections::HashSet;
use std::sync::Arc;

use pdf_model::xfa::{Redacted, redacted};
use pdf_syntax::Document;
use pdf_syntax::object::{Dictionary, Name, Object, ObjectId, Stream};
use pdf_syntax::serialize::Assembly;

use super::traces::Restated;
use crate::Refusal;

/// What the redaction does to the document's XFA resource, decided before anything is written.
pub(super) enum Reach {
    /// No resource, or none of its packets held a removed field's value.
    Untouched,
    /// Each packet stream rewritten, with its new text and what to say of it.
    Spliced(Vec<(ObjectId, String, String)>),
    /// `/XFA` goes whole, for this reason.
    Whole(String),
}

impl Reach {
    /// Whether `/XFA` goes whole.
    pub(super) fn is_whole(&self) -> bool {
        matches!(self, Self::Whole(_))
    }
}

/// What the removal of the annotations `gone` does to the XFA resource: the fields named are
/// those with a widget among them, the same fields [`super::traces::field_values`] takes the
/// value of.
pub(super) fn reach(document: &Document, gone: &HashSet<ObjectId>) -> Reach {
    let fields: Vec<String> = pdf_model::view::widgets_by_field_name(document)
        .into_iter()
        .filter(|(_, widgets)| widgets.iter().any(|widget| gone.contains(widget)))
        .map(|(name, _)| name)
        .collect();
    if fields.is_empty() {
        return Reach::Untouched;
    }
    match redacted(document, &fields) {
        None => Reach::Untouched,
        Some(Redacted::Whole(reason)) => Reach::Whole(reason),
        Some(Redacted::Spliced(streams)) if streams.is_empty() => Reach::Untouched,
        Some(Redacted::Spliced(streams)) => Reach::Spliced(
            streams
                .into_iter()
                .map(|rewritten| {
                    let detail = format!(
                        "XFA packet stream {}: {} emptied, because a widget of the field met \
                         the region and was removed (§K.2: the XFA field values shall be \
                         consistent with the V entries)",
                        rewritten.stream.number,
                        rewritten.places.join(", ")
                    );
                    (rewritten.stream, rewritten.text, detail)
                })
                .collect(),
        ),
    }
}

/// The sentence the report carries where `/XFA` goes whole.
pub(super) fn whole_detail(reason: &str, needs_rendering: bool) -> String {
    let rendering = if needs_rendering {
        ", and the catalog's /NeedsRendering with it, which asks for pages generated from it"
    } else {
        ""
    };
    format!(
        "the interactive form dictionary's /XFA not written{rendering}: a removed widget's field \
         holds its value in the XFA resource, and the places it holds it cannot be found with \
         certainty ({reason}); the calculations, validations and formatting the template carried \
         go with it, and the fields and their remaining values stay"
    )
}

/// The interactive form dictionary without `/XFA`, to be restated in its place where it is an
/// indirect object; `None` where the catalog states it directly, which the catalog's own
/// restatement then writes without the entry.
pub(super) fn indirect_form(
    document: &Document,
    catalog: &Dictionary,
    detail: String,
) -> Option<Restated> {
    let id = catalog.get("AcroForm").and_then(Object::as_reference)?;
    let dict = document.get(id).as_dict()?.clone();
    Some(Restated {
        id,
        dict: without_xfa(&dict),
        detail,
        added: Vec::new(),
        kids_added: Vec::new(),
        parent_tree_kids: Vec::new(),
    })
}

/// The refusal where the object the XFA rewrite needs is one the removal already restated: the
/// value would otherwise be written as the source held it.
pub(super) fn already_replaced(id: ObjectId) -> Refusal {
    Refusal::Assembly(format!(
        "§12.5.6.23: object {} holds the XFA resource and is also restated by the removal, so the \
         field values it holds could not be taken out of it",
        id.number
    ))
}

/// `form` without its `/XFA`.
pub(super) fn without_xfa(form: &Dictionary) -> Dictionary {
    let mut out = Dictionary::new();
    for (key, value) in form.iter() {
        if key.as_bytes() != b"XFA" {
            out.insert(key.clone(), value.clone());
        }
    }
    out
}

/// Places each rewritten packet in the slot reserved for it: the decoded text, unfiltered, with
/// every entry of the source dictionary but the filter, its parameters and the length carried.
pub(super) fn place_packets(
    assembly: &mut Assembly<'_>,
    document: &Document,
    slots: &[(ObjectId, ObjectId, String)],
) -> Result<(), Refusal> {
    for (slot, source, text) in slots {
        let mut dict = Dictionary::new();
        if let Some(stream) = document.get(*source).as_stream() {
            for (key, value) in stream.dict.iter() {
                if !matches!(
                    key.as_bytes(),
                    b"Filter" | b"DecodeParms" | b"Length" | b"DL"
                ) {
                    dict.insert(key.clone(), super::carry(assembly, document, value, 0));
                }
            }
        }
        dict.insert(
            Name::new(&b"Length"[..]),
            Object::Integer(i64::try_from(text.len()).unwrap_or(i64::MAX)),
        );
        let stream = Object::Stream(Arc::new(Stream {
            dict,
            data: Arc::from(text.as_bytes()),
            decryption_failed: false,
        }));
        assembly
            .place(*slot, stream)
            .map_err(|error| Refusal::Assembly(error.to_string()))?;
    }
    Ok(())
}
