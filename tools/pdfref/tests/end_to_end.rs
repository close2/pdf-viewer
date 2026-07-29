//! Spike E: the comparison harness, end to end, before a parser exists.
//!
//! # How this can work without a PDF parser
//!
//! `test_scenes` holds the same page twice: [`test_scenes::basic`] as a display list,
//! and [`test_scenes::basic_pdf`] as PDF bytes. The harness renders the PDF with three
//! external renderers and the display list with our own backend, then compares.
//!
//! That pairing is a deliberate stand-in. Today the display list is written by hand
//! alongside the PDF; once `pdf-syntax` exists it will be *produced from* the PDF, and
//! this test becomes a real end-to-end check with no change to the harness. Building it
//! now means the comparison plumbing — renderer invocation, format normalisation,
//! metrics, triangulation, artefacts — is proven before any PDF-specific code depends
//! on it.
//!
//! The risk being bought off is real: a harness written after the parser tends to be
//! tuned until it passes, rather than trusted to say when something is wrong.

#![expect(
    // Fires here, unlike in the GPU tests: these `expect`s live in helper functions
    // rather than inside `#[test]` fns, which is where clippy's test allowance applies.
    clippy::expect_used,
    clippy::print_stdout,
    reason = "test code: `expect` in helpers is the intended failure mode, and the \
              measurements are worth printing since the thresholds derive from them"
)]

use pdf_render::{Rasterizer, TargetSpec};
use pdfref::{Outcome, Reference, Tolerance, report};
use render_cpu::CpuRasterizer;

/// Comparison resolution. 72 dpi means one pixel per PDF unit, so a mismatch is a
/// mismatch rather than a resampling artefact.
const DPI: u32 = 72;

/// The same resolution expressed as our scale factor, in pixels per PDF unit.
///
/// Stated rather than divided out of [`DPI`], because the conversion needs a lossy
/// integer-to-float cast for a value that is a constant here anyway. The two are tied
/// together by `dpi_and_scale_agree`.
const SCALE: f32 = 1.0;

/// The fixture has one page, and it is page one.
const PAGE_ONE: u32 = 1;

/// Pixel budget; far above an A4 page at this resolution.
const GENEROUS: u64 = 1 << 30;

/// Writes the fixture PDF to the work directory and returns its path.
fn fixture(work_dir: &std::path::Path) -> std::path::PathBuf {
    std::fs::create_dir_all(work_dir).expect("work directory is writable");
    let path = work_dir.join("basic.pdf");
    std::fs::write(&path, test_scenes::basic_pdf()).expect("fixture is writable");
    path
}

/// Renders the display-list half with our CPU backend.
fn ours() -> pdf_render::Raster {
    let list = test_scenes::basic();
    let target = TargetSpec::for_page(&list, SCALE, GENEROUS).expect("A4 target is valid");
    CpuRasterizer::new()
        .rasterize(&list, target)
        .expect("basic scene is supported")
}

/// Guards the pair of constants above against drifting apart.
#[test]
fn dpi_and_scale_agree() {
    assert_eq!(DPI, 72, "SCALE below assumes 72 dpi");
    assert!(
        (SCALE - 1.0).abs() < f32::EPSILON,
        "one pixel per PDF unit at 72 dpi"
    );
}

/// The full pipeline: our render against three independent implementations.
#[test]
fn our_render_agrees_with_the_reference_consensus() {
    let work_dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("pdfref");
    let pdf = fixture(&work_dir);

    let available = Reference::available();
    assert!(
        available.len() >= 2,
        "only {} reference renderer(s) available: {:?}\n\
         At least two are needed to triangulate — a single reference cannot distinguish \
         our bug from its own. Install: {}",
        available.len(),
        available,
        Reference::ALL
            .iter()
            .filter(|r| !r.is_available())
            .map(|r| r.package_hint())
            .collect::<Vec<_>>()
            .join(", ")
    );

    // Versions belong in the record: reference renderers change their output between
    // releases, so a difference appearing tomorrow may be an upstream change rather than
    // our regression.
    for reference in &available {
        println!(
            "{}: {}",
            reference.name(),
            reference.version().unwrap_or_default()
        );
    }

    let mut references = Vec::new();
    for reference in available {
        let raster = reference
            .render(&pdf, PAGE_ONE, DPI, &work_dir)
            .unwrap_or_else(|e| panic!("{reference} failed to render the fixture: {e}"));
        references.push((reference, raster));
    }

    let ours = ours();
    let triangulation = pdfref::triangulate(&ours, &references, &Tolerance::DEFAULT)
        .expect("all renders should share the page size");

    // Printed unconditionally: on success these numbers justify the thresholds, and on
    // failure they are the first thing anyone needs.
    print!("{}", report::summarise("basic", &triangulation));

    let artefacts = report::write_artefacts(&work_dir, "basic", &ours, &references, &triangulation)
        .expect("artefacts are writable");
    for path in &artefacts {
        println!("artefact: {}", path.display());
    }

    match &triangulation.outcome {
        Outcome::Agrees { with } => {
            assert!(
                with.len() >= 2,
                "consensus must rest on at least two references"
            );
        }
        other => panic!(
            "expected agreement with the reference consensus, got {other:?}\n\
             See the artefacts listed above."
        ),
    }
}

/// Every reference must agree on the *page size*, independently of pixels.
///
/// This is checked separately and first because a size disagreement is a different and
/// more serious class of bug than a pixel disagreement — it means someone read the
/// `MediaBox` differently — and because it would otherwise surface as an opaque
/// comparison error.
#[test]
fn every_renderer_agrees_on_the_page_size() {
    let work_dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("pdfref-size");
    let pdf = fixture(&work_dir);

    let expected = (595, 842);
    let ours = ours();
    assert_eq!((ours.width, ours.height), expected, "our own render");

    for reference in Reference::available() {
        let raster = reference
            .render(&pdf, PAGE_ONE, DPI, &work_dir)
            .unwrap_or_else(|e| panic!("{reference} failed: {e}"));
        assert_eq!(
            (raster.width, raster.height),
            expected,
            "{reference} disagrees about the page size"
        );
    }
}

/// A renderer that outlives its budget must be killed and reported, not waited on.
///
/// The budget here is a millisecond against a render that takes hundreds — a margin of
/// two orders of magnitude, so this is not a race — because the only way to observe the
/// bound from outside is to set one the work cannot meet. Without it a corpus run over a
/// thousand untrusted files has no way to survive the first one built to loop.
#[test]
fn a_renderer_that_outlives_its_budget_is_killed() {
    let work_dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("pdfref-budget");
    let pdf = fixture(&work_dir);

    let reference = Reference::Poppler;
    assert!(
        reference.is_available(),
        "{reference} is required for this test ({})",
        reference.package_hint()
    );

    // 600 dpi over A4 is 35 megapixels, which no renderer produces in a millisecond.
    let error = reference
        .render_within(
            &pdf,
            PAGE_ONE,
            600,
            &work_dir,
            std::time::Duration::from_millis(1),
        )
        .expect_err("a millisecond is not enough to render 35 megapixels");
    let message = error.to_string();
    assert!(
        message.contains("exceeded"),
        "the budget must be named as the cause, got: {message}"
    );
    // The variant, not only the wording. `pdfref::cache` decides whether to remember a
    // failure by matching on it, and a timeout that arrived as `RendererFailed` would be
    // stored — pinning a page as unrenderable because the machine was busy once.
    assert!(
        matches!(error, pdfref::HarnessError::RendererTimedOut { .. }),
        "a timeout must be its own outcome, got: {error:?}"
    );
}

/// The harness must reject a corrupt file rather than silently comparing nothing.
#[test]
fn a_corrupt_pdf_is_reported_as_a_renderer_failure() {
    let work_dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("pdfref-corrupt");
    std::fs::create_dir_all(&work_dir).expect("work directory is writable");
    let path = work_dir.join("corrupt.pdf");
    std::fs::write(&path, b"this is not a PDF at all").expect("writable");

    // Poppler is the strictest of the three about a file with no header at all.
    let reference = Reference::Poppler;
    assert!(
        reference.is_available(),
        "{reference} is required for this test ({})",
        reference.package_hint()
    );

    let result = reference.render(&path, PAGE_ONE, DPI, &work_dir);
    assert!(
        result.is_err(),
        "a file that is not a PDF must be an error, not an empty comparison"
    );
}

/// A cached reference render must be the render, not a rendering of it.
///
/// This is the claim `pdfref::cache` has to earn, and it is checked the only way that
/// settles it: the same page is rendered without a cache, then with one twice, and all three
/// rasters must be byte-identical. The middle run is the miss that stores the entry and the
/// last is the hit that reads it, so a hit that differed in a single pixel — a re-encoding, a
/// colour type normalised twice, a truncated file trusted — would fail here rather than
/// quietly moving a verdict in the oracle.
#[test]
fn a_hit_reproduces_what_the_renderer_produced() {
    let work_dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("pdfref-cache-test");
    let pdf = fixture(&work_dir);

    let reference = Reference::Poppler;
    assert!(
        reference.is_available(),
        "{reference} is required for this test ({})",
        reference.package_hint()
    );

    let uncached = reference
        .render(&pdf, PAGE_ONE, DPI, &work_dir)
        .expect("the fixture renders");

    let cache = pdfref::Cache::at(work_dir.join("entries"));
    cache
        .clear()
        .expect("an empty cache directory is removable");

    let missed = cache
        .render(reference, &pdf, PAGE_ONE, DPI, &work_dir)
        .expect("the fixture renders");
    assert_eq!(
        cache.statistics(),
        pdfref::cache::Statistics {
            hits: 0,
            misses: 1,
            remembered_timeouts: 0
        },
        "the first request for an entry cannot be a hit"
    );

    let hit = cache
        .render(reference, &pdf, PAGE_ONE, DPI, &work_dir)
        .expect("the stored entry is readable");
    assert_eq!(
        cache.statistics(),
        pdfref::cache::Statistics {
            hits: 1,
            misses: 1,
            remembered_timeouts: 0
        },
        "the second request for the same entry must be answered from disk"
    );

    assert_eq!(
        (uncached.width, uncached.height),
        (hit.width, hit.height),
        "a cached render must have the size the renderer produced"
    );
    assert_eq!(
        uncached.data, missed.data,
        "storing an entry must not change what the caller is given"
    );
    assert_eq!(
        uncached.data, hit.data,
        "a cached render must be the render, pixel for pixel"
    );

    // The evidence directory must look the same whether a page was cached or not: the
    // side-by-side artefacts are what anybody diagnosing a disagreement opens.
    assert!(
        work_dir.join(format!("{}.png", reference.name())).is_file(),
        "a hit must leave the renderer's own output where an uncached run left it"
    );
}

/// The page box must be part of what a cache entry is keyed on.
///
/// Not a test of hashing — a test that the *signature* the key is built from carries the one
/// variable this project has already been bitten by. Trap 3 in `doc/HANDOVER.md`: two of the
/// three renderers default to the media box, which put 54 documents beyond comparison until
/// they were told otherwise. A cache keyed on a signature that omitted those flags would
/// answer a corrected invocation with a render made under the old one, and nothing would say
/// so.
#[test]
fn the_cache_key_carries_the_page_box_flags() {
    let work_dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("pdfref-signature");
    let pdf = fixture(&work_dir);

    for (reference, flag) in [
        (Reference::Poppler, "-cropbox"),
        (Reference::Ghostscript, "-dUseCropBox"),
        (Reference::MuPdf, "CropBox"),
    ] {
        let signature = reference.command_signature(&pdf, PAGE_ONE, DPI, &work_dir);
        assert!(
            signature.iter().any(|word| word == flag),
            "{reference}'s cache key must carry {flag}, got {signature:?}"
        );
        // The two paths that vary by page must not be in it, or every page of every
        // document would be its own entry and nothing would ever hit.
        assert!(
            !signature
                .iter()
                .any(|word| word.contains(&*work_dir.to_string_lossy())),
            "{reference}'s signature must not carry the work directory: {signature:?}"
        );
    }
}
