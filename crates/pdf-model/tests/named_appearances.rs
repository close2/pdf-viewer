//! §7.7.4 Table 32's `/AP` name tree: read by the general reader, referred to by nothing.
//!
//! Table 32 gives the name dictionary an `/AP` entry:
//!
//! > ( Optional; PDF 1.3 ) A name tree mapping name strings to annotation appearance streams
//! > (see 12.5.5, "Appearance streams").
//!
//! It is the last of Table 32's ten trees this tree had not read, and what it owes turned out to
//! be a *reader* and never a resolution, because §12.5.5 — the clause Table 32 sends the reader
//! to — closes its appearance-dictionary paragraph by forbidding the reference outright:
//!
//! > For convenience in managing appearance streams that are used repeatedly, the AP entry in a
//! > PDF document's name dictionary ( see 7.7.4, "Name dictionary") may contain a name tree
//! > mapping name strings to appearance streams. The name strings have no standard meanings; no
//! > PDF objects may refer to appearance streams by name.
//!
//! So the tree is a producer's filing cabinet: a stamp used on forty pages is written once and
//! named, and every annotation that shows it names it by **object reference** in its own `/AP`,
//! as Table 170 requires. An annotation whose `/AP` were a *name* would be the thing the last
//! clause of that sentence forbids, and this project does not implement a construction the
//! standard says does not exist (`CLAUDE.md` principle 5).
//!
//! What follows is that the reader owed is the one `pdf_syntax::tree` already is — the shape
//! round 1121 and round 1145 both landed on, where a reader general over any tree leaves no
//! item-specific reader and no consumer owed. This file is that claim's witness.
//!
//! **Measured, so that the prohibition is not merely quoted**
//! (`crates/pdf-model/examples/name_dictionary_and_file_spec_census.rs`): of 3427 documents
//! opened across the pdf.js corpus, the four curated corpora and a 1-in-45 sample of the crawl,
//! 25 state a `/Names /AP` tree, holding 48 names of which 47 resolve to a stream — and **not one
//! annotation in any of them states an `/AP` that is a name**.

#![expect(
    clippy::expect_used,
    reason = "test code: a malformed fixture should fail loudly"
)]

use std::fmt::Write as _;

use pdf_syntax::{Document, tree};

/// Wraps object bodies numbered from 1 in a header, a cross-reference table and a trailer.
fn document(objects: &[&str]) -> Document {
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index.saturating_add(1));
    }
    let xref_at = out.len();
    let _ = write!(
        out,
        "xref\n0 {}\n0000000000 65535 f \n",
        objects.len().saturating_add(1)
    );
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n",
        objects.len().saturating_add(1)
    );
    Document::open(out.into_bytes()).expect("a valid file")
}

/// A document whose name dictionary holds `names`, which may be empty.
///
/// The widget's `/AP /N` is the **object reference** Table 170 requires, naming the same stream
/// the tree files under `Approved` — which is the construction the tree exists for, and the one
/// the corpus's ten pdf.js witnesses are all in.
fn with_name_dictionary(names: &str) -> Document {
    document(&[
        &format!("<< /Type /Catalog /Pages 2 0 R /Names << {names} >> >>"),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 200 200] >>",
        "<< /Type /Page /Parent 2 0 R /Annots [4 0 R] >>",
        "<< /Type /Annot /Subtype /Widget /Rect [0 0 20 20] /F 4 /AP << /N 6 0 R >> >>",
        "<< /Names [(Approved) 6 0 R] >>",
        "<< /Type /XObject /Subtype /Form /BBox [0 0 20 20] /Length 26 >>\nstream\n\
         1 0 0 rg 0 0 20 20 re f\nendstream",
    ])
}

/// The tree is walked, and its names map to the appearance streams Table 32 says they do.
///
/// The calibration is the second half (trap 13): the same document with the `/AP` entry gone
/// yields nothing, so what the walk measures is the tree rather than the fixture's other objects.
#[test]
fn table_32s_appearance_tree_is_read_by_the_general_name_tree_reader() {
    let doc = with_name_dictionary("/AP 5 0 R");
    let catalog = doc.catalog().expect("a catalog");
    let names = doc.get_key(&catalog, "Names");
    let names = names.as_dict().expect("the name dictionary");
    let root = doc.get_key(names, "AP");
    let root = root.as_dict().expect("Table 32's /AP tree");

    let pairs = tree::name_pairs(root, &|object| doc.resolve(object));
    let [(key, value)] = pairs.as_slice() else {
        panic!("one named appearance, got {pairs:?}");
    };
    assert_eq!(key.as_slice(), b"Approved", "the tree's key is bytes");
    let stream = value.as_stream().expect("an appearance stream");
    assert_eq!(
        doc.get_key(&stream.dict, "Subtype")
            .as_name()
            .map(|name| { String::from_utf8_lossy(name.as_bytes()).into_owned() }),
        Some("Form".to_owned()),
        "§12.5.5: \"[e]ach appearance stream is a form XObject\""
    );

    // The same tree by key, which is what §7.9.6's structure exists to make cheap.
    let found = tree::lookup(root, &tree::TreeKey::Name(b"Approved"), &|object| {
        doc.resolve(object)
    });
    assert!(found.is_some_and(|found| found.as_stream().is_some()));

    let without = with_name_dictionary("/Dests 5 0 R");
    let catalog = without.catalog().expect("a catalog");
    let names = without.get_key(&catalog, "Names");
    let names = names.as_dict().expect("the name dictionary");
    assert!(
        without.get_key(names, "AP").as_dict().is_none(),
        "a name dictionary stating no /AP names no appearance, which is 3402 of 3427 documents"
    );
}

/// A named appearance is shown by an annotation that names its **object**, never its name.
///
/// §12.5.5's last clause — "no PDF objects may refer to appearance streams by name" — is why
/// this file has no fixture resolving a widget's `/AP` through the tree: such a fixture would
/// pin a construction the standard forbids. What it pins instead is the construction the
/// standard leaves: Table 170's `/N` is an object, and the stream the tree names is that same
/// object, so the tree costs the drawing path nothing and can be ignored by it entirely.
#[test]
fn a_named_appearance_and_a_widgets_appearance_are_one_object() {
    let doc = with_name_dictionary("/AP 5 0 R");
    let catalog = doc.catalog().expect("a catalog");
    let names = doc.get_key(&catalog, "Names");
    let names = names.as_dict().expect("the name dictionary");
    let root = doc.get_key(names, "AP");
    let root = root.as_dict().expect("Table 32's /AP tree");
    let looked_up = tree::lookup_unresolved(root, &tree::TreeKey::Name(b"Approved"), &|object| {
        doc.resolve(object)
    })
    .expect("the tree names one");

    let pages = pdf_model::Pages::new(&doc);
    let page = pages.get(0).expect("one page");
    let annots = doc.get_key(&page.dict, "Annots");
    let annots = annots.as_array().expect("one annotation");
    let annotation = doc.resolve(&annots[0]);
    let annotation = annotation.as_dict().expect("a widget");
    let appearances = doc.get_key(annotation, "AP");
    let appearances = appearances.as_dict().expect("Table 170's dictionary");

    assert_eq!(
        appearances
            .get("N")
            .and_then(pdf_syntax::Object::as_reference),
        looked_up.as_reference(),
        "the widget names the object the tree files, which is the only reference the clause allows"
    );
    assert!(
        doc.get_key(annotation, "AP").as_name().is_none(),
        "an /AP that is a name is what §12.5.5's last clause forbids, and no corpus document \
         states one"
    );
}
