//! Ninety-six ways of encoding one image, which must all decode to it.
//!
//! # Why this is the strongest JBIG2 check available
//!
//! The pdf.js corpus contains 96 documents named `bitmap-*.pdf`, and they are the same
//! drawing — a small smiling face — encoded through nearly every coding path ISO/IEC 14492
//! defines. Their names are the inventory: `template0` to `template3` and `customat` for
//! the generic region templates and their adaptive pixels, `tpgdon` and `tpgron` for typical
//! prediction, `mmr` for the T.6 fallback, `refine` for refinement regions, `symbol` for
//! symbol dictionaries with `symhuff` and `texthuff` for their Huffman variants, `halftone`
//! with `skip` and `grid` for halftone regions, `stripe` for striped pages, `composite` with
//! `or`, `xor`, `and`, `xnor` and `replace` for the page composition operators, and
//! `context-reuse` for retained arithmetic coding contexts.
//!
//! So the corpus states an invariant about itself: **every one of them must produce the
//! same pixels**. That is a far better test than comparing against another renderer, for
//! three reasons.
//!
//! - It needs no reference at all, so principle 5 is not even in tension: the expectation
//!   comes from the documents, not from anyone's output.
//! - It is *sensitive*. A decoder that gets refinement subtly wrong produces a face that
//!   still looks like a face, and no eye and no tolerance would catch it — but it will not
//!   be byte-identical to the ninety-five others.
//! - It found something the reference oracle got backwards. Six of these pages are recorded
//!   there as contradicted, because `mupdf` and `ghostscript` agree about them and we
//!   differ. They agree because **they are the same decoder**: both link `jbig2dec`. On four
//!   of the six it renders a blank page or a page strewn with noise, and `poppler`, which
//!   has its own implementation, agrees with us. Two implementations agreeing is evidence
//!   only where they can fail independently, and the oracle could not know that these two
//!   cannot.
//!
//! # What it does not check
//!
//! That the shared image is the *right* image. A decoder that produced the same wrong
//! picture ninety-six times would pass. That is what
//! `pdf_sandbox`'s own test of ISO 32000-2 §7.4.7's worked example is for: it decodes a
//! specification-supplied bitstream and checks the marks it makes. The two together — one
//! image known from the specification, ninety-six agreeing with each other — cover both
//! halves.

#![expect(
    clippy::print_stdout,
    reason = "test code: the survey output is the point of the run"
)]

use std::path::{Path, PathBuf};

use pdf_render::{Rasterizer as _, TargetSpec};
use pdf_syntax::Document;
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
use render_cpu::CpuRasterizer;

/// Pixel budget per page. These are all small.
const PIXEL_BUDGET: u64 = 16 << 20;

/// How many `bitmap-*.pdf` documents the corpus is known to hold.
///
/// A ratchet in the direction that matters: if the corpus grows a new coding mode this
/// should rise, and if a document stops decoding the count of *rendered* pages falls below
/// it and the test fails. It is not an upper bound.
const KNOWN_DOCUMENTS: usize = 96;

/// The documents, or `None` when the submodule is not checked out.
fn documents() -> Option<Vec<PathBuf>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/pdf.js/test/pdfs");
    let mut found: Vec<PathBuf> = std::fs::read_dir(root)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("bitmap-"))
                && path.extension().is_some_and(|extension| extension == "pdf")
        })
        .collect();
    found.sort();
    (!found.is_empty()).then_some(found)
}

/// One rendered page, compared by value.
#[derive(Debug, PartialEq, Eq)]
struct Rendered {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

/// Renders page one, or says why it could not.
fn render(path: &Path) -> Result<Rendered, String> {
    let bytes = std::fs::read(path).map_err(|error| error.to_string())?;
    let document = Document::open(bytes).map_err(|error| error.to_string())?;
    let page = pdf_model::Pages::new(&document)
        .get(0)
        .ok_or("no first page")?;
    let interpretation = pdf_model::interpret(&document, &page);
    if !interpretation.unsupported.is_empty() {
        return Err(format!("{:?}", interpretation.unsupported));
    }
    let target = TargetSpec::for_page(&interpretation.display_list, 1.0, PIXEL_BUDGET)
        .map_err(|error| error.to_string())?;
    let raster = CpuRasterizer::new()
        .rasterize(&interpretation.display_list, target)
        .map_err(|error| error.to_string())?;
    Ok(Rendered {
        width: raster.width,
        height: raster.height,
        pixels: raster.data.clone(),
    })
}

#[test]
fn every_jbig2_coding_mode_decodes_to_the_same_image() {
    if let Err(error) = pdf_sandbox::Sandbox::shared().confinement() {
        panic!("the sandboxed image decoder is not available: {error}");
    }
    let Some(documents) = documents() else {
        println!("skipped: the doc/pdf.js submodule is not checked out");
        return;
    };

    let rendered: Vec<(String, Result<Rendered, String>)> = documents
        .par_iter()
        .map(|path| {
            let name = path
                .file_stem()
                .map(|stem| stem.to_string_lossy().into_owned())
                .unwrap_or_default();
            (name, render(path))
        })
        .collect();

    let failed: Vec<String> = rendered
        .iter()
        .filter_map(|(name, outcome)| {
            outcome
                .as_ref()
                .err()
                .map(|error| format!("{name}: {error}"))
        })
        .collect();
    assert!(
        failed.is_empty(),
        "{} of {} JBIG2 documents did not draw completely: {failed:#?}",
        failed.len(),
        rendered.len()
    );

    assert!(
        rendered.len() >= KNOWN_DOCUMENTS,
        "{} bitmap-*.pdf documents found, was {KNOWN_DOCUMENTS}",
        rendered.len()
    );

    // Grouped rather than compared pairwise, so a failure names *which* encodings disagree
    // rather than only that two of ninety-six did.
    let mut groups: Vec<(&Rendered, Vec<&str>)> = Vec::new();
    for (name, outcome) in &rendered {
        let Ok(image) = outcome else { continue };
        match groups.iter_mut().find(|(seen, _)| *seen == image) {
            Some((_, names)) => names.push(name),
            None => groups.push((image, vec![name])),
        }
    }

    let summary: Vec<String> = groups
        .iter()
        .map(|(image, names)| format!("{}x{} from {names:?}", image.width, image.height))
        .collect();
    assert_eq!(
        groups.len(),
        1,
        "the corpus encodes one image {} ways and they did not agree: {summary:#?}",
        rendered.len()
    );

    let (image, names) = groups.first().expect("one group");
    println!(
        "{} JBIG2 encodings, {}x{}, all identical",
        names.len(),
        image.width,
        image.height
    );

    // The invariant above is satisfied by ninety-six blank pages, so this is the half of the
    // check that says something was drawn. The page is a line drawing: mostly white, with
    // ink on a few percent of it.
    let dark = image
        .pixels
        .chunks_exact(4)
        .filter(|pixel| pixel.first().is_some_and(|red| *red < 128))
        .count();
    let pixels = image.pixels.len() / 4;
    assert!(
        (pixels / 100..pixels / 2).contains(&dark),
        "the shared image is {}% ink, which is not a line drawing",
        dark.saturating_mul(100) / pixels.max(1)
    );
}
