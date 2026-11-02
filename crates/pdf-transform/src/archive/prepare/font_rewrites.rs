//! The fonts family of the preparations: the font objects the two Unicode requirements failed at,
//! the writing direction of each program whose metrics are restated, and the substitutions a
//! font the document does not embed is recorded under.
//!
//! They are one family because each answers a row whose requirement identifier begins `fonts/`,
//! and each starts from the same population, the font objects the validator's report names. The
//! module is not called `fonts` because the parent already reads `super::fonts`, the font
//! rewriter these preparations feed. The family's headings are `decision/fonts`'s, section 3 of
//! `doc/pdfa-blocking-issues.md`.

use super::super::ArchivePlan;
use super::super::decision::{Because, Loss, REMEDIES};
use super::super::fonts::{self, Directions, Metrics, Substitutes};
use super::super::rewrite::Rewrite;
use super::super::sites;
use super::{NOT_ASKED_FOR, Spare};
use pdf_archive::survey::Survey;
use pdf_syntax::Document;
use pdf_syntax::object::ObjectId;
use std::collections::{BTreeMap, BTreeSet};

/// Every font object the two Unicode requirements were found to fail at.
///
/// The population the derivation runs over, read off the validator's report rather than out of
/// the document: ISO 19005-2 section 6.2.11.7.2 excuses four kinds of font and which fonts those
/// are is the validator's reading, not this converter's. **Every distinct place**,
/// [`super::super::sites::places_failed`]'s, and not the thirty-two [`Outcome::Failed`](pdf_archive::Outcome::Failed) keeps
/// (`doc/adr/2616`).
///
/// # Errors
///
/// [`super::super::sites::PLACES_CUT_SHORT`]'s sentence where either requirement's place list is itself a
/// prefix, so that a derivation over it would leave fonts without a map.
pub(super) fn unicode_fonts(input: &pdf_archive::Report) -> Result<Vec<ObjectId>, &'static str> {
    let mut out: Vec<ObjectId> = Vec::new();
    for judgement in input
        .failures()
        .filter(|judgement| UNICODE_REQUIREMENTS.contains(&judgement.id))
    {
        let places = sites::places_failed(judgement).map_err(|_| sites::PLACES_CUT_SHORT)?;
        out.extend(places.iter().filter_map(|place| place.object));
    }
    out.sort_unstable();
    out.dedup();
    Ok(out)
}

/// The two requirements a derived `/ToUnicode` `CMap` answers.
const UNICODE_REQUIREMENTS: [&str; 2] = [
    "fonts/to-unicode-present",
    "fonts/to-unicode-values-are-usable",
];

/// How many substituted fonts the `xmpMM:History` entry names before it stops listing them.
const MOST_FONTS_NAMED: usize = 8;

/// Why a font is refused rather than substituted when the caller asked for that.
///
/// `doc/pdf-a-conversion-limits.md` section 4.9's `--no-substitute`, and the sentence says which
/// of the four kinds of *no* this is — the one the caller can take back.
const SUBSTITUTION_DECLINED: &str = "this document renders a font it does not embed, and \
     --no-substitute was asked for. The default is to embed one of the faces this program ships \
     and report which, on the argument that a file whose font is not embedded has no appearance \
     of its own; the flag is for a curator who would rather be told the font is missing than be \
     given a stand-in. Drop the flag, or supply the font itself with --font";

/// Which of the two metric restatements a document's failures ask for.
///
/// Separate questions rather than one, because the two requirements are separate: ISO 19005-2
/// states only the horizontal one, so a part 2 target must leave a disagreeing `vmtx` exactly
/// where its producer left it.
pub(super) fn metric_directions(failed: &BTreeSet<&'static str>) -> Directions {
    Directions {
        widths: wanted_by(failed, Rewrite::RestateFontMetrics),
        vertical: wanted_by(failed, Rewrite::RestateVerticalFontMetrics),
    }
}

/// Whether any requirement the document failed is answered by this rewrite.
pub(super) fn wanted_by(failed: &BTreeSet<&'static str>, rewrite: Rewrite) -> bool {
    REMEDIES.iter().any(|remedy| {
        failed.contains(remedy.requirement) && remedy.answer.rewrites().contains(&Some(rewrite))
    })
}

/// What the two font rewrites need, worked out from one content walk.
pub(super) struct PreparedFonts {
    /// The faces to embed where the file embedded none, or why none can be.
    pub(super) substitutes: Result<Substitutes, Because>,
    /// The programs whose stated advances are to be restated, or why none can be.
    pub(super) metrics: Result<Metrics, Because>,
    /// The `xmpMM:History` parameters the substitution has to record, empty where there is none.
    pub(super) recorded: String,
}

/// Works out both font rewrites, from one walk of the document's content.
///
/// **The content walk is made once and only where a font rewrite asked for it.** Both
/// preparations need the codes the content streams showed — which glyph a width belongs to is a
/// question about what was drawn — and `pdf_archive::check` has already made this walk for its
/// own rules without keeping it. Walking a second time is the cost of that seam; a document
/// whose fonts all conform pays none of it.
pub(super) fn prepare_fonts(
    plan: &ArchivePlan,
    document: &Document,
    spare: &mut Spare,
    wants_substitutes: bool,
    wants_metrics: Directions,
    // What the checker read: the survey, the input's report and whether the runs it read as
    // operators were written as numbers (`doc/adr/2638`), so that their pages are not fenced.
    (survey, input, runs_written): (Option<&Survey>, &pdf_archive::Report, bool),
    defined: &BTreeMap<Vec<u8>, ObjectId>,
) -> PreparedFonts {
    // **The wider photograph gate, where the profile authorised it** (A694 item 1): the pages that
    // hold a fault of their content stream are photographed too, which asks for the same
    // preparation a font asks for even when no font failed.
    let anywhere = plan.authorised.grants(Loss::PageAsImage) && fonts::Gate::admits(plan.target);
    let fenced = match survey {
        Some(survey) if anywhere => super::super::fenced::pages(input, survey, runs_written),
        _ => super::super::fenced::Fenced::default(),
    };
    let substitutes = match (wants_substitutes || !fenced.pages.is_empty(), survey) {
        (true, _) if !plan.substitute_fonts => Err(Because::Declined(SUBSTITUTION_DECLINED)),
        (true, Some(survey)) => fonts::embed_faces(
            document,
            survey,
            spare,
            &fonts::Routes {
                supplied: &plan.supplied_fonts,
                pack: plan.cjk_pack.as_ref(),
                target: plan.target,
                gate: fonts::Gate::new(
                    plan.target,
                    (
                        anywhere,
                        plan.authorised.grants(Loss::TaggingOfPhotographedPages),
                        plan.authorised
                            .grants(Loss::OptionalContentOfPhotographedPages),
                    ),
                    &fenced,
                ),
                defined,
            },
        ),
        _ => Err(Because::NotBuiltYet(NOT_ASKED_FOR)),
    };
    let metrics = match (wants_metrics.widths || wants_metrics.vertical, survey) {
        (true, Some(survey)) => fonts::restate_metrics(document, survey, wants_metrics, spare),
        _ => Err(Because::NotBuiltYet(NOT_ASKED_FOR)),
    };
    let recorded = substitutes
        .as_ref()
        .map(Substitutes::recorded)
        .unwrap_or_default();
    PreparedFonts {
        substitutes,
        metrics,
        recorded,
    }
}

/// The `parameters` field of the substitution's recorded action: which face went where.
///
/// Empty where nothing was substituted. Each font is named by what the file asked for, what was
/// embedded and which of section 4.9's two metric routes it took, because those three are what
/// section 4.9 asks a converter to report per font and a provenance entry a reader can act on
/// has to carry them in the file rather than only in the run's own report. The resource name the
/// content selected it by is stated beside the `/BaseFont`, as the run's report states it, because
/// two fonts asking for one face are otherwise two identical sentences a later reader cannot
/// tell apart.
pub(in crate::archive) fn substituted_fonts_recorded(fonts: &[fonts::SubstitutedFont]) -> String {
    if fonts.is_empty() {
        return String::new();
    }
    let named: Vec<String> = fonts
        .iter()
        .take(MOST_FONTS_NAMED)
        .map(|font| {
            let described = font
                .described
                .map(|described| format!(" ({})", described.describe()))
                .unwrap_or_default();
            // Every loss the font's embedding incurred is in the file's own history as well as
            // the run's report, the fitting with the scale it removed (`doc/adr/2262`).
            let lost = if font.losses.is_empty() {
                String::new()
            } else {
                format!(", losing {}", font.losses.words())
            };
            let fitting = font
                .fitting
                .map(|fitting| {
                    format!(" (the face had been drawn at the horizontal scale {fitting})")
                })
                .unwrap_or_default();
            // The widths of a font the file described by nothing are this viewer's layout of
            // the name, which the file's own history says in as many words (`doc/adr/2504`).
            let layout = if font.losses.contains(Loss::LayoutFromThisViewer) {
                " (the file states no widths for this font; the widths written are the layout \
                 this viewer drew it with)"
            } else {
                ""
            };
            // `doc/questions/A48`'s condition: each empty glyph is in the file's own history,
            // naming the clause it answers (`doc/adr/2458`).
            let empty = if font.empty_glyphs.is_empty() {
                String::new()
            } else {
                format!("; {}", fonts::empty_glyphs_given(&font.empty_glyphs))
            };
            // A cut program's front is removed from the file, and the file's history says so
            // (`doc/adr/2636`).
            let cut = font
                .cut_program
                .map(|cut| format!("; {}", cut.describe()))
                .unwrap_or_default();
            format!(
                "{} (resource {}) was not embedded and {} was embedded in its place, {}, taking \
                 the {} route{described}{lost}{fitting}{layout}{empty}{cut}",
                font.requested,
                font.resource,
                font.face,
                font.authority.describe(),
                font.route.word()
            )
        })
        .collect();
    let rest = fonts.len().saturating_sub(named.len());
    let tail = if rest == 0 {
        String::new()
    } else {
        format!(", and {rest} more")
    };
    format!(
        "fonts were substituted so that this file carries the programs ISO 19005-2 clause \
         6.2.11.4.1 requires: {}{tail}. The glyph widths the font dictionaries state were not \
         changed, so no mark moved; the shapes drawn are the substituted faces'",
        named.join("; ")
    )
}
