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

use pdf_model::Pages;
use pdf_syntax::{Document, Limits, Object};
use pdf_transform::range::Selection;
use pdf_transform::render::{ImageFormat, RenderPlan, Sizing};
use pdf_transform::split::{Pieces, SplitPlan};
use pdf_transform::{Budget, MemorySinks, Origin, Plan, Policy, Report, Source, apply};

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
#[test]
fn what_a_piece_does_not_carry_is_named() {
    let bytes = std::fs::read(committed("PDF20_AN001-BPC.pdf")).expect("a committed document");
    let source = Document::open_with_limits(bytes.clone(), Limits::DEFAULT).expect("it opens");
    let catalog = source.catalog().expect("a catalog");
    let stated: Vec<&str> = [
        "Outlines",
        "Names",
        "PageLabels",
        "StructTreeRoot",
        "Metadata",
    ]
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
