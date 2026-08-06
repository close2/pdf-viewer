//! Every page the corpus draws, through quorra and through the CPU oracle.
//!
//! # Why this gate exists
//!
//! `real_pages.rs` renders four pages of the specification's own PDFs, and trap 12b is the
//! standing warning about what a small suite proves: the Vello backend passed fourteen
//! cross-backend scenes and then drew a *blank page* the first time it was handed a real one
//! at a real window's size. Four real pages is better than fourteen scenes and it is still
//! four. This is the same comparison over **974 documents' first pages** — every generator
//! anyone has pointed at pdf.js in fifteen years, including the malformed and the hostile.
//!
//! Both backends are handed the **same display list**, so nothing here is about PDF
//! semantics: a difference is a difference between two rasterisers, and a refusal is a
//! command the new backend cannot draw. That is the whole point — `CLAUDE.md` keeps the CPU
//! backend as the correctness oracle, and this is the instrument that holds a second backend
//! to it at the corpus's scale rather than at a fixture's.
//!
//! # What it measures
//!
//! - **Fidelity**, with the glyph-phase quantum **off**, so that what is measured is the
//!   adapter and the translation rather than a trade `real_pages.rs` already gates.
//! - **Refusals**, by name: a page quorra cannot draw is a hole in the backend, and it must
//!   be one somebody wrote down rather than one nobody counted.
//! - **Speed**, both totals and the per-page median ratio, which answer different questions.
//!   A GPU frame here includes the readback to system memory, which a windowed host does not
//!   pay — `RENDER_LIBRARY.md` section 6.1 measures that at 55% to 92% of a frame — so the
//!   ratio below is the *offscreen* one and says nothing directly about the window.
//!
//! # Running it
//!
//! ``text
//! cargo test --release -p render-quorra --test corpus -- --ignored --nocapture
//! ``
//!
//! `PDFVIEWER_QUORRA_ONLY=a,b` restricts it to matching file names and **refuses to check the
//! ratchets**, saying so — a list held to equality over a subset would report every document
//! the filter excluded as fixed. Debug builds are ~15× slower here; the numbers below are
//! release numbers and the run says which it took.

#![expect(
    clippy::print_stdout,
    reason = "test code: an explanatory panic is the intended failure, and the survey \
              output is the point of the run"
)]

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use pdf_render::{DisplayList, Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;
use render_quorra::QuorraRasterizer;

/// Pixel budget per page, generous enough that no real page reaches it.
const PIXEL_BUDGET: u64 = 64 << 20;

/// The scale everything is rendered at by default: the page's own resolution.
///
/// The same scale the reference oracle compares at, so a page named here can be looked at
/// beside the artefacts that gate already writes.
const SCALE: f32 = 1.0;

/// The scale to render at, which `PDFVIEWER_QUORRA_SCALE` may override.
///
/// It exists for the speed half of this gate rather than the fidelity half. A GPU frame's
/// cost is dominated by a per-pixel floor and a readback (`RENDER_LIBRARY.md` section 6.1) while
/// this tree's CPU rasterisation grows with the pixels, so the ratio between them is a
/// *function of the scale* and one scale cannot say which way it runs — the same trap ADR
/// 0136 met comparing `rasterrocket` at 72 and 150 dpi. Overriding it **skips the ratchets**,
/// which are measured at the default.
fn scale() -> f32 {
    std::env::var("PDFVIEWER_QUORRA_SCALE")
        .ok()
        .and_then(|value| value.trim().parse().ok())
        .filter(|scale: &f32| scale.is_finite() && *scale > 0.0)
        .unwrap_or(SCALE)
}

/// How far a page may sit from the oracle before it is counted as differing.
///
/// Three numbers because they catch different failures, and they are `real_pages.rs`'s own
/// gates: the mean catches a page-wide shift, the worst tile catches a mark that is missing
/// or in the wrong place, and the structural similarity catches a page that has the right ink
/// in the wrong shapes. A page failing any of them is listed.
const MAX_MEAN_ERROR: f64 = 1.5;
/// See [`MAX_MEAN_ERROR`].
const MAX_WORST_TILE_ERROR: f64 = 7.0;
/// See [`MAX_MEAN_ERROR`].
const MIN_STRUCTURAL_SIMILARITY: f64 = 0.99;

/// Documents whose first page quorra refuses to draw, by name.
///
/// Held to equality in both directions: a page arriving here is a new hole in the backend,
/// and a page leaving it is a hole closed. The reason each one gives is printed by the run.
///
/// **One page, and its refusal now says what actually ran out.** `bug1721218_reduced.pdf` is
/// this viewer's most pathological page by some margin, and quorra refuses it with "the
/// frame's rasterised coverage outgrew the 16384x16384 scratch image this adapter allows" —
/// a texture-capacity limit named as one. The six pages that used to sit here beside it were
/// casualties of the *old* scratch sheet (2048 texels wide, and a refusal message whose
/// arithmetic contradicted itself — `QUORRA_FEEDBACK.md` section 3); quorra widened the sheet
/// to the device dimension and all six draw. `issue17848.pdf` left for a different reason:
/// its mesh shading has no visible raster, pdf.js's issue #17848 traced that to a defective
/// document, and the backend now draws nothing for it exactly as this viewer's own two
/// backends always have.
const REFUSED: [&str; 1] = ["bug1721218_reduced.pdf"];

/// Pages where the two rasterisers differ only at the **edges** of what they draw.
///
/// Structural similarity above 0.99 — `raster_compare`'s own vector threshold — is the
/// statement that the same shapes are in the same places, so what is left is coverage at a
/// boundary. Twenty of these are one document family (`tracemonkey.pdf` and its variants and
/// relatives) sitting at mean 1.52 with a worst tile of 5.09, which is a page of dense text
/// measured against a different glyph rasteriser: `real_pages.rs` measures the specification's
/// own pages at 1.18 and this is the same floor on a heavier page. **This group is the floor,
/// not a defect list** — what would make it one is a page arriving in it whose similarity is
/// high because the difference is uniform.
///
/// `issue4260_reduced.pdf` — once the worst page in the run at similarity 0.49, a grid of
/// zero-height rectangles drawn blank — arrived here from the shape list when the backend
/// started asking `pdf_render::split_collapsed_fill` the §10.7.4 question, and **left
/// altogether in the three-hundred-and-sixty-eighth session, when that question started being
/// answered with whole device pixels.** What kept it here was the last thing two rasterisers
/// can disagree about: a mark laid down at the shape's own fractional position is an
/// anti-aliased band, and each backend distributes such a band across two rows in its own way.
/// A mark that is a whole pixel row has nothing left to distribute — the two backends draw the
/// same rows, from the same shared geometry, and the page agrees to the byte. ADR 0208.
///
/// **`issue21068.pdf` left in the two-hundred-and-seventh session and the reason is worth
/// keeping**: it is four rows of comb fields whose separators sit exactly on their `/BBox`, and
/// the anti-aliased clip was costing each of them a fraction of a pixel — differently in the two
/// rasterisers, which is what put it here. Both draw from the *same* display list, so once the
/// redundant clip came off (ADR 0165) there was nothing left to differ about. **A page in this
/// list can be here because of something upstream of both backends**, which is not what its name
/// suggests.
const DIFFERS_AT_THE_EDGES: [&str; 27] = [
    "bug1308536.pdf",
    "bug1885505.pdf",
    "bug1992868.pdf",
    "chrome-text-selection-markedContent.pdf",
    "endchar.pdf",
    "extgstate.pdf",
    "inks_basic.pdf",
    "issue11473.pdf",
    "issue11913.pdf",
    "issue12810.pdf",
    "issue13447.pdf",
    "issue14415.pdf",
    "issue14438.pdf",
    "issue15012.pdf",
    "issue18911.pdf",
    "issue19239.pdf",
    "issue2884_reduced.pdf",
    "issue7014.pdf",
    "issue7492.pdf",
    "issue8187.pdf",
    "pr12564.pdf",
    "tracemonkey.pdf",
    "tracemonkey_a11y.pdf",
    "tracemonkey_annotation_on_page_8.pdf",
    "tracemonkey_freetext.pdf",
    "tracemonkey_with_annotations.pdf",
    "tracemonkey_with_editable_annotations.pdf",
];

/// Pages where the difference is **structural**: similarity at or below 0.99.
///
/// The name is the *classifier's*, and every page here has now been examined; none is an
/// open question. What left, and why: `issue4260_reduced.pdf` (§10.7.4's degenerate fills)
/// moved to the edge group; `issue2177.pdf`, `issue6769.pdf`, `issue6769_no_matrix.pdf` and
/// `bug946506.pdf` agreed when anisotropically-transformed strokes started outlining in path
/// space; `issue10572.pdf` and `issue14165.pdf` agreed when quorra raised its ramp sampling
/// from 256 to 4096 texels — a banded shading's hard stop boundary snapped to the coarser
/// grid and sat ~3.5 px off on a page-spanning axis.
///
/// What stays, measured pixel by pixel: six pages (`copy_paste_ligatures`,
/// `issue4402_reduced`, `issue18030`, `issue7454`, `issue15150`, `160F-2019`) have **zero**
/// pixels differing by more than 64 of 255 — miniature or enormous pages where uniform
/// sub-step coverage differences drag the similarity score below the threshold without any
/// shape moving. `knockout_groups_test.pdf` and `issue840.pdf` differ on 2 and 4 pixels
/// respectively. The rest are the hairline-and-texture family — sub-half-pixel rules
/// (`issue16038`, `issue12295`, `issue20232`, `22060_A1_01_Plans`), 8-px text
/// (`issue16316`, `standard_fonts`), halftone photographs under the stated linear-sampler
/// variance (`issue269_2`) — where the two rasterisers put the same ink on different sides
/// of a pixel boundary. Matching `tiny-skia`'s sub-pixel distribution byte-for-byte would be
/// curve-fitting to another renderer, which quorra's charter forbids; they stay listed so a
/// *growth* in their numbers is still a finding.
const DIFFERS_IN_SHAPE: [&str; 15] = [
    "160F-2019.pdf",
    "22060_A1_01_Plans.pdf",
    "copy_paste_ligatures.pdf",
    "issue12295.pdf",
    "issue15150.pdf",
    "issue16038.pdf",
    "issue16316.pdf",
    "issue18030.pdf",
    "issue20232.pdf",
    "issue269_2.pdf",
    "issue4402_reduced.pdf",
    "issue7454.pdf",
    "issue840.pdf",
    "knockout_groups_test.pdf",
    "standard_fonts.pdf",
];

/// The two groups as one list, sorted as the run produces them.
fn differing_pages() -> Vec<&'static str> {
    let mut all: Vec<&'static str> = DIFFERS_AT_THE_EDGES
        .iter()
        .chain(&DIFFERS_IN_SHAPE)
        .copied()
        .collect();
    all.sort_unstable();
    all
}

/// One document's outcome.
enum Outcome {
    /// Rendered by both, within tolerance.
    Agrees,
    /// Rendered by both, outside it.
    Differs(String),
    /// quorra refused the display list.
    Refused(String),
    /// Nothing to compare: no page, no display list, or a target past the budget.
    Skipped,
}

#[test]
#[ignore = "renders 974 pages twice; run explicitly"]
fn every_corpus_page_agrees_with_the_cpu_oracle() {
    let Some(files) = corpus() else {
        println!("skipped: the doc/pdf.js submodule is not checked out");
        return;
    };
    let scale = scale();
    let only = std::env::var("PDFVIEWER_QUORRA_ONLY").ok();
    let files = selected(files, only.as_deref());

    // The quantum off: this gate isolates the backend's fidelity from the deliberate
    // sub-1/32-pixel trade `real_pages.rs` gates separately.
    let mut quorra = QuorraRasterizer::with_options(&quorra_gpu::Options {
        glyph_quantum: None,
        ..quorra_gpu::Options::default()
    })
    .unwrap_or_else(|e| panic!("no adapter available for quorra: {e}"));
    announce(&quorra, files.len(), scale);

    let started = Instant::now();
    let mut agreed = 0usize;
    let mut skipped = 0usize;
    let mut differing = Vec::new();
    let mut refused = Vec::new();
    let mut worst: Vec<(f64, String)> = Vec::new();
    let (mut cpu_total, mut gpu_total) = (Duration::ZERO, Duration::ZERO);
    let mut ratios: Vec<f64> = Vec::new();

    for path in &files {
        let name = path.file_name().map_or_else(
            || path.display().to_string(),
            |n| n.to_string_lossy().into(),
        );
        let Some(list) = page_one(path) else {
            skipped = skipped.saturating_add(1);
            continue;
        };
        let Ok(target) = TargetSpec::for_page(&list, scale, PIXEL_BUDGET) else {
            skipped = skipped.saturating_add(1);
            continue;
        };

        let at = Instant::now();
        let Ok(cpu) = CpuRasterizer::new().rasterize(&list, target) else {
            // A page the oracle itself refuses says nothing about the backend under test.
            skipped = skipped.saturating_add(1);
            continue;
        };
        let cpu_took = at.elapsed();
        let at = Instant::now();
        let ours = quorra.rasterize(&list, target);
        let gpu_took = at.elapsed();
        let verdict = outcome(&cpu, ours);
        // **A refused frame is a fast frame**, and counting one as a time would report a
        // backend that draws nothing as the quickest there is: at four times the page's own
        // scale, 533 of these documents are refused and the median ratio came back as 0.00×
        // before this line existed. Only a frame that was produced is timed.
        if !matches!(verdict, Outcome::Refused(_)) {
            cpu_total = cpu_total.saturating_add(cpu_took);
            gpu_total = gpu_total.saturating_add(gpu_took);
            if cpu_took > Duration::from_millis(1) {
                // Below a millisecond the clock is measuring itself; a ratio taken there is
                // noise with a decimal point.
                ratios.push(gpu_took.as_secs_f64() / cpu_took.as_secs_f64());
            }
        }

        match verdict {
            Outcome::Agrees => agreed = agreed.saturating_add(1),
            Outcome::Differs(how) => {
                write_artefacts(&name, &cpu, &quorra.rasterize(&list, target));
                let mean = how
                    .split_once("mean ")
                    .and_then(|(_, rest)| rest.split_whitespace().next())
                    .and_then(|value| value.parse::<f64>().ok())
                    .unwrap_or(0.0);
                worst.push((mean, name.clone()));
                differing.push(name.clone());
                println!("  differs: {name}: {how}");
            }
            Outcome::Refused(why) => {
                refused.push(name.clone());
                println!("  refused: {name}: {why}");
            }
            Outcome::Skipped => skipped = skipped.saturating_add(1),
        }
    }

    report(
        &mut Tally {
            agreed,
            skipped,
            differing: &differing,
            refused: &refused,
            worst: &mut worst,
        },
        (cpu_total, gpu_total, &mut ratios),
        started.elapsed(),
    );

    // Compared exactly: the ratchets were measured at exactly this scale, so anything else —
    // however near — is a different measurement.
    let rescaled = (scale - SCALE).abs() > 0.0;
    if only.is_some() || rescaled {
        println!(
            "{} of the corpus at scale {scale}. The ratchets below are NOT checked: they are \
             measured over the whole corpus at scale {SCALE}, and a list held to equality \
             over anything else would report every document it excluded as fixed.",
            files.len()
        );
        return;
    }
    assert_eq!(refused, REFUSED, "the pages quorra refuses have changed");
    assert_eq!(
        differing,
        differing_pages(),
        "the pages quorra draws differently from the oracle have changed"
    );
}

/// Says which adapter and which build produced the numbers below them.
///
/// Both matter to every number this gate prints: a software adapter and a discrete GPU are
/// different machines, and a debug build is ~15× slower here.
fn announce(quorra: &QuorraRasterizer, documents: usize, scale: f32) {
    println!("adapter: {}", quorra.adapter_description());
    println!(
        "{documents} documents, page one, at scale {scale}, {} build",
        if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        }
    );
}

/// The corpus, or the part of it `PDFVIEWER_QUORRA_ONLY` names.
fn selected(files: Vec<PathBuf>, filter: Option<&str>) -> Vec<PathBuf> {
    let Some(filter) = filter else { return files };
    files
        .into_iter()
        .filter(|path| {
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            filter.split(',').any(|want| name.contains(want.trim()))
        })
        .collect()
}

/// What the run found, for [`report`].
struct Tally<'a> {
    agreed: usize,
    skipped: usize,
    differing: &'a [String],
    refused: &'a [String],
    worst: &'a mut Vec<(f64, String)>,
}

/// Prints the survey: the counts, the two clocks, the median ratio and the ten worst pages.
///
/// The totals and the median answer different questions and only quoting both is honest —
/// `hayro`'s comparison learned that, and a distribution with a long tail makes a total say
/// something a per-page ratio does not.
fn report(tally: &mut Tally<'_>, timing: (Duration, Duration, &mut Vec<f64>), took: Duration) {
    let (cpu_total, gpu_total, ratios) = timing;
    let compared = tally
        .agreed
        .saturating_add(tally.differing.len())
        .saturating_add(tally.refused.len());
    println!(
        "\n{compared} pages compared in {took:.1?}: {} agree, {} differ, {} refused, {} not \
         comparable",
        tally.agreed,
        tally.differing.len(),
        tally.refused.len(),
        tally.skipped
    );
    println!(
        "  rasterisation: {cpu_total:.2?} on the CPU backend, {gpu_total:.2?} through quorra \
         (offscreen, readback included)"
    );
    ratios.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    if let Some(median) = ratios.get(ratios.len() / 2) {
        println!(
            "  median page: quorra takes {median:.2}× the CPU backend's time, over {} pages \
             above a millisecond",
            ratios.len()
        );
    }
    tally
        .worst
        .sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    if !tally.worst.is_empty() {
        println!("  furthest from the oracle:");
        for (mean, name) in tally.worst.iter().take(10) {
            println!("    {mean:8.3} mean  {name}");
        }
    }
}

/// Writes both renders of a differing page beside each other, for looking at.
///
/// The oracle's own artefacts are the fastest diagnostic in this tree and this gate had
/// none: a list of names and three numbers cannot tell a missing mark from a soft edge. Only
/// the pages that differ get one, so what is on disk is exactly the set worth opening.
fn write_artefacts(
    name: &str,
    cpu: &pdf_render::Raster,
    ours: &Result<pdf_render::Raster, impl ToString>,
) {
    let Ok(ours) = ours else { return };
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/tmp/quorra")
        .join(name.trim_end_matches(".pdf"));
    if std::fs::create_dir_all(&root).is_err() {
        return;
    }
    for (what, raster) in [("cpu", cpu), ("quorra", ours)] {
        let Ok(file) = std::fs::File::create(root.join(format!("{what}.png"))) else {
            continue;
        };
        let mut encoder =
            png::Encoder::new(std::io::BufWriter::new(file), raster.width, raster.height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        if let Ok(mut writer) = encoder.write_header() {
            let _ = writer.write_image_data(&raster.data);
        }
    }
}

/// Compares one page, or says why it could not be.
fn outcome(cpu: &pdf_render::Raster, ours: Result<pdf_render::Raster, impl ToString>) -> Outcome {
    let ours = match ours {
        Ok(raster) => raster,
        Err(why) => return Outcome::Refused(why.to_string()),
    };
    let Ok(c) = raster_compare::compare(cpu, &ours) else {
        return Outcome::Skipped;
    };
    if c.mean_error < MAX_MEAN_ERROR
        && c.worst_tile_error < MAX_WORST_TILE_ERROR
        && c.structural_similarity > MIN_STRUCTURAL_SIMILARITY
    {
        return Outcome::Agrees;
    }
    Outcome::Differs(format!(
        "mean {:.4} worst tile {:.2} at {:?} differing {:.4} ssim {:.5}",
        c.mean_error,
        c.worst_tile_error,
        c.worst_tile_at,
        c.differing_fraction,
        c.structural_similarity,
    ))
}

/// The display list of a document's first page, or `None` where there is not one.
fn page_one(path: &Path) -> Option<DisplayList> {
    let bytes = std::fs::read(path).ok()?;
    let document = Document::open(bytes).ok()?;
    let pages = pdf_model::Pages::new(&document);
    let page = pages.get(0)?;
    Some(pdf_model::content::interpret(&document, &page).display_list)
}

/// The corpus files, or `None` when the submodule is not checked out.
fn corpus() -> Option<Vec<PathBuf>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/pdf.js/test/pdfs");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&root)
        .ok()?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().is_some_and(|extension| extension == "pdf"))
        .collect();
    if files.is_empty() {
        return None;
    }
    files.sort();
    Some(files)
}
