//! What the corpus's font descriptors actually state for §9.8.1's `/Ascent` and `/Descent`, and
//! how many fonts state a `/Widths` that is all zeroes.
//!
//! ADR 0216 owed this census before anything was changed, which is `doc/todo/13`'s rule: the height
//! of a selection highlight is not in the file, so this program invents it from Table 120's two
//! entries, and the guard on them used to check only that one exceeded the other. That accepts a
//! descriptor stating a box entirely below the baseline, a sliver, or one four ems tall. Whether
//! that is one file or a population is what decides how the round is spent, and nothing in this
//! tree knew.
//!
//! The band is [`pdf_font::measured_extent`]'s — this counts what the program itself would
//! reject, rather than a copy of the rule that could drift from it.
//!
//! ```sh
//! cargo run --release -p pdf-model --example font_metric_census -- doc/pdf.js/test/pdfs/*.pdf
//! ```

#![expect(
    clippy::print_stdout,
    reason = "an example whose entire output is a measurement"
)]

use std::collections::{BTreeMap, BTreeSet};

use pdf_syntax::{Dictionary, Document, Object, ObjectId};

/// How far a form `XObject`'s own resources are followed.
const MAX_DEPTH: usize = 8;

/// How many pages of one document are walked.
const MAX_PAGES: usize = 100;

/// What one font dictionary's vertical metrics turned out to be.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Verdict {
    /// No descriptor, or a descriptor stating neither entry: the em box already answers.
    Silent,
    /// A pair [`pdf_font::measured_extent`] accepts.
    Measured,
    /// A pair it rejects, and which the old `ascent > descent` guard also rejected.
    RejectedBefore,
    /// A pair it rejects which the old guard **accepted** — the defect this census is about.
    NewlyRejected,
}

/// One document's tallies.
#[derive(Default)]
struct Counts {
    fonts: usize,
    verdicts: BTreeMap<&'static str, usize>,
    /// Fonts whose `/Widths` is present, non-empty and every entry zero.
    zero_widths: usize,
    /// Fonts whose `/Ascent` is below their own `/CapHeight`.
    ///
    /// Table 120 defines `/CapHeight` as "the vertical coordinate of the top of flat capital
    /// letters, measured from the baseline" and `/Ascent` as the maximum height any glyph
    /// reaches; a capital letter is a glyph, so a descriptor whose ascent is under its cap
    /// height contradicts itself. Counted rather than acted on: whether it is worth a fourth
    /// condition in the band depends on whether any file states it, and on how badly.
    contradicts_cap_height: usize,
    /// Of those, the ones the band otherwise accepts — what a fourth condition would cost.
    cap_height_would_reject: usize,
    /// Fonts believed only because a positive `/Descent` is read as the depth it states.
    repaired: usize,
}

#[expect(
    clippy::too_many_lines,
    reason = "an example whose body is one measurement, printed as it is taken"
)]
fn main() {
    let mut documents = 0_usize;
    let mut fonts = 0_usize;
    let mut verdicts: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut zero_width_fonts = 0_usize;
    let mut contradicting = 0_usize;
    let mut cap_height_cost = 0_usize;
    let mut newly_rejected: Vec<String> = Vec::new();
    let mut rejected_before: Vec<String> = Vec::new();
    let mut repaired_documents: Vec<String> = Vec::new();
    let mut zero_widths: Vec<String> = Vec::new();
    let mut extremes: Vec<(String, f32, f32)> = Vec::new();

    for path in std::env::args().skip(1) {
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        documents = documents.saturating_add(1);
        let name = path.rsplit('/').next().unwrap_or(&path).to_owned();
        let pages = pdf_model::Pages::new(&document);
        let mut counts = Counts::default();
        let mut seen_fonts: BTreeSet<ObjectId> = BTreeSet::new();
        for index in 0..pages.len().min(MAX_PAGES) {
            let Some(page) = pages.get(index) else {
                continue;
            };
            let mut seen = BTreeSet::new();
            walk(
                &document,
                &page.resources,
                0,
                &mut seen,
                &mut seen_fonts,
                &mut counts,
                &name,
                &mut extremes,
            );
        }
        fonts = fonts.saturating_add(counts.fonts);
        zero_width_fonts = zero_width_fonts.saturating_add(counts.zero_widths);
        contradicting = contradicting.saturating_add(counts.contradicts_cap_height);
        cap_height_cost = cap_height_cost.saturating_add(counts.cap_height_would_reject);
        for (label, count) in &counts.verdicts {
            let total = verdicts.entry(label).or_default();
            *total = total.saturating_add(*count);
        }
        if counts.verdicts.contains_key(label(Verdict::NewlyRejected)) {
            newly_rejected.push(name.clone());
        }
        if counts.verdicts.contains_key(label(Verdict::RejectedBefore)) {
            rejected_before.push(name.clone());
        }
        if counts.repaired > 0 {
            repaired_documents.push(name.clone());
        }
        if counts.zero_widths > 0 {
            zero_widths.push(name);
        }
    }

    println!("{documents} document(s) opened, {fonts} distinct font dictionaries on their pages");
    for (label, count) in &verdicts {
        println!("  {count:6} {label}");
    }
    println!("  {zero_width_fonts:6} state a /Widths that is present, non-empty and all zeroes");
    println!(
        "  {contradicting:6} state an /Ascent below their own /CapHeight, {cap_height_cost} of \
         them in pairs the band accepts"
    );
    println!(
        "\n**{} document(s) state a pair the old `ascent > descent` guard accepted and the band \
         rejects**: {}",
        newly_rejected.len(),
        newly_rejected.join(" ")
    );
    println!(
        "\n{} document(s) state a pair both guards reject: {}",
        rejected_before.len(),
        rejected_before.join(" ")
    );
    println!(
        "\n{} document(s) state an all-zero /Widths: {}",
        zero_widths.len(),
        zero_widths.join(" ")
    );
    println!(
        "\n{} document(s) state a positive /Descent, read as the depth it states: {}",
        repaired_documents.len(),
        repaired_documents.join(" ")
    );

    let mut offending: BTreeSet<String> = BTreeSet::new();
    // How many *fonts* the sign repair is what saves, and how many the band still rejects for
    // each of its own conditions. A pair may fail more than one, so these do not partition.
    let (mut repaired, mut no_ascent, mut short, mut tall) = (0_usize, 0_usize, 0_usize, 0_usize);
    for (name, ascent, descent) in &extremes {
        if *descent > 0.0 && pdf_font::measured_extent(*ascent, *descent).is_some() {
            repaired = repaired.saturating_add(1);
        }
        if pdf_font::measured_extent(*ascent, *descent).is_some() || ascent <= descent {
            continue;
        }
        offending.insert(format!("  /Ascent {ascent:8} /Descent {descent:8}  {name}"));
        let line = ascent - descent.min(0.0);
        if *ascent <= 0.0 {
            no_ascent = no_ascent.saturating_add(1);
        }
        if line < 600.0 {
            short = short.saturating_add(1);
        }
        if line > 2400.0 {
            tall = tall.saturating_add(1);
        }
    }
    println!(
        "\n{repaired} font(s) are believed only because a positive /Descent is read as the depth \
         it states; of the pairs still rejected, {no_ascent} reach nothing above the baseline, \
         {short} state a line under 0.6 em and {tall} one over 2.4 em"
    );
    println!("\nthe distinct pairs the old guard accepted and the band rejects:");
    for line in &offending {
        println!("{line}");
    }

    extremes.sort_by(|a, b| (a.1 - a.2).total_cmp(&(b.1 - b.2)));
    println!("\nthe ten shortest lines any descriptor states, in glyph-space units:");
    for (name, ascent, descent) in extremes.iter().take(10) {
        println!("  {ascent:8} {descent:8}  {name}");
    }
    println!("the ten tallest:");
    for (name, ascent, descent) in extremes.iter().rev().take(10) {
        println!("  {ascent:8} {descent:8}  {name}");
    }
}

/// The label a verdict is counted under.
fn label(verdict: Verdict) -> &'static str {
    match verdict {
        Verdict::Silent => "state neither /Ascent nor /Descent (the em box answers)",
        Verdict::Measured => "state a pair the band accepts",
        Verdict::RejectedBefore => "state a pair `ascent > descent` already rejected",
        Verdict::NewlyRejected => "state a pair `ascent > descent` ACCEPTED and the band rejects",
    }
}

/// Every font dictionary reachable from one resource dictionary, including through form
/// `XObject`s — which is where `pdf_model::interpret` finds them too.
#[expect(
    clippy::too_many_arguments,
    reason = "an example's walker, whose arguments are its whole state"
)]
fn walk(
    document: &Document,
    resources: &Dictionary,
    depth: usize,
    seen: &mut BTreeSet<ObjectId>,
    seen_fonts: &mut BTreeSet<ObjectId>,
    counts: &mut Counts,
    name: &str,
    extremes: &mut Vec<(String, f32, f32)>,
) {
    if depth > MAX_DEPTH {
        return;
    }
    if let Some(dict) = document.get_key(resources, "Font").as_dict() {
        for (_, value) in dict.iter() {
            if let Some(id) = value.as_reference()
                && !seen_fonts.insert(id)
            {
                continue;
            }
            let resolved = document.resolve(value);
            let Some(font) = resolved.as_dict() else {
                continue;
            };
            counts.fonts = counts.fonts.saturating_add(1);
            let (verdict, repaired) = judge(document, font, name, extremes);
            if repaired {
                counts.repaired = counts.repaired.saturating_add(1);
            }
            let counter = counts.verdicts.entry(label(verdict)).or_default();
            *counter = counter.saturating_add(1);
            if all_zero_widths(document, font) {
                counts.zero_widths = counts.zero_widths.saturating_add(1);
            }
            if contradicts_cap_height(document, font) {
                counts.contradicts_cap_height = counts.contradicts_cap_height.saturating_add(1);
                if verdict == Verdict::Measured {
                    counts.cap_height_would_reject =
                        counts.cap_height_would_reject.saturating_add(1);
                }
            }
        }
    }
    let objects = document.get_key(resources, "XObject");
    let Some(dict) = objects.as_dict() else {
        return;
    };
    for (_, value) in dict.iter() {
        if let Some(id) = value.as_reference()
            && !seen.insert(id)
        {
            continue;
        }
        let resolved = document.resolve(value);
        let Object::Stream(stream) = &resolved else {
            continue;
        };
        if let Some(inner) = document.get_key(&stream.dict, "Resources").as_dict() {
            walk(
                document,
                inner,
                depth.saturating_add(1),
                seen,
                seen_fonts,
                counts,
                name,
                extremes,
            );
        }
    }
}

/// What one font dictionary's descriptor states about its glyphs' reach.
fn judge(
    document: &Document,
    font: &Dictionary,
    name: &str,
    extremes: &mut Vec<(String, f32, f32)>,
) -> (Verdict, bool) {
    let Some(descriptor) = descriptor(document, font) else {
        return (Verdict::Silent, false);
    };
    let entry = |key: &str| {
        document
            .get_key(&descriptor, key)
            .as_number()
            .map(narrow)
            .filter(|value| value.is_finite())
    };
    let (Some(ascent), Some(descent)) = (entry("Ascent"), entry("Descent")) else {
        return (Verdict::Silent, false);
    };
    extremes.push((name.to_owned(), ascent, descent));
    let measured = pdf_font::measured_extent(ascent, descent).is_some();
    let verdict = if measured {
        Verdict::Measured
    } else if ascent > descent {
        Verdict::NewlyRejected
    } else {
        Verdict::RejectedBefore
    };
    (verdict, measured && descent > 0.0)
}

/// The font descriptor a font dictionary reaches, following a Type 0 font to its descendant.
fn descriptor(document: &Document, font: &Dictionary) -> Option<Dictionary> {
    let direct = document.get_key(font, "FontDescriptor");
    if let Some(dict) = direct.as_dict() {
        return Some(dict.clone());
    }
    let descendants = document.get_key(font, "DescendantFonts");
    let first = descendants.as_array()?.first()?;
    let descendant = document.resolve(first);
    let descendant = descendant.as_dict()?;
    document
        .get_key(descendant, "FontDescriptor")
        .as_dict()
        .cloned()
}

/// Whether the font's descriptor states an `/Ascent` under its own `/CapHeight`.
///
/// See [`Counts::contradicts_cap_height`] for why the two cannot both be measurements.
fn contradicts_cap_height(document: &Document, font: &Dictionary) -> bool {
    let Some(descriptor) = descriptor(document, font) else {
        return false;
    };
    let entry = |key: &str| {
        document
            .get_key(&descriptor, key)
            .as_number()
            .map(narrow)
            .filter(|value| value.is_finite())
    };
    match (entry("Ascent"), entry("CapHeight")) {
        (Some(ascent), Some(cap)) => cap > 0.0 && ascent < cap,
        _ => false,
    }
}

/// Narrows a PDF number to `f32`.
fn narrow(value: f64) -> f32 {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "a font metric outside f32's range is not a metric"
    )]
    {
        value as f32
    }
}

/// Whether the font states a `/Widths` array that is present, non-empty and every entry zero.
///
/// §9.6.2 does not forbid a zero width, so this is a shape to count rather than a rule to check:
/// every glyph of such a font is placed at the same point, and the quadrilateral built from the
/// advance has no width at all.
fn all_zero_widths(document: &Document, font: &Dictionary) -> bool {
    let widths = document.get_key(font, "Widths");
    let Some(items) = widths.as_array() else {
        return false;
    };
    !items.is_empty()
        && items
            .iter()
            .all(|item| document.resolve(item).as_number() == Some(0.0))
}
