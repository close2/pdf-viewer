//! Every corpus document's first page, drawn by us and by three independent renderers.
//!
//! # What this gate is for, and how it differs from `corpus.rs`
//!
//! `corpus.rs` asks whether we *reported* everything we could not draw. This asks whether
//! what we did draw is *right*, which no amount of self-reporting can establish. The two
//! defects found in the fourth working session — every gradient mirrored about the page's
//! centre line, every image sampled through a doubled transform — sat on pages that
//! reported nothing wrong, on both backends at once, and would have been caught the first
//! time this ran.
//!
//! The oracle is [`pdfref`]'s triangulation rule: where two independent implementations
//! agree with each other and we differ from both, we are wrong. Where they disagree among
//! themselves there is no answer to hold us to, and the page is recorded rather than
//! failed. Principle 5 governs what happens next: a contradiction is a question to take
//! back to the specification, never a target to move towards. The value of this gate is
//! that it *finds the page*; it never says what the answer is.
//!
//! # Why the bound is relative to the references' own disagreement
//!
//! A fixed tolerance has to serve a page of flat fills, where the references agree to a
//! worst tile of 0.4, and a page of small text, where they disagree at 26 among
//! themselves. No single number separates signal from noise on both. So the references'
//! own spread on *that page* sets the bound — see [`pdfref::Judgement::CORPUS`] — and the
//! question asked is whether we are further from the consensus than the consensus is from
//! itself. On a spread sample of this corpus that distinction was the difference between
//! 15 pages outside the fixed bounds and the 8 among them that are genuinely contradicted.
//!
//! # What is gated, and what is only recorded
//!
//! Only pages we claim to draw *completely*. A page whose interpretation reports an
//! unsupported font or an undecodable image is expected to differ from a renderer that
//! implements it, and listing all of them would drown the signal. Those pages are counted
//! and printed — the count is a rough measure of how much the missing features cost
//! visually — but they cannot fail this gate. `corpus.rs` owns them.
//!
//! # Running it
//!
//! ```text
//! cargo test --release -p pdf-model --test oracle -- --ignored --nocapture
//! ```
//!
//! Artefacts for every page that is not agreement — our render, each reference's, and a
//! difference heatmap — are kept under the test's temporary directory and named in the
//! output. Pages that agree have theirs deleted: three thousand PNGs of pages nobody will
//! look at is a gigabyte of evidence for nothing.

#![expect(
    clippy::print_stdout,
    reason = "test code: the survey output is the point of the run, and on a failure it \
              is the evidence"
)]

use std::path::{Path, PathBuf};
use std::time::Instant;

use pdf_render::{Raster, Rasterizer, TargetSpec};
use pdf_syntax::Document;
use pdfref::{Judgement, Outcome, Reference, Tolerance, normalise, report};
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
use render_cpu::CpuRasterizer;

/// Comparison resolution. 72 dpi means one pixel per PDF unit, so a difference is a
/// difference rather than a resampling artefact.
const DPI: u32 = 72;

/// The same resolution as our scale factor, in pixels per PDF unit.
const SCALE: f32 = 1.0;

/// Pixel budget per page, the same one `corpus.rs` uses.
const PIXEL_BUDGET: u64 = 64 << 20;

/// Documents whose first page we claim to draw completely, and which two independent
/// reference renderers contradict: pages carrying an annotation appearance we do not draw.
///
/// 44 documents, and the largest single group. The annotations on them are 122 `Widget`,
/// 24 `Ink`, 17 `FreeText`, 4 `Stamp`, and one each of `Square` and `Highlight` — all
/// visible, all carrying an `/AP`, none drawn and none reported.
///
/// # How these four lists work
///
/// Named rather than counted, and checked for *equality* rather than as an upper bound: a
/// page that starts disagreeing fails the gate even if another was fixed the same day, and
/// a page that is fixed must be deleted from its list rather than left to rot. That is what
/// "a fixed one can never come back" requires.
///
/// The grouping is by what page one *carries*, which is a hypothesis about the cause and
/// not a diagnosis. A page here may differ for some quite other reason, and only the
/// artefacts settle it. What every entry does establish is that two implementations sharing
/// no code agree about this page and we do not. The groups are also how the list is meant
/// to shrink: drawing annotation appearances should empty this one, honouring optional
/// content the next.
const CONTRADICTED_ANNOTATIONS: [&str; 44] = [
    "annotation-stamp.pdf",
    "annotation-tx2.pdf",
    "annotation-tx3.pdf",
    "annotation_hidden_noview.pdf",
    "bug1669097.pdf",
    "bug1770750.pdf",
    "bug1782564.pdf",
    "bug1796741.pdf",
    "bug1802888.pdf",
    "bug1811510.pdf",
    "bug1811694.pdf",
    "bug1851498.pdf",
    "bug1883609.pdf",
    "bug1963407.pdf",
    "dates.pdf",
    "evaljs.pdf",
    "fields_order.pdf",
    "firefox_logo.pdf",
    "firefox_stamp.pdf",
    "inks.pdf",
    "inks_basic.pdf",
    "issue12706.pdf",
    "issue13003.pdf",
    "issue14023.pdf",
    "issue14502.pdf",
    "issue14705.pdf",
    "issue15053.pdf",
    "issue15096.pdf",
    "issue15597.pdf",
    "issue15815.pdf",
    "issue16500.pdf",
    "issue16553.pdf",
    "issue16633.pdf",
    "issue17492.pdf",
    "issue17998.pdf",
    "issue18536.pdf",
    "issue19424.pdf",
    "issue19505.pdf",
    "issue3885.pdf",
    "js-authors.pdf",
    "js-colors.pdf",
    "pr12828.pdf",
    "pr6531_2.pdf",
    "red_stamp.pdf",
];

/// Contradicted, with optional content configured off on page one.
///
/// 4 documents. We ignore `/OCProperties`, so a hidden layer is drawn anyway.
/// `issue12007_reduced.pdf` is the extreme case: a whole hidden screenshot over a page the
/// references leave nearly blank.
const CONTRADICTED_OPTIONAL_CONTENT: [&str; 4] = [
    "issue11144_reduced.pdf",
    "issue12007_reduced.pdf",
    "issue18823.pdf",
    "visibility_expressions.pdf",
];

/// Contradicted, with a font on page one that carries no embedded program.
///
/// 21 documents. The weakest entries here, because the difference need not be anyone's
/// defect: every renderer substitutes, and where two references happen to choose the same
/// system font and we choose another, the consensus is about their font rather than about
/// the page. `pdf-font`'s `substitute` module is the only machine-dependent code in the
/// tree, so this is also the group that could legitimately differ on another machine.
///
/// Listed rather than excluded, because a page in this group can *also* be wrong for a real
/// reason and dropping it would hide that — and two of them are. `calgray.pdf` and
/// `calrgb.pdf` land here because they label their swatches with a non-embedded font, while
/// what actually differs is the swatches: ours come out markedly darker than all three
/// references, `A = 0.35` reading as a near-black rather than a mid grey. §8.6.5.2 and
/// §8.6.5.3 define both spaces in CIE terms, so the conversion ends in XYZ and the
/// destination's encoding transfer function still has to be applied; ours looks like linear
/// luminance written straight into an sRGB raster. That is what "hypothesis, not diagnosis"
/// means in practice.
const CONTRADICTED_SUBSTITUTED_FONT: [&str; 21] = [
    "Type3WordSpacing.pdf",
    "alphatrans.pdf",
    "bad-PageLabels.pdf",
    "bug1011159.pdf",
    "bug1671312_reduced.pdf",
    "calgray.pdf",
    "calrgb.pdf",
    "franz_2.pdf",
    "hello_world_rotated.pdf",
    "issue4304.pdf",
    "issue5039.pdf",
    "issue5238.pdf",
    "issue6019.pdf",
    "issue6108.pdf",
    "issue6605.pdf",
    "issue7580.pdf",
    "issue8088.pdf",
    "issue8092.pdf",
    "issue8125.pdf",
    "issue918.pdf",
    "pr4922.pdf",
];

/// Contradicted with nothing on the page to explain it. **This is the interesting list.**
///
/// 74 documents whose page one carries no undrawn annotation, no hidden optional content and
/// no substituted font — so the difference is in something we believe we implement. Five
/// were examined by looking at the artefacts; the rest are unexamined, and working through
/// them is the highest-value use of this gate:
///
/// - `knockout_*.pdf` are knockout transparency groups (§11.4.5.6), where an object
///   composites against the group's initial backdrop rather than against what is already
///   there. `mutool` and `gs` show no blend where two rectangles overlap; we and `poppler`
///   show the blend. Unimplemented, and — unlike soft masks — unreported.
/// - `mesh_shading_empty.pdf` draws the same mesh as the references, displaced
///   horizontally. A placement question rather than a missing feature.
const CONTRADICTED_UNEXPLAINED: [&str; 74] = [
    "annotation-square-circle-without-appearance.pdf",
    "annotation-tx.pdf",
    "bug1108301.pdf",
    "bug1132849.pdf",
    "bug1151216.pdf",
    "bug1175962.pdf",
    "bug1200096.pdf",
    "bug1252420.pdf",
    "bug1539074.1.pdf",
    "bug868745.pdf",
    "close-path-bug.pdf",
    "colors.pdf",
    "function_based_shading_cmyk.pdf",
    "issue1002.pdf",
    "issue10572.pdf",
    "issue10900.pdf",
    "issue11279.pdf",
    "issue11477_reduced.pdf",
    "issue11549_reduced.pdf",
    "issue1171.pdf",
    "issue11740_reduced.pdf",
    "issue13107_reduced.pdf",
    "issue14117.pdf",
    "issue14462_reduced.pdf",
    "issue1453.pdf",
    "issue15516_reduced.pdf",
    "issue1655r.pdf",
    "issue17333.pdf",
    "issue18548_reduced.pdf",
    "issue18816.pdf",
    "issue19633.pdf",
    "issue20062.pdf",
    "issue215.pdf",
    "issue2948.pdf",
    "issue3207r.pdf",
    "issue3405r.pdf",
    "issue3566.pdf",
    "issue3694_reduced.pdf",
    "issue3928.pdf",
    "issue4061.pdf",
    "issue4550.pdf",
    "issue4650.pdf",
    "issue5686.pdf",
    "issue5751.pdf",
    "issue5994.pdf",
    "issue6068.pdf",
    "issue6231_1.pdf",
    "issue6336.pdf",
    "issue6387.pdf",
    "issue6721_reduced.pdf",
    "issue6889.pdf",
    "issue6894.pdf",
    "issue6901.pdf",
    "issue6961.pdf",
    "issue7180.pdf",
    "issue7439.pdf",
    "issue7492.pdf",
    "issue7696.pdf",
    "issue8097_reduced.pdf",
    "issue8234.pdf",
    "issue845r.pdf",
    "issue8570.pdf",
    "issue8960_reduced.pdf",
    "knockout_inner_backdrop.pdf",
    "knockout_isolated_overlap.pdf",
    "knockout_nested.pdf",
    "knockout_nested_group_alpha.pdf",
    "mesh_shading_empty.pdf",
    "openoffice.pdf",
    "pattern_text_embedded_font.pdf",
    "postscript_type4_many_outputs.pdf",
    "tiling-pattern-large-steps.pdf",
    "transparent.pdf",
    "type4psfunc.pdf",
];

/// Documents where our page geometry differs from the references' by more than the one
/// pixel a fractional page size can round to.
///
/// Separate from the four lists above because it is a different and more serious class of
/// defect: a page box, `/Rotate` or `/UserUnit` read differently, not pixels drawn
/// differently. The comparison cannot even proceed.
///
/// Three documents, and the cause of two of them is known. `bug1947248_forms.pdf` and
/// `bug1947248_text.pdf` carry `/UserUnit 3`, which §7.7.3.3 defines as the size of a
/// default user-space unit in multiples of 1/72 inch: `mutool` and `gs` scale the page by
/// it and produce 1836x2376, we and `poppler` do not and produce 612x792. We neither apply
/// it nor report it, which is the same silence `/Mask` and the text clipping modes were in.
/// `issue19176.pdf` is the reverse case — we and `poppler` take a 9x11 page where `mutool`
/// and `gs` fall back to 612x792 — and has not been looked into.
const GEOMETRY: [&str; 3] = [
    "bug1947248_forms.pdf",
    "bug1947248_text.pdf",
    "issue19176.pdf",
];

/// What the oracle concluded about one document.
#[derive(Debug)]
enum Verdict {
    /// We agree with a consensus of at least two references.
    Agrees,
    /// The references agree with each other and contradict us. A defect.
    Contradicted(String),
    /// The references disagree among themselves. No answer to hold us to.
    Ambiguous(String),
    /// Our page size differs from the references' by more than rounding. A defect.
    OurGeometry(String),
    /// The references disagree about the page size. Not ours to answer.
    ReferenceGeometry(String),
    /// Fewer than two references produced an image, so nothing can be triangulated.
    NotComparable(String),
    /// We could not produce a page to compare. `corpus.rs` owns this outcome.
    NoRender(String),
}

impl Verdict {
    /// Short label for the summary line.
    fn label(&self) -> &'static str {
        match self {
            Self::Agrees => "agrees",
            Self::Contradicted(_) => "CONTRADICTED",
            Self::Ambiguous(_) => "ambiguous",
            Self::OurGeometry(_) => "GEOMETRY",
            Self::ReferenceGeometry(_) => "reference geometry",
            Self::NotComparable(_) => "not comparable",
            Self::NoRender(_) => "no render",
        }
    }

    /// What was measured, for the printed survey.
    fn detail(&self) -> &str {
        match self {
            Self::Agrees => "",
            Self::Contradicted(detail)
            | Self::Ambiguous(detail)
            | Self::OurGeometry(detail)
            | Self::ReferenceGeometry(detail)
            | Self::NotComparable(detail)
            | Self::NoRender(detail) => detail,
        }
    }
}

/// One document's result.
#[derive(Debug)]
struct Examined {
    name: String,
    verdict: Verdict,
    /// Whether we reported the page as fully drawn. Only complete pages are gated.
    complete: bool,
}

/// Every document to compare: the corpus, plus the specification PDFs in `doc/`.
///
/// `doc/` is included because those are the documents whose rendering is looked at by hand
/// most often, and because they are the largest and most typographically demanding files
/// in the tree. Returns `None` when the corpus submodule is absent, in which case the gate
/// reports being skipped rather than failing — but `doc/` alone is not a substitute, and
/// the ratchet only means anything where the corpus is present.
fn documents() -> Option<Vec<PathBuf>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut files: Vec<PathBuf> = std::fs::read_dir(root.join("doc/pdf.js/test/pdfs"))
        .ok()?
        .chain(std::fs::read_dir(root.join("doc")).ok()?)
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().is_some_and(|extension| extension == "pdf"))
        .collect();
    if files.is_empty() {
        return None;
    }
    files.sort();
    Some(files)
}

/// Our own render of a page, with the two facts about it the comparison needs.
#[derive(Debug)]
struct OurRender {
    raster: Raster,
    /// Whether the interpretation reported nothing it was unable to draw.
    complete: bool,
    /// Whether the page drew any glyphs, which decides which bounds apply.
    has_text: bool,
}

/// Renders one page with our own pipeline, from the file's bytes.
fn render_ours(path: &Path) -> Result<OurRender, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("unreadable: {e}"))?;
    let document = Document::open(bytes).map_err(|e| format!("will not open: {e}"))?;
    let page = pdf_model::Pages::new(&document)
        .get(0)
        .ok_or_else(|| "no first page".to_owned())?;
    let interpretation = pdf_model::interpret(&document, &page);
    let complete = interpretation.is_complete();
    // The readback of the glyphs actually drawn, not a guess from the resources: a page
    // listing a font it never uses is a vector page.
    let has_text = !interpretation.text.trim().is_empty();

    let list = interpretation.display_list;
    let target =
        TargetSpec::for_page(&list, SCALE, PIXEL_BUDGET).map_err(|e| format!("no target: {e}"))?;
    let raster = CpuRasterizer::new()
        .rasterize(&list, target)
        .map_err(|e| format!("will not rasterise: {e}"))?;
    Ok(OurRender {
        raster,
        complete,
        has_text,
    })
}

/// Compares one document against the references.
fn examine(path: &Path, work_root: &Path, available: &[Reference]) -> Examined {
    let name = path.file_name().map_or_else(
        || path.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    );
    let stem = path.file_stem().unwrap_or_default().to_string_lossy();
    let work_dir = work_root.join(stem.as_ref());

    let OurRender {
        raster: mut ours,
        complete,
        has_text,
    } = match render_ours(path) {
        Ok(rendered) => rendered,
        Err(detail) => {
            return Examined {
                name,
                verdict: Verdict::NoRender(detail),
                complete: false,
            };
        }
    };

    let mut references = match render_references(path, &work_dir, available) {
        Ok(references) => references,
        Err(detail) => {
            let _ = std::fs::remove_dir_all(&work_dir);
            return Examined {
                name,
                verdict: Verdict::NotComparable(detail),
                complete,
            };
        }
    };

    let outvoted = match reconcile(&mut ours, &mut references) {
        Ok(outvoted) => outvoted,
        Err(verdict) => {
            return Examined {
                name,
                verdict,
                complete,
            };
        }
    };

    // Text sets the noise floor between independent rasterisers — glyph hinting differs
    // between implementations in a way flat fills do not — so a page carrying glyphs is
    // judged by the bounds measured on text pages.
    let tolerance = if has_text {
        Tolerance::TEXT_HEAVY
    } else {
        Tolerance::VECTOR
    };

    let triangulation =
        match pdfref::triangulate_with(&ours, &references, &tolerance, Judgement::CORPUS) {
            Ok(triangulation) => triangulation,
            Err(e) => {
                return Examined {
                    name,
                    verdict: Verdict::NotComparable(format!("{e}")),
                    complete,
                };
            }
        };

    let verdict = verdict_of(&triangulation, outvoted.as_deref());
    if matches!(verdict, Verdict::Agrees) {
        // Nothing to look at, and three thousand agreeing pages of PNGs is a gigabyte.
        let _ = std::fs::remove_dir_all(&work_dir);
    } else {
        let _ = report::write_artefacts(&work_dir, &stem, &ours, &references, &triangulation);
    }

    Examined {
        name,
        verdict,
        complete,
    }
}

/// Renders one page with every available reference.
///
/// A reference that fails on a document is not evidence of anything — many of these files
/// are deliberately damaged, and a renderer refusing one is the correct behaviour — so its
/// absence is tolerated as long as two remain. Fewer than two is reported with every
/// failure's own message, because "not comparable" without a reason is not actionable.
fn render_references(
    path: &Path,
    work_dir: &Path,
    available: &[Reference],
) -> Result<Vec<(Reference, Raster)>, String> {
    let mut rendered = Vec::new();
    let mut failures = Vec::new();
    for reference in available {
        match reference.render(path, DPI, work_dir) {
            Ok(raster) => rendered.push((*reference, raster)),
            Err(e) => failures.push(format!("{e}")),
        }
    }
    if rendered.len() < 2 {
        return Err(failures.join("; "));
    }
    Ok(rendered)
}

/// Brings every raster to one size, or says whose geometry disagrees.
///
/// Two steps, so that a size disagreement names who disagreed.
///
/// The references first, and by *majority*: a renderer that reads the page's extent
/// differently from the other two is outvoted and dropped, exactly as it would be on
/// pixels. This is not leniency — 55 corpus documents are otherwise not compared at all,
/// among them pages where `mutool` draws a 196x59 region of a 612x792 page — and the
/// dropped renderer is named in the outcome. Where no two agree there is no consensus
/// about the page at all, and nothing to hold us to.
///
/// Then us against what they settled on, where a failure is ours: a `MediaBox`, `CropBox`
/// or `/Rotate` read differently, which is a more serious defect than any pixel.
fn reconcile(
    ours: &mut Raster,
    references: &mut Vec<(Reference, Raster)>,
) -> Result<Option<String>, Verdict> {
    let describe = |rasters: &[(Reference, Raster)]| {
        rasters
            .iter()
            .map(|(r, raster)| format!("{r} {}x{}", raster.width, raster.height))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let all_sizes = describe(references);

    // The largest set of references agreeing about the page's extent. Sizes within the
    // rounding slack count as the same size, which is what `to_common_size` decides.
    let majority = references
        .iter()
        .map(|(_, raster)| (raster.width, raster.height))
        .max_by_key(|size| {
            references
                .iter()
                .filter(|(_, other)| {
                    other.width.abs_diff(size.0) <= normalise::MAX_ROUNDING_SLACK
                        && other.height.abs_diff(size.1) <= normalise::MAX_ROUNDING_SLACK
                })
                .count()
        })
        .ok_or_else(|| Verdict::ReferenceGeometry("no references".to_owned()))?;

    let outvoted = references
        .iter()
        .filter(|(_, raster)| {
            raster.width.abs_diff(majority.0) > normalise::MAX_ROUNDING_SLACK
                || raster.height.abs_diff(majority.1) > normalise::MAX_ROUNDING_SLACK
        })
        .map(|(r, raster)| format!("{r} {}x{}", raster.width, raster.height))
        .collect::<Vec<_>>()
        .join(", ");
    references.retain(|(_, raster)| {
        raster.width.abs_diff(majority.0) <= normalise::MAX_ROUNDING_SLACK
            && raster.height.abs_diff(majority.1) <= normalise::MAX_ROUNDING_SLACK
    });
    if references.len() < 2 {
        return Err(Verdict::ReferenceGeometry(format!(
            "no two references agree about the page size ({all_sizes})"
        )));
    }

    {
        let mut views: Vec<&mut Raster> = references.iter_mut().map(|(_, r)| r).collect();
        normalise::to_common_size(&mut views)
            .map_err(|e| Verdict::ReferenceGeometry(format!("{e} ({all_sizes})")))?;
    }

    let our_size = (ours.width, ours.height);
    let mut views: Vec<&mut Raster> = std::iter::once(ours)
        .chain(references.iter_mut().map(|(_, r)| r))
        .collect();
    normalise::to_common_size(&mut views).map_err(|e| {
        Verdict::OurGeometry(format!(
            "{e} (ours {}x{}, {all_sizes})",
            our_size.0, our_size.1
        ))
    })?;

    Ok((!outvoted.is_empty()).then_some(outvoted))
}

/// Translates a triangulation into this gate's vocabulary.
///
/// `outvoted` names any reference dropped for disagreeing about the page size, which
/// belongs in the outcome: a verdict reached by two renderers rather than three is worth
/// less, and hiding that would be the harness flattering itself.
fn verdict_of(triangulation: &pdfref::Triangulation, outvoted: Option<&str>) -> Verdict {
    let note = outvoted.map_or_else(String::new, |dropped| {
        format!(" [{dropped} outvoted on page size]")
    });
    match &triangulation.outcome {
        Outcome::Agrees { .. } => Verdict::Agrees,
        Outcome::Regression { agreeing } => {
            let names: Vec<&str> = agreeing.iter().map(|r| r.name()).collect();
            Verdict::Contradicted(format!(
                "{} agree, we differ: {}{note}",
                names.join(" and "),
                measurements(triangulation)
            ))
        }
        Outcome::Ambiguous => Verdict::Ambiguous(format!("{}{note}", measurements(triangulation))),
        Outcome::NotEnoughReferences { available } => {
            Verdict::NotComparable(format!("{available} reference(s)"))
        }
        // `Outcome` is non-exhaustive. A conclusion this gate has never seen must be
        // visible rather than quietly folded into one of the outcomes above.
        other => Verdict::NotComparable(format!("unrecognised outcome {other:?}")),
    }
}

/// How far we sit from the references, next to the bounds that were applied.
///
/// Both, always: our number means nothing on its own. A worst tile of 30 is a defect on a
/// page where the references agree to 0.4 and unremarkable on one where they differ by 26,
/// and the bound is what carries that distinction — it is derived from this page's own
/// consensus. Printing the references' raw spread instead would mislead, because the pair
/// that sets the bound is the *consensus* pair, not the widest.
fn measurements(triangulation: &pdfref::Triangulation) -> String {
    let worst = triangulation.ours.iter().map(|(_, c)| c).fold(
        None::<&raster_compare::Comparison>,
        |worst, c| match worst {
            Some(previous) if previous.worst_tile_error >= c.worst_tile_error => Some(previous),
            _ => Some(c),
        },
    );
    let bounds = &triangulation.judged_by;
    let applied = format!(
        "bound mean {:.2} worst tile {:.2} ssim {:.4}",
        bounds.max_mean, bounds.max_worst_tile, bounds.min_structural_similarity
    );
    worst.map_or_else(
        || format!("nothing measured; {applied}"),
        |c| {
            format!(
                "ours at worst mean {:.2} worst tile {:.2} differing {:.2}% ssim {:.4}; {applied}",
                c.mean_error,
                c.worst_tile_error,
                c.differing_fraction * 100.0,
                c.structural_similarity
            )
        },
    )
}

/// The gate.
#[test]
#[ignore = "renders every corpus document four times; run explicitly, in release"]
fn our_rendering_agrees_with_the_reference_consensus_across_the_corpus() {
    let Some(files) = documents() else {
        println!("skipped: the doc/pdf.js submodule is not checked out");
        return;
    };

    let available = Reference::available();
    assert!(
        available.len() >= 2,
        "at least two reference renderers are needed to triangulate; found {}. Install: {}",
        available.len(),
        Reference::ALL
            .iter()
            .filter(|r| !r.is_available())
            .map(|r| r.package_hint())
            .collect::<Vec<_>>()
            .join(", ")
    );
    // Versions belong in the record: these renderers change their output between releases,
    // so a disagreement appearing tomorrow may be an upstream change rather than ours.
    for reference in &available {
        println!(
            "{}: {}",
            reference.name(),
            reference.version().unwrap_or_default()
        );
    }

    let work_root = Path::new(env!("CARGO_TARGET_TMPDIR")).join("oracle");
    let started = Instant::now();
    let mut results: Vec<Examined> = files
        .par_iter()
        .map(|path| examine(path, &work_root, &available))
        .collect();
    results.sort_by(|a, b| a.name.cmp(&b.name));
    let elapsed = started.elapsed();

    report(&results, elapsed);
    println!("artefacts under {}", work_root.display());

    // Only pages we claim to draw completely are gated: see the module documentation.
    let named = |predicate: &dyn Fn(&Examined) -> bool| -> Vec<&str> {
        results
            .iter()
            .filter(|e| e.complete && predicate(e))
            .map(|e| e.name.as_str())
            .collect()
    };
    // The four groups are one ratchet: which group a page belongs to is a hypothesis about
    // it, and holding each group separately would fail the build every time a hypothesis
    // turned out to be wrong rather than every time the rendering changed.
    let contradicted: Vec<&str> = CONTRADICTED_ANNOTATIONS
        .iter()
        .chain(&CONTRADICTED_OPTIONAL_CONTENT)
        .chain(&CONTRADICTED_SUBSTITUTED_FONT)
        .chain(&CONTRADICTED_UNEXPLAINED)
        .copied()
        .collect();
    assert_ratchet(
        "contradicted by the reference consensus",
        &named(&|e| matches!(e.verdict, Verdict::Contradicted(_))),
        &contradicted,
    );
    assert_ratchet(
        "disagreeing with the references about page geometry",
        &named(&|e| matches!(e.verdict, Verdict::OurGeometry(_))),
        &GEOMETRY,
    );
}

/// Prints every document that is not agreement, then the totals.
///
/// The per-document lines come first and unabbreviated: on a failure they are the evidence,
/// and a summary that hides which page moved is a summary nobody can act on. Each count is
/// given twice — over everything, and over the pages we claim to draw completely — because
/// only the second is gated and conflating them flatters the result.
fn report(results: &[Examined], elapsed: std::time::Duration) {
    for examined in results {
        if !matches!(examined.verdict, Verdict::Agrees) {
            println!(
                "  {}: {}{} — {}",
                examined.name,
                examined.verdict.label(),
                if examined.complete {
                    ""
                } else {
                    " (incomplete)"
                },
                examined.verdict.detail()
            );
        }
    }

    let count =
        |predicate: &dyn Fn(&Examined) -> bool| results.iter().filter(|e| predicate(e)).count();
    println!(
        "\n{} documents in {:.1}s ({} we call complete, {} incomplete)",
        results.len(),
        elapsed.as_secs_f64(),
        count(&|e| e.complete),
        count(&|e| !e.complete)
    );

    let summary = |name: &str, predicate: &dyn Fn(&Examined) -> bool| {
        println!(
            "  {name:<20} {:>4} total, {:>4} of them on pages we call complete",
            count(predicate),
            count(&|e| predicate(e) && e.complete)
        );
    };
    summary("agrees", &|e| matches!(e.verdict, Verdict::Agrees));
    summary("contradicted", &|e| {
        matches!(e.verdict, Verdict::Contradicted(_))
    });
    summary("ambiguous", &|e| matches!(e.verdict, Verdict::Ambiguous(_)));
    summary("our geometry", &|e| {
        matches!(e.verdict, Verdict::OurGeometry(_))
    });
    summary("reference geometry", &|e| {
        matches!(e.verdict, Verdict::ReferenceGeometry(_))
    });
    summary("not comparable", &|e| {
        matches!(e.verdict, Verdict::NotComparable(_))
    });
    summary("no render", &|e| matches!(e.verdict, Verdict::NoRender(_)));
}

/// Holds an outcome to an exact set of documents.
///
/// Both directions fail. A new name is a regression; a missing one means the list is stale
/// and the entry must be deleted, which is what keeps a fixed page from silently coming
/// back.
fn assert_ratchet(what: &str, actual: &[&str], expected: &[&str]) {
    let new: Vec<&str> = actual
        .iter()
        .copied()
        .filter(|name| !expected.contains(name))
        .collect();
    let gone: Vec<&str> = expected
        .iter()
        .copied()
        .filter(|name| !actual.contains(name))
        .collect();

    assert!(
        new.is_empty(),
        "{} document(s) newly {what}: {new:?}\n\
         Each is a page two independent implementations agree about and we do not. Read \
         the artefacts named above, then take the disagreement to the specification — \
         never to what the references produce.",
        new.len()
    );
    assert!(
        gone.is_empty(),
        "{} document(s) no longer {what}: {gone:?}\n\
         Delete them from the list: a fixed page must not be able to come back.",
        gone.len()
    );
}
