//! `optimize`: RFC 0002 section 6.5's verb, held to the clauses it writes against.
//!
//! The committed documents are the population — every checkout has them once
//! `doc/specifications.zip` is unpacked — and each test states one property of a rewritten
//! file: §7.5.5's closure with nothing unreachable in it, §7.5.7's Table 16 and its
//! prohibitions, §7.5.8's form forced by Table 18, the producer's *decoded* content unchanged
//! where the encoding was not, the file smaller, RFC 0002 section 9's byte determinism, and its
//! idempotence property gate.
//!
//! **The comparison here is of decoded content, and that is this verb's own question.**
//! `tests/split.rs` compares `/Contents` *encoded*, because `split` promises pass-through and a
//! decoded comparison would pass on a piece whose streams had been re-encoded. `optimize`
//! promises the opposite — `CLAUDE.md`'s "carried byte for byte **or recompressed without
//! reinterpretation**" — so what has to be identical is what the marks are made of, and the
//! encoded bytes are expected to differ.
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

use std::process::Command;

use pdf_model::Pages;
use pdf_syntax::serialize::{ObjectStreams, Streams};
use pdf_syntax::{Document, Limits, Object};
use pdf_transform::optimize::OptimizePlan;
use pdf_transform::render::{ImageFormat, RenderPlan, Sizing};
use pdf_transform::{Budget, MemorySinks, Origin, Plan, Policy, Report, Source, apply};

use support::{check_optimized, check_structure, committed};

/// The lossless default: every pass on.
fn default_plan() -> OptimizePlan {
    OptimizePlan {
        source: 0,
        names: "out.pdf".parse().expect("a pattern"),
        prune: true,
        object_streams: ObjectStreams::DEFAULT,
        streams: Streams::DEFAULT,
    }
}

/// Optimises `bytes` under `plan`, answering the report and the one output.
fn optimize_with(bytes: &[u8], plan: OptimizePlan) -> (Report, Vec<u8>) {
    let sinks = MemorySinks::new();
    let report = apply(
        &Plan::Optimize(plan),
        &[Source::new(bytes.to_vec())],
        &sinks,
        &Policy::default(),
        &Budget::default(),
    )
    .expect("the rewrite applies");
    let mut outputs = sinks.into_outputs();
    assert_eq!(outputs.len(), 1, "one input, one output");
    (report, outputs.remove(0).1)
}

/// The same, under the lossless default.
fn optimize(bytes: &[u8]) -> (Report, Vec<u8>) {
    optimize_with(bytes, default_plan())
}

/// Every page's `/Contents`, decoded and concatenated, in page order.
///
/// §7.8.2 makes an array of content streams one stream — "the division between streams may
/// occur only at the boundaries between lexical tokens" — so a comparison of the concatenation
/// is a comparison of the page's marks whatever shape the producer stored them in.
fn decoded_contents(document: &Document) -> Vec<Vec<u8>> {
    let pages = Pages::new(document);
    (0..pages.len())
        .map(|index| {
            let mut out = Vec::new();
            let Some(id) = pages.get(index).and_then(|page| page.id) else {
                return out;
            };
            let contents = document
                .get_key_of(id, "Contents")
                .map(|value| document.resolve(&value));
            let parts = match contents {
                Some(Object::Array(items)) => items,
                Some(other) => vec![other],
                None => Vec::new(),
            };
            for part in &parts {
                if let Some(stream) = document.resolve(part).as_stream()
                    && let Some(data) = document.decoded_stream_data(stream)
                {
                    out.extend_from_slice(&data);
                }
            }
            out
        })
        .collect()
}

/// Page `index` of `bytes` as a PPM, or `None` where nothing was drawn.
fn draw(bytes: &[u8], index: usize) -> Option<Vec<u8>> {
    let sinks = MemorySinks::new();
    apply(
        &Plan::Render(RenderPlan {
            source: 0,
            pages: format!("{}", index.saturating_add(1))
                .parse()
                .expect("a selection"),
            size: Sizing::Dpi(72.0),
            format: ImageFormat::Ppm,
            page_box: None,
            annotations: true,
            names: "page.ppm".parse().expect("a pattern"),
            // The unconfined default: this suite draws in its own process (ADR 0847).
            strips: None,
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
    let directory = std::env::temp_dir().join(format!(
        "pdfv-optimize-{}-{}",
        std::process::id(),
        bytes.len()
    ));
    std::fs::create_dir_all(&directory).ok()?;
    let path = directory.join("out.pdf");
    std::fs::write(&path, bytes).ok()?;
    let output = Command::new("qpdf").arg("--check").arg(&path).output().ok();
    let _ = std::fs::remove_dir_all(&directory);
    Some(output?.status.success())
}

/// The rewritten document holds the same pages, drawn the same way, and is smaller.
#[test]
fn a_rewritten_document_is_the_same_document_and_fewer_bytes() {
    let bytes = std::fs::read(committed("PDF20_AN001-BPC.pdf")).expect("a committed document");
    let source = Document::open_with_limits(bytes.clone(), Limits::DEFAULT).expect("it opens");
    let count = Pages::new(&source).len();

    let (report, out) = optimize(&bytes);
    assert!(
        out.len() < bytes.len(),
        "a verb called optimize wrote {} bytes over {}",
        out.len(),
        bytes.len()
    );
    let read = Document::open_with_limits(out.clone(), Limits::DEFAULT).expect("the output opens");
    assert_eq!(Pages::new(&read).len(), count, "the pages are all there");

    // RFC 0002 section 9 layer 3: a page a lossless transform carried draws identically.
    for index in 0..count {
        assert_eq!(
            draw(&out, index),
            draw(&bytes, index),
            "page {} draws differently",
            index.saturating_add(1)
        );
    }

    let Some(Origin::Optimized { savings, .. }) =
        report.outputs.first().map(|output| output.origin.clone())
    else {
        panic!("the report states an optimized origin");
    };
    assert_eq!(savings.after, out.len() as u64);
    assert!(savings.before > savings.after);
    assert!(
        savings.object_streams >= 1 && savings.compressed >= 1,
        "the default generates object streams: {savings:?}"
    );
}

/// §7.4: the encoding changes and the marks do not.
#[test]
fn the_decoded_content_is_the_producers_and_the_encoding_need_not_be() {
    let bytes = std::fs::read(committed("PDF20_AN001-BPC.pdf")).expect("a committed document");
    let source = Document::open_with_limits(bytes.clone(), Limits::DEFAULT).expect("it opens");
    let (_, out) = optimize(&bytes);
    let read = Document::open_with_limits(out, Limits::DEFAULT).expect("the output opens");
    assert_eq!(
        decoded_contents(&read),
        decoded_contents(&source),
        "a content stream was reinterpreted, which CLAUDE.md's amended exclusion does not permit"
    );
}

/// §7.5.5 and §7.5.7, asked of the output rather than of the writer.
#[test]
fn the_output_states_a_closed_object_graph_and_conforming_object_streams() {
    for name in ["PDF20_AN001-BPC.pdf", "PDF20_AN002-AF.pdf"] {
        let bytes = std::fs::read(committed(name)).expect("a committed document");
        let (_, out) = optimize(&bytes);
        let read = Document::open_with_limits(out, Limits::DEFAULT).expect("the output opens");
        let check = check_optimized(&read);
        assert!(check.faults.is_empty(), "{name}: {:?}", check.faults);
        assert!(
            check.unreachable.is_empty(),
            "{name}: objects nothing reaches survived pruning: {:?}",
            check.unreachable
        );
        assert!(check.object_streams >= 1, "{name}: no object stream");
    }
}

/// §7.5.7's carriers force §7.5.8's form and NOTE 3's version, and `disable` leaves both alone.
#[test]
fn object_streams_force_a_cross_reference_stream_and_the_clauses_version() {
    let bytes = std::fs::read(committed("PDF20_AN001-BPC.pdf")).expect("a committed document");

    let (_, packed) = optimize(&bytes);
    let read = Document::open_with_limits(packed, Limits::DEFAULT).expect("the output opens");
    // §7.5.8.1: a cross-reference stream's own dictionary is the trailer, and its `/Type` is
    // `/XRef`; a classic table's trailer has no `/Type` at all.
    assert_eq!(
        read.trailer()
            .get("Type")
            .and_then(Object::as_name)
            .map(|name| name.as_bytes().to_vec()),
        Some(b"XRef".to_vec()),
        "Table 18's type 2 entry exists only in a cross-reference stream"
    );
    let header = read.header_version().expect("a header version");
    assert!(
        (header.major, header.minor) >= (1, 5),
        "§7.5.7 NOTE 3: \"[u]se of compressed objects requires a PDF 1.5 PDF reader\", and the \
         header states {header}"
    );

    let (_, plain) = optimize_with(
        &bytes,
        OptimizePlan {
            object_streams: ObjectStreams::Disable,
            ..default_plan()
        },
    );
    let read = Document::open_with_limits(plain, Limits::DEFAULT).expect("the output opens");
    assert_eq!(check_optimized(&read).object_streams, 0);
}

/// RFC 0002 section 9's property gate: optimising an optimised file changes nothing.
#[test]
fn optimising_an_optimised_file_is_byte_identical() {
    for name in ["PDF20_AN001-BPC.pdf", "PDF20_AN002-AF.pdf"] {
        let bytes = std::fs::read(committed(name)).expect("a committed document");
        let (_, once) = optimize(&bytes);
        let (_, twice) = optimize(&once);
        assert_eq!(once, twice, "{name}: optimize is not idempotent");
    }
}

/// RFC 0002 section 9 layer 1: the same input under the same plan is the same file.
#[test]
fn the_same_document_rewritten_twice_is_the_same_bytes() {
    let bytes = std::fs::read(committed("PDF20_AN001-BPC.pdf")).expect("a committed document");
    let (_, first) = optimize(&bytes);
    let (_, second) = optimize(&bytes);
    assert_eq!(first, second, "the rewrite is not deterministic");
}

/// Each pass can be switched off, and each of them is what saves what it claims to.
#[test]
fn every_pass_is_switchable_and_each_one_saves_something() {
    let bytes = std::fs::read(committed("PDF20_AN001-BPC.pdf")).expect("a committed document");
    let (_, nothing) = optimize_with(
        &bytes,
        OptimizePlan {
            prune: false,
            object_streams: ObjectStreams::Disable,
            streams: Streams::Carry,
            ..default_plan()
        },
    );
    let (_, pruned) = optimize_with(
        &bytes,
        OptimizePlan {
            object_streams: ObjectStreams::Disable,
            streams: Streams::Carry,
            ..default_plan()
        },
    );
    let (_, packed) = optimize_with(
        &bytes,
        OptimizePlan {
            streams: Streams::Carry,
            ..default_plan()
        },
    );
    let (_, everything) = optimize(&bytes);
    assert!(
        pruned.len() < nothing.len(),
        "pruning saved nothing: {} against {}",
        pruned.len(),
        nothing.len()
    );
    assert!(
        packed.len() < pruned.len(),
        "§7.5.7's object streams saved nothing: {} against {}",
        packed.len(),
        pruned.len()
    );
    assert!(
        everything.len() < packed.len(),
        "recompression saved nothing: {} against {}",
        everything.len(),
        packed.len()
    );
}

/// A stream that fails to shrink keeps what its producer wrote.
///
/// qpdf's rule for `--optimize-images`, applied to every stream: the output of a verb called
/// `optimize` is never larger than what it was given for the same construct. The fixture is a
/// document whose streams are already `FlateDecode` at a level this tree cannot beat, so
/// `recompression_saved` is the number that has to be honest rather than large.
#[test]
fn a_stream_that_does_not_shrink_is_carried_and_the_report_says_so() {
    let bytes = std::fs::read(committed("PDF20_AN001-BPC.pdf")).expect("a committed document");
    let (report, once) = optimize(&bytes);
    let Some(Origin::Optimized { savings, .. }) =
        report.outputs.first().map(|output| output.origin.clone())
    else {
        panic!("an optimized origin");
    };
    assert!(savings.recompressed > 0, "nothing was recompressed at all");

    // The second run's streams are this writer's own, already deflated at the same level, so
    // nothing can shrink further and nothing is counted as recompressed.
    let (again, _) = optimize(&once);
    let Some(Origin::Optimized { savings, .. }) =
        again.outputs.first().map(|output| output.origin.clone())
    else {
        panic!("an optimized origin");
    };
    assert_eq!(
        savings.recompressed, 0,
        "a stream already at this level was re-encoded and counted"
    );
    assert_eq!(savings.recompression_saved, 0);
}

/// §14.7's structure tree crosses a rewrite, because a rewrite copies rather than rebuilds.
#[test]
fn a_tagged_document_keeps_its_tagging() {
    let bytes = std::fs::read(committed("PDF20_AN001-BPC.pdf")).expect("a committed document");
    let source = Document::open_with_limits(bytes.clone(), Limits::DEFAULT).expect("it opens");
    let before = check_structure(&source);
    assert!(before.carried, "the fixture is tagged");

    let (_, out) = optimize(&bytes);
    let read = Document::open_with_limits(out, Limits::DEFAULT).expect("the output opens");
    let after = check_structure(&read);
    assert!(after.carried, "the tree did not survive the rewrite");
    assert!(after.faults.is_empty(), "{:?}", after.faults);
    assert_eq!(
        after.elements, before.elements,
        "a rewrite carries every element, because it carries every object the catalog reaches"
    );
}

/// Foreign evidence, principle 5's register: what qpdf makes of a file this program wrote.
#[test]
fn qpdf_reads_back_what_this_writer_wrote() {
    let bytes = std::fs::read(committed("PDF20_AN001-BPC.pdf")).expect("a committed document");
    let (_, out) = optimize(&bytes);
    match qpdf_accepts(&out) {
        Some(true) => {}
        Some(false) => panic!("qpdf --check refused the rewritten document"),
        None => eprintln!("skipped: qpdf is not installed"),
    }
}
