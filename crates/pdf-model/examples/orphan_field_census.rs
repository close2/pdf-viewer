//! How many documents draw a widget whose field is a root `/AcroForm /Fields` does not list, and
//! how many such roots each has.
//!
//! ISO 32000-2 §12.7.3's Table 224 states the array's entries as
//!
//! > (Required) An array of references to the document's root fields (those with no ancestors in
//! > the field hierarchy).
//!
//! and Table 226's `/Parent` is a field's statement of which field it belongs to. A widget a page
//! lists in `/Annots` whose `/Parent` chain climbs to a dictionary stating a `/T`, where that
//! dictionary is reached from no entry of `/Fields`, is the population ADR 1653 decides: a root the
//! file's own `/Parent` entries state and its `/Fields` array omits. A climb that meets no `/T` is
//! §12.7.4.2's "simply a Widget annotation" and is not counted.
//!
//! It walks the tree with `pdf_syntax` alone, and pages through [`pdf_model::page::Pages`], rather
//! than through `pdf_model::view::widgets_by_field_name`, because a census whose predicate is the
//! code under test measures the code rather than the corpus (`doc/HANDOVER.md` trap 8).
//!
//! ```sh
//! find <roots> -name '*.pdf' | cargo run --release -p pdf-model --example orphan_field_census
//! ```

#![expect(
    clippy::print_stdout,
    reason = "an example whose entire output is a measurement"
)]

use std::collections::BTreeSet;
use std::io::BufRead;

use pdf_model::page::Pages;
use pdf_syntax::{Document, Object, ObjectId};
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};

/// The bound `pdf_model::view::widgets_by_field_name` walks the field tree under.
const MAX_FIELD_DEPTH: usize = 32;

/// Files larger than this are not read, the script census's own bound.
const MAX_FILE_BYTES: u64 = 128 << 20;

/// What one document says.
struct Finding {
    /// The file.
    path: String,
    /// Entries of `/Fields`.
    listed: usize,
    /// Distinct named roots reached only through a page's widget.
    orphan_roots: usize,
    /// Widgets whose named root is one of those.
    orphan_widgets: usize,
    /// The information dictionary's `/Producer`, where it states one.
    producer: String,
}

fn main() {
    let paths: Vec<String> = std::io::stdin()
        .lock()
        .lines()
        .map_while(Result::ok)
        .filter(|line| !line.is_empty())
        .collect();
    let findings: Vec<Option<Finding>> = paths.par_iter().map(|path| examine(path)).collect();
    let with_form = findings.iter().flatten().count();
    let mut orphaned: Vec<&Finding> = findings
        .iter()
        .flatten()
        .filter(|finding| finding.orphan_roots > 0)
        .collect();
    orphaned.sort_by_key(|finding| std::cmp::Reverse(finding.orphan_roots));
    let roots: usize = orphaned.iter().map(|finding| finding.orphan_roots).sum();
    let widgets: usize = orphaned.iter().map(|finding| finding.orphan_widgets).sum();
    println!(
        "{} path(s); {with_form} with an /AcroForm /Fields array; {} with a named root /Fields \
         omits: {roots} such root(s), {widgets} widget(s) under them",
        paths.len(),
        orphaned.len()
    );
    for finding in orphaned {
        println!(
            "  {}: {} of {} root(s) omitted, {} widget(s); producer {:?}",
            finding.path,
            finding.orphan_roots,
            finding.orphan_roots.saturating_add(finding.listed),
            finding.orphan_widgets,
            finding.producer
        );
    }
}

/// One document. `None` where it cannot be read or states no `/AcroForm /Fields` array.
fn examine(path: &str) -> Option<Finding> {
    if std::fs::metadata(path).ok()?.len() > MAX_FILE_BYTES {
        return None;
    }
    let document = Document::open(std::fs::read(path).ok()?).ok()?;
    let catalog = document.catalog().ok()?;
    let form = document.get_key(&catalog, "AcroForm");
    let fields = document.get_key(form.as_dict()?, "Fields");
    let fields = fields.as_array().map(<[Object]>::to_vec)?;
    let mut reached = BTreeSet::new();
    for field in &fields {
        descend(&document, field, &mut reached, 0);
    }
    let mut orphan_roots = BTreeSet::new();
    let mut orphan_widgets = 0_usize;
    let pages = Pages::new(&document);
    for index in 0..pages.len() {
        let Some(page) = pages.get(index) else {
            continue;
        };
        let annotations = document.get_key(&page.dict, "Annots");
        for entry in annotations.as_array().into_iter().flatten() {
            let Some(id) = entry.as_reference() else {
                continue;
            };
            if reached.contains(&id) {
                continue;
            }
            if let Some(root) = named_root(&document, id)
                && !reached.contains(&root)
            {
                orphan_roots.insert(root);
                orphan_widgets = orphan_widgets.saturating_add(1);
            }
        }
    }
    let producer = document
        .get_key(document.trailer(), "Info")
        .as_dict()
        .map(|info| match document.get_key(info, "Producer") {
            Object::String(bytes) => pdf_syntax::text_string(&bytes),
            _ => String::new(),
        })
        .unwrap_or_default();
    Some(Finding {
        path: path.to_owned(),
        listed: fields.len(),
        orphan_roots: orphan_roots.len(),
        orphan_widgets,
        producer,
    })
}

/// Every dictionary below a `/Fields` entry, through `/Kids`.
fn descend(document: &Document, node: &Object, reached: &mut BTreeSet<ObjectId>, depth: usize) {
    let Some(id) = node.as_reference() else {
        return;
    };
    if depth > MAX_FIELD_DEPTH || !reached.insert(id) {
        return;
    }
    let object = document.get(id);
    let Some(dict) = object.as_dict() else {
        return;
    };
    let kids = document.get_key(dict, "Kids");
    for kid in kids.as_array().into_iter().flatten() {
        descend(document, kid, reached, depth.saturating_add(1));
    }
}

/// The top of a widget's `/Parent` chain, where the widget is one and something on the chain
/// states a `/T`; `None` otherwise.
fn named_root(document: &Document, widget: ObjectId) -> Option<ObjectId> {
    let object = document.get(widget);
    let dict = object.as_dict()?;
    if document
        .get_key(dict, "Subtype")
        .as_name()
        .map(pdf_syntax::Name::as_bytes)
        != Some(b"Widget")
    {
        return None;
    }
    let mut current = widget;
    let mut named = matches!(document.get_key(dict, "T"), Object::String(_));
    for _ in 0..MAX_FIELD_DEPTH {
        let object = document.get(current);
        let Some(parent) = object
            .as_dict()
            .and_then(|dict| dict.get("Parent"))
            .and_then(Object::as_reference)
        else {
            break;
        };
        current = parent;
        let object = document.get(current);
        named |= object
            .as_dict()
            .is_some_and(|dict| matches!(document.get_key(dict, "T"), Object::String(_)));
    }
    named.then_some(current)
}
