//! Table 379's `/BBox`, and the structure elements that reach an assistive technology with no
//! place at all.
//!
//! `viewer_core::AccessibilityNode::quads` is built from the text layer, so an element whose
//! content drew no *text* — a `Figure`, a table cell holding an image — crosses the boundary with
//! an empty list of shapes and a magnifier has nothing to point at. §14.8.5.4.3's Table 379 states
//! the rectangle that would fill it: "the coordinates of the left, bottom, right, and top edges,
//! respectively, of the structure element's bounding box (the rectangle that completely encloses
//! its visible content)", in default user space.
//!
//! This counts five things, and the third and fifth are the ones that decide whether reading an
//! entry is worth anything:
//!
//! 1. how many documents state a `/BBox` layout attribute at all, and on which structure types;
//! 2. how many elements mark **no text** and so have no place today;
//! 3. how many of those state a `/BBox` — the population the attribute actually rescues;
//! 4. how many of the rest §14.7.5.3's object reference places instead, out of §12.5.2's
//!    annotation rectangle — which is the only other statement the standard makes about where an
//!    element is;
//! 5. how many `Form` elements name a widget annotation whose §12.7 field type is readable, and
//!    which of §12.7.5's four types those are. §14.8.4.7.2 makes `Form` "[e]ncloses a PDF widget
//!    annotation and associated content, if any", so the count says how many of them a screen
//!    reader could be told the control of instead of a generic group.
//!
//! The second and third need the page's readback, so **every tagged document is interpreted** and
//! not only the ones stating a `/BBox`: taking the denominator from a subset of the documents the
//! numerator comes from would put two different populations in one ratio.
//!
//! ```sh
//! cargo run --release -p pdf-model --example element_bounds_census -- doc/pdf.js/test/pdfs/*.pdf
//! ```

#![expect(
    clippy::print_stdout,
    reason = "an example whose entire output is a measurement"
)]

use std::collections::{BTreeMap, BTreeSet};

use pdf_model::structure::{Child, Tree};
use pdf_syntax::{Document, ObjectId};

/// What one document contributes.
#[derive(Default)]
struct Counts {
    /// Documents with a structure tree at all.
    tagged: usize,
    /// Documents stating at least one Table 379 `/BBox`.
    with_bounds: usize,
    /// Structure elements walked.
    elements: usize,
    /// Elements stating a `/BBox` this reader could read as a rectangle.
    bounded: usize,
    /// Elements stating a `/BBox` entry that is not four numbers.
    malformed: usize,
    /// Elements whose content items produced no text on any page.
    placeless: usize,
    /// Of [`Self::placeless`], those a `/BBox` would give a place.
    placeless_bounded: usize,
    /// Of [`Self::placeless`], those whose only content is §14.7.5.3's object reference.
    placeless_object: usize,
    /// Of [`Self::placeless`], those with **no** `/BBox` that §12.5.2's annotation rectangle
    /// places.
    ///
    /// The population the object-reference route rescues, taken after the `/BBox` one so that the
    /// two do not both claim the same element.
    placeless_by_annotation: usize,
    /// `Form` elements walked, whatever they name.
    form: usize,
    /// Of [`Self::form`], those whose own object reference names a widget of a field on the page.
    form_with_control: usize,
    /// Which of §12.7.5's four types those widgets' fields are.
    by_control: BTreeMap<String, usize>,
    /// Elements with no content items anywhere below them, which are containers.
    childless: usize,
    /// Elements whose `/BBox` reaches outside the crop box of the page they name.
    ///
    /// §14.11.2.1 makes the crop box "the region to which the contents of the page shall be
    /// clipped (cropped) when displayed or printed", so a rectangle beyond it encloses nothing
    /// anybody can see. The count is what says whether that is a curiosity or a population.
    overreaching: usize,
    /// Which structure types the `/BBox` elements are, after §14.7.3's role map.
    by_type: BTreeMap<String, usize>,
    /// Which structure types the placeless elements are, after §14.7.3's role map.
    placeless_by_type: BTreeMap<String, usize>,
    /// Which structure types the placeless elements stating a `/BBox` are.
    rescued_by_type: BTreeMap<String, usize>,
}

impl Counts {
    /// Adds `other`'s totals to this one's.
    fn absorb(&mut self, other: &Self) {
        self.tagged = self.tagged.saturating_add(other.tagged);
        self.with_bounds = self.with_bounds.saturating_add(other.with_bounds);
        self.elements = self.elements.saturating_add(other.elements);
        self.bounded = self.bounded.saturating_add(other.bounded);
        self.malformed = self.malformed.saturating_add(other.malformed);
        self.placeless = self.placeless.saturating_add(other.placeless);
        self.placeless_bounded = self
            .placeless_bounded
            .saturating_add(other.placeless_bounded);
        self.placeless_object = self.placeless_object.saturating_add(other.placeless_object);
        self.placeless_by_annotation = self
            .placeless_by_annotation
            .saturating_add(other.placeless_by_annotation);
        self.form = self.form.saturating_add(other.form);
        self.form_with_control = self
            .form_with_control
            .saturating_add(other.form_with_control);
        self.childless = self.childless.saturating_add(other.childless);
        self.overreaching = self.overreaching.saturating_add(other.overreaching);
        merge(&mut self.by_control, &other.by_control);
        merge(&mut self.by_type, &other.by_type);
        merge(&mut self.placeless_by_type, &other.placeless_by_type);
        merge(&mut self.rescued_by_type, &other.rescued_by_type);
    }
}

/// Adds one tally of roles into another.
fn merge(into: &mut BTreeMap<String, usize>, from: &BTreeMap<String, usize>) {
    for (name, count) in from {
        let slot = into.entry(name.clone()).or_default();
        *slot = slot.saturating_add(*count);
    }
}

/// Counts one more of `name`.
fn bump(tally: &mut BTreeMap<String, usize>, name: &str) {
    let slot = tally.entry(name.to_owned()).or_default();
    *slot = slot.saturating_add(1);
}

/// One element of the walk, while its descendants are still being read.
struct Element {
    /// The role, after §14.7.3's map, or the empty name for an element stating no `/S`.
    role: String,
    /// Whether the element states a readable Table 379 `/BBox`.
    bounded: bool,
    /// Whether any marked-content sequence below it produced text.
    marks_text: bool,
    /// How many §14.7.5.2 marked-content sequences are below it.
    marked_items: usize,
    /// How many §14.7.5.3 object references are below it.
    object_items: usize,
    /// The objects the element's **own** object references name.
    ///
    /// Its own rather than its descendants', because §12.5.2's rectangle places the annotation
    /// that *is* the element's content item — an ancestor's extent is a different question and
    /// the standard states no union.
    own_objects: Vec<ObjectId>,
}

/// What §14.7.5.3's object references can be looked up in, for one document.
///
/// Both maps are over every page, because a `Child::Object` states a page only where its own
/// reference or its element carries `/Pg` — and this is counting a population rather than
/// answering for one page, which is what `viewer_core::accessibility` does with the page in hand.
struct Referenced {
    /// §12.5.2's `/Rect` for every annotation any page lists, in default user space.
    places: BTreeMap<ObjectId, [f32; 4]>,
    /// The §12.7.5 control of the field each widget annotation belongs to.
    controls: BTreeMap<ObjectId, String>,
}

/// Reads both maps, one walk of the page tree.
fn referenced(document: &Document) -> Referenced {
    let mut places = BTreeMap::new();
    let mut controls = BTreeMap::new();
    let pages = pdf_model::Pages::new(document);
    let view = pdf_model::view::ViewState::of(document);
    for index in 0..pages.len() {
        let Some(page) = pages.get(index) else {
            continue;
        };
        places.extend(pdf_model::structure::annotation_rectangles(
            document, &page.dict,
        ));
        for field in pdf_model::form::fields(document, &page, &view) {
            for widget in &field.widgets {
                controls.insert(widget.annotation, control_name(&field.control).to_owned());
            }
        }
    }
    Referenced { places, controls }
}

/// Which of §12.7.5's types a control is, as one word.
///
/// §12.7.5.2's two toggling kinds carry their state, because that is what crosses beside the role
/// and a corpus with no ticked box would leave the state untested on a real bus.
fn control_name(control: &pdf_model::form::Control) -> &'static str {
    match control {
        pdf_model::form::Control::PushButton => "PushButton",
        pdf_model::form::Control::CheckBox { on: true } => "CheckBox (on)",
        pdf_model::form::Control::RadioButton { on: true, .. } => "RadioButton (on)",
        pdf_model::form::Control::CheckBox { .. } => "CheckBox",
        pdf_model::form::Control::RadioButton { .. } => "RadioButton",
        pdf_model::form::Control::Text(_) => "Text",
        pdf_model::form::Control::Choice(_) => "Choice",
        pdf_model::form::Control::Signature => "Signature",
        pdf_model::form::Control::Unstated => "Unstated",
    }
}

/// Every `/MCID` that produced a non-empty span of the readback, per page object.
///
/// The page matters because an identifier is unique within one page's content, not within a
/// document — Table 355's `/Pg` is what says which. An element that states no `/Pg` anywhere in
/// its ancestry is matched against every page, which is the same latitude
/// [`pdf_model::structure::Tree::logical_order`] gives it.
fn text_producing(document: &Document) -> (BTreeMap<ObjectId, BTreeSet<i64>>, BTreeSet<i64>) {
    let mut per_page: BTreeMap<ObjectId, BTreeSet<i64>> = BTreeMap::new();
    let mut anywhere: BTreeSet<i64> = BTreeSet::new();
    let pages = pdf_model::Pages::new(document);
    for (object, index) in pages.indices() {
        let Some(page) = pages.get(index) else {
            continue;
        };
        let interpretation = pdf_model::interpret(document, &page);
        let entry = per_page.entry(object).or_default();
        for span in &interpretation.marked {
            if span.range.start < span.range.end {
                entry.insert(span.mcid);
                anywhere.insert(span.mcid);
            }
        }
    }
    (per_page, anywhere)
}

/// Whether the `/BBox` attribute is present, and whether it is four numbers.
fn bounds(document: &Document, tree: &Tree, element: &pdf_syntax::Dictionary) -> (bool, bool) {
    let Some(value) = tree.attribute(document, element, "BBox") else {
        return (false, false);
    };
    let readable = value.as_array().is_some_and(|items| {
        items.len() >= 4
            && items.iter().take(4).all(|item| {
                document
                    .resolve(item)
                    .as_number()
                    .is_some_and(f64::is_finite)
            })
    });
    (true, readable)
}

/// Whether the element's `/BBox` reaches outside the crop box of the page it names.
///
/// Table 355's `/Pg` is which page, and §14.11.2.1's crop box is what can be displayed of it. An
/// element naming no page is not counted either way: the question has no page to be asked about.
fn reaches_off_the_page(
    document: &Document,
    tree: &Tree,
    element: &pdf_syntax::Dictionary,
) -> bool {
    let Some(bbox) = tree.bounds(document, element) else {
        return false;
    };
    // `get_key` resolves, and a resolved page dictionary is not a reference — Table 355's `/Pg`
    // has to be read raw to learn *which* page it names.
    let Some(object) = element.get("Pg").and_then(pdf_syntax::Object::as_reference) else {
        return false;
    };
    let pages = pdf_model::Pages::new(document);
    let Some(page) = pages.index_of(object).and_then(|index| pages.get(index)) else {
        return false;
    };
    let crop = page.boundary(pdf_model::page::Boundary::Crop);
    bbox[0] < crop[0] || bbox[1] < crop[1] || bbox[2] > crop[2] || bbox[3] > crop[3]
}

/// Walks one document's structure tree, counting what the header comment names.
fn census(document: &Document) -> Counts {
    let mut counts = Counts::default();
    let Some(tree) = Tree::of(document) else {
        return counts;
    };
    counts.tagged = 1;

    let walk = tree.walk(document);
    // Every tagged document is interpreted, not only the ones stating a `/BBox`: the count this
    // is for is *placeless against bounded*, and taking the denominator from a subset of the
    // documents the numerator comes from would be a ratio of two different populations.
    let (per_page, anywhere) = text_producing(document);
    counts.with_bounds = usize::from(walk.items.iter().any(
        |(_, child)| matches!(child, Child::Element(dict) if bounds(document, &tree, dict).0),
    ));

    let mut elements: Vec<Element> = Vec::new();
    let mut stack: Vec<usize> = Vec::new();
    for (depth, child) in walk.items {
        stack.truncate(depth);
        match child {
            Child::Element(dict) => {
                let (present, readable) = bounds(document, &tree, &dict);
                if present && !readable {
                    counts.malformed = counts.malformed.saturating_add(1);
                }
                let role = tree.role(document, &dict).unwrap_or_default();
                if present && readable {
                    counts.bounded = counts.bounded.saturating_add(1);
                    bump(&mut counts.by_type, &role);
                    if reaches_off_the_page(document, &tree, &dict) {
                        counts.overreaching = counts.overreaching.saturating_add(1);
                    }
                }
                if role == "Form" {
                    counts.form = counts.form.saturating_add(1);
                }
                stack.push(elements.len());
                elements.push(Element {
                    role,
                    bounded: present && readable,
                    marks_text: false,
                    marked_items: 0,
                    object_items: 0,
                    own_objects: Vec::new(),
                });
            }
            Child::MarkedContent { mcid, page } => {
                let produced = match page {
                    Some(object) => per_page.get(&object).is_some_and(|set| set.contains(&mcid)),
                    None => anywhere.contains(&mcid),
                };
                for index in &stack {
                    if let Some(element) = elements.get_mut(*index) {
                        element.marked_items = element.marked_items.saturating_add(1);
                        element.marks_text |= produced;
                    }
                }
            }
            Child::Object { object, .. } => {
                for index in &stack {
                    if let Some(element) = elements.get_mut(*index) {
                        element.object_items = element.object_items.saturating_add(1);
                    }
                }
                if let Some(element) = stack.last().and_then(|index| elements.get_mut(*index)) {
                    element.own_objects.push(object);
                }
            }
        }
    }

    counts.elements = elements.len();
    tally(&elements, &referenced(document), &mut counts);
    counts
}

/// Turns the walked elements into the counts the header comment names.
fn tally(elements: &[Element], referenced: &Referenced, counts: &mut Counts) {
    for element in elements {
        if element.role == "Form"
            && let Some(name) = element
                .own_objects
                .iter()
                .find_map(|object| referenced.controls.get(object))
        {
            counts.form_with_control = counts.form_with_control.saturating_add(1);
            bump(&mut counts.by_control, name);
        }
        if element.marked_items == 0 && element.object_items == 0 {
            counts.childless = counts.childless.saturating_add(1);
            continue;
        }
        if element.marks_text {
            continue;
        }
        counts.placeless = counts.placeless.saturating_add(1);
        bump(&mut counts.placeless_by_type, &element.role);
        if element.bounded {
            counts.placeless_bounded = counts.placeless_bounded.saturating_add(1);
            bump(&mut counts.rescued_by_type, &element.role);
        }
        if element.object_items > 0 {
            counts.placeless_object = counts.placeless_object.saturating_add(1);
        }
        // Taken after the `/BBox` one so that the two routes do not both claim the same element:
        // Table 379's rectangle is the element's own statement and wins where it exists.
        if !element.bounded
            && element
                .own_objects
                .iter()
                .any(|object| referenced.places.contains_key(object))
        {
            counts.placeless_by_annotation = counts.placeless_by_annotation.saturating_add(1);
        }
    }
}

fn main() {
    let mut total = Counts::default();
    let mut documents = 0_usize;
    for path in std::env::args().skip(1) {
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        documents = documents.saturating_add(1);
        let counts = census(&document);
        if counts.bounded > 0 || counts.malformed > 0 || counts.form > 0 {
            println!(
                "{path}: {} element(s) with Table 379 /BBox, {} malformed, \
                 {} placeless of which {} bounded and {} placed by an annotation; \
                 {} Form element(s)",
                counts.bounded,
                counts.malformed,
                counts.placeless,
                counts.placeless_bounded,
                counts.placeless_by_annotation,
                counts.form,
            );
        }
        total.absorb(&counts);
    }
    println!(
        "\n{documents} documents, {} with a structure tree, {} elements",
        total.tagged, total.elements
    );
    println!(
        "{} documents state a Table 379 /BBox; {} elements do, {} state one that is not a rectangle",
        total.with_bounds, total.bounded, total.malformed
    );
    println!(
        "{} elements have content items that produced no text — {} of them state a /BBox, \
         {} name an object reference",
        total.placeless, total.placeless_bounded, total.placeless_object
    );
    println!(
        "{} of the placeless elements state no /BBox and are placed by §12.5.2's annotation \
         rectangle through §14.7.5.3's object reference",
        total.placeless_by_annotation
    );
    println!(
        "{} Form elements, {} of which name a widget whose field type is readable",
        total.form, total.form_with_control
    );
    for (control, count) in &total.by_control {
        println!("  Form naming a {control} field: {count}");
    }
    println!(
        "{} elements have no content items at all, which are containers",
        total.childless
    );
    println!(
        "{} of the /BBox rectangles reach outside their page's crop box",
        total.overreaching
    );
    for (role, count) in &total.by_type {
        println!("  /BBox on {role}: {count}");
    }
    println!("the placeless elements a /BBox would place, by role:");
    for (role, count) in &total.rescued_by_type {
        println!("  {role}: {count}");
    }
    println!("the placeless elements, by role:");
    for (role, count) in &total.placeless_by_type {
        println!("  {role}: {count}");
    }
}
