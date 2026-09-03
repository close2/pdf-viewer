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

use std::collections::BTreeSet;
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
        page_box: None,
        annotations: true,
        names: "page-%d.png".parse().expect("a pattern"),
        strips: None,
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

/// `--format pgm`: a `P5` header over one byte a pixel, and every byte the grey ISO 32000-2
/// §10.4.2.2 states for the oracle raster's pixel — "computed according to the NTSC video
/// standard", `0.3 × red + 0.59 × green + 0.11 × blue` — written out here in `f64` from the
/// clause rather than through the crate. A correctly rounded byte is within half a level of
/// the exact value, so the bound is that, plus a thousandth for the arithmetic's own error;
/// a conversion that truncated, or weighed the channels otherwise, is a whole level out on
/// most of a page.
#[test]
fn pgm_is_the_clauses_grey_of_the_oracles_raster() {
    let path = committed("PDF20_AN001-BPC.pdf");
    let bytes = std::fs::read(&path).expect("a committed document");
    let expected = oracle(&bytes, 0);
    let dir = scratch();
    let (code, _stdout, stderr) = run(
        &dir,
        &[
            "render",
            path.to_str().expect("utf-8"),
            "--pages",
            "1",
            "--format",
            "pgm",
            "-o",
            "page.pgm",
        ],
    );
    assert_eq!(code, 0, "{stderr}");
    let pgm = std::fs::read(dir.join("page.pgm")).expect("written");
    let header = format!("P5\n{} {}\n255\n", expected.width, expected.height);
    assert!(pgm.starts_with(header.as_bytes()), "{:?}", &pgm[..20]);
    let samples = &pgm[header.len()..];
    assert_eq!(samples.len(), expected.data.len() / 4);
    let mut greys = BTreeSet::new();
    for (pixel, &grey) in expected.data.chunks_exact(4).zip(samples) {
        let exact =
            0.3 * f64::from(pixel[0]) + 0.59 * f64::from(pixel[1]) + 0.11 * f64::from(pixel[2]);
        assert!(
            (f64::from(grey) - exact).abs() <= 0.501,
            "pixel {pixel:?}: the clause says {exact:.3}, the file says {grey}"
        );
        greys.insert(grey);
    }
    assert!(
        greys.len() > 2,
        "page 1 has text and a figure, so more than two greys"
    );
}

/// `images --format pgm` and `--format ppm`: the decoded image in the netpbm form asked for,
/// the PGM being §10.4.2.2's grey of the PNG route's RGB pixel for pixel; and, where the image
/// has a mask, the mask beside it as a `P5` PGM — a netpbm file has no alpha — whether the
/// image beside it is grey or RGB.
#[test]
fn images_in_a_netpbm_form_are_the_decoded_pixels_with_the_mask_beside() {
    let path = committed("ISO_32000-2_sponsored_EC3.pdf");
    let bytes = std::fs::read(&path).expect("a committed document");
    let extract = |format: ImageFormat, bytes: &[u8], pages: &str| -> Vec<(String, Vec<u8>)> {
        let sinks = MemorySinks::new();
        let report = apply(
            &Plan::Images(ImagesPlan {
                source: 0,
                pages: pages.parse().expect("a selection"),
                min_pixels: 0,
                list_only: false,
                native: false,
                no_mask: false,
                format,
                names: format!("img-%d.{}", format.extension())
                    .parse()
                    .expect("a pattern"),
            }),
            &[Source::new(bytes.to_vec())],
            &sinks,
            &Policy::default(),
            &Budget::default(),
        )
        .expect("extracts");
        assert!(report.refused.is_empty(), "{report:?}");
        sinks.into_outputs()
    };
    let pngs = extract(ImageFormat::Png, &bytes, "100-110");
    let greys = extract(ImageFormat::Pgm, &bytes, "100-110");
    let rgbs = extract(ImageFormat::Ppm, &bytes, "100-110");
    assert!(
        !pngs.is_empty(),
        "pages 100 to 110 of the standard embed images"
    );
    assert_eq!(pngs.len(), greys.len());
    assert_eq!(pngs.len(), rgbs.len());
    for ((png_name, png), ((grey_name, grey), (rgb_name, rgb))) in
        pngs.iter().zip(greys.iter().zip(&rgbs))
    {
        let (width, height, pixels) = decode_png(png);
        assert_eq!(grey_name, &png_name.replace(".png", ".pgm"));
        assert_eq!(rgb_name, &png_name.replace(".png", ".ppm"));
        let header = format!("P5\n{width} {height}\n255\n");
        assert!(grey.starts_with(header.as_bytes()), "{grey_name}");
        for (pixel, &level) in pixels.chunks_exact(4).zip(&grey[header.len()..]) {
            let exact =
                0.3 * f64::from(pixel[0]) + 0.59 * f64::from(pixel[1]) + 0.11 * f64::from(pixel[2]);
            assert!(
                (f64::from(level) - exact).abs() <= 0.501,
                "{grey_name}: {pixel:?} -> {level}"
            );
        }
        let header = format!("P6\n{width} {height}\n255\n");
        assert!(rgb.starts_with(header.as_bytes()), "{rgb_name}");
        for (pixel, triple) in pixels
            .chunks_exact(4)
            .zip(rgb[header.len()..].chunks_exact(3))
        {
            assert_eq!(&pixel[..3], triple, "{rgb_name}");
        }
    }

    // The masked JPEG: under PGM the base is grey and the mask is a PGM beside it, sample for
    // sample the PNG route's mask; under PPM the mask is the same PGM.
    let Some(masked) = masked_jpeg_document() else {
        eprintln!("skipped the masked half: the pdf.js corpus is not checked out");
        return;
    };
    let separate = images_of(&masked, false, true);
    let (_, _, mask_png) = decode_grey_png(&separate[1].1);
    for format in [ImageFormat::Pgm, ImageFormat::Ppm] {
        let outputs = extract(format, &masked, "1");
        assert_eq!(
            outputs
                .iter()
                .map(|(name, _)| name.as_str())
                .collect::<Vec<_>>(),
            [
                format!("img-1.{}", format.extension()),
                "img-1.mask.pgm".to_owned()
            ],
            "{format:?}: the image, and its mask beside it"
        );
        let mask = &outputs[1].1;
        let header_end = mask
            .windows(4)
            .position(|w| w == b"255\n")
            .expect("a P5 header")
            + 4;
        assert!(mask.starts_with(b"P5\n"));
        assert_eq!(
            &mask[header_end..],
            &mask_png[..],
            "one mask, whichever form"
        );
    }
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
    // The fourth level is a usage error on a command line, before the file is opened: a pipe
    // has nobody to ask, and the sentence says so rather than letting `ask` look like a level.
    let (code, _stdout, stderr) = run(&dir, &["images", path, "--restrictions=ask", "--list"]);
    assert_eq!(code, 1, "{stderr}");
    assert!(stderr.contains("cannot ask"), "{stderr}");
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
            no_mask: false,
            format: ImageFormat::Png,
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
    let objects: BTreeSet<_> = entries.iter().map(|e| &e.object).collect();
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
            no_mask: false,
            format: ImageFormat::Png,
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
            no_mask: false,
            format: ImageFormat::Png,
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
            no_mask: false,
            format: ImageFormat::Png,
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
            no_mask: false,
            format: ImageFormat::Png,
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
#[expect(
    clippy::too_many_lines,
    reason = "one document, through the seam and then the program, each output checked by name"
)]
fn native_writes_the_codec_stream_where_it_is_a_file_and_says_so_where_it_is_not() {
    let bytes =
        std::fs::read(committed("ISO_32000-2_sponsored_EC3.pdf")).expect("a committed document");
    let listing = apply(
        &Plan::Images(ImagesPlan {
            source: 0,
            pages: "100-130".parse().expect("a selection"),
            min_pixels: 0,
            list_only: true,
            native: true,
            no_mask: false,
            format: ImageFormat::Png,
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
            committed("ISO_32000-2_sponsored_EC3.pdf")
                .to_str()
                .expect("utf-8"),
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
            no_mask: false,
            format: ImageFormat::Png,
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

/// The corpus fixture with one `DCTDecode` image under an `/SMask`: what §8.9.6.1 calls soft
/// masking through the `SMask` entry, on a JPEG, which is the one case the native route could
/// only drop before this flag existed.
fn masked_jpeg_document() -> Option<Vec<u8>> {
    let path = corpus("issue21570.pdf")?;
    Some(std::fs::read(path).expect("a corpus document"))
}

/// Runs `images` over the fixture with these two switches, answering the outputs by name.
fn images_of(bytes: &[u8], native: bool, no_mask: bool) -> Vec<(String, Vec<u8>)> {
    let sinks = MemorySinks::new();
    let report = apply(
        &Plan::Images(ImagesPlan {
            source: 0,
            pages: Selection::all(),
            min_pixels: 0,
            list_only: false,
            native,
            no_mask,
            format: ImageFormat::Png,
            names: if native { "img-%d" } else { "img-%d.png" }
                .parse()
                .expect("a pattern"),
        }),
        &[Source::new(bytes.to_vec())],
        &sinks,
        &Policy::default(),
        &Budget::default(),
    )
    .expect("the plan applies");
    assert_eq!(report.exit(false, false), Exit::Success, "{report:?}");
    sinks.into_outputs()
}

/// The three outputs of the module comment, on one image, and what the standard makes them
/// to each other: the composite's alpha *is* the soft mask's sample on a shared grid
/// (§11.6.5.2 — the mask's values are the opacity), its colour is the base's, and the base
/// written without its mask is opaque everywhere, since an image with no mask "mark[s] all
/// areas [it] occup[ies] on the page as if with opaque paint" (§8.9.6.1).
#[test]
fn a_soft_mask_is_composited_by_default_and_kept_beside_the_image_under_no_mask() {
    let Some(bytes) = masked_jpeg_document() else {
        eprintln!("skipped: the pdf.js corpus is not checked out");
        return;
    };
    let composite = images_of(&bytes, false, false);
    assert_eq!(composite.len(), 1, "one image, its mask inside it");
    assert_eq!(composite[0].0, "img-1.png");
    let (width, height, composite) = decode_png(&composite[0].1);

    let separate = images_of(&bytes, false, true);
    assert_eq!(
        separate
            .iter()
            .map(|(name, _)| name.as_str())
            .collect::<Vec<_>>(),
        ["img-1.png", "img-1.mask.png"],
        "the base, and the mask beside it"
    );
    let (base_width, base_height, base) = decode_png(&separate[0].1);
    assert_eq!((base_width, base_height), (width, height));
    let (mask_width, mask_height, mask) = decode_grey_png(&separate[1].1);
    assert_eq!(
        (mask_width, mask_height),
        (width, height),
        "this fixture's mask shares the base's grid"
    );

    assert!(
        base.chunks_exact(4).all(|px| px[3] == 255),
        "the base without its mask is opaque everywhere"
    );
    assert!(
        mask.contains(&0) && mask.contains(&255),
        "the mask masks something and keeps something"
    );
    for ((c, b), &m) in composite
        .chunks_exact(4)
        .zip(base.chunks_exact(4))
        .zip(mask.iter())
    {
        assert_eq!(c[3], m, "the composite's alpha is the mask's sample");
        if m > 0 {
            assert_eq!(&c[..3], &b[..3], "and its colour is the base's");
        }
    }

    // The native route: the JPEG as it is, and the same mask beside it — a JPEG has nowhere
    // to put one.
    let native = images_of(&bytes, true, false);
    assert_eq!(
        native
            .iter()
            .map(|(name, _)| name.as_str())
            .collect::<Vec<_>>(),
        ["img-1.jpg", "img-1.mask.png"]
    );
    assert_eq!(
        &native[0].1[..2],
        &[0xFF, 0xD8],
        "ISO/IEC 10918-1's SOI marker"
    );
    assert_eq!(native[1].1, separate[1].1, "one mask, whichever route");
}

/// Where the mask goes, by name: the image's extension replaced, or appended where it has none.
#[test]
fn a_mask_is_named_beside_its_image() {
    use pdf_transform::images::mask_name;
    let png = ImageFormat::Png;
    assert_eq!(mask_name("img-3.png", png), "img-3.mask.png");
    assert_eq!(mask_name("img-3.jpg", png), "img-3.mask.png");
    assert_eq!(mask_name("img-3", png), "img-3.mask.png");
    assert_eq!(mask_name("out.v2/img-3", png), "out.v2/img-3.mask.png");
    assert_eq!(mask_name("out.v2/img-3.jp2", png), "out.v2/img-3.mask.png");
    // Beside a netpbm image the mask is the one-channel netpbm form, whichever of the two
    // the image took.
    assert_eq!(mask_name("img-3.pgm", ImageFormat::Pgm), "img-3.mask.pgm");
    assert_eq!(mask_name("img-3.ppm", ImageFormat::Ppm), "img-3.mask.pgm");
    assert_eq!(mask_name("img-3.jpg", ImageFormat::Pgm), "img-3.mask.pgm");
}

/// Decodes an 8-bit greyscale PNG.
fn decode_grey_png(bytes: &[u8]) -> (u32, u32, Vec<u8>) {
    let decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    let mut reader = decoder.read_info().expect("a PNG");
    let mut data = vec![0; reader.output_buffer_size().expect("a bounded size")];
    let info = reader.next_frame(&mut data).expect("a frame");
    assert_eq!(info.color_type, png::ColorType::Grayscale);
    assert_eq!(info.bit_depth, png::BitDepth::Eight);
    data.truncate(info.buffer_size());
    (info.width, info.height, data)
}

/// One page's rectangle under a key, read off the page dictionary independently of the crate
/// — and off its ancestors for the two boxes §7.7.3.4 makes inheritable, `MediaBox` and
/// `CropBox`; the other three "shall not be inherited".
fn stated_box(
    document: &pdf_syntax::Document,
    page: &pdf_syntax::Dictionary,
    key: &str,
) -> Option<[f32; 4]> {
    let inheritable = matches!(key, "MediaBox" | "CropBox");
    let mut node = page.clone();
    let mut array = document.get_key(&node, key);
    let mut depth: u32 = 0;
    while array.as_array().is_none() && inheritable && depth < 64 {
        let parent = document.get_key(&node, "Parent");
        node = parent.as_dict()?.clone();
        array = document.get_key(&node, key);
        depth = depth.saturating_add(1);
    }
    let array = array.as_array()?;
    let mut out = [0.0; 4];
    for (slot, item) in out.iter_mut().zip(array) {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a page box's coordinates are small numbers a file states"
        )]
        {
            *slot = document.resolve(item).as_number()? as f32;
        }
    }
    Some(out)
}

/// §8.3.2.3's 72 units to the inch at 150 dpi, rounded up to whole pixels the way
/// `pdf_render::TargetSpec::for_page` rounds — the raster contains the page.
fn pixels_at_150_dpi(extent: f32) -> u32 {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a page extent in pixels is a small positive number"
    )]
    let pixels = (f64::from(extent) * 150.0 / 72.0).ceil() as u32;
    pixels
}

/// `--page-box`: the raster's extent is the box asked for, each box defaulted as §7.7.3.3's
/// Table 31 states — the crop box "[d]efault value: the value of `MediaBox`", and the bleed,
/// trim and art boxes "[d]efault value: the value of `CropBox`" — on a corpus page whose crop
/// box is a quarter of its media box and which states none of the other three.
#[test]
fn the_page_box_asked_for_is_the_rasters_extent_and_the_unstated_boxes_default_to_the_crop_box() {
    use pdf_transform::render::Boundary;
    let Some(path) = corpus("issue2177.pdf") else {
        eprintln!("skipped: the pdf.js corpus is not checked out");
        return;
    };
    let bytes = std::fs::read(path).expect("a corpus document");
    let document = pdf_syntax::Document::open(bytes.clone()).expect("a PDF");
    let page = support::page_dictionaries(&document)
        .into_iter()
        .next()
        .expect("a page");
    let media = stated_box(&document, &page, "MediaBox").expect("the page states a media box");
    let crop = stated_box(&document, &page, "CropBox").expect("the fixture states a crop box");
    for absent in ["BleedBox", "TrimBox", "ArtBox"] {
        assert!(
            stated_box(&document, &page, absent).is_none(),
            "the fixture states no {absent}, so Table 31's default decides it"
        );
    }
    let expected = |rectangle: [f32; 4]| {
        (
            pixels_at_150_dpi(rectangle[2] - rectangle[0]),
            pixels_at_150_dpi(rectangle[3] - rectangle[1]),
        )
    };

    let extent_under = |page_box: Option<Boundary>| {
        let sinks = MemorySinks::new();
        let report = apply(
            &Plan::Render(RenderPlan {
                source: 0,
                pages: "1".parse().expect("a selection"),
                size: Sizing::Dpi(150.0),
                format: ImageFormat::Png,
                page_box,
                annotations: true,
                names: "p.png".parse().expect("a pattern"),
                strips: None,
            }),
            &[Source::new(bytes.clone())],
            &sinks,
            &Policy::default(),
            &Budget::default(),
        )
        .expect("the plan applies");
        match &report.outputs[..] {
            [
                Output {
                    origin: Origin::Page { width, height, .. },
                    ..
                },
            ] => (*width, *height),
            other => panic!("one page: {other:?}"),
        }
    };
    assert_eq!(extent_under(Some(Boundary::Media)), expected(media));
    assert_eq!(extent_under(Some(Boundary::Crop)), expected(crop));
    for defaulted in [Boundary::Bleed, Boundary::Trim, Boundary::Art] {
        assert_eq!(
            extent_under(Some(defaulted)),
            expected(crop),
            "{defaulted:?} defaults to the crop box"
        );
    }
    // No `/ViewArea` in this document, so the viewer's own display boundary is the crop box.
    assert_eq!(extent_under(None), expected(crop));
    assert_ne!(expected(media), expected(crop));
}

/// `--no-annotations`: the page is drawn as a page stating no `/Annots`, so §12.5.3's pass
/// draws nothing and the raster differs from the one §6.3.2.2 obliges — on a page of the
/// standard's own PDF that carries file attachment annotations with appearance streams.
#[test]
fn without_annotations_the_page_contents_alone_are_drawn() {
    let bytes =
        std::fs::read(committed("ISO_32000-2_sponsored_EC3.pdf")).expect("a committed document");
    let document = pdf_syntax::Document::open(bytes.clone()).expect("a PDF");
    let pages = pdf_model::Pages::new(&document);
    let page = pages.get(961).expect("page 962");
    assert!(page.dict.get("Annots").is_some(), "page 962 states /Annots");
    let drawn = pdf_transform::render::page_to_draw(&page, None, false);
    assert!(drawn.dict.get("Annots").is_none());
    assert_eq!(
        drawn.display_box.map(f32::to_bits),
        page.display_box.map(f32::to_bits),
        "the box is untouched"
    );

    let raster_with = |annotations: bool| {
        let sinks = MemorySinks::new();
        apply(
            &Plan::Render(RenderPlan {
                source: 0,
                pages: "962".parse().expect("a selection"),
                size: Sizing::Dpi(72.0),
                format: ImageFormat::Png,
                page_box: None,
                annotations,
                names: "p.png".parse().expect("a pattern"),
                strips: None,
            }),
            &[Source::new(bytes.clone())],
            &sinks,
            &Policy::default(),
            &Budget::default(),
        )
        .expect("the plan applies");
        decode_png(&sinks.into_outputs()[0].1)
    };
    let (w1, h1, with) = raster_with(true);
    let (w2, h2, without) = raster_with(false);
    assert_eq!((w1, h1), (w2, h2), "the same box either way");
    assert_ne!(with, without, "the annotations' appearances are marks");
}

/// `pages` through the program: the flag order is the composition order, `--rotate` takes
/// qpdf's `[+|-]angle:range`, and the three refusals the argument grammar owns say so on stderr.
///
/// The library's own tests (`tests/pages.rs`) hold the clauses; this holds the *command line*,
/// which is where RFC 0002 section 6.2's "operations compose left to right over the current page
/// list" actually lives — `Arguments::value` takes the last of a repeated flag, so a verb whose
/// flags are data in argv order has to read them off argv.
#[test]
fn pages_edits_compose_in_the_order_they_were_written() {
    let path = committed("PDF20_AN001-BPC.pdf");
    let path = path.to_str().expect("utf-8");
    let dir = scratch();

    // Two deletions of position 1 take out the first two pages, not the first and the third.
    let (code, _stdout, stderr) = run(
        &dir,
        &[
            "pages", path, "--delete", "1", "--delete", "1", "-o", "two.pdf",
        ],
    );
    assert!(code == 0 || code == 3, "{stderr}");
    let two = std::fs::read(dir.join("two.pdf")).expect("the output");
    let (code, _stdout, stderr) = run(&dir, &["pages", path, "--delete", "1-2", "-o", "both.pdf"]);
    assert!(code == 0 || code == 3, "{stderr}");
    assert_eq!(
        two,
        std::fs::read(dir.join("both.pdf")).expect("the output"),
        "two deletions of position 1 are one deletion of 1-2"
    );

    // §7.7.3.3: a multiple of 90, and the sign is what makes it relative.
    for (argument, accepted) in [
        ("+90:1", true),
        ("180:1-end:odd", true),
        ("-90:r1", true),
        ("45:1", false),
        ("90", false),
    ] {
        let (got, _stdout, stderr) =
            run(&dir, &["pages", path, "--rotate", argument, "-o", "r.pdf"]);
        // 3 rather than 0 because this fixture states constructs no verb of the suite carries,
        // and saying so is what exit 3 is for.
        if accepted {
            assert!(got == 0 || got == 3, "--rotate {argument}: {got}: {stderr}");
        } else {
            assert_eq!(got, 1, "--rotate {argument}: {stderr}");
        }
    }

    // The boundary between this verb and `merge` is the count of files, so a path in --insert
    // is a usage refusal that names the other verb (ADR 0830).
    let (code, _stdout, stderr) = run(
        &dir,
        &["pages", path, "--insert", "other.pdf:1@1", "-o", "x.pdf"],
    );
    assert_eq!(code, 1, "{stderr}");
    assert!(stderr.contains("merge"), "{stderr}");

    // A position the list does not have, and one past the end which appends.
    let (code, _stdout, stderr) = run(&dir, &["pages", path, "--move", "1:99", "-o", "x.pdf"]);
    assert_eq!(code, 1, "{stderr}");
    let (code, _stdout, stderr) = run(&dir, &["pages", path, "--move", "1:6", "-o", "end.pdf"]);
    assert!(code == 0 || code == 3, "{stderr}");

    // No edit at all is a usage error rather than a copy of the input.
    let (code, _stdout, stderr) = run(&dir, &["pages", path, "-o", "x.pdf"]);
    assert_eq!(code, 1, "{stderr}");
    assert!(stderr.contains("--delete"), "{stderr}");
}
