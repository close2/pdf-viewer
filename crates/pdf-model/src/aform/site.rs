//! Where a field's script is: Table 199's four entries in a field's additional-actions dictionary,
//! and the ECMAScript action each names.
//!
//! ISO 32000-2 §12.7.4.1 makes `/AA` an entry of the *field* dictionary — Table 226 gives a field
//! an additional-actions dictionary defining its behaviour in response to trigger events — and a
//! widget that is the only widget of its field is merged with it, so the entry is
//! looked for on the widget and then up its `/Parent` chain, nearest first: the first dictionary
//! whose `/AA` states the trigger is the field that owns the script. A widget's own `/AA` holds
//! Table 197's pointer events as well, which is why the walk asks for the trigger's key rather
//! than stopping at the first `/AA`.
//!
//! §12.6.4.17's Table 221 says what the action carries: `/JS`, "[a] text string or text stream
//! containing the ECMAScript script to be executed". Both are read as §7.9.2.2's text string.

use pdf_syntax::{Dictionary, Document, Object};

use super::{Call, NotOneCall, Trigger};

/// What a field states for one of Table 199's triggers.
///
/// Public so that an instrument can count what Tier 0 runs and what it reports without running
/// either — `tests/script_corpus.rs` and `examples/javascript_census.rs` ask it.
#[derive(Debug, Clone, PartialEq)]
pub enum Site {
    /// Nothing: no entry for the trigger, or an entry that is not an ECMAScript action.
    Absent,
    /// One call of the library, which Tier 0 runs.
    Library(Call),
    /// A script this tier does not run, with the sentence that says why.
    NotRun(String),
}

/// How far up a `/Parent` chain the walk looks, the field tree's own bound.
const MAX_ANCESTRY: usize = crate::appearance::MAX_FIELD_ANCESTRY;

/// The script a field states for `trigger`, read from a widget and its ancestors.
pub fn of_widget(document: &Document, widget: &Dictionary, trigger: Trigger) -> Site {
    let mut current = widget.clone();
    for _ in 0..=MAX_ANCESTRY {
        let additional = document.get_key(&current, "AA");
        if let Some(additional) = additional.as_dict()
            && let Some(entry) = additional.get(trigger.key())
        {
            return of_action(document, entry, trigger);
        }
        let Some(parent) = document.get_key(&current, "Parent").as_dict().cloned() else {
            return Site::Absent;
        };
        current = parent;
    }
    Site::Absent
}

/// The script a field dictionary states for `trigger` on that dictionary alone.
///
/// What Table 224's `/CO` names: "indirect references to field dictionaries with calculation
/// actions", so the `/C` is on the dictionary the array points at.
pub fn of_field(document: &Document, field: &Dictionary, trigger: Trigger) -> Site {
    let additional = document.get_key(field, "AA");
    match additional
        .as_dict()
        .and_then(|additional| additional.get(trigger.key()))
    {
        Some(entry) => of_action(document, entry, trigger),
        None => Site::Absent,
    }
}

/// One action entry, read as Tier 0 reads it.
fn of_action(document: &Document, entry: &Object, trigger: Trigger) -> Site {
    let Object::Dictionary(action) = document.resolve(entry) else {
        return Site::Absent;
    };
    if !matches!(document.get_key(&action, "S"), Object::Name(ref name) if name.as_bytes() == b"JavaScript")
    {
        return Site::Absent;
    }
    let field_script = format!("the field's {} script", trigger.noun());
    // Table 196's `/Next` makes the trigger perform more than this one action, and the shape
    // Tier 0 runs is one call and nothing else.
    let next = document.get_key(&action, "Next");
    let has_next = match &next {
        Object::Null => false,
        Object::Array(items) => !items.is_empty(),
        _ => true,
    };
    let Some(text) = script_text(document, &action) else {
        return Site::NotRun(format!(
            "{field_script} states no /JS this program can read as Table 221's text string or \
             text stream"
        ));
    };
    if has_next {
        return Site::NotRun(format!(
            "{field_script} is followed by a /Next action, and Tier 0 runs one call of the AF \
             library and nothing more (RFC 0008, ADR 1579)"
        ));
    }
    match Call::parse(&text) {
        Ok(call) => Site::Library(call),
        Err(NotOneCall::Unknown(name)) => Site::NotRun(format!(
            "{field_script} calls {name}, which is not one of the AF library's functions this \
             program defines"
        )),
        Err(NotOneCall::Shape) => Site::NotRun(format!(
            "{field_script} is a script this tier does not run: Tier 0 runs one AF library call \
             with literal arguments and constructs no ECMAScript engine (RFC 0008, ADR 1579)"
        )),
    }
}

/// Table 221's `/JS`, as text, bounded by the grammar's own limit.
fn script_text(document: &Document, action: &Dictionary) -> Option<String> {
    let bytes = match document.resolve(action.get("JS")?) {
        Object::String(bytes) => bytes.to_vec(),
        Object::Stream(stream) => document.decoded_stream_data(&stream)?.to_vec(),
        _ => return None,
    };
    if bytes.len() > super::call::MAX_CALL_BYTES.saturating_mul(2) {
        // Longer than any one call can be in either encoding: the grammar would refuse it, and
        // decoding it to say so is work a keystroke should not pay for.
        return Some(String::from("\u{0}"));
    }
    Some(pdf_syntax::text_string(&bytes))
}
