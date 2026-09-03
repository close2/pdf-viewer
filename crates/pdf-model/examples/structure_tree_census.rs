//! What §14.7's logical structure a corpus actually states, and where it is stated.
//!
//! ISO 32000-2 §14.7.2 puts the whole construct behind one catalog entry — "[a]t the root of the
//! hierarchy shall be a dictionary object called the structure tree root, located by means of the
//! `StructTreeRoot` entry in the document catalog dictionary" — and §14.7.5.4 puts the pointer from
//! the content back to it on the objects that hold the content:
//!
//! > Depending on the type of content item, this entry may appear in the page object of a page
//! > containing marked-content sequences, in the stream dictionary of a form or image XObject, or
//! > in an annotation dictionary.
//!
//! Those three homes are what a transform that carries the tree has to renumber, and they are not
//! equally easy: a page object and an annotation dictionary are dictionaries a writer can rebuild,
//! while a form or image `XObject` is a *stream*, whose bytes cross untouched. So this census
//! counts each home separately, and counts the entries of Table 354 that decide whether two
//! documents' trees can be merged at all.
//!
//! It reads with `pdf_syntax` alone rather than through [`pdf_model::structure`], because a census
//! whose predicate is the code under test measures the code rather than the corpus
//! (`doc/HANDOVER.md` trap 8).
//!
//! ```sh
//! cargo run --release -p pdf-model --example structure_tree_census -- <file.pdf>…
//! ```

#![expect(
    clippy::print_stdout,
    reason = "an example whose entire output is a measurement"
)]

use std::collections::{BTreeMap, BTreeSet};

use pdf_syntax::{Dictionary, Document, Object, ObjectId};

/// The bound `pdf_model::page` walks the page tree under.
const MAX_TREE_DEPTH: usize = 64;
/// As above: a `/Kids` cycle is what stops the page walk, not the tree's size.
const MAX_NODES_VISITED: usize = 100_000;
/// How deep the structure hierarchy is walked before the walk gives up on a cycle.
const MAX_STRUCTURE_DEPTH: usize = 128;

/// What one document says about the question.
#[derive(Default)]
struct Finding {
    /// Whether the catalog states a `/StructTreeRoot` at all.
    tagged: bool,
    /// Table 353's `/MarkInfo` `/Marked`, where the catalog states one.
    marked: Option<bool>,
    /// Pages, and how many state Table 359's `/StructParents`.
    pages: usize,
    /// Pages stating `/StructParents`.
    pages_keyed: usize,
    /// Whether page 1 states `/StructParents` — the page `split` and `merge` carry.
    first_page_keyed: bool,
    /// Annotations, and how many state Table 359's `/StructParent`.
    annotations: usize,
    /// Annotations stating `/StructParent`.
    annotations_keyed: usize,
    /// Streams stating `/StructParent`: §14.7.5.4's third home, and the one a writer that
    /// carries stream bytes untouched cannot rebuild without rebuilding the stream.
    streams_keyed: usize,
    /// Structure elements reached from the root.
    elements: usize,
    /// Elements stating Table 355's `/Pg`.
    elements_with_page: usize,
    /// Elements with a marked-content child (an integer or an `/MCR`) and no `/Pg` of their own
    /// and none on an ancestor — a content item whose page cannot be determined at all.
    elements_without_any_page: usize,
    /// Elements with a marked-content child and no `/Pg` of their own, whose nearest ancestor
    /// with one supplies it: what Table 355 does not license and real files do.
    elements_inheriting_page: usize,
    /// Elements stating Table 355's `/ID`, which §14.7.2's `/IDTree` maps.
    elements_with_id: usize,
    /// Which of Table 354's optional entries the root states.
    root_entries: BTreeSet<&'static str>,
    /// Table 354's `/RoleMap`, as pairs, so that a collision across documents can be counted.
    role_map: BTreeMap<Vec<u8>, Vec<u8>>,
    /// Table 354's `/ClassMap` keys.
    class_map: BTreeSet<Vec<u8>>,
    /// Every `/ID` the `/IDTree` maps, for the same reason.
    ids: BTreeSet<Vec<u8>>,
    /// How many keys the `/ParentTree` holds.
    parent_tree_keys: usize,
    /// The largest `/StructParents` or `/StructParent` any object states.
    largest_key: Option<i64>,
}

/// A dictionary's `/K` children, **unresolved**, so that a child keeps its object identity.
///
/// Table 354 and Table 355 both make `/K` "either a dictionary … or an array of such
/// dictionaries", and either the entry or its items may be indirect.
fn children_of(document: &Document, dict: &Dictionary) -> Vec<Object> {
    let Some(value) = dict.get("K") else {
        return Vec::new();
    };
    match value {
        Object::Array(items) => items.clone(),
        Object::Reference(_) => match document.resolve(value) {
            Object::Array(items) => items,
            _ => vec![value.clone()],
        },
        other => vec![other.clone()],
    }
}

/// Every page dictionary of a document, in tree order.
fn pages(document: &Document) -> Vec<Dictionary> {
    let catalog = document.catalog().unwrap_or_default();
    let Some(root) = catalog
        .get("Pages")
        .map(|value| document.resolve(value))
        .and_then(|value| match value {
            Object::Dictionary(dict) => Some(dict),
            _ => None,
        })
    else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut visited = 0_usize;
    walk_pages(document, &root, 0, &mut visited, &mut out);
    out
}

/// One level of the page-tree walk.
fn walk_pages(
    document: &Document,
    node: &Dictionary,
    depth: usize,
    visited: &mut usize,
    out: &mut Vec<Dictionary>,
) {
    if depth > MAX_TREE_DEPTH || *visited >= MAX_NODES_VISITED {
        return;
    }
    *visited = visited.saturating_add(1);
    let Some(Object::Array(kids)) = node.get("Kids").map(|value| document.resolve(value)) else {
        out.push(node.clone());
        return;
    };
    for kid in &kids {
        if let Object::Dictionary(dict) = document.resolve(kid) {
            walk_pages(document, &dict, depth.saturating_add(1), visited, out);
        }
    }
}

/// Whether this element's `/K` holds a content item that names a page rather than an object.
///
/// §14.7.5.2's two forms: "[a]n integer that specifies the marked-content identifier", and a
/// marked-content reference dictionary whose `/Type` is `/MCR`.
fn has_marked_content_child(document: &Document, element: &Dictionary) -> bool {
    children_of(document, element)
        .iter()
        .any(|item| match document.resolve(item) {
            Object::Integer(_) => true,
            Object::Dictionary(dict) => dict
                .get("Type")
                .and_then(Object::as_name)
                .is_some_and(|name| name.as_bytes() == b"MCR"),
            _ => false,
        })
}

/// Walks the structure hierarchy from one element, counting what the finding wants.
fn walk_elements(
    document: &Document,
    element: &Dictionary,
    ancestor_has_page: bool,
    depth: usize,
    seen: &mut BTreeSet<ObjectId>,
    finding: &mut Finding,
) {
    if depth > MAX_STRUCTURE_DEPTH {
        return;
    }
    finding.elements = finding.elements.saturating_add(1);
    let own_page = element.get("Pg").is_some();
    if own_page {
        finding.elements_with_page = finding.elements_with_page.saturating_add(1);
    }
    if element.get("ID").is_some() {
        finding.elements_with_id = finding.elements_with_id.saturating_add(1);
    }
    if !own_page && has_marked_content_child(document, element) {
        if ancestor_has_page {
            finding.elements_inheriting_page = finding.elements_inheriting_page.saturating_add(1);
        } else {
            finding.elements_without_any_page = finding.elements_without_any_page.saturating_add(1);
        }
    }
    for item in &children_of(document, element) {
        // Only a child that is itself a structure element is descended into: Table 355 makes a
        // dictionary with no `/Type` a structure element, and `/MCR` and `/OBJR` the two content
        // items, which §14.7.5.1.1 makes leaves.
        if let Some(id) = item.as_reference()
            && !seen.insert(id)
        {
            continue;
        }
        let Object::Dictionary(dict) = document.resolve(item) else {
            continue;
        };
        let kind = dict.get("Type").and_then(Object::as_name);
        if kind.is_some_and(|name| {
            let bytes = name.as_bytes();
            bytes == b"MCR" || bytes == b"OBJR"
        }) {
            continue;
        }
        walk_elements(
            document,
            &dict,
            ancestor_has_page || own_page,
            depth.saturating_add(1),
            seen,
            finding,
        );
    }
}

/// Every name-object key and name-object value of a dictionary, as bytes.
fn name_pairs_of(document: &Document, value: Option<&Object>) -> BTreeMap<Vec<u8>, Vec<u8>> {
    let mut out = BTreeMap::new();
    let Some(Object::Dictionary(dict)) = value.map(|value| document.resolve(value)) else {
        return out;
    };
    for (key, entry) in dict.iter() {
        if let Some(name) = document.resolve(entry).as_name() {
            out.insert(key.as_bytes().to_vec(), name.as_bytes().to_vec());
        }
    }
    out
}

/// One document's answer.
fn examine(document: &Document) -> Finding {
    let mut finding = Finding::default();
    let catalog = document.catalog().unwrap_or_default();
    finding.marked = match catalog.get("MarkInfo").map(|value| document.resolve(value)) {
        Some(Object::Dictionary(dict)) => match dict.get("Marked").map(|v| document.resolve(v)) {
            Some(Object::Boolean(flag)) => Some(flag),
            _ => None,
        },
        _ => None,
    };

    let mut largest: Option<i64> = None;
    let mut note = |key: Option<i64>| {
        if let Some(key) = key {
            largest = Some(largest.map_or(key, |seen: i64| seen.max(key)));
        }
    };

    let page_list = pages(document);
    finding.pages = page_list.len();
    for (index, page) in page_list.iter().enumerate() {
        let key = page
            .get("StructParents")
            .map(|value| document.resolve(value))
            .and_then(|value| value.as_integer());
        if key.is_some() {
            finding.pages_keyed = finding.pages_keyed.saturating_add(1);
            if index == 0 {
                finding.first_page_keyed = true;
            }
        }
        note(key);
        let Some(Object::Array(annots)) = page.get("Annots").map(|value| document.resolve(value))
        else {
            continue;
        };
        for annot in &annots {
            let Object::Dictionary(dict) = document.resolve(annot) else {
                continue;
            };
            finding.annotations = finding.annotations.saturating_add(1);
            let key = dict
                .get("StructParent")
                .map(|value| document.resolve(value))
                .and_then(|value| value.as_integer());
            if key.is_some() {
                finding.annotations_keyed = finding.annotations_keyed.saturating_add(1);
            }
            note(key);
        }
    }

    // §14.7.5.4's third home. Every object of the file is asked, because an XObject is reached
    // through a resource dictionary that this census does not walk.
    for number in document.xref().object_numbers() {
        let Object::Stream(stream) = document.get(ObjectId::new(number, 0)) else {
            continue;
        };
        let key = stream
            .dict
            .get("StructParent")
            .map(|value| document.resolve(value))
            .and_then(|value| value.as_integer());
        if key.is_some() {
            finding.streams_keyed = finding.streams_keyed.saturating_add(1);
        }
        note(key);
    }
    finding.largest_key = largest;

    let Some(Object::Dictionary(root)) = catalog
        .get("StructTreeRoot")
        .map(|value| document.resolve(value))
    else {
        return finding;
    };
    finding.tagged = true;
    read_root(document, &root, &mut finding);
    finding
}

/// Table 354's entries, and the hierarchy under `/K`.
fn read_root(document: &Document, root: &Dictionary, finding: &mut Finding) {
    for key in [
        "K",
        "IDTree",
        "ParentTree",
        "ParentTreeNextKey",
        "RoleMap",
        "ClassMap",
        "Namespaces",
        "PronunciationLexicon",
        "AF",
    ] {
        if root.get(key).is_some() {
            finding.root_entries.insert(match key {
                "K" => "K",
                "IDTree" => "IDTree",
                "ParentTree" => "ParentTree",
                "ParentTreeNextKey" => "ParentTreeNextKey",
                "RoleMap" => "RoleMap",
                "ClassMap" => "ClassMap",
                "Namespaces" => "Namespaces",
                "PronunciationLexicon" => "PronunciationLexicon",
                _ => "AF",
            });
        }
    }
    finding.role_map = name_pairs_of(document, root.get("RoleMap"));
    if let Some(Object::Dictionary(classes)) = root.get("ClassMap").map(|v| document.resolve(v)) {
        for (key, _) in classes.iter() {
            finding.class_map.insert(key.as_bytes().to_vec());
        }
    }
    if let Some(Object::Dictionary(tree)) = root.get("ParentTree").map(|v| document.resolve(v)) {
        finding.parent_tree_keys =
            pdf_syntax::tree::number_pairs(&tree, &|value| document.resolve(value)).len();
    }
    if let Some(Object::Dictionary(tree)) = root.get("IDTree").map(|v| document.resolve(v)) {
        for (key, _) in pdf_syntax::tree::name_pairs(&tree, &|value| document.resolve(value)) {
            finding.ids.insert(key);
        }
    }

    let mut seen = BTreeSet::new();
    for item in &children_of(document, root) {
        if let Some(id) = item.as_reference() {
            seen.insert(id);
        }
        if let Object::Dictionary(dict) = document.resolve(item) {
            walk_elements(document, &dict, false, 0, &mut seen, finding);
        }
    }
}

fn main() {
    let files: Vec<String> = std::env::args().skip(1).collect();
    let mut opened = 0_usize;
    let mut totals = Finding::default();
    let mut tagged_documents = 0_usize;
    let mut tagged_names: Vec<String> = Vec::new();
    let mut role_maps: BTreeMap<Vec<u8>, BTreeSet<Vec<u8>>> = BTreeMap::new();
    let mut id_owners: BTreeMap<Vec<u8>, usize> = BTreeMap::new();
    let mut class_owners: BTreeMap<Vec<u8>, usize> = BTreeMap::new();
    let mut root_entries: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut first_page_keyed = 0_usize;
    let mut marked_true = 0_usize;

    for path in &files {
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        opened = opened.saturating_add(1);
        let finding = examine(&document);
        let name = std::path::Path::new(path)
            .file_name()
            .map_or_else(|| path.clone(), |name| name.to_string_lossy().into_owned());
        totals.pages = totals.pages.saturating_add(finding.pages);
        totals.pages_keyed = totals.pages_keyed.saturating_add(finding.pages_keyed);
        totals.annotations = totals.annotations.saturating_add(finding.annotations);
        totals.annotations_keyed = totals
            .annotations_keyed
            .saturating_add(finding.annotations_keyed);
        totals.streams_keyed = totals.streams_keyed.saturating_add(finding.streams_keyed);
        totals.elements = totals.elements.saturating_add(finding.elements);
        totals.elements_with_page = totals
            .elements_with_page
            .saturating_add(finding.elements_with_page);
        totals.elements_without_any_page = totals
            .elements_without_any_page
            .saturating_add(finding.elements_without_any_page);
        totals.elements_inheriting_page = totals
            .elements_inheriting_page
            .saturating_add(finding.elements_inheriting_page);
        totals.elements_with_id = totals
            .elements_with_id
            .saturating_add(finding.elements_with_id);
        totals.parent_tree_keys = totals
            .parent_tree_keys
            .saturating_add(finding.parent_tree_keys);
        if finding.first_page_keyed {
            first_page_keyed = first_page_keyed.saturating_add(1);
        }
        if finding.marked == Some(true) {
            marked_true = marked_true.saturating_add(1);
        }
        if !finding.tagged {
            continue;
        }
        tagged_documents = tagged_documents.saturating_add(1);
        tagged_names.push(format!(
            "{name}: {} element(s), {} page key(s), {} annotation key(s), {} stream key(s), {} \
             parent-tree key(s)",
            finding.elements,
            finding.pages_keyed,
            finding.annotations_keyed,
            finding.streams_keyed,
            finding.parent_tree_keys
        ));
        for entry in &finding.root_entries {
            let count = root_entries.entry(entry).or_insert(0_usize);
            *count = count.saturating_add(1);
        }
        for (key, value) in &finding.role_map {
            role_maps
                .entry(key.clone())
                .or_default()
                .insert(value.clone());
        }
        for key in &finding.ids {
            let count = id_owners.entry(key.clone()).or_insert(0_usize);
            *count = count.saturating_add(1);
        }
        for key in &finding.class_map {
            let count = class_owners.entry(key.clone()).or_insert(0_usize);
            *count = count.saturating_add(1);
        }
    }

    report(
        opened,
        tagged_documents,
        marked_true,
        first_page_keyed,
        &totals,
        &tagged_names,
        &role_maps,
        &id_owners,
        &class_owners,
        &root_entries,
    );
}

/// Everything the walk counted, printed.
#[expect(
    clippy::too_many_arguments,
    reason = "a census's whole output, threaded rather than held in a struct nothing else reads"
)]
fn report(
    opened: usize,
    tagged_documents: usize,
    marked_true: usize,
    first_page_keyed: usize,
    totals: &Finding,
    tagged_names: &[String],
    role_maps: &BTreeMap<Vec<u8>, BTreeSet<Vec<u8>>>,
    id_owners: &BTreeMap<Vec<u8>, usize>,
    class_owners: &BTreeMap<Vec<u8>, usize>,
    root_entries: &BTreeMap<&'static str, usize>,
) {
    println!("documents opened: {opened}");
    println!("documents stating /StructTreeRoot: {tagged_documents}");
    println!("documents stating /MarkInfo << /Marked true >>: {marked_true}");
    println!(
        "pages: {} of which stating /StructParents: {}",
        totals.pages, totals.pages_keyed
    );
    println!("documents whose page 1 states /StructParents: {first_page_keyed}");
    println!(
        "annotations: {} of which stating /StructParent: {}",
        totals.annotations, totals.annotations_keyed
    );
    println!(
        "streams stating /StructParent (§14.7.5.4's third home): {}",
        totals.streams_keyed
    );
    println!(
        "structure elements reached: {} of which stating /Pg: {}",
        totals.elements, totals.elements_with_page
    );
    println!(
        "elements with a marked-content child and no /Pg: {} inheriting one from an ancestor, {} \
         with none anywhere",
        totals.elements_inheriting_page, totals.elements_without_any_page
    );
    println!("elements stating /ID: {}", totals.elements_with_id);
    println!("parent-tree keys in total: {}", totals.parent_tree_keys);
    println!("Table 354 entries stated, by document:");
    for (entry, count) in root_entries {
        println!("  /{entry}: {count}");
    }
    let colliding_roles: Vec<&Vec<u8>> = role_maps
        .iter()
        .filter(|(_, values)| values.len() > 1)
        .map(|(key, _)| key)
        .collect();
    println!(
        "/RoleMap keys across tagged documents: {} of which mapped to two different names: {}",
        role_maps.len(),
        colliding_roles.len()
    );
    for key in colliding_roles.iter().take(20) {
        let values: Vec<String> = role_maps
            .get(*key)
            .into_iter()
            .flatten()
            .map(|value| String::from_utf8_lossy(value).into_owned())
            .collect();
        println!(
            "  /{} -> {}",
            String::from_utf8_lossy(key),
            values.join(", ")
        );
    }
    println!(
        "/IDTree keys across tagged documents: {} of which stated by more than one document: {}",
        id_owners.len(),
        id_owners.values().filter(|count| **count > 1).count()
    );
    println!(
        "/ClassMap keys across tagged documents: {} of which stated by more than one document: {}",
        class_owners.len(),
        class_owners.values().filter(|count| **count > 1).count()
    );
    println!("--- tagged documents ---");
    for line in tagged_names {
        println!("{line}");
    }
}
