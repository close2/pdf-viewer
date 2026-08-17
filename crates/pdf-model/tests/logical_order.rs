//! ISO 32000-2 §14.8.2.5's two orders, measured over the corpus.
//!
//! A tagged page has a *page content order* — "the sequencing of graphics objects within a
//! page's content stream" — and a *logical content order*, "a depth-first traversal of the
//! document's logical structure hierarchy". The clause says the two "should coincide" and then
//! spends a NOTE on the cases where they cannot.
//!
//! "Should" is measurable, and this is the measurement: for every corpus page with a structure
//! tree, run the same readback through both orders and count the pages where they differ. It is
//! the only kind of check this clause admits — nothing here can decide that a producer's logical
//! order is *wrong*, only that it is not the stream's.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "test code: a fixture that cannot be built must fail loudly rather than pass by \
              doing nothing, and its byte offsets are computed from strings this file wrote"
)]

use std::path::{Path, PathBuf};

use pdf_model::structure::Tree;
use pdf_syntax::{Document, Object, ObjectId};

/// The pdf.js corpus, or `None` when the submodule is not checked out.
fn corpus() -> Option<Vec<PathBuf>> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/pdf.js/test/pdfs");
    let mut files: Vec<_> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "pdf"))
        .collect();
    files.sort();
    Some(files)
}

/// The object identity of a document's first page, which a bead, a bookmark and a
/// marked-content reference all name pages by.
fn first_page_id(document: &Document) -> Option<ObjectId> {
    let catalog = document.catalog().ok()?;
    let mut node = document.get_key(&catalog, "Pages").as_dict()?.clone();
    for _ in 0..64 {
        let kids = document.get_key(&node, "Kids");
        let id = kids
            .as_array()
            .and_then(<[Object]>::first)?
            .as_reference()?;
        let child = document.get(id);
        let child = child.as_dict()?;
        if document
            .get_key(child, "Type")
            .as_name()
            .is_some_and(|kind| kind.as_bytes() == b"Page")
            || child.get("Kids").is_none()
        {
            return Some(id);
        }
        node = child.clone();
    }
    None
}

/// How often the logical order and the page content order agree, across the corpus.
#[expect(
    clippy::too_many_lines,
    reason = "one measurement over the corpus, whose counts read better together than split \
              across helpers that each take five arguments"
)]
#[test]
fn the_two_orders_of_a_tagged_page_mostly_coincide() {
    let Some(files) = corpus() else {
        println!("skipped: the doc/pdf.js submodule is not checked out");
        return;
    };

    let mut tagged = 0usize;
    let mut with_items = 0usize;
    let mut coinciding = 0usize;
    let mut differing = Vec::new();
    let mut unreached = 0usize;
    let mut annotations_in_the_order = 0usize;
    let mut inferred_on_tagged = 0usize;
    let mut tagged_pages_needing_none = 0usize;
    for path in &files {
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        let Some(tree) = Tree::of(&document) else {
            continue;
        };
        let pages = pdf_model::Pages::new(&document);
        let (Some(page), Some(id)) = (pages.get(0), first_page_id(&document)) else {
            continue;
        };
        tagged = tagged.saturating_add(1);
        let interpretation = pdf_model::interpret(&document, &page);
        if interpretation.marked.is_empty() {
            continue;
        }
        with_items = with_items.saturating_add(1);
        // §14.8.2.6.2's requirement, counted rather than assumed: a conforming tagged page
        // states its own word breaks, so a reader should have to infer none.
        inferred_on_tagged = inferred_on_tagged.saturating_add(interpretation.inferred_separators);
        if interpretation.inferred_separators == 0 {
            tagged_pages_needing_none = tagged_pages_needing_none.saturating_add(1);
        }
        annotations_in_the_order = annotations_in_the_order.saturating_add(
            tree.logical_order(&document, id)
                .items
                .iter()
                .filter(|item| matches!(item, pdf_model::structure::Child::Object { .. }))
                .count(),
        );

        // Sequences the tree never reaches are text the logical order does not contain at all,
        // which is a fact about the *document* rather than a disagreement about order.
        let reached: Vec<i64> = tree
            .logical_order(&document, id)
            .items
            .iter()
            .filter_map(|item| match item {
                pdf_model::structure::Child::MarkedContent { mcid, .. } => Some(*mcid),
                pdf_model::structure::Child::Object { .. }
                | pdf_model::structure::Child::Element(_) => None,
            })
            .collect();
        unreached = unreached.saturating_add(
            interpretation
                .marked
                .iter()
                .filter(|span| !reached.contains(&span.mcid))
                .count(),
        );

        let logical = tree
            .logical_text(&document, id, &interpretation)
            .expect("no corpus tree comes near the walk's bound");
        // The comparison is on the text the *structure* reaches, in each order: comparing
        // against the whole readback would count every unreached sequence as a difference.
        let mut in_stream_order = String::new();
        for span in &interpretation.marked {
            if reached.contains(&span.mcid)
                && let Some(text) = interpretation.text.get(span.range.clone())
            {
                in_stream_order.push_str(text);
            }
        }
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        if logical == in_stream_order {
            coinciding = coinciding.saturating_add(1);
        } else {
            // Where they first differ, and what each says there: a permutation of the same
            // bytes is the interesting case and a different *length* would mean this reader
            // lost or duplicated a span.
            let at = logical
                .char_indices()
                .zip(in_stream_order.char_indices())
                .find(|((_, a), (_, b))| a != b)
                .map_or(logical.len().min(in_stream_order.len()), |((at, _), _)| at);
            let excerpt = |text: &str| {
                text.chars()
                    .skip(text[..at.min(text.len())].chars().count())
                    .take(24)
                    .collect::<String>()
                    .replace('\n', " ")
            };
            differing.push(format!(
                "{name}: {} vs {} bytes, first differ at {at}: logical {:?} vs stream {:?}",
                logical.len(),
                in_stream_order.len(),
                excerpt(&logical),
                excerpt(&in_stream_order)
            ));
        }
    }

    println!("{tagged} documents have a structure tree; {with_items} mark content on page one");
    println!("  {coinciding} pages: the two orders coincide");
    println!("  {} pages: they do not — {differing:?}", differing.len());
    println!("  {unreached} marked-content sequences the tree does not reach");
    println!("  {annotations_in_the_order} annotations placed in the logical order");
    println!(
        "  {inferred_on_tagged} separators inferred from position on tagged pages; \
         {tagged_pages_needing_none} of {with_items} pages needed none"
    );

    // 89 documents until session 560, where `issue17147.pdf` joined the population: its
    // cross-reference stream is unreadable, so it is rebuilt by scanning, and its
    // `/StructTreeRoot` is one of the nine objects §7.5.7 packs into an object stream that a
    // scan for `N G obj` headers cannot see. Nothing about the document changed (ADR 0395).
    assert_eq!(tagged, 90, "documents with a /StructTreeRoot");
    assert_eq!(with_items, 78, "first pages carrying an /MCID");
    assert_eq!(
        differing.len(),
        5,
        "pages whose logical order is not their stream order: {differing:?}"
    );
    assert_eq!(coinciding, 73);
    assert_eq!(
        unreached, 3,
        "sequences no structure element reaches, which are not part of either order"
    );
    assert!(
        inferred_on_tagged > 0,
        "§14.8.2.6.2 asks a tagged document to state its own word breaks; these pages do not"
    );
    assert_eq!(
        annotations_in_the_order, 295,
        "§14.8.2.5.2's annotations, placed by the structure rather than by the stream"
    );
}

/// A page whose structure deliberately reverses its stream, built here rather than found.
///
/// §14.8.2.5.1 says the two orders "should coincide" and NOTE 1 names the case where they cannot:
/// "the running text of a page, as encoded in the page's content strea m, can contain places
/// where it is not possible to make the order in which the text progresses match the logical
/// content order". A fixture is the only way to check the reordering itself — five corpus pages
/// disagree about order and none of them is a case anyone wrote on purpose.
fn reversed_fixture() -> (Document, pdf_model::Interpretation, Tree, ObjectId) {
    use std::fmt::Write as _;

    let content = "BT /F1 12 Tf 10 50 Td /P << /MCID 0 >> BDC (second) Tj EMC \
                   /P << /MCID 1 >> BDC (first) Tj EMC ET";
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 6 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] \
         /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R /StructParents 0 >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n\
         5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n\
         6 0 obj\n<< /Type /StructTreeRoot /K [7 0 R 8 0 R] /ParentTree 9 0 R >>\nendobj\n\
         7 0 obj\n<< /Type /StructElem /S /P /Pg 3 0 R /K 1 >>\nendobj\n\
         8 0 obj\n<< /Type /StructElem /S /P /Pg 3 0 R /K 0 >>\nendobj\n\
         9 0 obj\n<< /Nums [0 [8 0 R 7 0 R]] >>\nendobj\n",
        content.len() + 1,
    );
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    let mut cursor = out.len();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(cursor);
        cursor += object.len();
    }
    out.push_str(&body);
    let xref_at = out.len();
    let size = offsets.len() + 1;
    let _ = write!(out, "xref\n0 {size}\n0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );

    let document = Document::open(out.into_bytes()).expect("the fixture is a valid file");
    let interpretation = {
        let pages = pdf_model::Pages::new(&document);
        let page = pages.get(0).expect("one page");
        pdf_model::interpret(&document, &page)
    };
    let tree = Tree::of(&document).expect("a structure tree");
    let id = first_page_id(&document).expect("a first page");
    (document, interpretation, tree, id)
}

/// The two orders of the fixture above, whole.
#[test]
fn the_logical_order_reorders_what_the_stream_showed() {
    let (document, interpretation, tree, id) = reversed_fixture();
    assert_eq!(
        interpretation.text.replace(['\n', ' '], ""),
        "secondfirst",
        "the stream showed them in this order"
    );
    assert_eq!(
        interpretation
            .marked
            .iter()
            .map(|span| span.mcid)
            .collect::<Vec<_>>(),
        vec![0, 1],
        "one span per /MCID, in the order the sections closed"
    );

    let logical = tree
        .logical_text(&document, id, &interpretation)
        .expect("a two-item fixture is far below the walk's bound");
    assert_eq!(
        logical.replace(['\n', ' '], ""),
        "firstsecond",
        "the structure states the other order, and a depth-first walk finds it"
    );
    assert_eq!(
        tree.logical_order(&document, id).items.len(),
        2,
        "two content items, no elements"
    );
}

/// The same fixture, asked about a *range* of its readback rather than the whole page.
///
/// §14.8.2.5 is what a person pressing copy needs, and a person selects part of a page. The
/// invariant the range form has and [`pdf_model::structure::Tree::logical_text`] does not is that
/// what comes back is a rearrangement of exactly the bytes asked for — which is why it answers
/// `None` rather than a shorter string where the structure tree misses part of the range.
#[test]
fn a_range_of_the_readback_reorders_or_refuses() {
    let (document, interpretation, tree, id) = reversed_fixture();
    // A *range* of the readback, which is what a person selects. The whole of it reorders the
    // same way the whole page does, and a range covering only the second-shown word comes back as
    // that word alone — the logical order of one item is that item.
    let whole = 0..interpretation.text.len();
    assert_eq!(
        tree.logical_range(
            &document,
            id,
            &interpretation.text,
            &interpretation.marked,
            whole.clone(),
        )
        .expect("the tree reaches every byte of this page")
        .replace(['\n', ' '], ""),
        "firstsecond"
    );

    // Every span the tree reaches, so `logical_range` over the whole page is a rearrangement of
    // exactly the same characters — the invariant its `None` arm exists to protect.
    let mut ours: Vec<char> = tree
        .logical_range(
            &document,
            id,
            &interpretation.text,
            &interpretation.marked,
            whole,
        )
        .expect("covered")
        .chars()
        .collect();
    let mut theirs: Vec<char> = interpretation.text.chars().collect();
    ours.sort_unstable();
    theirs.sort_unstable();
    assert_eq!(ours, theirs, "the same characters, in the other order");

    // The word the stream showed *first* is `second`, and selecting only it gives only it.
    let at = interpretation
        .text
        .find("second")
        .expect("the stream showed it");
    assert_eq!(
        tree.logical_range(
            &document,
            id,
            &interpretation.text,
            &interpretation.marked,
            at..at + "second".len(),
        )
        .as_deref(),
        Some("second")
    );

    // An empty selection is not a rearrangement of anything, and a range beyond every span is
    // the case the `None` arm exists for: a copy must not silently lose what the tree missed.
    assert_eq!(
        tree.logical_range(
            &document,
            id,
            &interpretation.text,
            &interpretation.marked,
            0..0
        ),
        None
    );
    let past = interpretation.text.len();
    assert_eq!(
        tree.logical_range(
            &document,
            id,
            &interpretation.text,
            &interpretation.marked,
            past..past.saturating_add(1),
        ),
        None
    );
}
