//! §12.7.4.1's field tree walked from a name to its widgets, and from a widget to its name.
//!
//! Every walk here goes to the end of the tree, which §12.7.4.1 asks — "\[a]n interactive PDF
//! processor shall not limit the range of inheritance for field dictionaries" — and stops where
//! the document makes the tree come back on itself: a `/Parent` chain is followed by
//! [`crate::inheritance::Ancestors`], which finds a cycle without a set, and a walk down `/Kids`
//! keeps the set of fields it has entered, since a field reached twice is reached by more than
//! one path and not only round a loop (ADR 2448).

use std::collections::{BTreeMap, BTreeSet};

use pdf_syntax::{Dictionary, Document, Name, Object, ObjectId};

use crate::inheritance::{Ending, ancestors, ancestors_of};

/// Every widget annotation in the document, keyed by its field's fully qualified name.
///
/// §12.7.4.1 defines the tree and §12.7.4.2 the name built over it:
///
/// > For a field with no parent, the partial and fully qualified names are the same. For a
/// > field that is the child of another field, the fully qualified name shall be formed by
/// > appending the child field's partial name to the parent's fully qualified name, separated
/// > by a PERIOD (2Eh)
///
/// The two subtleties are both in the clause. A field's widget annotation may be the field
/// dictionary *itself* — §12.5.6.19 says the two "may be merged into a single dictionary" —
/// so a leaf with no `/Kids` is its own widget. And a kid with no `/T` "shall not be
/// considered a field but simply a Widget annotation", so it belongs to its parent's name
/// rather than starting a new one.
///
/// **That second sentence is unconditional, and applies to more than a kid.** A dictionary reached
/// from `/Fields` with no `/T` of its own and no ancestor stating one has no fully qualified name
/// at all, so it is not a field — keying it under the *empty* name would make one field out of
/// annotations sharing nothing. Table 226 printed `/T` as `(Required)`, so the case read as a
/// malformed file; Errata Collection 3's Issue #28 strikes the requirement level and writes
/// *Optional*, which is the cell catching up with §12.7.4.2's own paragraph. What decides the case
/// is therefore the paragraph, and the paragraph says such a dictionary is a widget. It is left out
/// of this table, drawn by `crate::annotation` as before, and handed to no host as a control —
/// which is the path [`crate::form::fields`] already documents for a widget the field tree does not
/// reach.
///
/// **And the ancestry a name is built from is the `/Parent` chain, which is why a root entry is
/// asked for one.** §12.7.3 makes `/Fields` "an array of references to the document's root
/// fields (those with no ancestors in the field hierarchy)", so an entry stating a `/Parent` has
/// contradicted that — and Table 226's `/Parent` is the file's own statement of which field it
/// belongs to. Taking it is a **recovery**, argued in §12.7.4.2's row: the alternative is to read
/// a document's radio group as nameless when the producer listed the buttons in `/Fields` instead
/// of their parent. `examples/unnamed_field_census` counts the population.
///
/// **A root `/Fields` omits is recovered from the pages, which is the other recovery and ADR 1653's
/// choice.** A widget a page's `/Annots` lists, reached from no `/Fields` entry, whose `/Parent`
/// chain climbs to a dictionary — itself stating a `/T` or with one on the way — is a field the
/// file states twice, by the widget's `/Parent` and by the root's `/T`, and omits once, from the
/// array; the root is walked as an entry of `/Fields` would be, after the entries the array lists,
/// in page order. `examples/orphan_field_census` counts the population: 79 documents of the census
/// population, 4 482 roots. A dictionary no page lists and no entry reaches stays unread, so a
/// signature field on no page still needs `/Fields` (§12.8.1).
///
/// **This walks every page, so it is for what a person asked of the whole document** — a value
/// typed by name, a reset, an import, a save, a submission. What runs while a document opens, a
/// page is drawn or the pointer moves asks `field_table` with a narrower `Omitted`: one page's
/// widgets, or the calculation order's own entries (ADR 1653 section 4).
pub fn widgets_by_field_name(document: &Document) -> BTreeMap<String, Vec<ObjectId>> {
    field_table(document, Omitted::EveryPage)
}

/// [`widgets_by_field_name`], with the roots `/Fields` omits recovered from one page's `/Annots`
/// alone (ADR 1653): what a reader drawing or laying controls over one page asks, so that the
/// recovery adds no walk of the page tree to the path that shows page one. Every field the array
/// lists is here whatever page it is on; an omitted root is here where this page holds one of its
/// widgets, under the name [`widgets_by_field_name`] gives it, with every widget its walk reaches.
#[must_use]
pub fn widgets_on_page_by_field_name(
    document: &Document,
    page: &Dictionary,
) -> BTreeMap<String, Vec<ObjectId>> {
    field_table(document, Omitted::OnPage(page))
}

/// Where [`field_table`] looks for a root `/Fields` omits (ADR 1653).
#[derive(Clone, Copy)]
pub(crate) enum Omitted<'a> {
    /// Every page's `/Annots`, in page order.
    EveryPage,
    /// One page's.
    OnPage(&'a Dictionary),
    /// Table 224's `/CO`: the root above each field the calculation order names, which asks no
    /// page at all.
    Calculation,
    /// Nowhere: the fields `/Fields` lists.
    Listed,
}

/// The field table for page `page` of the document by its zero-based index: [`Omitted::OnPage`]
/// for that page, and [`Omitted::Listed`] where the document has no such page.
pub(crate) fn field_table_on_page(
    document: &Document,
    page: usize,
) -> BTreeMap<String, Vec<ObjectId>> {
    match crate::page::Pages::new(document).get(page) {
        Some(page) => field_table(document, Omitted::OnPage(&page.dict)),
        None => field_table(document, Omitted::Listed),
    }
}

/// The field table: `/Fields` walked, then the roots it omits recovered where `omitted` says.
pub(crate) fn field_table(
    document: &Document,
    omitted: Omitted<'_>,
) -> BTreeMap<String, Vec<ObjectId>> {
    let mut out = BTreeMap::new();
    let Ok(catalog) = document.catalog() else {
        return out;
    };
    let form = document.get_key(&catalog, "AcroForm");
    let Some(form) = form.as_dict() else {
        return out;
    };
    let fields = document.get_key(form, "Fields");
    let Some(fields) = fields.as_array().map(<[Object]>::to_vec) else {
        return out;
    };
    let mut seen = BTreeSet::new();
    for field in &fields {
        let inherited = ancestry(document, field);
        walk(document, field, inherited.as_deref(), &mut out, &mut seen);
    }
    match omitted {
        Omitted::Listed => {}
        Omitted::OnPage(page) => recover_omitted(document, page, &mut out, &mut seen),
        Omitted::Calculation => {
            let order = document.get_key(form, "CO");
            for entry in order.as_array().into_iter().flatten() {
                let Some(field) = entry.as_reference() else {
                    continue;
                };
                if seen.contains(&field) {
                    continue;
                }
                if let Some(root) = named_root(document, field)
                    && !seen.contains(&root)
                {
                    walk(
                        document,
                        &Object::Reference(root),
                        None,
                        &mut out,
                        &mut seen,
                    );
                }
            }
        }
        Omitted::EveryPage => {
            let pages = crate::page::Pages::new(document);
            let mut in_order: Vec<(usize, ObjectId)> = pages
                .indices()
                .into_iter()
                .map(|(id, index)| (index, id))
                .collect();
            in_order.sort_unstable();
            for (_, id) in in_order {
                let object = document.get(id);
                if let Some(page) = object.as_dict() {
                    recover_omitted(document, page, &mut out, &mut seen);
                }
            }
        }
    }
    out
}

/// Walks every root `/Fields` omits that one page's widgets climb to (ADR 1653).
fn recover_omitted(
    document: &Document,
    page: &Dictionary,
    out: &mut BTreeMap<String, Vec<ObjectId>>,
    seen: &mut BTreeSet<ObjectId>,
) {
    let annotations = document.get_key(page, "Annots");
    for entry in annotations.as_array().into_iter().flatten() {
        let Some(widget) = entry.as_reference() else {
            continue;
        };
        if seen.contains(&widget) {
            continue;
        }
        let Some(root) = omitted_root(document, widget) else {
            continue;
        };
        // A root already walked whose walk did not reach this widget — a `/Parent` the root's
        // `/Kids` does not answer — leaves the widget out, as the `/Fields` walk would.
        if !seen.contains(&root) {
            walk(document, &Object::Reference(root), None, out, seen);
        }
    }
}

/// The top of a widget annotation's `/Parent` chain, where something on the chain states a `/T`;
/// `None` for an annotation that is not a widget, and for a chain no `/T` names — §12.7.4.2's
/// "simply a Widget annotation".
fn omitted_root(document: &Document, widget: ObjectId) -> Option<ObjectId> {
    let object = document.get(widget);
    let dict = object.as_dict()?;
    if document
        .get_key(dict, "Subtype")
        .as_name()
        .map(Name::as_bytes)
        != Some(b"Widget")
    {
        return None;
    }
    named_root(document, widget)
}

/// The top of a dictionary's `/Parent` chain, where the dictionary or something above it states a
/// `/T`; `None` where nothing on the chain does.
///
/// The chain is followed by reference only, since a dictionary written inside its child has no
/// identity to be a root.
fn named_root(document: &Document, node: ObjectId) -> Option<ObjectId> {
    let mut named = false;
    let mut top = None;
    for (id, dict) in ancestors_of(document, node) {
        let Some(id) = id else {
            break;
        };
        named |= matches!(document.get_key(&dict, "T"), Object::String(_));
        top = Some(id);
    }
    top.filter(|_| named)
}

/// The qualified name a `/Fields` entry's `/Parent` chain gives it, if any.
///
/// `None` where the entry states no `/Parent`, or where nothing above it states a `/T` — the two
/// cases in which the ancestry contributes no name — and a third, a chain that returns to a field
/// it has passed, which is no hierarchy for §12.7.4.2 to name a field in.
fn ancestry(document: &Document, node: &Object) -> Option<String> {
    let id = node.as_reference()?;
    let object = document.get(id);
    let parent = document
        .get_key(object.as_dict()?, "Parent")
        .as_dict()?
        .clone();
    // Nearest ancestor first, so the name is assembled by walking back down at the end.
    let mut partials: Vec<String> = Vec::new();
    let mut walk = ancestors(document, parent);
    for (_, current) in walk.by_ref() {
        if let Object::String(bytes) = document.get_key(&current, "T") {
            partials.push(pdf_syntax::text_string(&bytes));
        }
    }
    if partials.is_empty() || walk.ending() == Some(Ending::Cycle) {
        return None;
    }
    partials.reverse();
    // §12.7.4.2's separator, applied between the ancestors exactly as it is between a parent and
    // a child: this is the parent's own fully qualified name.
    Some(partials.join("."))
}

/// Walks one node of the field tree, extending the qualified name as §12.7.4.2 states.
///
/// `prefix` is the fully qualified name of this node's ancestors, or `None` where no ancestor
/// states a `/T` — which is the state in which a node stating none itself is not a field.
///
/// Taken with a stack of its own rather than by recursion, so that a tree as deep as a document
/// makes it costs memory and never the thread's stack; children are pushed last first, so the
/// fields are entered in the order recursion would enter them.
fn walk(
    document: &Document,
    node: &Object,
    prefix: Option<&str>,
    out: &mut BTreeMap<String, Vec<ObjectId>>,
    seen: &mut BTreeSet<ObjectId>,
) {
    let mut pending = vec![(node.clone(), prefix.map(str::to_owned))];
    while let Some((node, prefix)) = pending.pop() {
        let Some(id) = node.as_reference() else {
            // A field has to be an indirect object for anything to name it; a direct dictionary
            // in `/Kids` has no identity a hide action could reach.
            continue;
        };
        if !seen.insert(id) {
            continue;
        }
        let resolved = document.get(id);
        let Some(dict) = resolved.as_dict() else {
            continue;
        };

        let name = qualified_name(document, dict, prefix.as_deref());
        let kids = document.get_key(dict, "Kids");
        match kids.as_array() {
            Some(kids) if !kids.is_empty() => {
                pending.extend(kids.iter().rev().map(|kid| (kid.clone(), name.clone())));
            }
            // A leaf: the field dictionary and its widget annotation merged into one — unless
            // §12.7.4.2 leaves it nameless, in which case it is "simply a Widget annotation".
            _ => {
                if let Some(name) = name {
                    out.entry(name).or_default().push(id);
                }
            }
        }
    }
}

/// Every widget in one field's subtree, which is what Table 241's *reference* form names.
///
/// ISO 32000-2 §12.7.6.3's Table 241 gives a `/Fields` element two spellings — "an indirect
/// reference to a field dictionary or (PDF 1.3) a text string representing the fully qualified
/// name of a field" — and Table 242 makes either of them reach further than the field itself:
///
/// > All descendants of the specified fields in the field hierarchy are reset as well.
///
/// **A field dictionary is not in general a widget annotation**, which is the whole reason this
/// exists. §12.7.4.1 merges the two into one dictionary only where a field has a single widget,
/// so a reference to a field with `/Kids` names an object no page draws — and a reader that took
/// the reference for a widget identity reset *nothing* for such a field and none of the
/// descendants of a non-terminal one. That is the shape §12.6.4.11's hide action does not have:
/// Table 214's `/T` names an annotation, and there the object identity in the file is the thing
/// to be hidden.
///
/// A leaf is kept whether or not §12.7.4.2 gives it a name, because the caller wants the
/// annotations to reset rather than the fields to list, and a `/Kids` entry with no `/T` of its
/// own is exactly the widget of the field that was named.
///
/// Guarded against a cycle for [`walk`]'s reason, and walked with a stack of its own for the same
/// reason: `/Kids` is the document's to write and §12.7.4.1 states no acyclicity rule.
pub(crate) fn widgets_under(document: &Document, field: ObjectId) -> Vec<ObjectId> {
    let mut out = Vec::new();
    let mut seen = BTreeSet::new();
    let mut pending = vec![field];
    while let Some(node) = pending.pop() {
        if !seen.insert(node) {
            continue;
        }
        let object = document.get(node);
        let kids = object
            .as_dict()
            .map_or(Object::Null, |dict| document.get_key(dict, "Kids"));
        match kids.as_array() {
            Some(kids) if !kids.is_empty() => {
                pending.extend(kids.iter().rev().filter_map(Object::as_reference));
            }
            // A leaf: the field dictionary and its widget annotation merged into one, or a kid of
            // the referenced field that is simply a widget.
            _ => out.push(node),
        }
    }
    out
}

/// This node's fully qualified name: the prefix, and its own `/T` if it has one.
///
/// `None` where neither this dictionary nor any ancestor states a `/T`, which §12.7.4.2 makes the
/// test for a dictionary that is "simply a Widget annotation" rather than a field. The *entry* is
/// what the clause conditions on and not the string it holds, so a `/T` of zero length still
/// names a field — a degenerate name is a name, and two fields sharing one are the case the same
/// clause makes the writer answer for.
fn qualified_name(document: &Document, dict: &Dictionary, prefix: Option<&str>) -> Option<String> {
    let Object::String(bytes) = document.get_key(dict, "T") else {
        return prefix.map(str::to_owned);
    };
    let partial = pdf_syntax::text_string(&bytes);
    Some(match prefix {
        Some(prefix) if !prefix.is_empty() => format!("{prefix}.{partial}"),
        _ => partial,
    })
}
