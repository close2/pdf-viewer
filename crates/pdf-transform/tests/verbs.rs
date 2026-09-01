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

mod support;

use std::path::{Path, PathBuf};
use std::process::Command;

use pdf_transform::attachments::{Action, AttachmentsPlan};
use pdf_transform::images::{ImageFile, ImagesPlan};
use pdf_transform::range::Selection;
use pdf_transform::render::{ImageFormat, RenderPlan, Sizing};
use pdf_transform::{
    Budget, Exit, Listed, MemorySinks, Origin, Output, Plan, Policy, Source, apply,
};

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

use support::{decode_png, oracle};

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
            native: false,
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
            native: false,
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
            native: false,
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

/// §8.9.7's inline image is listed and extracted: `issue11124.pdf` writes one whose data —
/// unfiltered, `/W 4 /H 4 /CS /RGB /BPC 8` — contains the bytes `EI` twenty-four bytes in, so
/// its end can only be found by §8.9.3's arithmetic (4 × 4 × 3 components × 1 byte = 48
/// bytes), never by searching for the operator. The data begins `000` `00z`: §8.9.3's samples,
/// first row first, each an RGB triple, so pixel (0, 0) is (0x30, 0x30, 0x30) and pixel (1, 0)
/// is (0x30, 0x30, 0x7a), opaque.
#[test]
fn an_inline_image_is_listed_at_its_placement_and_extracted() {
    let Some(path) = corpus("issue11124.pdf") else {
        eprintln!("skipped: the doc/pdf.js submodule is not checked out");
        return;
    };
    let bytes = std::fs::read(&path).expect("a corpus document");
    let listing = apply(
        &Plan::Images(ImagesPlan {
            source: 0,
            pages: Selection::all(),
            min_pixels: 0,
            list_only: true,
            native: false,
            names: "%d".parse().expect("a pattern"),
        }),
        &[Source::new(bytes.clone())],
        &MemorySinks::new(),
        &Policy::default(),
        &Budget::default(),
    )
    .expect("lists");
    assert!(listing.warnings.is_empty(), "{:?}", listing.warnings);
    let [Listed::Image(entry)] = listing.listed.as_slice() else {
        panic!("one inline image, listed once: {:?}", listing.listed);
    };
    assert!(entry.inline);
    assert_eq!(entry.object, None);
    assert_eq!((entry.width, entry.height), (4, 4));
    assert_eq!(entry.bits_per_component, Some(8));
    // Table 92's abbreviation `/RGB` expanded to the name the clause says it means.
    assert_eq!(entry.colour_space.as_deref(), Some("DeviceRGB"));
    assert!(entry.filters.is_empty());
    assert_eq!(entry.page, 1);

    let sinks = MemorySinks::new();
    let report = apply(
        &Plan::Images(ImagesPlan {
            source: 0,
            pages: Selection::all(),
            min_pixels: 0,
            list_only: false,
            native: false,
            names: "img-%d.png".parse().expect("a pattern"),
        }),
        &[Source::new(bytes)],
        &sinks,
        &Policy::default(),
        &Budget::default(),
    )
    .expect("extracts");
    assert_eq!(report.exit(false, false), Exit::Success, "{report:?}");
    assert!(matches!(
        report.outputs.as_slice(),
        [Output {
            origin: Origin::Image {
                inline: true,
                object: None,
                file: ImageFile::Png,
                ..
            },
            ..
        }]
    ));
    let outputs = sinks.into_outputs();
    let (width, height, data) = decode_png(&outputs[0].1);
    assert_eq!((width, height), (4, 4));
    assert_eq!(&data[0..8], &[0x30, 0x30, 0x30, 255, 0x30, 0x30, 0x7a, 255]);
}

/// `--native`: a `DCTDecode` image is written as the JPEG it is — the bytes begin with
/// ISO/IEC 10918-1's SOI marker `FF D8` — under `.jpg`, and every other image as decoded PNG
/// under `.png`, the report naming the file form; and where the codec has no standalone file
/// form the image is decoded and the report says so, per image.
#[test]
fn native_writes_the_codec_stream_where_it_is_a_file_and_says_so_where_it_is_not() {
    let path = committed("ISO_32000-2_sponsored_EC3.pdf");
    let bytes = std::fs::read(&path).expect("a committed document");
    let listing = apply(
        &Plan::Images(ImagesPlan {
            source: 0,
            pages: "100-130".parse().expect("a selection"),
            min_pixels: 0,
            list_only: true,
            native: true,
            names: "%d".parse().expect("a pattern"),
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
            Listed::Image(entry) => entry,
            Listed::Attachment(_) => panic!("an attachment in an image listing"),
        })
        .collect();
    let jpegs = entries
        .iter()
        .filter(|entry| entry.filters.last().is_some_and(|f| f == "DCTDecode"))
        .count();
    assert!(jpegs > 0, "pages 100 to 130 of the standard embed JPEGs");
    assert!(jpegs < entries.len(), "and images that are not JPEGs");

    let dir = scratch();
    std::fs::create_dir(dir.join("out")).expect("created");
    let (code, _stdout, stderr) = run(
        &dir,
        &[
            "images",
            path.to_str().expect("utf-8"),
            "--pages",
            "100-130",
            "--native",
            "-o",
            "out/img-%d",
        ],
    );
    assert_eq!(code, 0, "{stderr}");
    for (at, entry) in entries.iter().enumerate() {
        let is_jpeg = entry.filters.last().is_some_and(|f| f == "DCTDecode");
        let name = format!("out/img-{}.{}", at + 1, if is_jpeg { "jpg" } else { "png" });
        let written = std::fs::read(dir.join(&name)).expect(&name);
        if is_jpeg {
            assert_eq!(&written[..2], &[0xFF, 0xD8], "{name} is not a JPEG");
        } else {
            let (width, height, _) = decode_png(&written);
            assert_eq!((width, height), (entry.width, entry.height), "{name}");
        }
    }

    // A CCITT-encoded inline image has no file form: decoded, and the run says so.
    let Some(path) = corpus("images_1bit_grayscale.pdf") else {
        eprintln!("skipped the CCITT half: the doc/pdf.js submodule is not checked out");
        return;
    };
    let bytes = std::fs::read(&path).expect("a corpus document");
    let sinks = MemorySinks::new();
    let report = apply(
        &Plan::Images(ImagesPlan {
            source: 0,
            pages: Selection::all(),
            min_pixels: 0,
            list_only: false,
            native: true,
            names: "img-%d".parse().expect("a pattern"),
        }),
        &[Source::new(bytes)],
        &sinks,
        &Policy::default(),
        &Budget::default(),
    )
    .expect("extracts");
    assert_eq!(report.exit(false, false), Exit::Warnings, "{report:?}");
    assert!(
        report.warnings.iter().any(|warning| warning
            .detail
            .contains("CCITTFaxDecode has no standalone file form")),
        "{:?}",
        report.warnings
    );
    assert!(
        report.outputs.iter().all(|output| matches!(
            output.origin,
            Origin::Image {
                file: ImageFile::Png,
                ..
            }
        ) && Path::new(&output.name)
            .extension()
            .is_some_and(|ext| ext == "png")),
        "{:?}",
        report.outputs
    );
}

/// §12.5.6.15's file attachment annotations are the third home of an embedded file: the
/// specification's own PDF files nothing in its name tree and carries every one of its files
/// on an annotation, and each is listed with its page and saved with Table 46's `/Size` bytes.
#[test]
fn file_attachment_annotations_are_listed_with_their_page_and_saved() {
    let path = committed("ISO_32000-2_sponsored_EC3.pdf");
    let bytes = std::fs::read(&path).expect("a committed document");
    let document = pdf_syntax::Document::open(bytes.clone()).expect("a PDF");
    let expected = support::annotation_file_names(&document);
    assert!(!expected.is_empty());

    let sinks = MemorySinks::new();
    let report = apply(
        &Plan::Attachments(AttachmentsPlan {
            source: 0,
            action: Action::SaveAll {
                names: "%t".parse().expect("a pattern"),
            },
        }),
        &[Source::new(bytes)],
        &sinks,
        &Policy::default(),
        &Budget::default(),
    )
    .expect("saves");
    assert_eq!(report.exit(false, false), Exit::Success, "{report:?}");
    assert!(
        report
            .outputs
            .iter()
            .all(|output| matches!(output.origin, Origin::Attachment { .. })),
        "{:?}",
        report.outputs
    );
    let listing = apply(
        &Plan::Attachments(AttachmentsPlan {
            source: 0,
            action: Action::List,
        }),
        &[Source::new(std::fs::read(&path).expect("read"))],
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
    let listed: Vec<(usize, String)> = entries
        .iter()
        .map(|entry| {
            (
                entry.page.expect("every file here is an annotation's"),
                entry.file_name.clone().expect("a file name"),
            )
        })
        .collect();
    assert_eq!(listed, expected);
    let outputs = sinks.into_outputs();
    assert_eq!(outputs.len(), entries.len());
    for (entry, (name, written)) in entries.iter().zip(&outputs) {
        assert_eq!(
            *name,
            pdf_transform::pattern::sanitise(entry.file_name.as_deref().unwrap())
        );
        assert_eq!(i64::try_from(written.len()).ok(), entry.size, "{name}");
    }
}
