//! The transform gate: RFC 0002 section 12's perf floor, and exact outputs derived from the
//! document rather than from what the tool printed.
//!
//! Three things, over ISO 32000-2's own PDF, which every checkout has:
//!
//! 1. **Throughput, with a floor.** `render` of pages 1–200 at 150 dpi to PNG **through the
//!    program the build produced** (`CARGO_BIN_EXE_pdf-transform`, so a stale binary in another
//!    directory cannot be what is measured — trap 16), timed by the wall clock and held above
//!    [`PAGES_PER_SECOND_FLOOR`]. The number measured is printed, because a floor is only a
//!    floor: the eight-hundred-and-sixty-eighth session's baseline is in ADR 0801 and the
//!    module comment of `src/render.rs`, not here (`CLAUDE.md`'s "a fact that can be counted is
//!    not written down").
//! 2. **The pixels are the oracle's.** One of the two hundred pages, re-rendered independently
//!    through `interpret` + `render-cpu` in this test, is byte for byte what the program wrote
//!    (RFC 0002 section 9, layer 3).
//! 3. **Exact inventories.** `images` lists exactly the image `XObject`s a walk of the page
//!    tree's resources reaches (a walk written in `support/`, not the crate's), and
//!    `attachments` lists exactly the file names §12.5.6.15's annotations carry, page by page.
//!
//! Ignored by default because the floor is meaningless at debug speed; the sequence in
//! `doc/todo/02` §2 runs it under `--profile gates`.

#![expect(
    clippy::expect_used,
    clippy::panic,
    clippy::print_stdout,
    reason = "test code: a gate that cannot run its fixture has not found a defect, and a \
              measurement is printed rather than written down"
)]

mod support;

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;
use std::time::Instant;

use pdf_transform::attachments::{Action, AttachmentsPlan};
use pdf_transform::images::ImagesPlan;
use pdf_transform::range::Selection;
use pdf_transform::{Budget, Listed, MemorySinks, Plan, Policy, Source, apply};

/// The pages rendered for the floor: the first two hundred of ISO 32000-2, the same range
/// the baseline was taken over.
const PAGES: &str = "1-200";

/// How many that is.
const PAGE_COUNT: f64 = 200.0;

/// The floor, in pages per second of wall clock for the whole program run — process start,
/// open, two hundred pages interpreted, rasterised and PNG-encoded on rayon's default pool.
///
/// A fifth of the baseline on this machine's twenty-four threads, and above the single-thread
/// figure, so that a change that loses the cross-page parallelism trips it while a neighbouring
/// round's gate sequence on the same machine does not. Set from the measurement in ADR 0801
/// rather than invented; when the machine changes, the number is re-derived, not argued with
/// (trap 16's second rule).
const PAGES_PER_SECOND_FLOOR: f64 = 40.0;

/// The page held to the oracle, counted from 1: a page with text in several fonts and a
/// figure, which is what the shared font cache and the rasteriser both touch.
const ORACLE_PAGE: usize = 100;

/// Fails the gate if this build cannot reach the sandboxed image decoder.
///
/// `CCITTFaxDecode`, `JBIG2Decode` and `JPXDecode` are decoded by a separate program, and Cargo
/// does not build another package's binaries when it tests this one (trap 10). A build without
/// it draws every other image and none of those three, so the pages this gate times would be
/// different pages — and the oracle comparison below would agree with itself either way.
fn require_the_sandbox() {
    if let Err(error) = pdf_model::image::sandboxed_decoder() {
        panic!(
            "the sandboxed image decoder is not available, so the pages timed would be the build's rather than the tree's: {error}"
        );
    }
}

/// A fresh directory for the run's outputs.
fn scratch() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("pdf-transform-gate-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("the temporary directory is writable");
    dir
}

#[test]
#[ignore = "a perf floor, meaningless at debug speed; run under --profile gates"]
fn the_transform_gate() {
    require_the_sandbox();
    let path = support::committed("ISO_32000-2_sponsored_EC3.pdf");
    let bytes = std::fs::read(&path).expect("the specification's own PDF is committed");
    let dir = scratch();
    throughput(&path, &dir);
    pixels(&bytes, &dir);
    inventories(&bytes);
}

/// 1. Throughput, through the program, against the floor.
fn throughput(path: &Path, dir: &Path) {
    let started = Instant::now();
    let output = Command::new(env!("CARGO_BIN_EXE_pdf-transform"))
        .args([
            "render",
            path.to_str().expect("utf-8"),
            "--pages",
            PAGES,
            "-o",
            "page-%d.png",
        ])
        .current_dir(dir)
        .output()
        .expect("the program runs");
    let took = started.elapsed();
    assert!(
        output.status.success(),
        "render exited {:?}: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    let pages_per_second = PAGE_COUNT / took.as_secs_f64();
    println!(
        "transform: {PAGES} of ISO 32000-2 rendered at 150 dpi in {:.3} s: {pages_per_second:.1} pages/s (floor {PAGES_PER_SECOND_FLOOR}), {} threads",
        took.as_secs_f64(),
        rayon::current_num_threads()
    );
    assert!(
        pages_per_second >= PAGES_PER_SECOND_FLOOR,
        "render fell to {pages_per_second:.1} pages/s, under the floor of {PAGES_PER_SECOND_FLOOR}"
    );
}

/// 2. One page the program wrote is the oracle backend's raster.
fn pixels(bytes: &[u8], dir: &Path) {
    let written = std::fs::read(dir.join(format!("page-{ORACLE_PAGE}.png"))).expect("page written");
    let (width, height, data) = support::decode_png(&written);
    let expected = support::oracle(bytes, ORACLE_PAGE.saturating_sub(1));
    assert_eq!((width, height), (expected.width, expected.height));
    assert!(
        data == expected.data,
        "page {ORACLE_PAGE} through the program is not the oracle backend's raster"
    );
    println!("transform: page {ORACLE_PAGE} is the oracle backend's raster byte for byte");
}

/// 3. The inventories are exactly what the document's structure reaches.
fn inventories(bytes: &[u8]) {
    let document = pdf_syntax::Document::open(bytes.to_vec()).expect("a PDF");
    let expected_images: BTreeSet<String> = support::reachable_image_objects(&document)
        .iter()
        .map(ToString::to_string)
        .collect();
    let listing = apply(
        &Plan::Images(ImagesPlan {
            source: 0,
            pages: Selection::all(),
            min_pixels: 0,
            list_only: true,
            native: false,
            names: "%d".parse().expect("a pattern"),
        }),
        &[Source::new(bytes.to_vec())],
        &MemorySinks::new(),
        &Policy::default(),
        &Budget::default(),
    )
    .expect("lists");
    let mut listed_images = BTreeSet::new();
    let mut inline = 0_usize;
    for entry in &listing.listed {
        let Listed::Image(entry) = entry else {
            panic!("an attachment in an image listing");
        };
        if entry.inline {
            inline = inline.saturating_add(1);
        } else {
            assert!(
                listed_images.insert(entry.object.clone().expect("an XObject has an id")),
                "{entry:?} listed twice"
            );
        }
    }
    assert_eq!(
        listed_images, expected_images,
        "the image XObjects listed are not the ones the page tree's resources reach"
    );
    println!(
        "transform: {} image XObjects listed, every one the page tree reaches, and {inline} inline",
        listed_images.len()
    );

    let expected_files = support::annotation_file_names(&document);
    assert!(
        !expected_files.is_empty(),
        "the specification's PDF carries file attachment annotations"
    );
    let listing = apply(
        &Plan::Attachments(AttachmentsPlan {
            source: 0,
            action: Action::List,
        }),
        &[Source::new(bytes.to_vec())],
        &MemorySinks::new(),
        &Policy::default(),
        &Budget::default(),
    )
    .expect("lists");
    let from_annotations: Vec<(usize, String)> = listing
        .listed
        .iter()
        .filter_map(|entry| match entry {
            Listed::Attachment(entry) => entry
                .page
                .map(|page| (page, entry.file_name.clone().expect("a file name"))),
            Listed::Image(_) => panic!("an image in an attachment listing"),
        })
        .collect();
    assert_eq!(
        from_annotations, expected_files,
        "the annotation-borne files listed are not the ones the pages' /Annots carry"
    );
    println!(
        "transform: {} files listed from §12.5.6.15 annotations, every one the pages carry",
        from_annotations.len()
    );
}
