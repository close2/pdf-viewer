//! `split`: RFC 0002 section 6.1's verb over §10's serializer, held to the clauses it writes against.
//!
//! The committed documents are the population — every checkout has them once
//! `doc/specifications.zip` is unpacked — and each test states one property of a piece:
//! §7.7.3.4's inheritance flattened onto the page, the producer's content stream carried byte
//! for byte, a reference out of the piece written as §7.3.10's null and reported,
//! determinism, and the page drawn bit-identically to the source's (RFC 0002 section 9 layer 3).
//!
//! `qpdf --check`, where it is installed, is **evidence about the reading and never its
//! definition** (principle 5): what it says about a file this program wrote raises or lowers
//! confidence that the writer-side clauses were read right, and a disagreement is a question
//! for the standard.

#![expect(
    clippy::expect_used,
    clippy::panic,
    clippy::print_stderr,
    reason = "test code: a fixture that cannot exercise the rule must fail loudly, and a \
              skipped test says so"
)]

mod support;

use std::process::Command;

use std::collections::BTreeSet;

use pdf_model::Pages;
use pdf_model::destination::Destination;
use pdf_model::outline::Outline;
use pdf_model::page_label::PageLabels;
use pdf_model::retrieval::sections;
use pdf_syntax::{Document, Limits, Object};
use pdf_transform::range::Selection;
use pdf_transform::render::{ImageFormat, RenderPlan, Sizing};
use pdf_transform::split::{Pieces, SplitPlan};
use pdf_transform::{Budget, MemorySinks, Origin, Plan, Policy, Refusal, Report, Source, apply};

use support::committed;

/// Splits `bytes`, answering the report and every piece by name.
fn split(bytes: &[u8], pages: &str, pieces: Pieces) -> (Report, Vec<(String, Vec<u8>)>) {
    let sinks = MemorySinks::new();
    let report = apply(
        &Plan::Split(SplitPlan {
            source: 0,
            pages: pages.parse::<Selection>().expect("a selection"),
            pieces,
            names: "piece-%d.pdf".parse().expect("a pattern"),
        }),
        &[Source::new(bytes.to_vec())],
        &sinks,
        &Policy::default(),
        &Budget::default(),
    )
    .expect("the split applies");
    (report, sinks.into_outputs())
}

/// One named output of a split, by the name the pattern gave it.
///
/// **By name, never by position.** `split` writes its pieces across rayon and [`MemorySinks`]
/// keeps them "in the order they were opened", which is thread order; three of these tests
/// indexed the vector instead and one of them failed on a run where the second piece finished
/// first — a test of the scheduler wearing a gate's name (trap 30).
fn piece(outputs: &[(String, Vec<u8>)], name: &str) -> Vec<u8> {
    outputs
        .iter()
        .find(|(written, _)| written == name)
        .map_or_else(
            || panic!("no piece named {name}: {:?}", names(outputs)),
            |(_, bytes)| bytes.clone(),
        )
}

/// Every output's name, for a failure to print.
fn names(outputs: &[(String, Vec<u8>)]) -> Vec<String> {
    outputs.iter().map(|(name, _)| name.clone()).collect()
}

/// Page 1 of `bytes` as a PPM, or `None` where nothing was drawn.
fn draw(bytes: &[u8]) -> Option<Vec<u8>> {
    let sinks = MemorySinks::new();
    apply(
        &Plan::Render(RenderPlan {
            source: 0,
            pages: "1".parse::<Selection>().expect("a selection"),
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

/// `qpdf --check` on these bytes, where qpdf is installed: `Some(accepted)`.
fn qpdf_accepts(bytes: &[u8]) -> Option<bool> {
    let directory =
        std::env::temp_dir().join(format!("pdfv-split-{}-{}", std::process::id(), bytes.len()));
    std::fs::create_dir_all(&directory).ok()?;
    let path = directory.join("piece.pdf");
    std::fs::write(&path, bytes).ok()?;
    let output = Command::new("qpdf").arg("--check").arg(&path).output().ok();
    let _ = std::fs::remove_dir_all(&directory);
    Some(output?.status.success())
}

/// Every page becomes a file, each holding exactly the page it names.
#[test]
fn a_document_becomes_one_file_per_page_and_each_holds_its_own() {
    let bytes = std::fs::read(committed("PDF20_AN001-BPC.pdf")).expect("a committed document");
    let source = Document::open_with_limits(bytes.clone(), Limits::DEFAULT).expect("it opens");
    let count = Pages::new(&source).len();
    assert!(count >= 1, "the fixture has pages");

    let (report, outputs) = split(&bytes, "1-end", Pieces::EachPage);
    assert_eq!(outputs.len(), count, "one file per page");
    assert_eq!(report.outputs.len(), count);
    assert!(report.refused.is_empty(), "{:?}", report.refused);

    for ordinal in 0..count {
        let name = format!("piece-{}.pdf", ordinal.saturating_add(1));
        let read = Document::open_with_limits(piece(&outputs, &name), Limits::DEFAULT)
            .unwrap_or_else(|error| panic!("{name}: does not open: {error}"));
        assert_eq!(Pages::new(&read).len(), 1, "{name}: one page");
        let origin = &report.outputs[ordinal].origin;
        assert!(
            matches!(
                origin,
                Origin::Piece { first_page, pages, .. }
                    if *first_page == ordinal + 1 && *pages == 1
            ),
            "{name}: {origin:?}"
        );
    }
}

/// ISO 32000-2 §7.7.3.4's four inheritable attributes are written onto the emitted page.
///
/// > If such an attribute is omitted from a page object, its value shall be inherited from an
/// > ancestor node in the page tree.
///
/// The ancestors are not in the piece, so an attribute one of them carried has to be on the
/// page or the piece has lost it — and a `/MediaBox` lost is a page whose size is a reader's
/// guess. What is asserted is the *result*: the piece's page states its own media box, and it
/// is the one the source's page inherited.
#[test]
fn the_inheritable_attributes_are_flattened_onto_every_piece() {
    let bytes = std::fs::read(committed("PDF20_AN001-BPC.pdf")).expect("a committed document");
    let source = Document::open_with_limits(bytes.clone(), Limits::DEFAULT).expect("it opens");
    let before = Pages::new(&source).get(0).expect("page 1").media_box;

    let (_, outputs) = split(&bytes, "1", Pieces::EachPage);
    let piece = &outputs[0].1;
    let read = Document::open_with_limits(piece.clone(), Limits::DEFAULT).expect("it opens");
    let page = Pages::new(&read).get(0).expect("page 1");
    // The comparison is exact on purpose and the values are not computed: both sides read the
    // same four numbers out of the same `/MediaBox` array, so a difference is a lost or
    // rewritten entry rather than an arithmetic one.
    #[expect(
        clippy::float_cmp,
        reason = "both sides are the same four numbers read out of the file, never computed"
    )]
    {
        assert_eq!(page.media_box, before, "the media box came across");
    }
    assert!(
        page.dict.get("MediaBox").is_some(),
        "and it is on the page itself, since no ancestor of it is in the piece"
    );
    assert!(
        page.dict.get("Resources").is_some(),
        "§7.7.3.4's /Resources likewise"
    );
    assert_eq!(
        page.dict.get("Parent").and_then(Object::as_reference),
        Some(pdf_syntax::ObjectId::new(2, 0)),
        "Table 30's /Parent names the piece's own tree, which takes the second number"
    );
}

/// RFC 0002 section 11.1: "every content stream in their output is a producer's, carried byte for
/// byte". The comparison is of the *encoded* bytes, because a re-encoded stream would pass a
/// comparison of decoded ones.
#[test]
fn the_producers_content_stream_crosses_byte_for_byte() {
    let bytes = std::fs::read(committed("PDF20_AN001-BPC.pdf")).expect("a committed document");
    let source = Document::open_with_limits(bytes.clone(), Limits::DEFAULT).expect("it opens");
    let before = encoded_contents(&source).expect("page 1 states contents");

    let (_, outputs) = split(&bytes, "1", Pieces::EachPage);
    let read = Document::open_with_limits(outputs[0].1.clone(), Limits::DEFAULT).expect("it opens");
    let after = encoded_contents(&read).expect("the piece states contents");
    assert_eq!(before, after, "the stream was re-encoded on the way out");
}

/// Page 1's `/Contents` as the file holds it, encoded.
fn encoded_contents(document: &Document) -> Option<Vec<u8>> {
    let page = Pages::new(document).get(0)?;
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

/// RFC 0002 section 9's layer 3, and the load-bearing one: the piece's page draws as the source's did.
#[test]
fn a_piece_draws_bit_identically_to_the_page_it_came_from() {
    let bytes = std::fs::read(committed("PDF20_AN001-BPC.pdf")).expect("a committed document");
    let (_, outputs) = split(&bytes, "1", Pieces::EachPage);
    let before = draw(&bytes).expect("the source page draws");
    let after = draw(&outputs[0].1).expect("the piece draws");
    assert_eq!(
        before, after,
        "the same content stream, resources and boxes shall mark the same pixels"
    );
}

/// RFC 0002 section 9's first layer: same source, same plan, same bytes, with no flag needed.
#[test]
fn the_same_split_twice_writes_the_same_bytes() {
    let bytes = std::fs::read(committed("PDF20_AN001-BPC.pdf")).expect("a committed document");
    let (_, once) = split(&bytes, "1", Pieces::EachPage);
    let (_, twice) = split(&bytes, "1", Pieces::EachPage);
    assert_eq!(once, twice);
}

/// `--pages 1-2,3` writes one file per comma-separated group, and `--every n` writes chunks.
#[test]
fn the_cuts_are_where_the_grammar_and_the_flag_say_they_are() {
    let bytes = std::fs::read(committed("PDF20_AN001-BPC.pdf")).expect("a committed document");
    let source = Document::open_with_limits(bytes.clone(), Limits::DEFAULT).expect("it opens");
    let count = Pages::new(&source).len();
    if count < 3 {
        eprintln!("skipped: the fixture has {count} pages and this needs three");
        return;
    }
    let (_, groups) = split(&bytes, "1-2,3", Pieces::Groups);
    assert_eq!(groups.len(), 2, "one piece per comma-separated group");
    let first =
        Document::open_with_limits(piece(&groups, "piece-1.pdf"), Limits::DEFAULT).expect("opens");
    assert_eq!(Pages::new(&first).len(), 2);
    let second =
        Document::open_with_limits(piece(&groups, "piece-2.pdf"), Limits::DEFAULT).expect("opens");
    assert_eq!(Pages::new(&second).len(), 1);

    let (_, chunks) = split(&bytes, "1-3", Pieces::Every(2));
    assert_eq!(chunks.len(), 2, "three pages in twos is two pieces");
    let last =
        Document::open_with_limits(piece(&chunks, "piece-2.pdf"), Limits::DEFAULT).expect("opens");
    assert_eq!(
        Pages::new(&last).len(),
        1,
        "the last piece is the remainder"
    );
}

/// The document-level constructs a piece does not carry are named in a warning, never dropped
/// in silence — trap 5, and RFC 0002 section 6.1's "not silently".
///
/// `/Outlines`, `/Names`, `/Dests` and `/PageLabels` **left this list in session 910** and have
/// their own tests below; what stays here is what `split` still leaves behind.
#[test]
fn what_a_piece_does_not_carry_is_named() {
    let bytes = std::fs::read(committed("PDF20_AN001-BPC.pdf")).expect("a committed document");
    let source = Document::open_with_limits(bytes.clone(), Limits::DEFAULT).expect("it opens");
    let catalog = source.catalog().expect("a catalog");
    let stated: Vec<&str> = ["Metadata", "Threads", "Collection", "Perms"]
        .into_iter()
        .filter(|key| catalog.get(key).is_some())
        .collect();
    let (report, _) = split(&bytes, "1", Pieces::EachPage);
    if stated.is_empty() {
        eprintln!("skipped: the fixture states none of the constructs a piece leaves behind");
        return;
    }
    let said: String = report
        .warnings
        .iter()
        .map(|warning| warning.detail.clone())
        .collect::<Vec<_>>()
        .join(" ");
    for key in stated {
        assert!(
            said.contains(key),
            "the report does not name /{key}: {said:?}"
        );
    }
}

/// §12.4.2: a piece's labels are the labels its pages had, keyed by the *piece's* own indices.
///
/// The clause makes a page index "the page's relative position within the document" and requires
/// the number tree to "include a value for page index 0", so a piece that carried the source's
/// tree unchanged would state one with no value for its own first page. What is asserted is the
/// property that follows: page *k* of the piece is labelled what the source page it came from
/// was labelled.
#[test]
fn a_pieces_page_labels_are_its_own_indices_and_its_sources_labels() {
    let bytes = std::fs::read(committed("PDF20_AN002-AF.pdf")).expect("a committed document");
    let source = Document::open_with_limits(bytes.clone(), Limits::DEFAULT).expect("it opens");
    let before = PageLabels::read(&source);
    if before.is_empty() {
        eprintln!("skipped: the fixture states no §12.4.2 labels");
        return;
    }
    let (_, outputs) = split(&bytes, "4-6", Pieces::Groups);
    let read =
        Document::open_with_limits(piece(&outputs, "piece-1.pdf"), Limits::DEFAULT).expect("opens");
    let after = PageLabels::read(&read);
    assert!(
        !after.is_empty(),
        "the source labels its pages and the piece labels none"
    );
    for (position, index) in (3..6_usize).enumerate() {
        assert_eq!(
            after.label(position),
            before.label(index),
            "piece page {position} came from source page {index}"
        );
    }
}

/// §12.3.3: the piece's outline is the subset that reaches its pages, and every item in it
/// resolves to a page the piece holds.
///
/// The strong half is the second clause of that sentence: an outline carried whole would name
/// pages the piece does not have, and Table 151's hierarchy would still read as valid. What
/// discriminates is asking the *output* where its items go.
#[test]
fn a_pieces_outline_resolves_only_to_pages_the_piece_holds() {
    let bytes = std::fs::read(committed("PDF20_AN002-AF.pdf")).expect("a committed document");
    let source = Document::open_with_limits(bytes.clone(), Limits::DEFAULT).expect("it opens");
    let pages = Pages::new(&source);
    if Outline::read(&source, &pages).is_empty() {
        eprintln!("skipped: the fixture states no §12.3.3 outline");
        return;
    }
    let (_, outputs) = split(&bytes, "5-7", Pieces::Groups);
    let read =
        Document::open_with_limits(piece(&outputs, "piece-1.pdf"), Limits::DEFAULT).expect("opens");
    let held = Pages::new(&read);
    assert_eq!(held.len(), 3);
    let outline = Outline::read(&read, &held);
    assert!(
        !outline.is_empty(),
        "three pages of a document with an outline and the piece carries none"
    );
    let carried = sections(&read, &held, &outline);
    assert!(
        !carried.is_empty(),
        "the piece states an outline whose items resolve nowhere"
    );
    for section in &carried {
        assert!(
            section.first_page < held.len(),
            "{:?} resolves to page {} of a {}-page piece",
            section.title,
            section.first_page,
            held.len()
        );
    }
    // Table 151: "The parent of a top-level item shall be the outline dictionary itself", and
    // `/Prev` is "( Required for all but the first item at each level )" — so the first item of
    // the rebuilt chain states none.
    let catalog = read.catalog().expect("a catalog");
    let root = read.get_key(&catalog, "Outlines");
    let root = root.as_dict().expect("an outline dictionary");
    let first = root
        .get("First")
        .and_then(Object::as_reference)
        .expect("/First");
    let item = read.get(first);
    let item = item.as_dict().expect("an outline item");
    assert!(
        item.get("Prev").is_none(),
        "the chain's first item states /Prev"
    );
    assert_eq!(
        item.get("Parent").and_then(Object::as_reference),
        catalog.get("Outlines").and_then(Object::as_reference),
        "a top-level item's /Parent is the outline dictionary itself"
    );
}

/// §12.3.2.4: the named destinations a piece keeps are the ones that resolve inside it.
///
/// A name is not an indirect reference, so §7.3.10's null cannot stand in for one that names a
/// page the piece does not hold — which is why the entry is dropped rather than carried.
#[test]
fn a_pieces_named_destinations_all_resolve_inside_it() {
    let bytes = std::fs::read(committed("Well-Tagged-PDF-WTPDF-1.0.pdf")).expect("committed");
    let source = Document::open_with_limits(bytes.clone(), Limits::DEFAULT).expect("it opens");
    let catalog = source.catalog().expect("a catalog");
    let names = source.get_key(&catalog, "Names");
    let stated = names
        .as_dict()
        .map(|names| source.get_key(names, "Dests"))
        .and_then(|root| {
            root.as_dict()
                .map(|root| pdf_syntax::tree::name_entries(root, &|o| source.resolve(o)).len())
        })
        .unwrap_or_default();
    if stated == 0 {
        eprintln!("skipped: the fixture states no §12.3.2.4 name tree");
        return;
    }
    let (_, outputs) = split(&bytes, "6-12", Pieces::Groups);
    let read =
        Document::open_with_limits(piece(&outputs, "piece-1.pdf"), Limits::DEFAULT).expect("opens");
    let held = Pages::new(&read);
    let catalog = read.catalog().expect("a catalog");
    let names = read.get_key(&catalog, "Names");
    let names = names.as_dict().expect("the piece states a name dictionary");
    let root = read.get_key(names, "Dests");
    let root = root.as_dict().expect("the piece states a /Dests tree");
    let kept = pdf_syntax::tree::name_entries(root, &|o| read.resolve(o));
    assert!(!kept.is_empty(), "seven pages and not one destination kept");
    assert!(
        kept.len() < stated,
        "a seven-page piece of a {}-page document kept all {stated} destinations",
        Pages::new(&source).len()
    );
    for (key, value) in &kept {
        let landed = Destination::read(&read, value)
            .and_then(|destination| destination.page_index(&read, &held));
        assert!(
            landed.is_some_and(|index| index < held.len()),
            "{:?} resolves to {landed:?} in a {}-page piece",
            String::from_utf8_lossy(key),
            held.len()
        );
    }
}

/// RFC 0002 section 6.1's `--at-bookmarks`: a piece begins where an outline item at the stated
/// depth or shallower lands, and the pieces cover the selection exactly once.
#[test]
fn at_bookmarks_cuts_where_the_outline_lands_and_loses_no_page() {
    let bytes = std::fs::read(committed("PDF20_AN002-AF.pdf")).expect("a committed document");
    let source = Document::open_with_limits(bytes.clone(), Limits::DEFAULT).expect("it opens");
    let pages = Pages::new(&source);
    let outline = Outline::read(&source, &pages);
    let marks: BTreeSet<usize> = sections(&source, &pages, &outline)
        .into_iter()
        .filter(|section| section.depth == 0)
        .map(|section| section.first_page)
        .collect();
    if marks.len() < 2 {
        eprintln!("skipped: the fixture's outline names fewer than two pages at level 1");
        return;
    }
    let (report, outputs) = split(&bytes, "1-end", Pieces::AtBookmarks(1));
    // One piece per mark, plus a leading one where the first mark is not the first page.
    let expected = marks.len() + usize::from(!marks.contains(&0));
    assert_eq!(outputs.len(), expected, "{:?}", names(&outputs));

    let mut covered = Vec::new();
    for output in &report.outputs {
        let Origin::Piece {
            first_page, pages, ..
        } = output.origin
        else {
            panic!("a split writes pieces");
        };
        covered.extend(first_page..first_page + pages);
    }
    covered.sort_unstable();
    assert_eq!(
        covered,
        (1..=Pages::new(&source).len()).collect::<Vec<_>>(),
        "the pieces cover every page exactly once"
    );

    // Every piece but the first begins on a marked page.
    let mut starts: Vec<usize> = report
        .outputs
        .iter()
        .filter_map(|output| match output.origin {
            Origin::Piece { first_page, .. } => Some(first_page - 1),
            _ => None,
        })
        .collect();
    starts.sort_unstable();
    for start in starts.iter().skip(usize::from(!marks.contains(&0))) {
        assert!(
            marks.contains(start),
            "a piece begins on unmarked page {start}"
        );
    }
}

/// `--at-bookmarks` on a document whose outline names no page is refused by name, never answered
/// with one piece that cut nowhere.
#[test]
fn at_bookmarks_without_an_outline_is_refused_by_name() {
    let sinks = MemorySinks::new();
    let empty = apply(
        &Plan::Split(SplitPlan {
            source: 0,
            pages: "1-end".parse::<Selection>().expect("a selection"),
            pieces: Pieces::AtBookmarks(1),
            names: "piece-%d.pdf".parse().expect("a pattern"),
        }),
        &[Source::new(
            std::fs::read(committed("PDF-Declarations.pdf")).expect("a committed document"),
        )],
        &sinks,
        &Policy::default(),
        &Budget::default(),
    );
    match empty {
        Err(Refusal::NoBookmarks { at, depth }) => {
            assert_eq!((at, depth), (0, 1));
            assert_eq!(empty_exit(), 2, "§12.3.3 says nowhere to cut, so exit 2");
        }
        Ok(_) => eprintln!(
            "skipped: this fixture's outline does resolve at level 1, so nothing is refused"
        ),
        Err(other) => panic!("--at-bookmarks answered {other}"),
    }
}

/// The status [`Refusal::NoBookmarks`] carries, named where the test above reads it.
fn empty_exit() -> u8 {
    Refusal::NoBookmarks { at: 0, depth: 1 }.exit().code()
}

/// Foreign evidence, in principle 5's register: `qpdf --check` accepts what this program wrote.
///
/// Agreement raises confidence that §7.5.4, §7.5.5 and §14.4 were read right on the way out; it
/// is never the definition of right, and a disagreement would be a question for the standard.
#[test]
fn qpdf_accepts_a_piece_this_program_wrote() {
    let bytes = std::fs::read(committed("PDF20_AN001-BPC.pdf")).expect("a committed document");
    let (_, outputs) = split(&bytes, "1", Pieces::EachPage);
    match qpdf_accepts(&outputs[0].1) {
        Some(true) => {}
        Some(false) => panic!("qpdf --check refused a piece this program wrote"),
        None => eprintln!("skipped: qpdf is not installed"),
    }
}
