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
//! This counts six things, and the third, fifth and sixth are the ones that decide whether reading
//! an entry is worth anything:
//!
//! 1. how many documents state a `/BBox` layout attribute at all, and on which structure types;
//! 2. how many elements mark **no text** and so have no place today;
//! 3. how many of those state a `/BBox` — the population the attribute actually rescues;
//! 4. how many of the rest §14.7.5.3's object reference places instead, out of §12.5.2's
//!    annotation rectangle — which is the only other statement the standard makes about where an
//!    element is;
//! 5. how many of the remainder the page's *marks* place — §14.8.3.3's content rectangle, "derived
//!    from the shape of the enclosed content", which is a **derived** extent rather than a stated
//!    one and is the only route left for an element that marks no text and names no annotation;
//! 6. how many `Form` elements name a widget annotation whose §12.7 field type is readable, and
//!    which of §12.7.5's four types those are. §14.8.4.7.2 makes `Form` "[e]ncloses a PDF widget
//!    annotation and associated content, if any", so the count says how many of them a screen
//!    reader could be told the control of instead of a generic group.
//!
//! Two more, added in the eight-hundred-and-forty-first session, and each prices one of the two
//! questions `doc/todo/31` had left open about §14.8.3.3:
//!
//! 7. of the elements no route places, how many **enclose an element that one does**. §14.8.5.4.5
//!    derives a container's content rectangle from what it contains — "the height of the content
//!    rectangle shall be the sum of the heights of all BLSEs it contains" for a BLSE, and "finding
//!    the extreme top and bottom for all elements" for an ILSE holding a mixture — so a container
//!    whose own sequences marked nothing is not necessarily an element the standard leaves
//!    placeless. This is the population that fifth route would rescue;
//! 8. of the elements whose content is **both** text and other marks, on how many the marks reach
//!    outside the text. That is the question `tree::place` answers by asking the quadrilaterals
//!    first: §14.8.5.4.5 gives an ILSE holding a mixture the extremes of everything it contains, so
//!    where the two disagree the text alone is not the content rectangle.
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

use std::collections::BTreeMap;

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
    /// Of [`Self::placeless`], those whose marked-content sequences drew something.
    ///
    /// The population §14.8.3.3's content rectangle answers for and Table 379's `/BBox` does not:
    /// an element that marks no *text* but whose sequences painted an image, a rule or a vector
    /// drawing has an extent this program can derive from what it drew.
    placeless_by_marks: usize,
    /// Of [`Self::placeless`], those the marks place and **no stated route does**.
    ///
    /// The number that says what deriving the rectangle is worth: an element with no Table 379
    /// `/BBox` and no §12.5.2 annotation behind it had no place at all before this, whatever a
    /// client asked.
    placeless_only_by_marks: usize,
    /// Of [`Self::placeless`], those that neither route places.
    ///
    /// An element whose content items mark nothing at all — a sequence a producer opened and
    /// closed around no operator, or one whose every command was clipped away. The residue after
    /// all four routes, which is what a later round has left to argue about.
    placeless_unanswered: usize,
    /// Which structure types the elements a union places are.
    marked_by_type: BTreeMap<String, usize>,
    /// Which structure types the residue are.
    unanswered_by_type: BTreeMap<String, usize>,
    /// Of [`Self::placeless_unanswered`], those enclosing an element that some route places.
    ///
    /// §14.8.5.4.5 derives a container's content rectangle from the elements it contains, so an
    /// element whose own sequences marked nothing still has one where what it encloses does. This
    /// is the population that route would rescue, and it is a subset of the residue rather than a
    /// fifth thing beside it.
    residue_over_placed: usize,
    /// Which structure types those are.
    residue_over_placed_by_type: BTreeMap<String, usize>,
    /// Elements whose enclosed sequences produced **both** text and marks with an extent.
    ///
    /// The denominator for the two counts below: where an element has only one of the two there is
    /// nothing to disagree about.
    text_and_marks: usize,
    /// Of [`Self::text_and_marks`], those whose marks reach outside the text.
    ///
    /// `tree::place` asks the text quadrilaterals first, so on these elements what a magnifier is
    /// pointed at is smaller than the content rectangle §14.8.3.3 defines.
    marks_outside_text: usize,
    /// Of [`Self::marks_outside_text`], those where the loss is more than a tenth of the area.
    ///
    /// A rule and a rounding error are different findings, and a count that cannot tell them apart
    /// would rank a stroke's mitre with a picture beside a caption.
    marks_outside_text_widely: usize,
    /// Which structure types those are, over the wider set.
    outside_by_type: BTreeMap<String, usize>,
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
        self.placeless_by_marks = self
            .placeless_by_marks
            .saturating_add(other.placeless_by_marks);
        self.placeless_only_by_marks = self
            .placeless_only_by_marks
            .saturating_add(other.placeless_only_by_marks);
        self.placeless_unanswered = self
            .placeless_unanswered
            .saturating_add(other.placeless_unanswered);
        merge(&mut self.marked_by_type, &other.marked_by_type);
        merge(&mut self.unanswered_by_type, &other.unanswered_by_type);
        self.residue_over_placed = self
            .residue_over_placed
            .saturating_add(other.residue_over_placed);
        merge(
            &mut self.residue_over_placed_by_type,
            &other.residue_over_placed_by_type,
        );
        self.text_and_marks = self.text_and_marks.saturating_add(other.text_and_marks);
        self.marks_outside_text = self
            .marks_outside_text
            .saturating_add(other.marks_outside_text);
        self.marks_outside_text_widely = self
            .marks_outside_text_widely
            .saturating_add(other.marks_outside_text_widely);
        merge(&mut self.outside_by_type, &other.outside_by_type);
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
    /// The element enclosing this one, as an index into the walk's own list.
    parent: Option<usize>,
    /// The union of the text quadrilaterals of every sequence below it, where there is one.
    text_extent: Option<[f32; 4]>,
    /// The union of §14.8.3.3's content rectangles of every sequence below it.
    drawn_extent: Option<[f32; 4]>,
    /// Whether the element states a readable Table 379 `/BBox`.
    bounded: bool,
    /// Whether any marked-content sequence below it produced text.
    marks_text: bool,
    /// Whether any marked-content sequence below it *marked the page*, text or not.
    ///
    /// §14.8.3.3's content rectangle is "derived from the shape of the enclosed content", so this
    /// is the question that decides whether a union answers where a text layer cannot.
    marks_drawn: bool,
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

/// One §14.7.5.2 marked-content sequence: the identifier, and the stream it is unique within.
type Sequence = (i64, pdf_model::content::ContentStream);

/// Which sequences produced something, per page object and anywhere in the document.
///
/// The page matters because an identifier is unique within one page's content, not within a
/// document — Table 355's `/Pg` is what says which, and Errata Collection 3's Issue #308 adds a
/// NOTE saying so outright: identifiers are scoped by content stream and start at zero, so the
/// same one may reappear on another page or in a form `XObject`. An element that states no `/Pg`
/// anywhere in its ancestry is matched against every page, which is the same latitude
/// [`pdf_model::structure::Tree::logical_order`] gives it.
fn produced(document: &Document) -> Produced {
    let mut answer = Produced::default();
    let pages = pdf_model::Pages::new(document);
    for (object, index) in pages.indices() {
        let Some(page) = pages.get(index) else {
            continue;
        };
        let interpretation = pdf_model::interpret(document, &page);
        let text = answer.text.entry(object).or_default();
        let drawn = answer.drawn.entry(object).or_default();
        for span in &interpretation.marked {
            // Both halves of §14.7.5.2's key: the identifier and the stream it is unique within.
            let sequence = (span.mcid, span.stream);
            if span.range.start < span.range.end {
                let quads = text_extent(&interpretation, span.range.start, span.range.end);
                text.insert(sequence, quads);
                grow(answer.text_anywhere.entry(sequence).or_default(), quads);
            }
            if let Some(rect) = span.drawn {
                drawn.insert(sequence, Some(rect));
                grow(
                    answer.drawn_anywhere.entry(sequence).or_default(),
                    Some(rect),
                );
            }
        }
    }
    answer
}

/// The box round the glyphs a range of the readback was drawn with.
///
/// [`pdf_model::content::Placed`] carries one quadrilateral per character code, in the same space
/// [`pdf_model::content::MarkedSpan::drawn`] is in, which is what makes the two comparable at all.
/// A code whose readback is empty is included where it sits inside the range, for
/// `viewer_core::select`'s reason: it is ink the sequence drew, and leaving it out would understate
/// where the text is.
fn text_extent(
    interpretation: &pdf_model::Interpretation,
    from: usize,
    to: usize,
) -> Option<[f32; 4]> {
    let mut union: Option<[f32; 4]> = None;
    for placed in &interpretation.text_layer {
        let overlaps = placed.span.start < to && placed.span.end > from;
        let unnameable = placed.span.is_empty() && placed.span.start > from && placed.span.end < to;
        if !(overlaps || unnameable) {
            continue;
        }
        for corner in placed.quad.chunks_exact(2) {
            let (Some(&x), Some(&y)) = (corner.first(), corner.get(1)) else {
                continue;
            };
            grow(&mut union, Some([x, y, x, y]));
        }
    }
    union
}

/// Unions `rect` into `into`, which is the only arithmetic these two extents need.
fn grow(into: &mut Option<[f32; 4]>, rect: Option<[f32; 4]>) {
    let Some(rect) = rect else {
        return;
    };
    *into = Some(match *into {
        None => rect,
        Some(so_far) => [
            so_far[0].min(rect[0]),
            so_far[1].min(rect[1]),
            so_far[2].max(rect[2]),
            so_far[3].max(rect[3]),
        ],
    });
}

/// What one §14.7.5.2 sequence produced: that it produced something, and where it is.
///
/// A type rather than a nested option, so that "this sequence is not in the set" and "this sequence
/// is in it and marked nowhere" are two answers a reader can tell apart.
#[derive(Clone, Copy)]
struct Marks {
    /// The union of the shapes, in the display list's own coordinates.
    extent: Option<[f32; 4]>,
}

/// What one document's pages turned each `/MCID` into.
#[derive(Default)]
struct Produced {
    /// The sequences that read back text, per page, and where their glyphs were drawn.
    text: BTreeMap<ObjectId, BTreeMap<Sequence, Option<[f32; 4]>>>,
    /// The same, for an element whose ancestry names no page at all.
    text_anywhere: BTreeMap<Sequence, Option<[f32; 4]>>,
    /// The sequences that marked the page — §14.8.3.3's content rectangle, per page.
    drawn: BTreeMap<ObjectId, BTreeMap<Sequence, Option<[f32; 4]>>>,
    /// The same, for an element whose ancestry names no page at all.
    drawn_anywhere: BTreeMap<Sequence, Option<[f32; 4]>>,
}

impl Produced {
    /// The entry `mcid` has in one of the two sets, taking the page where the content item named
    /// one.
    ///
    /// The pair is *whether* the sequence is in the set and *where* its marks are, and the two are
    /// asked separately below because a sequence can be in the set with no extent — which happens
    /// to no member as the sets are filled above, and is not a distinction worth a type.
    fn entry(
        per_page: &BTreeMap<ObjectId, BTreeMap<Sequence, Option<[f32; 4]>>>,
        anywhere: &BTreeMap<Sequence, Option<[f32; 4]>>,
        mcid: i64,
        page: Option<ObjectId>,
        stream: Option<ObjectId>,
    ) -> Option<[f32; 4]> {
        Self::found(per_page, anywhere, mcid, page, stream).and_then(|marks| marks.extent)
    }

    /// Whether `mcid` is in the set at all.
    fn holds(
        per_page: &BTreeMap<ObjectId, BTreeMap<Sequence, Option<[f32; 4]>>>,
        anywhere: &BTreeMap<Sequence, Option<[f32; 4]>>,
        mcid: i64,
        page: Option<ObjectId>,
        stream: Option<ObjectId>,
    ) -> bool {
        Self::found(per_page, anywhere, mcid, page, stream).is_some()
    }

    /// The lookup both of the two above are made of.
    fn found(
        per_page: &BTreeMap<ObjectId, BTreeMap<Sequence, Option<[f32; 4]>>>,
        anywhere: &BTreeMap<Sequence, Option<[f32; 4]>>,
        mcid: i64,
        page: Option<ObjectId>,
        stream: Option<ObjectId>,
    ) -> Option<Marks> {
        let names = |set: &BTreeMap<Sequence, Option<[f32; 4]>>| {
            set.iter()
                .find(|((is, within), _)| *is == mcid && within.named_by(stream))
                .map(|(_, extent)| Marks { extent: *extent })
        };
        match page {
            Some(object) => per_page.get(&object).and_then(names),
            None => names(anywhere),
        }
    }
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
    let produced = produced(document);
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
                let parent = stack.last().copied();
                stack.push(elements.len());
                elements.push(Element {
                    role,
                    parent,
                    text_extent: None,
                    drawn_extent: None,
                    bounded: present && readable,
                    marks_text: false,
                    marks_drawn: false,
                    marked_items: 0,
                    object_items: 0,
                    own_objects: Vec::new(),
                });
            }
            Child::MarkedContent {
                mcid, page, stream, ..
            } => {
                let text =
                    Produced::holds(&produced.text, &produced.text_anywhere, mcid, page, stream);
                let drawn = Produced::holds(
                    &produced.drawn,
                    &produced.drawn_anywhere,
                    mcid,
                    page,
                    stream,
                );
                let (text_at, drawn_at) = (
                    Produced::entry(&produced.text, &produced.text_anywhere, mcid, page, stream),
                    Produced::entry(
                        &produced.drawn,
                        &produced.drawn_anywhere,
                        mcid,
                        page,
                        stream,
                    ),
                );
                for index in &stack {
                    if let Some(element) = elements.get_mut(*index) {
                        element.marked_items = element.marked_items.saturating_add(1);
                        element.marks_text |= text;
                        element.marks_drawn |= drawn;
                        grow(&mut element.text_extent, text_at);
                        grow(&mut element.drawn_extent, drawn_at);
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

/// Whether `inner` sits inside `outer`, allowing a unit of slack on each edge.
///
/// The slack is there because the two rectangles are measured differently and not because a
/// tolerance is wanted: a glyph's quadrilateral is its advance box between the font's ascent and
/// descent (§9.4.4), and a command's own bound counts a curve by its control polygon, so a
/// well-behaved line of text can put a fraction of a unit either side of its own text layer. What
/// this question is about is a mark somewhere the text is not.
fn inside(inner: [f32; 4], outer: [f32; 4]) -> bool {
    const SLACK: f32 = 1.0;
    inner[0] >= outer[0] - SLACK
        && inner[1] >= outer[1] - SLACK
        && inner[2] <= outer[2] + SLACK
        && inner[3] <= outer[3] + SLACK
}

/// The area of a rectangle, for ranking one disagreement against another.
fn area(rect: [f32; 4]) -> f32 {
    (rect[2] - rect[0]).max(0.0) * (rect[3] - rect[1]).max(0.0)
}

/// Turns the walked elements into the counts the header comment names.
fn tally(elements: &[Element], referenced: &Referenced, counts: &mut Counts) {
    // Whether some route places this element itself, which is what a container's own rectangle is
    // derived from one level up (§14.8.5.4.5).
    let places = |element: &Element| {
        element.marks_text
            || element.marks_drawn
            || element.bounded
            || element
                .own_objects
                .iter()
                .any(|object| referenced.places.contains_key(object))
    };
    let mut over_placed = vec![false; elements.len()];
    for element in elements {
        if !places(element) {
            continue;
        }
        let mut at = element.parent;
        while let Some(index) = at {
            match over_placed.get_mut(index) {
                // An ancestor already marked has marked ancestors of its own.
                Some(true) | None => break,
                Some(slot) => *slot = true,
            }
            at = elements.get(index).and_then(|above| above.parent);
        }
    }

    for (index, element) in elements.iter().enumerate() {
        if let (Some(text), Some(drawn)) = (element.text_extent, element.drawn_extent) {
            counts.text_and_marks = counts.text_and_marks.saturating_add(1);
            if !inside(drawn, text) {
                counts.marks_outside_text = counts.marks_outside_text.saturating_add(1);
                bump(&mut counts.outside_by_type, &element.role);
                let mut both = Some(text);
                grow(&mut both, Some(drawn));
                if both.is_some_and(|union| area(union) > area(text) * 1.1) {
                    counts.marks_outside_text_widely =
                        counts.marks_outside_text_widely.saturating_add(1);
                }
            }
        }
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
        if element.marks_drawn {
            counts.placeless_by_marks = counts.placeless_by_marks.saturating_add(1);
            bump(&mut counts.marked_by_type, &element.role);
        }
        // Taken after the `/BBox` one so that the two routes do not both claim the same element:
        // Table 379's rectangle is the element's own statement and wins where it exists.
        let by_annotation = !element.bounded
            && element
                .own_objects
                .iter()
                .any(|object| referenced.places.contains_key(object));
        if by_annotation {
            counts.placeless_by_annotation = counts.placeless_by_annotation.saturating_add(1);
        }
        if !element.bounded && !by_annotation && element.marks_drawn {
            counts.placeless_only_by_marks = counts.placeless_only_by_marks.saturating_add(1);
        }
        if !element.bounded && !by_annotation && !element.marks_drawn {
            counts.placeless_unanswered = counts.placeless_unanswered.saturating_add(1);
            bump(&mut counts.unanswered_by_type, &element.role);
            if over_placed.get(index).copied().unwrap_or(false) {
                counts.residue_over_placed = counts.residue_over_placed.saturating_add(1);
                bump(&mut counts.residue_over_placed_by_type, &element.role);
            }
        }
    }
}

/// §14.8.5.4.5's other two derivations, which `viewer_core::places` reads and this prices.
fn the_two_derivations_a_container_and_a_mixture_need(total: &Counts) {
    println!(
        "{} of the elements no route places enclose an element that some route places, which is \
         what §14.8.5.4.5 derives a container's own rectangle from",
        total.residue_over_placed
    );
    for (role, count) in &total.residue_over_placed_by_type {
        println!("  {role}: {count}");
    }
    println!(
        "{} elements enclose both text and other marks; on {} of them the marks reach outside the \
         text, {} of those by more than a tenth of the area",
        total.text_and_marks, total.marks_outside_text, total.marks_outside_text_widely
    );
    for (role, count) in &total.outside_by_type {
        println!("  {role}: {count}");
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
        if counts.marks_outside_text > 0 || counts.residue_over_placed > 0 {
            println!(
                "{path}: {} element(s) whose marks reach outside their text ({} widely), \
                 {} placeless element(s) enclosing a placed one",
                counts.marks_outside_text,
                counts.marks_outside_text_widely,
                counts.residue_over_placed,
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
        "{} of the placeless elements have marked-content sequences that drew something, so \
         §14.8.3.3's content rectangle places them; {} of those had no stated route at all and \
         are the ones deriving it rescues; {} are placed by no route at all",
        total.placeless_by_marks, total.placeless_only_by_marks, total.placeless_unanswered
    );
    println!("the placeless elements a union of their marks places, by role:");
    for (role, count) in &total.marked_by_type {
        println!("  {role}: {count}");
    }
    println!("the placeless elements no route places, by role:");
    for (role, count) in &total.unanswered_by_type {
        println!("  {role}: {count}");
    }
    the_two_derivations_a_container_and_a_mixture_need(&total);
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
