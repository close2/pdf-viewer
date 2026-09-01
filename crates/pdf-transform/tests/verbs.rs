//! The three verbs against documents already in the tree, through the seam and through the
//! program: exit statuses, the report, and — the load-bearing one — that a rendered page is
//! byte for byte what `render-cpu` draws, so the tool cannot become a fourth rasteriser.
//!
//! The documents are the committed ones under `doc/` (trap 4: real documents, not fragments)
//! plus one corpus document for the restriction test, skipped when the submodule is absent
//! exactly as `crates/pdf-syntax/tests/encryption.rs` skips it.

#![expect(
    clippy::expect_used,
    clippy::print_stderr,
    reason = "test code: a fixture that cannot exercise the rule must fail loudly, and a \
              skipped test says so"
)]

use std::path::{Path, PathBuf};
use std::process::Command;

use pdf_model::content::FontCache;
use pdf_model::view::ViewState;
use pdf_render::{Rasterizer as _, TargetSpec};
use pdf_transform::attachments::{Action, AttachmentsPlan};
use pdf_transform::images::ImagesPlan;
use pdf_transform::range::Selection;
use pdf_transform::render::{ImageFormat, RenderPlan, Sizing};
use pdf_transform::{Budget, Exit, Listed, MemorySinks, Origin, Plan, Policy, Source, apply};

/// A committed document, which every checkout has once `doc/specifications.zip` is unpacked.
fn committed(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../doc")
        .join(name)
}

/// A corpus document's path, or `None` when the submodule is not checked out.
fn corpus(name: &str) -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../doc/pdf.js/test/pdfs")
        .join(name);
    path.exists().then_some(path)
}

/// A fresh directory for one test's outputs, named by process and counter so that the
/// harness's threads do not share one.
fn scratch() -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let serial = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "pdf-transform-test-{}-{serial}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).expect("the temporary directory is writable");
    dir
}

/// Runs the program with these arguments in `dir`, answering (exit code, stdout, stderr).
fn run(dir: &Path, arguments: &[&str]) -> (i32, Vec<u8>, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_pdf-transform"))
        .args(arguments)
        .current_dir(dir)
        .output()
        .expect("the program runs");
    (
        output.status.code().expect("the program exited"),
        output.stdout,
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

/// The oracle backend's raster of one page at 150 dpi — the pipeline
/// `crates/pdf-model/examples/render_at.rs` runs, stated here independently of the crate.
fn oracle(bytes: &[u8], index: usize) -> pdf_render::Raster {
    let document = pdf_syntax::Document::open(bytes.to_vec()).expect("a PDF");
    let pages = pdf_model::Pages::new(&document);
    let page = pages.get(index).expect("that page");
    let view = ViewState::of(&document);
    let interpretation =
        pdf_model::content::interpret_with_fonts(&document, &page, &view, &FontCache::new());
    let list = interpretation.display_list;
    // ISO 32000-2 §8.3.2.3: 72 user-space units to the inch, so 150 dpi is 150/72.
    let target = TargetSpec::for_page(&list, 150.0 / 72.0, 1 << 28).expect("a target");
    render_cpu::CpuRasterizer::new()
        .rasterize(&list, target)
        .expect("drawn")
}

/// Decodes a PNG to its RGBA8 samples.
fn decode_png(bytes: &[u8]) -> (u32, u32, Vec<u8>) {
    let decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    let mut reader = decoder.read_info().expect("a PNG");
    let mut data = vec![0; reader.output_buffer_size().expect("a bounded size")];
    let info = reader.next_frame(&mut data).expect("a frame");
    assert_eq!(info.color_type, png::ColorType::Rgba);
    assert_eq!(info.bit_depth, png::BitDepth::Eight);
    data.truncate(info.buffer_size());
    (info.width, info.height, data)
}

/// The rendered page is `render-cpu`'s own raster, through the seam and through the program,
/// and the two agree byte for byte — RFC 0002 section 9's determinism, and the reason the tool is not a
/// fourth rasteriser.
#[test]
fn a_rendered_page_is_the_oracle_backends_raster_byte_for_byte() {
    let path = committed("PDF20_AN001-BPC.pdf");
    let bytes = std::fs::read(&path).expect("a committed document");
    let expected = oracle(&bytes, 0);

    // Through the seam.
    let sinks = MemorySinks::new();
    let plan = Plan::Render(RenderPlan {
        source: 0,
        pages: "1".parse().expect("a selection"),
        size: Sizing::Dpi(150.0),
        format: ImageFormat::Png,
        names: "page-%d.png".parse().expect("a pattern"),
    });
    let report = apply(
        &plan,
        &[Source::new(bytes.clone())],
        &sinks,
        &Policy::default(),
        &Budget::default(),
    )
    .expect("the plan applies");
    assert_eq!(report.exit(false, false), Exit::Success, "{report:?}");
    assert_eq!(report.outputs.len(), 1);
    assert!(matches!(
        report.outputs[0].origin,
        Origin::Page {
            page: 1,
            width,
            height,
            ..
        } if width == expected.width && height == expected.height
    ));
    let outputs = sinks.into_outputs();
    assert_eq!(outputs[0].0, "page-1.png");
    let (width, height, data) = decode_png(&outputs[0].1);
    assert_eq!((width, height), (expected.width, expected.height));
    assert_eq!(
        data, expected.data,
        "the seam's PNG is not the oracle's raster"
    );

    // Through the program, to a file and to stdout, both the same bytes as the seam's.
    let dir = scratch();
    let (code, stdout, stderr) = run(
        &dir,
        &[
            "render",
            path.to_str().expect("utf-8"),
            "--pages",
            "1",
            "-o",
            "out-%d.png",
        ],
    );
    assert_eq!(code, 0, "{stderr}");
    assert!(
        stdout.is_empty(),
        "stdout carries nothing without -o - or --report"
    );
    let written = std::fs::read(dir.join("out-1.png")).expect("the program wrote the page");
    assert_eq!(
        written, outputs[0].1,
        "the program's bytes are not the seam's"
    );
    let (code, stdout, stderr) = run(
        &dir,
        &[
            "render",
            path.to_str().expect("utf-8"),
            "--pages",
            "1",
            "-o",
            "-",
        ],
    );
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(stdout, written, "-o - is not the same bytes");
}

/// `--scale-to` and `--format ppm` produce what they say: the longer side at the size asked
/// for, and a `P6` header over three bytes a pixel.
#[test]
fn scale_to_and_ppm() {
    let path = committed("PDF20_AN001-BPC.pdf");
    let dir = scratch();
    let (code, _stdout, stderr) = run(
        &dir,
        &[
            "render",
            path.to_str().expect("utf-8"),
            "--pages",
            "1",
            "--scale-to",
            "400",
            "--format",
            "ppm",
            "-o",
            "page.ppm",
        ],
    );
    assert_eq!(code, 0, "{stderr}");
    let ppm = std::fs::read(dir.join("page.ppm")).expect("written");
    let header_end = ppm
        .windows(4)
        .position(|window| window == b"255\n")
        .expect("a P6 header")
        + 4;
    let header = std::str::from_utf8(&ppm[..header_end]).expect("ascii");
    let mut fields = header.split_whitespace();
    assert_eq!(fields.next(), Some("P6"));
    let width: usize = fields.next().expect("width").parse().expect("a number");
    let height: usize = fields.next().expect("height").parse().expect("a number");
    assert_eq!(width.max(height), 400, "{header}");
    assert_eq!(ppm.len() - header_end, width * height * 3);
}

/// RFC 0002 section 4.4's statuses, each from the condition it names.
#[test]
fn exit_statuses() {
    let path = committed("PDF20_AN001-BPC.pdf");
    let path = path.to_str().expect("utf-8");
    let dir = scratch();

    // 1: argument parsing — no verb, an unknown verb, an unknown option, a bad selection, and
    // more than one output with no %d.
    for arguments in [
        vec![],
        vec!["explode", path, "-o", "x"],
        vec!["render", path, "--loud", "-o", "x"],
        vec!["render", path, "--pages", "0", "-o", "x"],
        vec!["render", path, "--pages", "1-2", "-o", "same.png"],
        vec!["render", path, "--report=json", "-o", "-"],
    ] {
        let (code, _stdout, stderr) = run(&dir, &arguments);
        assert_eq!(code, 1, "{arguments:?}: {stderr}");
        assert!(stderr.starts_with("usage:"), "{arguments:?}: {stderr}");
    }

    // 2: the file — unreadable, or a page it does not have.
    let (code, _stdout, stderr) = run(&dir, &["render", "no-such.pdf", "-o", "x.png"]);
    assert_eq!(code, 2, "{stderr}");
    let (code, _stdout, stderr) = run(&dir, &["render", path, "--pages", "999", "-o", "x.png"]);
    assert_eq!(code, 2, "{stderr}");
    assert!(stderr.contains("past the end"), "{stderr}");
    let (code, _stdout, stderr) = run(&dir, &["render", path, "--pages", "@zz", "-o", "x.png"]);
    assert_eq!(code, 2, "{stderr}");
    assert!(stderr.contains("labelled"), "{stderr}");

    // 4: refused by name — the budget will not admit the page, and the refusal names it while
    // the run still completes.
    let (code, stdout, stderr) = run(
        &dir,
        &[
            "render",
            path,
            "--pages",
            "1",
            "--max-pixels",
            "1",
            "--report=json",
            "-o",
            "x.png",
        ],
    );
    assert_eq!(code, 4, "{stderr}");
    assert!(stderr.starts_with("refused: x.png:"), "{stderr}");
    let report = String::from_utf8(stdout).expect("json");
    assert!(report.contains("\"refused\": [\n    {"), "{report}");
    assert!(!dir.join("x.png").exists(), "a refused page was written");

    // 0 with a usage-free `--help`, on stdout.
    let (code, stdout, _stderr) = run(&dir, &["--help"]);
    assert_eq!(code, 0);
    assert!(String::from_utf8_lossy(&stdout).starts_with("usage:"));
}

/// Under `--restrictions=on` a document whose Table 22 bit 5 is clear refuses extraction with
/// exit 4; under `warn` it extracts and says so; by default it extracts silently — the reader's.
/// Listing is not extraction and is never restricted.
#[test]
fn restrictions_have_levels() {
    // `/P -1084`, bit 5 clear, and the empty password is the *user* password — the same document
    // `crates/pdf-syntax/tests/encryption.rs` reads Table 22 from.
    let Some(path) = corpus("bug1815476.pdf") else {
        eprintln!("skipped: doc/pdf.js is not checked out");
        return;
    };
    let path = path.to_str().expect("utf-8");
    let dir = scratch();
    // Extraction of embedded files rather than of images, so that the answer does not depend
    // on the codec worker being built beside the test binary (trap 10): this document's images
    // are CCITT, and the worker's absence is its own refusal, by name, which is not the one
    // under test here.
    let (code, _stdout, stderr) = run(
        &dir,
        &[
            "attachments",
            path,
            "--restrictions=on",
            "--save-all",
            "-o",
            "files/",
        ],
    );
    assert_eq!(code, 4, "{stderr}");
    assert!(stderr.contains("Table 22 bit 5"), "{stderr}");
    let (code, _stdout, stderr) = run(
        &dir,
        &[
            "attachments",
            path,
            "--restrictions=warn",
            "--save-all",
            "-o",
            "files/",
        ],
    );
    assert_eq!(code, 3, "{stderr}");
    assert!(
        stderr.starts_with("warning: this document restricts"),
        "{stderr}"
    );
    let (code, _stdout, stderr) = run(&dir, &["attachments", path, "--save-all", "-o", "files/"]);
    assert_eq!(code, 0, "{stderr}");
    // Listing is not extraction, so it is not restricted even at `on`.
    let (code, stdout, stderr) = run(&dir, &["images", path, "--restrictions=on", "--list"]);
    assert_eq!(code, 0, "{stderr}");
    assert!(!stdout.is_empty(), "the document places images");
}

/// `images`: the inventory names every image `XObject` the pages reach, each object once, and
/// extraction writes a PNG of the stated size for each.
#[test]
fn images_are_listed_and_extracted() {
    let path = committed("Tagged-PDF-Best-Practice-Guide.pdf");
    let bytes = std::fs::read(&path).expect("a committed document");
    let listing = apply(
        &Plan::Images(ImagesPlan {
            source: 0,
            pages: Selection::all(),
            min_pixels: 0,
            list_only: true,
            names: "%d".parse().expect("a pattern"),
        }),
        &[Source::new(bytes.clone())],
        &MemorySinks::new(),
        &Policy::default(),
        &Budget::default(),
    )
    .expect("lists");
    assert!(listing.outputs.is_empty());
    let entries: Vec<_> = listing
        .listed
        .iter()
        .map(|listed| match listed {
            Listed::Image(entry) => entry,
            Listed::Attachment(_) => panic!("an attachment in an image listing"),
        })
        .collect();
    assert!(!entries.is_empty(), "the guide embeds images");
    let objects: std::collections::BTreeSet<_> = entries.iter().map(|e| &e.object).collect();
    assert_eq!(objects.len(), entries.len(), "an object was listed twice");
    // `--min-pixels` leaves the small ones out.
    let floor = entries
        .iter()
        .map(|e| u64::from(e.width) * u64::from(e.height))
        .max()
        .unwrap();
    let largest_only = apply(
        &Plan::Images(ImagesPlan {
            source: 0,
            pages: Selection::all(),
            min_pixels: floor,
            list_only: true,
            names: "%d".parse().expect("a pattern"),
        }),
        &[Source::new(bytes.clone())],
        &MemorySinks::new(),
        &Policy::default(),
        &Budget::default(),
    )
    .expect("lists");
    assert!(largest_only.listed.len() < entries.len());

    let sinks = MemorySinks::new();
    let report = apply(
        &Plan::Images(ImagesPlan {
            source: 0,
            pages: Selection::all(),
            min_pixels: 0,
            list_only: false,
            names: "img-%d.png".parse().expect("a pattern"),
        }),
        &[Source::new(bytes)],
        &sinks,
        &Policy::default(),
        &Budget::default(),
    )
    .expect("extracts");
    assert_eq!(report.exit(false, false), Exit::Success, "{report:?}");
    assert_eq!(report.outputs.len(), entries.len());
    let outputs = sinks.into_outputs();
    for (entry, output) in entries.iter().zip(&report.outputs) {
        let (_, png) = outputs
            .iter()
            .find(|(name, _)| *name == output.name)
            .expect("written");
        let (width, height, _) = decode_png(png);
        assert_eq!(
            (width, height),
            (entry.width, entry.height),
            "{}",
            output.name
        );
        assert!(matches!(&output.origin, Origin::Image { page, .. } if *page == entry.page));
    }
}

/// `attachments`: listed, saved all into a directory under their own names, and saved one by
/// name — with the report naming what was sanitised.
#[test]
fn attachments_are_listed_and_saved() {
    let path = committed("PDF-Declarations.pdf");
    let bytes = std::fs::read(&path).expect("a committed document");
    let listing = apply(
        &Plan::Attachments(AttachmentsPlan {
            source: 0,
            action: Action::List,
        }),
        &[Source::new(bytes.clone())],
        &MemorySinks::new(),
        &Policy::default(),
        &Budget::default(),
    )
    .expect("lists");
    let entries: Vec<_> = listing
        .listed
        .iter()
        .map(|listed| match listed {
            Listed::Attachment(entry) => entry,
            Listed::Image(_) => panic!("an image in an attachment listing"),
        })
        .collect();
    assert!(
        !entries.is_empty(),
        "the declarations document embeds files"
    );
    // A filing name with a colon in it is the sanitisation case, by a real document.
    assert!(
        entries.iter().any(|entry| entry.name.contains(':')),
        "{entries:?}"
    );

    let sinks = MemorySinks::new();
    let report = apply(
        &Plan::Attachments(AttachmentsPlan {
            source: 0,
            action: Action::SaveAll {
                names: "%t".parse().expect("a pattern"),
            },
        }),
        &[Source::new(bytes.clone())],
        &sinks,
        &Policy::default(),
        &Budget::default(),
    )
    .expect("saves");
    assert_eq!(report.exit(false, false), Exit::Success, "{report:?}");
    assert_eq!(report.outputs.len(), entries.len());
    for (entry, output) in entries.iter().zip(&report.outputs) {
        assert_eq!(
            output.name,
            pdf_transform::pattern::sanitise(entry.file_name.as_deref().unwrap_or(&entry.name))
        );
        assert!(!output.name.contains(['/', ':']), "{}", output.name);
        assert_eq!(Some(i64::try_from(output.bytes).unwrap()), entry.size);
    }
    // The saved bytes are the embedded file's: the first is itself a PDF, and opens.
    let outputs = sinks.into_outputs();
    let embedded_pdf = outputs
        .iter()
        .find(|(name, _)| Path::new(name).extension().is_some_and(|ext| ext == "pdf"))
        .expect("an embedded PDF");
    pdf_syntax::Document::open(embedded_pdf.1.clone()).expect("the embedded file is a PDF");
}

/// One embedded file by its own name, through the program, into a directory: `-o dir/` is
/// `dir/%t`; a directory that does not exist is the machine's failure (2), and a name the
/// document does not file is the file's (2).
#[test]
fn one_attachment_is_saved_by_name() {
    let path = committed("PDF-Declarations.pdf");
    let bytes = std::fs::read(&path).expect("a committed document");
    let listing = apply(
        &Plan::Attachments(AttachmentsPlan {
            source: 0,
            action: Action::List,
        }),
        &[Source::new(bytes)],
        &MemorySinks::new(),
        &Policy::default(),
        &Budget::default(),
    )
    .expect("lists");
    let Some(Listed::Attachment(first)) = listing.listed.first() else {
        panic!("the declarations document embeds files");
    };
    let dir = scratch();
    let wanted = first.file_name.as_deref().unwrap_or(&first.name);
    let (code, _stdout, stderr) = run(
        &dir,
        &[
            "attachments",
            path.to_str().expect("utf-8"),
            "--save",
            wanted,
            "-o",
            "files/",
        ],
    );
    assert_eq!(
        code, 2,
        "a directory that does not exist is the machine's: {stderr}"
    );
    std::fs::create_dir(dir.join("files")).expect("created");
    let (code, _stdout, stderr) = run(
        &dir,
        &[
            "attachments",
            path.to_str().expect("utf-8"),
            "--save",
            wanted,
            "-o",
            "files/",
        ],
    );
    assert_eq!(code, 0, "{stderr}");
    let saved = std::fs::read(
        dir.join("files")
            .join(pdf_transform::pattern::sanitise(wanted)),
    )
    .expect("saved under its own name");
    assert_eq!(i64::try_from(saved.len()).ok(), first.size);
    let (code, _stdout, stderr) = run(
        &dir,
        &[
            "attachments",
            path.to_str().expect("utf-8"),
            "--save",
            "no such file",
            "-o",
            "files/",
        ],
    );
    assert_eq!(code, 2, "{stderr}");
    assert!(stderr.contains("no embedded file is named"), "{stderr}");
}
