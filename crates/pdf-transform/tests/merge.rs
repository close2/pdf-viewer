//! `merge`: RFC 0002 section 6.2's verb, held to the clauses each of its reconciliations is
//! derived from.
//!
//! The committed documents are the population — every checkout has them once
//! `doc/specifications.zip` is unpacked — and each test states one property of the merged
//! document: the pages are the sources' in order and draw as they did, §7.9.6's colliding
//! destination names are renamed and their references rewritten, §12.3.3's outlines are one
//! spliced chain, §12.4.2's labels are the ones the pages had, §14.11.5's array lands where the
//! clause puts it, and §12.7.4.2's field-name collision is refused by name.
//!
//! Two fixtures are built here rather than found, in the form `tests/writer.rs` builds its own:
//! no committed document has an interactive form with a *filled* field, so nothing in the tree
//! can make §12.7.4.2's collision happen. Everything else is a real document (trap 4).
//!
//! `qpdf --check`, where it is installed, is **evidence about the reading and never its
//! definition** (principle 5).

#![expect(
    clippy::expect_used,
    clippy::print_stderr,
    reason = "test code: a fixture that cannot exercise the rule must fail loudly, and a \
              skipped test says so"
)]

mod support;

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::process::Command;

use pdf_model::Pages;
use pdf_model::page_label::PageLabels;
use pdf_syntax::object::ObjectId;
use pdf_syntax::{Document, Limits, Object};
use pdf_transform::merge::{Input, MergePlan};
use pdf_transform::range::Selection;
use pdf_transform::render::{ImageFormat, RenderPlan, Sizing};
use pdf_transform::split::{Pieces, SplitPlan};
use pdf_transform::{Budget, MemorySinks, Origin, Plan, Policy, Refusal, Report, Source, apply};

use support::committed;

/// The first fixture, and the one most tests use: five pages, an outline, `/PageLabels` and an
/// `/AcroForm`.
const FIRST: &str = "PDF20_AN001-BPC.pdf";

/// The second: fourteen pages, an outline, a `/Names` `/Dests` tree of clause anchors, and its
/// own `/AcroForm` default resources.
const SECOND: &str = "ISO_TS_32001-2022_sponsored_EC3.pdf";

/// The one committed document that states a catalog `/OutputIntents` (§14.11.5).
const WITH_INTENT: &str = "Tagged-PDF-Best-Practice-Guide.pdf";

/// Merges these inputs, answering the report and the one output.
fn merge(inputs: &[(&[u8], &str)], collate: bool) -> Result<(Report, Vec<u8>), Refusal> {
    let sinks = MemorySinks::new();
    let plan = Plan::Merge(MergePlan {
        inputs: inputs
            .iter()
            .enumerate()
            .map(|(source, (_, pages))| Input {
                source,
                pages: pages.parse::<Selection>().expect("a selection"),
            })
            .collect(),
        collate,
        names: "merged.pdf".parse().expect("a pattern"),
    });
    let sources: Vec<Source> = inputs
        .iter()
        .map(|(bytes, _)| Source::new(bytes.to_vec()))
        .collect();
    let report = apply(
        &plan,
        &sources,
        &sinks,
        &Policy::default(),
        &Budget::default(),
    )?;
    let mut outputs = sinks.into_outputs();
    assert_eq!(outputs.len(), 1, "a merge writes one file");
    Ok((report, outputs.remove(0).1))
}

/// One page of `bytes` as a PPM, or `None` where nothing was drawn.
fn draw(bytes: &[u8], page: usize) -> Option<Vec<u8>> {
    let sinks = MemorySinks::new();
    apply(
        &Plan::Render(RenderPlan {
            source: 0,
            pages: page.to_string().parse::<Selection>().expect("a selection"),
            size: Sizing::Dpi(72.0),
            format: ImageFormat::Ppm,
            page_box: None,
            annotations: true,
            names: "page.ppm".parse().expect("a pattern"),
        }),
        &[Source::new(bytes.to_vec())],
        &sinks,
        &Policy::default(),
        &Budget::default(),
    )
    .ok()?;
    let mut outputs = sinks.into_outputs();
    (!outputs.is_empty()).then(|| outputs.remove(0).1)
}

/// One page's `/Contents` as the file holds it: the encoded bytes, joined.
fn encoded_contents(document: &Document, index: usize) -> Option<Vec<u8>> {
    let page = Pages::new(document).get(index)?;
    let mut out = Vec::new();
    match document.get_key(&page.dict, "Contents") {
        Object::Stream(stream) => out.extend_from_slice(&stream.data),
        Object::Array(items) => {
            for item in &items {
                if let Some(stream) = document.resolve(item).as_stream() {
                    out.extend_from_slice(&stream.data);
                }
            }
        }
        _ => return None,
    }
    Some(out)
}

/// `qpdf --check` on these bytes, where qpdf is installed: `Some(accepted)`.
fn qpdf_accepts(bytes: &[u8]) -> Option<bool> {
    let directory =
        std::env::temp_dir().join(format!("pdfv-merge-{}-{}", std::process::id(), bytes.len()));
    std::fs::create_dir_all(&directory).ok()?;
    let path = directory.join("merged.pdf");
    std::fs::write(&path, bytes).ok()?;
    let output = Command::new("qpdf").arg("--check").arg(&path).output().ok();
    let _ = std::fs::remove_dir_all(&directory);
    Some(output?.status.success())
}

/// The named destination an outline item or annotation states, as §12.3.2.3 and §12.6.4.2 put
/// one: a `/Dest` entry, or a `/GoTo` action's `/D`, either as a name or as a string.
fn named_destination(document: &Document, dict: &pdf_syntax::Dictionary) -> Option<Vec<u8>> {
    let direct = document.get_key(dict, "Dest");
    let value = if direct.is_null() {
        let action = document.get_key(dict, "A");
        action
            .as_dict()
            .map_or(Object::Null, |action| document.get_key(action, "D"))
    } else {
        direct
    };
    match value {
        Object::Name(name) => Some(name.as_bytes().to_vec()),
        Object::String(bytes) => Some(bytes.to_vec()),
        _ => None,
    }
}

/// Every top-level outline item of a document, in chain order.
fn top_level_items(document: &Document) -> Vec<ObjectId> {
    let Ok(catalog) = document.catalog() else {
        return Vec::new();
    };
    let outlines = document.get_key(&catalog, "Outlines");
    let Some(dict) = outlines.as_dict() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut next = dict.get("First").and_then(Object::as_reference);
    while let Some(id) = next {
        if out.contains(&id) || out.len() > 4096 {
            break;
        }
        out.push(id);
        let item = document.get(id);
        next = item
            .as_dict()
            .and_then(|item| item.get("Next"))
            .and_then(Object::as_reference);
    }
    out
}

/// The pages of two documents become one document's, in the order the inputs were given, and
/// each carries its producer's content stream byte for byte.
#[test]
fn the_merged_pages_are_the_sources_in_order_and_carry_their_own_streams() {
    let first = std::fs::read(committed(FIRST)).expect("a committed document");
    let second = std::fs::read(committed(SECOND)).expect("a committed document");
    let (report, merged) = merge(&[(&first, "1-2"), (&second, "1-3")], false).expect("it merges");

    let read = Document::open_with_limits(merged.clone(), Limits::DEFAULT).expect("it opens");
    assert_eq!(Pages::new(&read).len(), 5, "two pages then three");
    assert!(
        matches!(
            &report.outputs.first().expect("one output").origin,
            Origin::Merged { sources, pages, .. } if sources == &[0, 1] && *pages == 5
        ),
        "{:?}",
        report.outputs.first().map(|output| &output.origin)
    );

    let a = Document::open_with_limits(first, Limits::DEFAULT).expect("it opens");
    let b = Document::open_with_limits(second, Limits::DEFAULT).expect("it opens");
    // RFC 0002 section 11.1: "every content stream in their output is a producer's, carried
    // byte for byte". The *encoded* bytes, because a re-encoded stream would pass a comparison
    // of decoded ones.
    for (index, (source, at)) in [(&a, 0), (&a, 1), (&b, 0), (&b, 1), (&b, 2)]
        .into_iter()
        .enumerate()
    {
        assert_eq!(
            encoded_contents(source, at),
            encoded_contents(&read, index),
            "page {} of the merge is not its source page's stream",
            index.saturating_add(1)
        );
    }
}

/// RFC 0002 section 9's layer 3, and the load-bearing one: a carried page draws as it did.
#[test]
fn every_carried_page_draws_bit_identically_to_the_page_it_came_from() {
    let first = std::fs::read(committed(FIRST)).expect("a committed document");
    let second = std::fs::read(committed(SECOND)).expect("a committed document");
    let (_, merged) = merge(&[(&first, "1-2"), (&second, "1-2")], false).expect("it merges");
    for (position, (bytes, page)) in [(&first, 1), (&first, 2), (&second, 1), (&second, 2)]
        .into_iter()
        .enumerate()
    {
        let before = draw(bytes, page).expect("the source page draws");
        let after = draw(&merged, position.saturating_add(1)).expect("the merged page draws");
        assert_eq!(
            before,
            after,
            "page {} of the merge does not mark what its source page marked",
            position.saturating_add(1)
        );
    }
}

/// RFC 0002 section 9's first layer: same sources, same plan, same bytes, with no flag needed.
#[test]
fn the_same_merge_twice_writes_the_same_bytes() {
    let first = std::fs::read(committed(FIRST)).expect("a committed document");
    let second = std::fs::read(committed(SECOND)).expect("a committed document");
    let (_, once) = merge(&[(&first, "1-2"), (&second, "1-3")], false).expect("it merges");
    let (_, twice) = merge(&[(&first, "1-2"), (&second, "1-3")], false).expect("it merges");
    assert_eq!(once, twice);
}

/// RFC 0002 section 9's property gate: `split` then `merge` reproduces the source's pages.
///
/// The document is cut into one file per page and the pieces are put back in order; every page
/// of the result must draw as the corresponding page of the original did. It is the strongest
/// statement available about the two verbs together, because it composes both writers and judges
/// the composition by layer 3 rather than by either verb's own bookkeeping.
///
/// **The suite's other property gate is not taken and this is what it costs**: `optimize` is
/// idempotent — its own output, optimised again, byte-identical — and `optimize` does not exist
/// (RFC 0002 section 6.5). Writing the gate now would be a test of nothing, and it is a line of
/// the round that lands that verb.
#[test]
fn splitting_a_document_and_merging_the_pieces_back_reproduces_its_pages() {
    let bytes = std::fs::read(committed(FIRST)).expect("a committed document");
    let source = Document::open_with_limits(bytes.clone(), Limits::DEFAULT).expect("it opens");
    let count = Pages::new(&source).len();

    let sinks = MemorySinks::new();
    apply(
        &Plan::Split(SplitPlan {
            source: 0,
            pages: "1-end".parse::<Selection>().expect("a selection"),
            pieces: Pieces::EachPage,
            names: "piece-%d.pdf".parse().expect("a pattern"),
        }),
        &[Source::new(bytes.clone())],
        &sinks,
        &Policy::default(),
        &Budget::default(),
    )
    .expect("the split applies");
    // **By name, not by position.** `split` writes its pieces across rayon and `MemorySinks`
    // keeps them in the order they were *opened*, which is thread order; taking them in that
    // order would put the pages back in a different one on some runs and none on others.
    let written = sinks.into_outputs();
    assert_eq!(written.len(), count, "one piece per page");
    let pieces: Vec<Vec<u8>> = (1..=count)
        .map(|page| {
            written
                .iter()
                .find(|(name, _)| name == &format!("piece-{page}.pdf"))
                .map(|(_, bytes)| bytes.clone())
                .expect("a piece named for every page")
        })
        .collect();

    let inputs: Vec<(&[u8], &str)> = pieces.iter().map(|piece| (piece.as_slice(), "1")).collect();
    let (_, merged) = merge(&inputs, false).expect("the pieces merge");
    let read = Document::open_with_limits(merged.clone(), Limits::DEFAULT).expect("it opens");
    assert_eq!(Pages::new(&read).len(), count, "the pages all came back");
    for page in 1..=count {
        assert_eq!(
            draw(&bytes, page),
            draw(&merged, page),
            "page {page} of the round trip does not mark what the original marked"
        );
    }
}

/// §7.9.6: "[t]he keys contained within the various nodes' Names entries shall not overlap", so
/// a key two sources share is renamed — and every `/Dest` and `/GoTo` naming it is rewritten,
/// which is what keeps the second source's outline pointing where its own document pointed.
#[test]
fn a_colliding_destination_name_is_renamed_and_the_references_to_it_rewritten() {
    let bytes = std::fs::read(committed(SECOND)).expect("a committed document");
    let source = Document::open_with_limits(bytes.clone(), Limits::DEFAULT).expect("it opens");
    let before = top_level_items(&source);
    let named: Vec<Vec<u8>> = before
        .iter()
        .filter_map(|id| named_destination(&source, source.get(*id).as_dict()?))
        .collect();
    assert!(
        !named.is_empty(),
        "the fixture's outline names destinations by string"
    );

    // The same document as both inputs: every one of its destination names collides with
    // itself, which is the sharpest form of the case.
    let (report, merged) = merge(&[(&bytes, "1-2"), (&bytes, "1-2")], false).expect("it merges");
    let said: Vec<&str> = report
        .warnings
        .iter()
        .map(|warning| warning.detail.as_str())
        .filter(|detail| detail.contains("§7.9.6"))
        .collect();
    assert!(!said.is_empty(), "no rename was reported");

    let read = Document::open_with_limits(merged, Limits::DEFAULT).expect("it opens");
    let after = top_level_items(&read);
    assert_eq!(
        after.len(),
        before.len().saturating_mul(2),
        "both outlines' top-level items are in the merged chain"
    );
    let destinations: Vec<Vec<u8>> = after
        .iter()
        .filter_map(|id| named_destination(&read, read.get(*id).as_dict()?))
        .collect();
    for name in &named {
        let mut renamed = name.clone();
        renamed.extend_from_slice(b" (2)");
        assert!(
            destinations.contains(name),
            "the first source's outline lost {}",
            String::from_utf8_lossy(name)
        );
        assert!(
            destinations.contains(&renamed),
            "the second source's outline still names {} rather than the key it was given",
            String::from_utf8_lossy(name)
        );
    }
    // And the key it was given is in the merged tree, so the reference resolves.
    let catalog = read.catalog().expect("a catalog");
    let dictionary = read.get_key(&catalog, "Names");
    let dictionary = dictionary.as_dict().expect("a /Names dictionary");
    let root = read.get_key(dictionary, "Dests");
    let root = root.as_dict().expect("a /Dests tree");
    let keys: Vec<Vec<u8>> = pdf_syntax::tree::name_entries(root, &|object| read.resolve(object))
        .into_iter()
        .map(|(key, _)| key)
        .collect();
    for name in &named {
        let mut renamed = name.clone();
        renamed.extend_from_slice(b" (2)");
        assert!(
            keys.contains(&renamed),
            "the renamed key is not in the tree"
        );
    }
    // §7.9.6's own order, which is what a reader binary-searches by.
    let mut sorted = keys.clone();
    sorted.sort();
    assert_eq!(
        keys, sorted,
        "the merged tree's keys are not in lexical order"
    );
}

/// §12.3.3: Table 151 makes a top-level item's `/Parent` "the outline dictionary itself", and
/// the items "a linked list, chained together through their Prev and Next entries". The merged
/// chain is one list over both sources' items, in input order.
#[test]
fn the_outlines_are_spliced_into_one_chain() {
    let first = std::fs::read(committed(FIRST)).expect("a committed document");
    let second = std::fs::read(committed(SECOND)).expect("a committed document");
    let a = Document::open_with_limits(first.clone(), Limits::DEFAULT).expect("it opens");
    let b = Document::open_with_limits(second.clone(), Limits::DEFAULT).expect("it opens");
    let expected = top_level_items(&a)
        .len()
        .saturating_add(top_level_items(&b).len());
    assert!(expected > 2, "both fixtures have outlines");

    let (_, merged) = merge(&[(&first, "1-2"), (&second, "1-2")], false).expect("it merges");
    let read = Document::open_with_limits(merged, Limits::DEFAULT).expect("it opens");
    let catalog = read.catalog().expect("a catalog");
    let outlines = read.get_key(&catalog, "Outlines");
    let root = outlines.as_dict().expect("an outline dictionary");
    let items = top_level_items(&read);
    assert_eq!(items.len(), expected, "the chain is both sources' items");
    assert_eq!(
        root.get("First").and_then(Object::as_reference),
        items.first().copied(),
        "Table 150's /First is the head of the chain"
    );
    assert_eq!(
        root.get("Last").and_then(Object::as_reference),
        items.last().copied(),
        "and /Last is its tail"
    );
    let outline_id = catalog
        .get("Outlines")
        .and_then(Object::as_reference)
        .expect("the outline is an indirect object");
    for (position, id) in items.iter().enumerate() {
        let item = read.get(*id);
        let item = item.as_dict().expect("an outline item");
        assert_eq!(
            item.get("Parent").and_then(Object::as_reference),
            Some(outline_id),
            "item {position}'s parent is not the merged outline dictionary"
        );
        assert_eq!(
            item.get("Prev").and_then(Object::as_reference),
            position
                .checked_sub(1)
                .and_then(|before| items.get(before))
                .copied(),
            "item {position}'s /Prev"
        );
        assert_eq!(
            item.get("Next").and_then(Object::as_reference),
            items.get(position.saturating_add(1)).copied(),
            "item {position}'s /Next"
        );
    }
}

/// §12.4.2: the merged tree holds one entry per page, each reproducing the label that page had.
#[test]
fn every_page_keeps_the_label_it_had_in_its_own_document() {
    let first = std::fs::read(committed(FIRST)).expect("a committed document");
    let second = std::fs::read(committed(SECOND)).expect("a committed document");
    let one = Document::open_with_limits(first.clone(), Limits::DEFAULT).expect("it opens");
    let two = Document::open_with_limits(second.clone(), Limits::DEFAULT).expect("it opens");
    let ones = PageLabels::read(&one);
    let twos = PageLabels::read(&two);
    assert!(
        !ones.is_empty() && !twos.is_empty(),
        "both fixtures label their pages"
    );

    let (_, merged) = merge(&[(&first, "1-3"), (&second, "1-3")], false).expect("it merges");
    let read = Document::open_with_limits(merged, Limits::DEFAULT).expect("it opens");
    let labels = PageLabels::read(&read);
    for index in 0..3 {
        assert_eq!(
            labels.label(index),
            ones.label(index),
            "page {index} of the first source lost its label"
        );
        assert_eq!(
            labels.label(index.saturating_add(3)),
            twos.label(index),
            "page {index} of the second source lost its label"
        );
    }
}

/// §14.11.5: "when processing a page that has an associated (page-level) output intent, that
/// page-level output intent shall be used" — so where the sources disagree, each source's array
/// goes onto its own pages and the merged catalog states none.
#[test]
fn the_output_intent_goes_onto_the_pages_of_the_source_that_stated_it() {
    let with = std::fs::read(committed(WITH_INTENT)).expect("a committed document");
    let without = std::fs::read(committed(FIRST)).expect("a committed document");
    let stating = Document::open_with_limits(with.clone(), Limits::DEFAULT).expect("it opens");
    let catalog = stating.catalog().expect("a catalog");
    assert!(
        catalog.get("OutputIntents").is_some(),
        "the fixture states a catalog /OutputIntents"
    );

    let (_, merged) = merge(&[(&with, "1-2"), (&without, "1-2")], false).expect("it merges");
    let read = Document::open_with_limits(merged, Limits::DEFAULT).expect("it opens");
    let catalog = read.catalog().expect("a catalog");
    assert!(
        catalog.get("OutputIntents").is_none(),
        "a catalog array would claim the other source's pages too"
    );
    let pages = Pages::new(&read);
    for index in 0..2 {
        assert!(
            pages
                .get(index)
                .expect("a page")
                .dict
                .get("OutputIntents")
                .is_some(),
            "page {index} came from the source that stated one and does not carry it"
        );
    }
    for index in 2..4 {
        assert!(
            pages
                .get(index)
                .expect("a page")
                .dict
                .get("OutputIntents")
                .is_none(),
            "page {index} came from a source that stated none and was given one"
        );
    }
}

/// §12.7.4.2: "actual field dictionaries with the same fully qualified field name shall have
/// the same field type ( FT ), value ( V ), and default value ( DV )."
///
/// Two documents whose field agrees are one field with two representations, which the clause
/// permits: they merge, with a warning naming the field. Two whose `/V` differs cannot both be
/// in one document, and the merge is refused by name.
#[test]
fn a_field_name_two_sources_disagree_about_is_refused_by_name() {
    let agreeing = form_document("(typed)");
    let differing = form_document("(other)");

    let (report, _) = merge(&[(&agreeing, "1"), (&agreeing.clone(), "1")], false)
        .expect("two identical fields are one field with two representations");
    assert!(
        report
            .warnings
            .iter()
            .any(|warning| warning.detail.contains("§12.7.4.2") && warning.detail.contains("Name")),
        "the permitted collision is not named: {:?}",
        report.warnings
    );

    match merge(&[(&agreeing, "1"), (&differing, "1")], false) {
        Err(Refusal::FieldCollision { fields }) => {
            assert!(fields.contains("Name"), "{fields}");
        }
        Err(other) => panic!("refused for the wrong reason: {other}"),
        Ok(_) => panic!("two fields with one name and two values were written into one document"),
    }
}

/// Table 31: a page's `/Parent` is "the page tree node that is the immediate parent of this page
/// object", so a page cannot be in the merged tree twice.
#[test]
fn a_page_named_twice_is_refused() {
    let bytes = std::fs::read(committed(FIRST)).expect("a committed document");
    match merge(&[(&bytes, "1-2"), (&bytes.clone(), "1")], false) {
        Ok(_) => {}
        Err(other) => {
            panic!("two different sources holding the same bytes are two documents: {other}")
        }
    }
    match merge(&[(&bytes, "1,1")], false) {
        Err(Refusal::PageTwice { at, page }) => {
            assert_eq!((at, page), (0, 1));
        }
        Err(other) => panic!("refused for the wrong reason: {other}"),
        Ok(_) => panic!("one page was put in the tree twice"),
    }
}

/// `--collate` interleaves the inputs a page at a time — pdftk's `shuffle`.
#[test]
fn collate_takes_one_page_from_each_input_in_turn() {
    let first = std::fs::read(committed(FIRST)).expect("a committed document");
    let second = std::fs::read(committed(SECOND)).expect("a committed document");
    let (_, merged) = merge(&[(&first, "1-2"), (&second, "1-2")], true).expect("it merges");
    let a = Document::open_with_limits(first, Limits::DEFAULT).expect("it opens");
    let b = Document::open_with_limits(second, Limits::DEFAULT).expect("it opens");
    let read = Document::open_with_limits(merged, Limits::DEFAULT).expect("it opens");
    assert_eq!(Pages::new(&read).len(), 4);
    for (index, (source, at)) in [(&a, 0), (&b, 0), (&a, 1), (&b, 1)].into_iter().enumerate() {
        assert_eq!(
            encoded_contents(source, at),
            encoded_contents(&read, index),
            "collated page {index} is not the page it should be"
        );
    }
}

/// What the merged document does not carry is named, never dropped in silence — trap 5.
#[test]
fn what_the_merge_does_not_carry_is_named() {
    let first = std::fs::read(committed(FIRST)).expect("a committed document");
    let second = std::fs::read(committed(SECOND)).expect("a committed document");
    let (report, _) = merge(&[(&first, "1"), (&second, "1")], false).expect("it merges");
    let said: String = report
        .warnings
        .iter()
        .map(|warning| warning.detail.clone())
        .collect::<Vec<_>>()
        .join(" ");
    for key in ["Metadata", "Info"] {
        assert!(
            said.contains(key),
            "the report does not name /{key}: {said:?}"
        );
    }
}

/// Foreign evidence, in principle 5's register: `qpdf --check` accepts what this program wrote.
#[test]
fn qpdf_accepts_a_merged_document() {
    let first = std::fs::read(committed(FIRST)).expect("a committed document");
    let second = std::fs::read(committed(SECOND)).expect("a committed document");
    let (_, merged) = merge(&[(&first, "1-2"), (&second, "1-3")], false).expect("it merges");
    match qpdf_accepts(&merged) {
        Some(true) => {}
        Some(false) => panic!("qpdf --check refused a document this program wrote"),
        // A machine without qpdf runs every other assertion here; failing would make this a
        // coin toss on the environment rather than a statement about the file.
        None => eprintln!("skipped: qpdf is not installed"),
    }
}

/// A one-page document with one filled text field called `Name`.
///
/// Built rather than found: no committed document has an interactive form with a value in it,
/// so §12.7.4.2's collision cannot be made to happen out of the tree's own files. The shape is
/// Table 224's minimum — an `/AcroForm` with `/Fields` — and Table 226's, with the field and its
/// single widget merged into one dictionary as §12.7.2 permits.
fn form_document(value: &str) -> Vec<u8> {
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [5 0 R] /DA (/Helv 0 Tf 0 g) >> >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Resources << >> /Contents 4 0 R /Annots [5 0 R] >>\nendobj\n\
         4 0 obj\n<< /Length 0 >>\nstream\n\nendstream\nendobj\n\
         5 0 obj\n<< /Type /Annot /Subtype /Widget /FT /Tx /T (Name) /V {value} /Rect [10 10 190 40] /F 4 /P 3 0 R >>\nendobj\n"
    );
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(out.len());
        out.push_str(object);
    }
    let xref_at = out.len();
    let size = offsets.len().saturating_add(1);
    let _ = writeln!(out, "xref\n0 {size}");
    out.push_str("0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R /ID [<0102> <0304>] >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

/// A one-page tagged document, with the three §14.7 namespaces two sources can collide in.
///
/// Built rather than found, on `form_document`'s precedent and for the same reason: the corpus
/// walk merges each document with **one** fixed second document, so no pair of corpus files
/// exercises an `/ID` two sources share or a class two sources define differently, and neither
/// collision can be made to happen out of the tree's own files. The shape is Table 354's and
/// Table 355's minimum — a root with `/K`, `/ParentTree`, `/RoleMap`, `/ClassMap` and `/IDTree`,
/// and one `/Document` element over one leaf whose `/K` is §14.7.5.2's integer.
fn tagged_document(identifier: &str, role: &str, align: &str) -> Vec<u8> {
    // §14.7.5.2's marked-content sequence, with the identifier the parent tree indexes by.
    let content = "/P << /MCID 0 >> BDC\nEMC\n";
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 5 0 R /MarkInfo << /Marked true \
         >> >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Resources << >> \
         /Contents 4 0 R /StructParents 7 >>\nendobj\n\
         4 0 obj\n<< /Length {len} >>\nstream\n{content}endstream\nendobj\n\
         5 0 obj\n<< /Type /StructTreeRoot /K [6 0 R] /ParentTree 8 0 R /ParentTreeNextKey 8 \
         /RoleMap << /Title /{role} >> /ClassMap << /Pa1 << /O /Layout /TextAlign /{align} >> >> \
         /IDTree << /Names [({identifier}) 7 0 R] >> >>\nendobj\n\
         6 0 obj\n<< /Type /StructElem /S /Document /P 5 0 R /K [7 0 R] >>\nendobj\n\
         7 0 obj\n<< /Type /StructElem /S /Title /P 6 0 R /Pg 3 0 R /K [0] /ID ({identifier}) \
         /C /Pa1 >>\nendobj\n\
         8 0 obj\n<< /Nums [7 [7 0 R]] >>\nendobj\n",
        len = content.len(),
    );
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(out.len());
        out.push_str(object);
    }
    let xref_at = out.len();
    let size = offsets.len().saturating_add(1);
    let _ = writeln!(out, "xref\n0 {size}");
    out.push_str("0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R /ID [<0102> <0304>] >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

/// Table 355 makes an element's `/ID` "unique among all elements in the document's structure
/// hierarchy", and the merged document is one hierarchy — so two sources stating one identifier
/// is refused by name rather than renamed.
///
/// A rename is what §14.7.6.2's class names get, and the difference is which clause closes the
/// set of referrers: §14.7.6.2 says a class is named by an element's `/C` and nothing else,
/// while §14.8.5's `/Headers` attribute is "an array of byte strings, where each byte string
/// shall be the element identifier" and Annex E permits further attributes nobody here knows.
#[test]
fn two_sources_stating_one_element_identifier_are_refused_by_name() {
    let first = tagged_document("Chapter1", "H1", "Start");
    let second = tagged_document("Chapter1", "H1", "Start");
    let refusal = merge(&[(&first, "1"), (&second, "1")], false).expect_err("it refuses");
    let Refusal::StructureConflict { clause, keys } = &refusal else {
        panic!("§14.7 refuses by its own name: {refusal:?}");
    };
    assert!(
        clause.contains("unique among all elements"),
        "the refusal names Table 355's sentence: {clause}"
    );
    assert!(
        keys.contains("Chapter1"),
        "and every colliding identifier: {keys}"
    );
    assert_eq!(refusal.exit(), pdf_transform::Exit::Refused);
    // Two sources whose identifiers differ merge, which is what makes the refusal about the
    // collision rather than about tagging.
    let other = tagged_document("Chapter2", "H1", "Start");
    merge(&[(&first, "1"), (&other, "1")], false).expect("distinct identifiers merge");
}

/// §14.7.6.2 attaches a class's attributes to the element that names it, so a class two sources
/// define differently is renamed and every carried element's `/C` follows.
#[test]
fn an_attribute_class_two_sources_disagree_on_is_renamed_and_the_elements_follow() {
    let first = tagged_document("A", "H1", "Start");
    let second = tagged_document("B", "H1", "End");
    let (report, merged) = merge(&[(&first, "1"), (&second, "1")], false).expect("it merges");
    let said: String = report
        .warnings
        .iter()
        .map(|warning| warning.detail.clone())
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        said.contains("§14.7.6.2") && said.contains("/Pa1"),
        "the rename is reported by name: {said:?}"
    );
    let read = Document::open_with_limits(merged, Limits::DEFAULT).expect("re-read");
    let catalog = read.catalog().expect("a catalog");
    let root = read.get_key(&catalog, "StructTreeRoot");
    let root = root.as_dict().expect("a structure tree root");
    let classes = read.get_key(root, "ClassMap");
    let classes = classes.as_dict().expect("Table 354's /ClassMap");
    assert!(
        classes.get("Pa1").is_some() && classes.get("Pa1 (2)").is_some(),
        "both definitions survive under distinct names: {classes:?}"
    );
    // Every element's `/C` names a class the merged map defines, and the two elements name
    // different ones — which is the property the rename exists for.
    let named: BTreeSet<Vec<u8>> = read
        .xref()
        .object_numbers()
        .filter_map(|number| match read.get(ObjectId::new(number, 0)) {
            Object::Dictionary(dict) => dict
                .get("C")
                .and_then(Object::as_name)
                .map(|name| name.as_bytes().to_vec()),
            _ => None,
        })
        .collect();
    assert_eq!(
        named,
        BTreeSet::from([b"Pa1".to_vec(), b"Pa1 (2)".to_vec()]),
        "each source's element names its own source's class"
    );
}

/// §14.7.3's NOTE 1 makes a role map "an approximate analogy between types", so a name two
/// sources map differently keeps the first source's mapping and the disagreement is warned about.
#[test]
fn a_role_map_the_sources_disagree_on_keeps_the_first_and_says_so() {
    let first = tagged_document("A", "H1", "Start");
    let second = tagged_document("B", "P", "Start");
    let (report, merged) = merge(&[(&first, "1"), (&second, "1")], false).expect("it merges");
    let said: String = report
        .warnings
        .iter()
        .map(|warning| warning.detail.clone())
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        said.contains("§14.7.3") && said.contains("/Title"),
        "the disagreement is reported by name: {said:?}"
    );
    let read = Document::open_with_limits(merged, Limits::DEFAULT).expect("re-read");
    let catalog = read.catalog().expect("a catalog");
    let root = read.get_key(&catalog, "StructTreeRoot");
    let root = root.as_dict().expect("a structure tree root");
    let roles = read.get_key(root, "RoleMap");
    let roles = roles.as_dict().expect("Table 354's /RoleMap");
    assert_eq!(
        read.get_key(roles, "Title")
            .as_name()
            .map(|name| name.as_bytes().to_vec()),
        Some(b"H1".to_vec()),
        "the first source's approximation wins"
    );
}

/// The marked-content identifiers in a carried content stream are **not** rewritten, and this is
/// the assertion that says why they need not be.
///
/// §14.7.5.2 makes an `/MCID` unique "within its content stream" and §14.7.5.4 makes it "a
/// zero-based index into the array" the stream's parent-tree key names. The stream crosses byte
/// for byte, so the property that has to hold is that the array is carried at its own length and
/// in its own order — index 0 still names the element it named. Both ends move together, and the
/// key itself is the only thing renumbered.
#[test]
fn the_marked_content_identifiers_are_not_rewritten_and_the_array_still_indexes_them() {
    let first = tagged_document("A", "H1", "Start");
    let second = tagged_document("B", "H1", "Start");
    let (_, merged) = merge(&[(&first, "1"), (&second, "1")], false).expect("it merges");
    let read = Document::open_with_limits(merged, Limits::DEFAULT).expect("re-read");
    let catalog = read.catalog().expect("a catalog");
    let root = read.get_key(&catalog, "StructTreeRoot");
    let root = root.as_dict().expect("a structure tree root");
    let tree = read.get_key(root, "ParentTree");
    let tree = tree.as_dict().expect("§14.7.5.4's parent tree");

    for (index, page) in support::page_dictionaries(&read).iter().enumerate() {
        // The source stated 7 and the output states its own key, which is the whole renumbering.
        let key = read
            .get_key(page, "StructParents")
            .as_integer()
            .expect("the carried page states a key");
        assert_eq!(
            key,
            i64::try_from(index).expect("two pages"),
            "the keys are the output's, assigned in page order"
        );
        let entry = pdf_syntax::tree::lookup_unresolved(
            tree,
            &pdf_syntax::tree::TreeKey::Number(key),
            &|value| read.resolve(value),
        )
        .expect("the key resolves");
        let Object::Array(items) = entry else {
            panic!("a content stream's parent-tree value is an array: {entry:?}");
        };
        assert_eq!(
            items.len(),
            1,
            "the source's array was one element long and the index is preserved"
        );
        // The stream's own `/MCID 0` therefore still selects a structure element, and it is the
        // one whose `/Pg` is this page.
        let element = items
            .first()
            .and_then(Object::as_reference)
            .expect("index 0");
        let Object::Dictionary(element) = read.get(element) else {
            panic!("index 0 names a structure element");
        };
        assert_eq!(
            element.get("Pg").and_then(Object::as_reference),
            page_id(&read, index),
            "and the element it names is on this page"
        );
    }
    // The identifier in the stream is untouched, which is what the two assertions above rest on.
    for page in support::page_dictionaries(&read) {
        let stream = read.get_key(&page, "Contents");
        let Object::Stream(stream) = stream else {
            panic!("a page states one content stream");
        };
        let bytes = read.decoded_stream_data(&stream).expect("it decodes");
        assert!(
            bytes.windows(6).any(|window| window == b"/MCID "),
            "the marked-content identifier crossed with the stream"
        );
    }
}

/// The object number of the page at this index, for the `/Pg` comparison above.
fn page_id(document: &Document, index: usize) -> Option<ObjectId> {
    Pages::new(document).get(index).and_then(|page| page.id)
}
