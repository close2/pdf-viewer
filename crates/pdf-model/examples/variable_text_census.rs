//! What §12.7.4.3 lays out in the corpus, and what its two remaining edges are worth.
//!
//! `doc/todo/13`'s rule — measure the population before deciding what to build — over the two
//! edges `doc/todo/22` left with a claim rather than a number:
//!
//! - **the baseline.** `variable_text::Metrics::read` builds a field's baseline from Table 120's
//!   `/Ascent` and `/Descent` under a guard of its own, `ascent > 0 && descent < 0`, while
//!   `pdf_font::measured_extent` asks whether the pair could be a measurement of a face at all
//!   (ADR 0216). Where the two disagree the baseline moves, and this counts the widgets that
//!   would move — not the font dictionaries a page draws with, which
//!   `examples/font_metric_census` already counts and which are a different population.
//! - **§12.7.5.4's list box**, whose options are drawn since the five-hundred-and-seventy-first
//!   (ADR 0405) and whose *selection* is reported, because that is the only part of the control
//!   the clause states no appearance for. The counts here were taken while the whole control was
//!   refused, and they are what said the refusal cost no untouched page: every list box in the
//!   corpus states an `/AP` and none is in a `/NeedAppearances` document, so the population that
//!   was owed was the one the file cannot speak for — a value a person changed. The three counts
//!   below still answer that question and are worth re-running against a corpus this tree has
//!   not seen.
//!
//! The field types, their flags and their widgets are `pdf_model::form::fields`' — the program's
//! own reading — so the only rule spelled a second time here is the guard being replaced, which
//! is named as such below.
//!
//! ```sh
//! cargo run --release -p pdf-model --example variable_text_census -- doc/pdf.js/test/pdfs/*.pdf
//! ```

#![expect(
    clippy::print_stdout,
    reason = "an example whose entire output is a measurement"
)]

use std::collections::{BTreeMap, BTreeSet};

use pdf_model::form::Control;
use pdf_syntax::{Dictionary, Document, Object, ObjectId};

/// How many pages of one document are walked, matching `font_metric_census`'s bound.
const MAX_PAGES: usize = 100;

/// How far §12.7.4.1's `/Parent` chain is followed, matching `appearance.rs`'s own bound.
const MAX_ANCESTRY: usize = 32;

/// What one widget's `/DA` font says about where the field's baseline goes.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Baseline {
    /// The `/DA` names no font, or `/DR` defines none: a stand-in with no descriptor, and the
    /// em-relative split answers under either rule.
    NoDescriptor,
    /// A descriptor stating neither entry — the standard 14's usual case.
    Silent,
    /// Both rules read the pair, and to the same two numbers.
    Unmoved,
    /// The old guard believed the pair and the band does not: the baseline moves to the split.
    MovesToTheSplit,
    /// The band believes a pair the old guard refused: the baseline moves to the stated pair.
    MovesToTheStatedPair,
    /// Neither rule believes it, and the split answers under both.
    RefusedByBoth,
}

/// The label a verdict is counted under.
fn label(verdict: Baseline) -> &'static str {
    match verdict {
        Baseline::NoDescriptor => "draw in a font with no descriptor (/DR defines none)",
        Baseline::Silent => "draw in a font whose descriptor states neither entry",
        Baseline::Unmoved => "state a pair both rules believe, to the same numbers",
        Baseline::MovesToTheSplit => "state a pair the OLD guard believed and the band refuses",
        Baseline::MovesToTheStatedPair => {
            "state a pair the old guard refused and the BAND believes"
        }
        Baseline::RefusedByBoth => "state a pair neither rule believes",
    }
}

/// Every verdict, in the order they are printed.
const VERDICTS: &[Baseline] = &[
    Baseline::NoDescriptor,
    Baseline::Silent,
    Baseline::Unmoved,
    Baseline::MovesToTheSplit,
    Baseline::MovesToTheStatedPair,
    Baseline::RefusedByBoth,
];

/// One document's tallies.
#[derive(Default)]
struct Counts {
    /// Widgets whose field is a text field or a combo box — the two §12.7.4.3 lays text out for.
    laid_out: usize,
    /// Of those, the ones with no `/AP` `/N` stream, where the construction is the only mark.
    without_appearance: usize,
    /// Baseline verdicts over the widgets whose appearance is constructed.
    verdicts: BTreeMap<Baseline, usize>,
    /// §12.7.5.4's combo boxes: Table 233 bit 18 set on a `/Ch` field, and the half that draws.
    combo_boxes: usize,
    /// Of those, the ones with no `/AP` `/N` stream — the other half of §12.7.5.4's own claim.
    combo_boxes_without_appearance: usize,
    /// §12.7.5.4's list boxes: Table 233 bit 18 clear on a `/Ch` field.
    list_boxes: usize,
    /// Of those, the ones with no `/AP` `/N` stream, where the construction is the only mark.
    list_boxes_without_appearance: usize,
    /// §12.5.6.6's free text annotations, the *other* thing §12.7.4.3 lays text out for.
    ///
    /// Counted with the widgets rather than beside them because `Metrics::read` is one function
    /// and a `/DA` is a `/DA`: an annotation whose descriptor the two rules disagree about moves
    /// its baseline exactly as a field's does.
    free_text: usize,
    /// Objects whose `/DA` names a Table 119 Type 0 font, which §9.7.6.2's inverse now reaches.
    composite: usize,
    /// Of those, the ones whose `CMap` asks for §9.7.5.1's writing mode 1, which is refused.
    composite_vertical: usize,
}

impl Counts {
    /// Adds one document's tallies to a running total.
    fn add(&mut self, counts: &Self) {
        self.laid_out = self.laid_out.saturating_add(counts.laid_out);
        self.without_appearance = self
            .without_appearance
            .saturating_add(counts.without_appearance);
        self.combo_boxes = self.combo_boxes.saturating_add(counts.combo_boxes);
        self.combo_boxes_without_appearance = self
            .combo_boxes_without_appearance
            .saturating_add(counts.combo_boxes_without_appearance);
        self.list_boxes = self.list_boxes.saturating_add(counts.list_boxes);
        self.list_boxes_without_appearance = self
            .list_boxes_without_appearance
            .saturating_add(counts.list_boxes_without_appearance);
        self.free_text = self.free_text.saturating_add(counts.free_text);
        self.composite = self.composite.saturating_add(counts.composite);
        self.composite_vertical = self
            .composite_vertical
            .saturating_add(counts.composite_vertical);
        for (verdict, count) in &counts.verdicts {
            let total = self.verdicts.entry(*verdict).or_default();
            *total = total.saturating_add(*count);
        }
    }
}

/// Everything the run accumulates, so that taking a document and reporting are two functions.
#[derive(Default)]
struct Census {
    /// Files that opened, whatever they turned out to hold.
    documents: usize,
    /// Of those, the ones with something §12.7.4.3 or §12.7.5.4 reaches.
    with_widgets: usize,
    /// Of those, the ones setting Table 224's `/NeedAppearances`.
    need_appearances: usize,
    /// Every count above, summed.
    totals: Counts,
    /// Which documents a baseline moves in, by the verdict that moves it.
    moving: BTreeMap<String, BTreeSet<String>>,
    /// Documents with a list box, with one in a `/NeedAppearances` document, and with a bare one.
    list_boxes: Vec<String>,
    regenerated_list_boxes: Vec<String>,
    bare_list_boxes: Vec<String>,
    /// Documents with a combo box whose appearance §12.7.4.3 therefore constructs.
    bare_combo_boxes: Vec<String>,
    /// Documents whose `/DA` names a composite font, which is `doc/todo/22`'s last edge.
    composite_fonts: Vec<String>,
}

impl Census {
    /// Opens one file and adds what it holds.
    fn take(&mut self, path: &str) {
        let Ok(bytes) = std::fs::read(path) else {
            return;
        };
        let Ok(document) = Document::open(bytes) else {
            return;
        };
        self.documents = self.documents.saturating_add(1);
        let name = path.rsplit('/').next().unwrap_or(path).to_owned();
        let counts = walk(&document, &name, &mut self.moving);
        if counts.laid_out == 0 && counts.list_boxes == 0 && counts.free_text == 0 {
            return;
        }
        self.with_widgets = self.with_widgets.saturating_add(1);
        let regenerating = need_appearances_set(&document);
        if regenerating {
            self.need_appearances = self.need_appearances.saturating_add(1);
        }
        if counts.list_boxes > 0 {
            self.list_boxes.push(name.clone());
            if regenerating {
                self.regenerated_list_boxes.push(name.clone());
            }
        }
        if counts.combo_boxes_without_appearance > 0 {
            self.bare_combo_boxes.push(name.clone());
        }
        if counts.list_boxes_without_appearance > 0 {
            self.bare_list_boxes.push(name.clone());
        }
        if counts.composite > 0 {
            self.composite_fonts.push(name);
        }
        self.totals.add(&counts);
    }

    /// Prints the whole measurement.
    fn report(&self) {
        let (documents, with_widgets) = (self.documents, self.with_widgets);
        let need_appearances = self.need_appearances;
        let totals = &self.totals;
        println!(
            "{documents} document(s) opened, {with_widgets} with a widget §12.7.4.3 or §12.7.5.4 \
             reaches, {need_appearances} of them setting Table 224's /NeedAppearances"
        );
        println!(
            "\n§12.7.4.3: {} widget(s) of a text field or a combo box, {} of them with no /AP /N \
             stream, and {} free text annotation(s) sent to the same layout by §12.5.6.6",
            totals.laid_out, totals.without_appearance, totals.free_text
        );
        for verdict in VERDICTS {
            let count = totals.verdicts.get(verdict).copied().unwrap_or_default();
            println!("  {count:6} {}", label(*verdict));
        }
        for verdict in [Baseline::MovesToTheSplit, Baseline::MovesToTheStatedPair] {
            let files = self.moving.get(label(verdict)).cloned().unwrap_or_default();
            println!(
                "\n{} document(s) whose baseline {}: {}",
                files.len(),
                label(verdict),
                files.into_iter().collect::<Vec<_>>().join(" ")
            );
        }
        println!(
            "\n§9.7 + §12.7.4.3: {} object(s) whose /DA names a composite font, {} of them in \
             §9.7.5.1's writing mode 1, over {} document(s): {}",
            totals.composite,
            totals.composite_vertical,
            self.composite_fonts.len(),
            self.composite_fonts.join(" ")
        );
        println!(
            "\n§12.7.5.4: {} combo-box widget(s), {} of them with no /AP /N stream: {}",
            totals.combo_boxes,
            totals.combo_boxes_without_appearance,
            self.bare_combo_boxes.join(" ")
        );
        println!(
            "§12.7.5.4: {} list-box widget(s) over {} document(s), {} of them with no /AP /N \
             stream over {} document(s)",
            totals.list_boxes,
            self.list_boxes.len(),
            totals.list_boxes_without_appearance,
            self.bare_list_boxes.len()
        );
        println!("  with a list box: {}", self.list_boxes.join(" "));
        println!(
            "  with a list box and no appearance stream: {}",
            self.bare_list_boxes.join(" ")
        );
        // The second way the construction is reached: Table 224's `/NeedAppearances` sets a
        // stored stream aside for both choice-field arms, so a list box in such a document is
        // regenerated from `/Opt` rather than drawn from the stream its producer wrote.
        println!(
            "  with a list box in a /NeedAppearances document: {}",
            self.regenerated_list_boxes.join(" ")
        );
    }
}

fn main() {
    let mut census = Census::default();
    for path in std::env::args().skip(1) {
        census.take(&path);
    }
    census.report();
}

/// Every widget and free text annotation of every page of one document, classified.
fn walk(
    document: &Document,
    name: &str,
    moving: &mut BTreeMap<String, BTreeSet<String>>,
) -> Counts {
    let mut counts = Counts::default();
    let mut seen: BTreeSet<ObjectId> = BTreeSet::new();
    let pages = pdf_model::Pages::new(document);
    let view = pdf_model::view::ViewState::of(document);
    for index in 0..pages.len().min(MAX_PAGES) {
        let Some(page) = pages.get(index) else {
            continue;
        };
        for field in pdf_model::form::fields(document, &page, &view) {
            // §12.7.4.3's subject is "a field that may contain text whose value is not known
            // until viewing time", and a combo box's value is such a text. A list box is counted
            // apart because what it lays out is Table 234's `/Opt` rather than a value, which is
            // the other arm of the same match in `appearance.rs`.
            let lays_out = match field.control {
                Control::Text(_) => true,
                Control::Choice(ref choice) => choice.combo,
                _ => false,
            };
            let list_box = matches!(field.control, Control::Choice(ref choice) if !choice.combo);
            if !lays_out && !list_box {
                continue;
            }
            for widget in &field.widgets {
                if !seen.insert(widget.annotation) {
                    continue;
                }
                let object = document.get(widget.annotation);
                let Some(dict) = object.as_dict() else {
                    continue;
                };
                let bare = normal_appearance(document, dict).is_none();
                if list_box {
                    counts.list_boxes = counts.list_boxes.saturating_add(1);
                    if bare {
                        counts.list_boxes_without_appearance =
                            counts.list_boxes_without_appearance.saturating_add(1);
                    }
                    continue;
                }
                counts.laid_out = counts.laid_out.saturating_add(1);
                if bare {
                    counts.without_appearance = counts.without_appearance.saturating_add(1);
                }
                if matches!(field.control, Control::Choice(_)) {
                    counts.combo_boxes = counts.combo_boxes.saturating_add(1);
                    if bare {
                        counts.combo_boxes_without_appearance =
                            counts.combo_boxes_without_appearance.saturating_add(1);
                    }
                }
                let verdict = baseline(document, dict);
                let counter = counts.verdicts.entry(verdict).or_default();
                *counter = counter.saturating_add(1);
                record(verdict, name, moving);
                take_composite(document, dict, &mut counts);
            }
        }
        // §12.5.6.6 sends a free text annotation's `/DA` to §12.7.4.3 as well — "[t]he default
        // appearance string … shall be used in formatting the text (see 12.7.4.3)" — so its font
        // reaches the same `Metrics::read` and belongs in the same population.
        for entry in free_text_annotations(document, &page) {
            if !seen.insert(entry) {
                continue;
            }
            let object = document.get(entry);
            let Some(dict) = object.as_dict() else {
                continue;
            };
            counts.free_text = counts.free_text.saturating_add(1);
            let verdict = baseline(document, dict);
            let counter = counts.verdicts.entry(verdict).or_default();
            *counter = counter.saturating_add(1);
            record(verdict, name, moving);
            take_composite(document, dict, &mut counts);
        }
    }
    counts
}

/// Remembers which document a moving baseline was found in, for the two verdicts that move one.
fn record(verdict: Baseline, name: &str, moving: &mut BTreeMap<String, BTreeSet<String>>) {
    if !matches!(
        verdict,
        Baseline::MovesToTheSplit | Baseline::MovesToTheStatedPair
    ) {
        return;
    }
    moving
        .entry(label(verdict).to_owned())
        .or_default()
        .insert(name.to_owned());
}

/// §12.5.6.6's annotations on one page, which Table 177 makes `/Subtype /FreeText`.
fn free_text_annotations(document: &Document, page: &pdf_model::Page) -> Vec<ObjectId> {
    let annotations = document.get_key(&page.dict, "Annots");
    let Some(annotations) = annotations.as_array() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for entry in annotations {
        let Some(id) = entry.as_reference() else {
            continue;
        };
        let resolved = document.resolve(entry);
        let Some(dict) = resolved.as_dict() else {
            continue;
        };
        if document
            .get_key(dict, "Subtype")
            .as_name()
            .map(pdf_syntax::Name::as_bytes)
            == Some(b"FreeText")
        {
            out.push(id);
        }
    }
    out
}

/// Whether the widget's `/AP` states a normal appearance at all.
///
/// §12.5.5 makes `/N` "the annotation's normal appearance", either a stream or a subdictionary of
/// them; either is something to draw, and neither is what a constructed appearance replaces
/// unless Table 224's `/NeedAppearances` says so.
fn normal_appearance(document: &Document, widget: &Dictionary) -> Option<Object> {
    let appearance = document.get_key(widget, "AP");
    let dict = appearance.as_dict()?;
    match document.get_key(dict, "N") {
        Object::Null => None,
        normal => Some(normal),
    }
}

/// Table 224's `/NeedAppearances`, which is what puts a widget that *has* a stream on the
/// constructing path as well.
fn need_appearances_set(document: &Document) -> bool {
    let Some(form) = interactive_form(document) else {
        return false;
    };
    matches!(
        document.get_key(&form, "NeedAppearances"),
        Object::Boolean(true)
    )
}

/// Table 224's interactive form dictionary, where both `/DR` and the document-wide `/DA` live.
fn interactive_form(document: &Document) -> Option<Dictionary> {
    let catalog = document.catalog().ok()?;
    document.get_key(&catalog, "AcroForm").as_dict().cloned()
}

/// Where one widget's baseline comes from under each of the two rules.
fn baseline(document: &Document, widget: &Dictionary) -> Baseline {
    // A `/DA` naming no font, or naming one `/DR` does not define, is laid out in a stand-in,
    // and a stand-in this crate invents states no descriptor.
    let Some((_, font)) = da_font(document, widget) else {
        return Baseline::NoDescriptor;
    };
    let descriptor = document.get_key(&font, "FontDescriptor");
    let Some(descriptor) = descriptor.as_dict() else {
        return Baseline::Silent;
    };
    let entry = |key: &str| {
        document
            .get_key(descriptor, key)
            .as_number()
            .map(narrow)
            .filter(|value| value.is_finite())
    };
    let (Some(ascent), Some(descent)) = (entry("Ascent"), entry("Descent")) else {
        return Baseline::Silent;
    };
    // The rule being replaced, spelled here because it is about to stop existing: it asks that
    // the pair straddle the baseline and nothing else.
    let old = ascent > 0.0 && descent < 0.0;
    match (old, pdf_font::measured_extent(ascent, descent).is_some()) {
        (true, true) => Baseline::Unmoved,
        (true, false) => Baseline::MovesToTheSplit,
        (false, true) => Baseline::MovesToTheStatedPair,
        (false, false) => Baseline::RefusedByBoth,
    }
}

/// Table 228's `/DA`, taken from the nearest ancestor that states one and then from Table 224's
/// document-wide default (§12.7.4.1's inheritance, which `appearance.rs` walks the same way).
fn inherited(document: &Document, widget: &Dictionary, key: &str) -> Option<Vec<u8>> {
    let mut current = widget.clone();
    for _ in 0..MAX_ANCESTRY {
        if let Some(value) = document.get_key(&current, key).as_string() {
            return Some(value.to_vec());
        }
        let parent = document.get_key(&current, "Parent");
        let Some(parent) = parent.as_dict() else {
            break;
        };
        current = parent.clone();
    }
    let form = interactive_form(document)?;
    document.get_key(&form, key).as_string().map(<[u8]>::to_vec)
}

/// Counts one object's `/DA` font if it is composite, and whether it writes vertically.
///
/// **The population `doc/todo/22`'s last edge is about.** §12.7.4.3 lays a value out in the font
/// its `/DA` names, and until the five-hundred-and-second session a Type 0 one was refused by
/// name because a code cannot be produced from a character without inverting §9.7.6.2's
/// codespace ranges. The count says how much of the corpus that reached, then and now.
///
/// The writing mode is asked of `pdf_font` rather than of the `/Encoding` name, because the
/// program's own reading is what decides the refusal: a stream `CMap` states `/WMode` in its
/// file and in its dictionary, and neither is a name ending in `V`.
fn take_composite(document: &Document, widget: &Dictionary, counts: &mut Counts) {
    let Some((name, font)) = da_font(document, widget) else {
        return;
    };
    if document
        .get_key(&font, "Subtype")
        .as_name()
        .map(pdf_syntax::Name::as_bytes)
        != Some(b"Type0")
    {
        return;
    }
    counts.composite = counts.composite.saturating_add(1);
    if pdf_font::LoadedFont::load(document, &font, &name).is_ok_and(|font| font.is_vertical()) {
        counts.composite_vertical = counts.composite_vertical.saturating_add(1);
    }
}

/// The `/DA`'s font name and the dictionary `/DR` gives it, which two questions here both need.
fn da_font(document: &Document, widget: &Dictionary) -> Option<(String, Dictionary)> {
    let appearance = inherited(document, widget, "DA")?;
    let name = font_of(&appearance)?;
    let font = resource_font(document, &name)?;
    Some((name, font))
}

/// The font name a `/DA`'s `Tf` operand names.
///
/// §12.7.4.3 requires the string to "include a Tf (text font) operator along with its two
/// operands"; the name is the first of the two, so the last `/Name size Tf` in the string is what
/// the layout uses. Read here rather than by `variable_text`'s own parser because that one is
/// private, and the answer needed is only which resource the name reaches.
fn font_of(appearance: &[u8]) -> Option<String> {
    let text = String::from_utf8_lossy(appearance);
    let tokens: Vec<&str> = text.split_ascii_whitespace().collect();
    let position = tokens.iter().rposition(|token| *token == "Tf")?;
    let name = tokens.get(position.checked_sub(2)?)?;
    name.strip_prefix('/').map(str::to_owned)
}

/// The font dictionary `/DR`'s `/Font` gives that name (§12.7.4.3's own `shall`).
fn resource_font(document: &Document, name: &str) -> Option<Dictionary> {
    let form = interactive_form(document)?;
    let resources = document.get_key(&form, "DR");
    let resources = resources.as_dict()?;
    let fonts = document.get_key(resources, "Font");
    let fonts = fonts.as_dict()?;
    document.get_key(fonts, name).as_dict().cloned()
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
