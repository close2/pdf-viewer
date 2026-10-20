//! ISO 19005-2 section 6.7.4: a `/Lang` that is not a language identifier, restated.
//!
//! The subclause's one requirement is that a `/Lang` the catalog, a structure element or a
//! marked-content property list does state be a language identifier as ISO 32000-2 §14.9.2.2
//! defines one:
//!
//! > A language identifier shall either be the empty text string, to indicate that the language
//! > is unknown, or a Language-Tag as defined in BCP 47.
//!
//! **What is written in its place is one of the two values that sentence admits.** Correcting a
//! malformed tag — `en_US` to `en-US`, `EN-GB` with a stray byte after it to `EN-GB` — is this
//! converter deciding what the producer meant, which `doc/questions/A48` forbids. So the loss
//! answer writes the empty text string, §14.9.2.2's own statement that the language is unknown,
//! and the `supply` answer writes the operator's tag. Neither *removes* the entry, and §14.9.2.3 is
//! the reason: "If a structure element does not have a Lang entry, the element shall inherit its
//! language from any parent element that has one", so a removal would assert the parent's
//! language over text whose producer said something else. `doc/adr/2066` is the argument.
//!
//! **Three places, three constructions.** The catalog and a structure element are dictionaries
//! this verb already rewrites by object. A property list named through the resources' `/Properties`
//! (§14.6.2) is one too, where it or the `/Properties` dictionary holding it is an object. A
//! property list written into a `BDC` or `DP` operator's operands is bytes in a producer's content
//! stream, and the value is respelled in place: the lexer finds the token, its span is replaced
//! with the new string, and every other byte of the stream crosses as it was — no operator is
//! added, removed or moved, and no mark changes, because §14.6.2 makes a property list information
//! *about* content rather than content (`doc/adr/2067`).

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use pdf_archive::survey::Survey;
use pdf_model::Pages;
use pdf_syntax::object::{Dictionary, Name, Object, ObjectId};
use pdf_syntax::{Document, Lexer, Token};

use super::decision::Because;
use super::hexadecimal::{self, Completed};

/// The catalog's half of section 6.7.4.
pub(super) const CATALOG_SITE: &str = "logical-structure/catalog-language-identifier";

/// The structure elements' and property lists' half of section 6.7.4.
pub(super) const MARKED_SITE: &str =
    "logical-structure/element-and-property-list-language-identifiers";

/// How many structure tree nodes the descent visits before it stops, a bound rather than a
/// reading (`CLAUDE.md` principle 3): a tree past it is refused, not half-restated.
const MAX_NODES: usize = 1 << 20;

/// How many operand tokens one operator may collect before they are dropped unread.
///
/// A bound on what an adversarial stream can make this walk hold. A property list longer than it
/// is not examined, so a malformed `/Lang` inside one survives, and the third stage's validation
/// of the output refuses the document by name rather than this walk passing it.
const MAX_OPERANDS: usize = 1 << 16;

/// How many characters of a producer's value the report prints before cutting it.
const STATED_SHOWN: usize = 64;

/// Why a site that failed has nothing this walk can restate.
const NOTHING_TO_RESTATE: &str = "this document states a Lang entry that is not a language \
     identifier, and no catalog, structure element or property list this conversion reads states \
     one - so the entry is somewhere this walk does not reach, and writing a value elsewhere would \
     fix nothing";

/// Why a structure element written directly inside its parent is not restated.
const DIRECT_ELEMENT: &str = "a structure element written directly inside its parent's /K \
     states a Lang entry that is not a language identifier. This conversion restates an element \
     it can reach as an object of its own, and a direct one would need its parent rewritten in \
     turn; that construction is not built";

/// Why a property list written directly into a resource dictionary is not restated.
const DIRECT_PROPERTY_LIST: &str = "a property list written directly into a resource \
     dictionary's /Properties states a Lang entry that is not a language identifier. This \
     conversion restates a property list that is an object of its own or sits in a /Properties \
     dictionary that is one; one written inside the resources themselves is not built";

/// Why a content stream whose bytes cannot be written back is not restated.
const STREAM_WILL_NOT_RE_ENCODE: &str = "a content stream whose marked-content property list \
     states a Lang entry that is not a language identifier cannot be written back: it will not \
     re-encode, or its /DecodeParms names another object that discarding the parameters would \
     orphan";

/// Why a structure tree past [`MAX_NODES`] is refused.
const TREE_PAST_THE_BOUND: &str = "the structure tree holds more nodes than this conversion \
     walks, so a Lang entry past the bound could not be restated and the tree is not half-edited";

/// The two values the rewrite writes, one per site.
#[derive(Debug, Clone, Copy)]
pub(super) struct Values<'a> {
    /// What the catalog's `/Lang` becomes: the operator's tag, or the empty text string.
    pub(super) catalog: &'a str,
    /// What a structure element's or a property list's becomes.
    pub(super) marked: &'a str,
}

/// Which of the two sites a restated entry belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Class {
    /// The catalog's `/Lang`.
    Catalog,
    /// A structure element's or a property list's.
    Marked,
}

/// One `/Lang` this conversion restated, as the report names it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestatedLanguage {
    /// Where it stood, in words a reader can find it by.
    pub place: String,
    /// What the producer wrote, cut down to something a report can print.
    pub stated: String,
    /// What the output states instead; empty is §14.9.2.2's *the language is unknown*.
    pub now: String,
    /// Which site it answered.
    pub(super) class: Class,
}

impl RestatedLanguage {
    /// One restated entry as JSON.
    pub(super) fn to_json(&self) -> crate::json::Value {
        use crate::json::Value;
        Value::Object(vec![
            ("place".to_owned(), Value::text(self.place.clone())),
            ("stated".to_owned(), Value::text(self.stated.clone())),
            ("now".to_owned(), Value::text(self.now.clone())),
        ])
    }
}

/// Every `/Lang` that is not a language identifier, and what each one's object becomes.
#[derive(Debug)]
pub(super) struct Restated {
    /// The catalog, where its `/Lang` is restated.
    pub(super) catalog: Option<ObjectId>,
    /// Per dictionary object, the entries whose `/Lang` is restated: `None` for the object's own,
    /// `Some(name)` for the direct property list it files under that name.
    pub(super) dictionaries: BTreeMap<ObjectId, Vec<Option<Name>>>,
    /// The content streams whose inline property lists are respelled, whole.
    pub(super) streams: BTreeMap<ObjectId, Object>,
    /// The content streams [`hexadecimal`] had already repaired, whose repair these carry too.
    pub(super) carries_hexadecimal: BTreeSet<ObjectId>,
    /// What the catalog's `/Lang` becomes.
    pub(super) catalog_value: Object,
    /// What every other restated `/Lang` becomes.
    pub(super) marked_value: Object,
    /// Each restated entry, for the report.
    pub(super) rows: Vec<RestatedLanguage>,
    /// Why the catalog's half cannot be carried out, where it cannot.
    pub(super) catalog_obstacle: Option<Because>,
    /// Why the other half cannot be carried out, where it cannot.
    pub(super) marked_obstacle: Option<Because>,
}

impl Restated {
    /// How many entries of one class were restated.
    pub(super) fn places(&self, class: Class) -> usize {
        self.rows.iter().filter(|row| row.class == class).count()
    }
}

/// Whether a stated `/Lang` is a language identifier §14.9.2.2 defines.
///
/// The same reading `pdf_archive`'s two rows take, through the one BCP 47 grammar this project
/// has: a text string that is empty or well formed. Anything that is not a text string at all is
/// not one either.
fn is_identifier(stated: &Object) -> bool {
    match stated {
        Object::String(bytes) => {
            let text = pdf_syntax::text_string(bytes);
            text.is_empty() || pdf_model::structure::well_formed_language_tag(&text)
        }
        _ => false,
    }
}

/// What the report prints of a stated value.
fn shown(stated: &Object) -> String {
    let text = match stated {
        Object::String(bytes) => pdf_syntax::text_string(bytes),
        other => format!("a {}", other.type_name()),
    };
    text.chars().take(STATED_SHOWN).collect()
}

/// Restates every malformed `/Lang` the two sites name, with the values given.
///
/// `survey` is the walk that found the content streams a resource dictionary reaches, and
/// `repaired` is [`hexadecimal`]'s repair where one was prepared: a stream both rewrite starts
/// from the repaired bytes, so the one stream written carries both edits.
pub(super) fn restate(
    document: &Document,
    survey: Option<&Survey>,
    repaired: Option<&Completed>,
    values: Values<'_>,
) -> Restated {
    let mut out = Restated {
        catalog_value: Object::String(Arc::from(values.catalog.as_bytes())),
        marked_value: Object::String(Arc::from(values.marked.as_bytes())),
        catalog: None,
        dictionaries: BTreeMap::new(),
        streams: BTreeMap::new(),
        carries_hexadecimal: BTreeSet::new(),
        rows: Vec::new(),
        catalog_obstacle: None,
        marked_obstacle: None,
    };
    restate_the_catalog(document, values, &mut out);
    if let Err(because) = restate_the_elements(document, values, &mut out) {
        out.marked_obstacle = Some(because);
    }
    if let Err(because) = restate_the_property_lists(document, survey, repaired, values, &mut out) {
        out.marked_obstacle.get_or_insert(because);
    }
    if out.catalog.is_none() {
        out.catalog_obstacle = Some(Because::NotBuiltYet(NOTHING_TO_RESTATE));
    }
    if out.places(Class::Marked) == 0 {
        out.marked_obstacle
            .get_or_insert(Because::NotBuiltYet(NOTHING_TO_RESTATE));
    }
    out
}

/// The catalog's `/Lang`, where it is present and not an identifier.
fn restate_the_catalog(document: &Document, values: Values<'_>, out: &mut Restated) {
    let Some(at) = document
        .trailer()
        .get("Root")
        .and_then(Object::as_reference)
    else {
        return;
    };
    let Some(catalog) = document.get(at).as_dict().cloned() else {
        return;
    };
    let stated = document.get_key(&catalog, "Lang");
    if stated.is_null() || is_identifier(&stated) {
        return;
    }
    out.catalog = Some(at);
    out.rows.push(RestatedLanguage {
        place: "the document catalog".to_owned(),
        stated: shown(&stated),
        now: values.catalog.to_owned(),
        class: Class::Catalog,
    });
}

/// Every structure element the tree reaches whose `/Lang` is not an identifier.
///
/// Its own descent rather than `pdf_model::structure::Tree::walk`, because that reading answers
/// dictionaries and this one needs the object each was read from: the edit is made by object.
fn restate_the_elements(
    document: &Document,
    values: Values<'_>,
    out: &mut Restated,
) -> Result<(), Because> {
    let Ok(catalog) = document.catalog() else {
        return Ok(());
    };
    let root = document.get_key(&catalog, "StructTreeRoot");
    let Some(root) = root.as_dict() else {
        return Ok(());
    };
    let mut pending: Vec<Object> = root.get("K").cloned().into_iter().collect();
    let mut seen: BTreeSet<ObjectId> = BTreeSet::new();
    let mut visited = 0_usize;
    let mut obstacle = None;
    while let Some(node) = pending.pop() {
        visited = visited.saturating_add(1);
        if visited > MAX_NODES {
            return Err(Because::NotBuiltYet(TREE_PAST_THE_BOUND));
        }
        let (at, dict) = match node {
            Object::Reference(id) => {
                if !seen.insert(id) {
                    continue;
                }
                match document.get(id) {
                    Object::Dictionary(dict) => (Some(id), dict),
                    Object::Array(items) => {
                        pending.extend(items);
                        continue;
                    }
                    _ => continue,
                }
            }
            Object::Dictionary(dict) => (None, dict),
            Object::Array(items) => {
                pending.extend(items);
                continue;
            }
            _ => continue,
        };
        // §14.7.5's marked-content and object references are content items, not elements, and
        // Table 355 makes `/S` the one entry every element states.
        let is_reference = ["MCR", "OBJR"].iter().any(|kind| {
            document
                .get_key(&dict, "Type")
                .as_name()
                .is_some_and(|name| name.as_bytes() == kind.as_bytes())
        });
        if is_reference || dict.get("S").is_none() {
            continue;
        }
        let stated = document.get_key(&dict, "Lang");
        if !stated.is_null() && !is_identifier(&stated) {
            match at {
                Some(id) => {
                    out.dictionaries.entry(id).or_default().push(None);
                    out.rows.push(RestatedLanguage {
                        place: format!(
                            "the structure element in object {} {}",
                            id.number, id.generation
                        ),
                        stated: shown(&stated),
                        now: values.marked.to_owned(),
                        class: Class::Marked,
                    });
                }
                None => obstacle = Some(Because::NotBuiltYet(DIRECT_ELEMENT)),
            }
        }
        if let Some(children) = dict.get("K") {
            pending.push(children.clone());
        }
    }
    obstacle.map_or(Ok(()), Err)
}

/// Every property list a content stream can reach whose `/Lang` is not an identifier: the ones
/// named through the resources, and the ones written into the operands.
fn restate_the_property_lists(
    document: &Document,
    survey: Option<&Survey>,
    repaired: Option<&Completed>,
    values: Values<'_>,
    out: &mut Restated,
) -> Result<(), Because> {
    let pages = Pages::new(document);
    let mut obstacle = None;
    let mut named_seen: BTreeSet<ObjectId> = BTreeSet::new();
    for (id, resources) in hexadecimal::streams(document, &pages, survey) {
        if let Err(because) = restate_named(document, &resources, values, &mut named_seen, out) {
            obstacle = Some(because);
        }
        let carried = repaired.and_then(|completed| completed.at.get(&id));
        let object = carried.cloned().unwrap_or_else(|| document.get(id));
        let Some(stream) = object.as_stream() else {
            continue;
        };
        let Some(data) = document.decoded_stream_data(stream) else {
            continue;
        };
        let spans = malformed_inline(document, &data, &resources);
        if spans.is_empty() {
            continue;
        }
        let replacement = literal(values.marked.as_bytes());
        let mut written = Vec::with_capacity(data.len());
        let mut from = 0;
        for span in &spans {
            written.extend_from_slice(data.get(from..span.start).unwrap_or_default());
            written.extend_from_slice(&replacement);
            from = span.end;
        }
        written.extend_from_slice(data.get(from..).unwrap_or_default());
        let Some(rewritten) = hexadecimal::repaired(stream, &written) else {
            obstacle = Some(Because::NotBuiltYet(STREAM_WILL_NOT_RE_ENCODE));
            continue;
        };
        for span in spans {
            out.rows.push(RestatedLanguage {
                place: format!(
                    "a marked-content property list in the content stream in object {} {}",
                    id.number, id.generation
                ),
                stated: span.stated,
                now: values.marked.to_owned(),
                class: Class::Marked,
            });
        }
        if carried.is_some() {
            out.carries_hexadecimal.insert(id);
        }
        out.streams.insert(id, rewritten);
    }
    obstacle.map_or(Ok(()), Err)
}

/// The property lists one resource dictionary's `/Properties` names.
fn restate_named(
    document: &Document,
    resources: &Dictionary,
    values: Values<'_>,
    seen: &mut BTreeSet<ObjectId>,
    out: &mut Restated,
) -> Result<(), Because> {
    let (holder, lists) = match resources.get("Properties") {
        Some(Object::Reference(id)) => match document.get(*id) {
            Object::Dictionary(lists) => (Some(*id), lists),
            _ => return Ok(()),
        },
        Some(Object::Dictionary(lists)) => (None, lists.clone()),
        _ => return Ok(()),
    };
    let mut obstacle = None;
    for (name, entry) in lists.iter() {
        let (list, at) = match entry {
            Object::Reference(id) => match document.get(*id) {
                Object::Dictionary(list) => (list, Some((*id, None))),
                _ => continue,
            },
            Object::Dictionary(list) => (list.clone(), holder.map(|id| (id, Some(name.clone())))),
            _ => continue,
        };
        let stated = document.get_key(&list, "Lang");
        if stated.is_null() || is_identifier(&stated) {
            continue;
        }
        let Some((id, edit)) = at else {
            obstacle = Some(Because::NotBuiltYet(DIRECT_PROPERTY_LIST));
            continue;
        };
        // One property list may be named by many resource dictionaries; it is one entry.
        let key = edit.is_none().then_some(id);
        if key.is_some_and(|id| !seen.insert(id)) {
            continue;
        }
        let edits = out.dictionaries.entry(id).or_default();
        if edits.contains(&edit) {
            continue;
        }
        edits.push(edit);
        out.rows.push(RestatedLanguage {
            place: format!(
                "the property list {} named through /Properties, in object {} {}",
                String::from_utf8_lossy(name.as_bytes()),
                id.number,
                id.generation
            ),
            stated: shown(&stated),
            now: values.marked.to_owned(),
            class: Class::Marked,
        });
    }
    obstacle.map_or(Ok(()), Err)
}

/// One `/Lang` value written inside a content stream, by its span in the decoded bytes.
struct Span {
    /// The first byte of the value token.
    start: usize,
    /// One past its last byte.
    end: usize,
    /// What the producer wrote, for the report.
    stated: String,
}

/// One operand token, with its span, as much of it as [`malformed_inline`] needs.
enum Operand {
    /// `<<`
    DictOpen,
    /// `[`
    ArrayOpen,
    /// `>>` or `]`
    Close,
    /// A name, which a dictionary's key always is.
    Name(Vec<u8>),
    /// A string, which a language identifier is.
    String(Vec<u8>),
    /// Any other operand.
    Other,
}

/// Every inline property list's `/Lang` in one decoded content stream that is not an identifier.
///
/// **The lexer decides what is a token**, as [`hexadecimal`]'s walk lets it decide which `<` opens
/// a string, and an inline image's data is skipped by `pdf_model::inline_image::scan` for the same
/// reason: its bytes are samples, not syntax. A property list is §14.6.2's dictionary operand of
/// `BDC` or `DP`, the second of the two; its `/Lang` is read at the dictionary's own level only.
fn malformed_inline(document: &Document, data: &[u8], resources: &Dictionary) -> Vec<Span> {
    let mut lexer = Lexer::new(data);
    let mut operands: Vec<(Operand, usize, usize)> = Vec::new();
    let mut out = Vec::new();
    loop {
        lexer.skip_whitespace();
        let start = lexer.position();
        let Some(token) = lexer.next_token() else {
            break;
        };
        let end = lexer.position();
        let operand = match token {
            Token::Keyword(b"BI") => {
                let scan = pdf_model::inline_image::scan(
                    document,
                    data,
                    lexer.position(),
                    resources,
                    true,
                );
                lexer.seek(scan.resume);
                operands.clear();
                continue;
            }
            Token::Keyword(b"true" | b"false" | b"null") | Token::Integer(_) | Token::Real(_) => {
                Operand::Other
            }
            Token::Keyword(b"BDC" | b"DP") => {
                out.extend(lang_of_property_list(&operands));
                operands.clear();
                continue;
            }
            Token::Keyword(_) => {
                operands.clear();
                continue;
            }
            Token::DictOpen => Operand::DictOpen,
            Token::ArrayOpen => Operand::ArrayOpen,
            Token::DictClose | Token::ArrayClose => Operand::Close,
            Token::Name(name) => Operand::Name(name),
            Token::String(bytes) => Operand::String(bytes),
        };
        if operands.len() >= MAX_OPERANDS {
            operands.clear();
        }
        operands.push((operand, start, end));
    }
    out
}

/// The malformed `/Lang` of the property list a `BDC` or `DP` was handed inline, if it has one.
fn lang_of_property_list(operands: &[(Operand, usize, usize)]) -> Option<Span> {
    // §14.6.2: the tag, then the property list; a name in second place is the resource form.
    let (Some((Operand::Name(_), _, _)), Some((Operand::DictOpen, _, _))) =
        (operands.first(), operands.get(1))
    else {
        return None;
    };
    let mut index = 2_usize;
    loop {
        // A key at the dictionary's own level, or its close.
        let key = match operands.get(index)? {
            (Operand::Name(key), _, _) => key.clone(),
            _ => return None,
        };
        index = index.saturating_add(1);
        let (value, start, _) = operands.get(index)?;
        let last = match value {
            Operand::DictOpen | Operand::ArrayOpen => matching_close(operands, index)?,
            Operand::Close => return None,
            _ => index,
        };
        let (_, _, end) = operands.get(last)?;
        if key == b"Lang" {
            let stated = match value {
                Operand::String(bytes) => {
                    let text = pdf_syntax::text_string(bytes);
                    if text.is_empty() || pdf_model::structure::well_formed_language_tag(&text) {
                        return None;
                    }
                    text.chars().take(STATED_SHOWN).collect()
                }
                Operand::Name(_) => "a name".to_owned(),
                _ => "a value that is not a string".to_owned(),
            };
            return Some(Span {
                start: *start,
                end: *end,
                stated,
            });
        }
        index = last.saturating_add(1);
    }
}

/// The index of the token closing the container opened at `open`.
fn matching_close(operands: &[(Operand, usize, usize)], open: usize) -> Option<usize> {
    let mut depth = 0_usize;
    for (index, (operand, _, _)) in operands.iter().enumerate().skip(open) {
        match operand {
            Operand::DictOpen | Operand::ArrayOpen => depth = depth.saturating_add(1),
            Operand::Close => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

/// A literal string holding `text`, its three delimiters escaped as §7.3.4.2's Table 3 says.
fn literal(text: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(text.len().saturating_add(2));
    out.push(b'(');
    for byte in text {
        if matches!(byte, b'(' | b')' | b'\\') {
            out.push(b'\\');
        }
        out.push(*byte);
    }
    out.push(b')');
    out
}

/// Applies the restated entries to one dictionary object; whether it changed.
pub(super) fn apply(restated: &Restated, id: ObjectId, out: &mut Dictionary) -> bool {
    let Some(edits) = restated.dictionaries.get(&id) else {
        return false;
    };
    let lang = || Name::new(&b"Lang"[..]);
    let mut changed = false;
    for edit in edits {
        match edit {
            None => {
                out.insert(lang(), restated.marked_value.clone());
                changed = true;
            }
            Some(name) => {
                if let Some(Object::Dictionary(list)) = out.get_by_name(name) {
                    let mut list = list.clone();
                    list.insert(lang(), restated.marked_value.clone());
                    out.insert(name.clone(), Object::Dictionary(list));
                    changed = true;
                }
            }
        }
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    fn operands_of(data: &[u8]) -> Vec<Span> {
        let document = Document::open(minimal_pdf()).expect("the fixture is a well-formed file");
        malformed_inline(&document, data, &Dictionary::new())
    }

    fn minimal_pdf() -> Vec<u8> {
        b"%PDF-1.7\n1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n2 0 obj\n<< /Type /Pages /Kids [] /Count 0 >>\nendobj\ntrailer\n<< /Root 1 0 R >>\n%%EOF\n".to_vec()
    }

    /// ISO 32000-2 §14.9.2.3's EXAMPLE 1 is a property list whose `/Lang` is well formed, so the
    /// walk names nothing in it; the same list with a tag BCP 47 does not admit is named, by the
    /// span of the value alone.
    #[test]
    fn the_standard_s_example_passes_and_its_malformed_twin_is_named_by_span() {
        let example = b"BT (See you later, or as Arnold would say, ) Tj /Span <</Lang (es-MX)>> BDC (Hasta la vista.) Tj EMC ET";
        assert!(operands_of(example).is_empty());
        let twin = b"BT /Span <</Lang (es_MX) /ActualText (x)>> BDC (Hasta la vista.) Tj EMC ET";
        let spans = operands_of(twin);
        assert_eq!(spans.len(), 1);
        let span = spans.first().expect("one span");
        assert_eq!(twin.get(span.start..span.end), Some(&b"(es_MX)"[..]));
        assert_eq!(span.stated, "es_MX");
    }

    /// §14.9.2.2's empty text string is the identifier for an unknown language, and a `/Lang`
    /// nested one dictionary down is not the property list's own.
    #[test]
    fn the_empty_identifier_and_a_nested_lang_are_left_alone() {
        assert!(operands_of(b"/Span <</Lang ()>> BDC EMC").is_empty());
        assert!(operands_of(b"/Span <</Inner <</Lang (es_MX)>> >> BDC EMC").is_empty());
        assert!(operands_of(b"/Span /Named BDC EMC").is_empty());
    }

    /// A name is not a text string, so it is not a language identifier either.
    #[test]
    fn a_name_where_the_identifier_belongs_is_named() {
        let data = b"/Span <</Lang /en>> DP";
        let spans = operands_of(data);
        let span = spans.first().expect("one span");
        assert_eq!(data.get(span.start..span.end), Some(&b"/en"[..]));
    }

    #[test]
    fn a_literal_escapes_its_delimiters() {
        assert_eq!(literal(b"a(b)\\"), b"(a\\(b\\)\\\\)".to_vec());
        assert_eq!(literal(b""), b"()".to_vec());
    }
}
