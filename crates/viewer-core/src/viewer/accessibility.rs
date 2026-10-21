//! The structure an assistive technology is told about, §14.7's elements and the controls no
//! element names.

use std::collections::{BTreeMap, BTreeSet};

use pdf_syntax::ObjectId;

use super::Viewer;
use super::geometry::as_displayed;
#[cfg(doc)]
use crate::Answer;
#[cfg(doc)]
use crate::Query;
use crate::open::Open;
use crate::query::PageStructure;

impl Viewer {
    /// §14.7's structure tree for every page the arrangement shows, with §14.9's entries applied.
    ///
    /// **One tree per page rather than one for the screen**, which is [`Query::AccessibilityTree`]'s
    /// own note: §14.7's structure is the document's, the route to a page's elements is
    /// §14.7.5.4's structural parent tree keyed by *that page's* `/StructParents`, and two pages'
    /// answers share their ancestors — so they are answered side by side and joining them is the
    /// platform's question rather than this crate's.
    ///
    /// Each page also says which of [`crate::Tagging`]'s four answers its list is (ADR 1393): the
    /// document's `/StructTreeRoot` and `/MarkInfo` are read once for the screen, because §14.7.1
    /// and §14.7.2 put both at the catalog and they are the same for every page on it.
    pub(super) fn accessibility(&self, open: &Open) -> Vec<PageStructure> {
        let structured = pdf_model::structure::Tree::of(&open.document).is_some();
        let marked = structured && pdf_model::structure::MarkInfo::read(&open.document).marked;
        open.on_screen
            .iter()
            .map(|on_screen| {
                let (nodes, widgets, tagging) = match self.structure(open, on_screen) {
                    Some((nodes, named)) => {
                        let widgets = self.unreached_widgets(open, on_screen, &named);
                        let tagging = if !structured {
                            crate::Tagging::Untagged
                        } else if nodes.is_empty() {
                            crate::Tagging::Unreached { marked }
                        } else {
                            crate::Tagging::Reached
                        };
                        (nodes, widgets, tagging)
                    }
                    // A page not yet interpreted has no elements to answer, and on a tagged page
                    // which widgets they reach is unknown until it has; an untagged page's
                    // widgets are reached by nothing whatever the interpreter says.
                    None if !structured => (
                        Vec::new(),
                        self.unreached_widgets(open, on_screen, &BTreeSet::new()),
                        crate::Tagging::Untagged,
                    ),
                    None => (Vec::new(), Vec::new(), crate::Tagging::Unread),
                };
                PageStructure {
                    page: on_screen.page,
                    nodes,
                    widgets,
                    tagging,
                }
            })
            .collect()
    }

    /// The page's widget annotations that no published element names, as
    /// [`PageStructure::widgets`] states them: every one on an untagged page, and on a tagged page
    /// each one no element's §14.7.5.3 object reference reached.
    ///
    /// **What a person must be able to reach is decided by what the page lets them do, not by
    /// whether the producer tagged it.** §12.5.1 makes an annotation something "the user activates
    /// … by clicking it", and ADR 0425 made that one definition for the mouse and for an assistive
    /// technology; a field on an untagged page is exactly as interactive as one on a tagged page.
    /// What tagging adds is a *reading order*, and nothing here is one: the order is §12.5.1's
    /// tab order, which the page states for its annotations (Table 31's `/Tabs`) or which its
    /// default derives, and the name is Table 226's `/TU` — "\[a]n alternative field name that
    /// shall be used in place of the actual field name wherever the field shall be identified in
    /// the user interface" — or the §12.7.4.2 name where there is none. ADR 1369.
    ///
    /// **A tagged page's structure may leave a widget out, and the widget is still there.** Table
    /// 368's "\[i]n a tagged PDF, Form shall be used for each PDF widget annotation that belongs to
    /// the real content of the document" binds the producer; where one did not, the field is still
    /// §12.5.1's interactive content, and nothing in §14.7 lets a reader withhold what the page
    /// lets a person do. `named` is every object the page's published elements name through
    /// §14.7.5.3's `/OBJR` or Table 357's `/StmOwn`, so a widget is offered here exactly when no
    /// node already offers it — never twice, and never given a place in the structure's reading
    /// order it does not state. ADR 1381.
    ///
    /// §12.5.3's `Hidden`, `NoView` and `ReadOnly` decide whether a widget is offered at all, as they
    /// decide whether a click reaches it.
    fn unreached_widgets(
        &self,
        open: &Open,
        on_screen: &crate::open::OnScreen,
        named: &BTreeSet<ObjectId>,
    ) -> Vec<crate::AccessibilityNode> {
        let Some(page_id) = on_screen.object.id else {
            return Vec::new();
        };
        let mut fields = pdf_model::form::fields(&open.document, &on_screen.object, &open.view);
        if fields.is_empty() {
            return Vec::new();
        }
        as_displayed(open, &mut fields);
        let languages =
            pdf_model::structure::annotation_languages(&open.document, &on_screen.object.dict);
        let order = pdf_model::tab_order::order(&open.document, &on_screen.object, page_id);
        order
            .iter()
            .filter(|annotation| {
                !named.contains(*annotation)
                    && pdf_model::view::annotation_interacts(
                        &open.document,
                        **annotation,
                        &open.view,
                    )
            })
            .filter_map(|annotation| {
                let (field, widget) = fields.iter().find_map(|field| {
                    field
                        .widgets
                        .iter()
                        .find(|widget| widget.annotation == *annotation)
                        .map(|widget| (field, widget))
                })?;
                // Where §12.7.4.3's layout placed each character, for the platform's text
                // interface (ADR 1501).
                let value_lines = field
                    .value
                    .as_ref()
                    .and_then(|value| {
                        let glyphs = open.view.field_glyphs(&open.document, *annotation)?;
                        Some(crate::accessibility::value_lines(
                            &value.text,
                            &glyphs,
                            |rect| self.device_rect(open, on_screen.page, rect),
                        ))
                    })
                    .unwrap_or_default();
                Some(crate::AccessibilityNode {
                    role: "Form".to_owned(),
                    // §14.8.4.7.2's widget type, which is what this node is whatever namespace a
                    // tagged page's elements are in: no element stated it (ADR 1369).
                    standard: Ok(pdf_model::structure::StandardType::Form),
                    name: field.name.shown().to_owned(),
                    language: languages.get(annotation).cloned(),
                    bounds: self.device_rect(open, on_screen.page, widget.rect),
                    control: Some(this_widgets_control(&field.control, widget)),
                    value: field.value.clone(),
                    value_lines,
                    annotation: Some(*annotation),
                    ..crate::AccessibilityNode::blank()
                })
            })
            .collect()
    }

    /// One page's elements, as [`crate::AccessibilityNode`]s indexed within that page.
    ///
    /// Built on demand rather than kept, because a screen reader asks when it attaches and on a
    /// page change, and no other consumer asks at all — while a drag asks
    /// [`Query::Selection`] sixty times a second, which is why *that* one's inputs are cached.
    ///
    /// **Everything about the page comes from the arrangement's own entry**, and that is the
    /// answer to what a column costs: [`crate::open::OnScreen::object`] is the page ADR 0124
    /// cached to keep `Pages::get`'s tree walk — 3.8 ms on ISO 32000-2's thousandth page — off
    /// the paths a person drives. Asking the page tree once per page on the screen would have put
    /// several of those walks behind one question.
    ///
    /// Answered beside the elements: every object their own content items name, which is what
    /// [`Self::unreached_widgets`] reads to offer a widget no element reached. `None` while the page
    /// is not yet interpreted, when neither half can be known.
    fn structure(
        &self,
        open: &Open,
        on_screen: &crate::open::OnScreen,
    ) -> Option<(Vec<crate::AccessibilityNode>, BTreeSet<ObjectId>)> {
        let interpreted = on_screen.interpreted.as_ref()?;
        // Table 355's `/Pg` names a page *object*, and what this crate holds is an index.
        let Some(page) = on_screen.object.id else {
            return Some((Vec::new(), BTreeSet::new()));
        };
        let index = on_screen.page;
        let gathered =
            crate::accessibility::nodes(&open.document, page, interpreted.language.as_deref());
        let named: BTreeSet<ObjectId> = gathered
            .iter()
            .flat_map(|(_, element)| element.objects.iter().copied())
            .collect();
        // §14.7.5.3's object references, answered once for the page. Both readings are skipped
        // entirely where no element states one, which is nearly every page: `annotation_rectangles`
        // walks `/Annots` and `form::fields` walks §12.7.4.1's field tree, and neither is worth
        // paying for an answer nothing will look at.
        let referenced = gathered
            .iter()
            .any(|(_, element)| !element.objects.is_empty());
        let referenced = if referenced {
            referenced_objects(open, &on_screen.object)
        } else {
            Referenced::default()
        };
        let page = crate::accessibility::Readback {
            text: &interpreted.text,
            marked: &interpreted.marked,
            described: &interpreted.described,
            places: &referenced.places,
            languages: &referenced.languages,
            controls: &referenced.controls,
            fields: &referenced.fields,
            values: &referenced.values,
            glyphs: &referenced.glyphs,
        };
        let nodes = gathered
            .into_iter()
            .map(|(parent, gathered)| {
                crate::accessibility::finish(
                    gathered,
                    parent,
                    &page,
                    |start, end| self.device_quads(open, index, (start, end)),
                    |ranges| self.device_lines(open, index, ranges),
                    |rect| self.device_rect(open, index, rect),
                    |rect| self.device_marks(open, index, rect),
                )
            })
            .collect();
        Some((nodes, named))
    }
}

/// What §14.7.5.3's object references can name on this page: §12.5.2's rectangles and `/Lang`,
/// and §12.7's controls.
///
/// Both are keyed by the annotation, which is what an object reference names, and both are the
/// same readings the rest of this crate uses — `form::fields` with **this view's** state, so a
/// check box a person has just ticked answers `on` in the accessibility tree exactly as it does
/// in [`Answer::Fields`].
///
/// **Keyed by the annotation means answering for the annotation**, which is what
/// [`this_widgets_control`] is for: storing the *field's* control under each of a radio set's
/// widgets would make every button of the set say it was selected.
///
/// Takes the page the arrangement is already holding rather than an index into the page tree:
/// under a column this is asked once per page on the screen, and `Pages::get` is the walk ADR
/// 0124 cached `OnScreen::object` to avoid.
fn referenced_objects(open: &Open, shown: &pdf_model::Page) -> Referenced {
    let places = pdf_model::structure::annotation_rectangles(&open.document, &shown.dict);
    let languages = pdf_model::structure::annotation_languages(&open.document, &shown.dict);
    let mut controls = BTreeMap::new();
    let mut fields = BTreeMap::new();
    let mut values = BTreeMap::new();
    let mut glyphs = BTreeMap::new();
    let mut on_page = pdf_model::form::fields(&open.document, shown, &open.view);
    as_displayed(open, &mut on_page);
    for field in on_page {
        for widget in &field.widgets {
            if let Some(value) = &field.value {
                values.insert(widget.annotation, value.clone());
                if let Some(placed) = open.view.field_glyphs(&open.document, widget.annotation) {
                    glyphs.insert(widget.annotation, placed);
                }
            }
            controls.insert(
                widget.annotation,
                this_widgets_control(&field.control, widget),
            );
            fields.insert(widget.annotation, field.name.shown().to_owned());
        }
    }
    Referenced {
        places,
        languages,
        controls,
        fields,
        values,
        glyphs,
    }
}

/// The six readings [`referenced_objects`] answers with, keyed by the annotation each is about.
///
/// A value rather than a tuple because they are five different facts about one page and a caller
/// reading `.1` would have to remember which; `Default` is the answer for a page whose structure
/// tree names no object at all, which is nearly every page.
#[derive(Default)]
struct Referenced {
    /// §12.5.2's `/Rect` for each annotation the page lists, in default user space.
    places: BTreeMap<ObjectId, [f32; 4]>,
    /// Table 166's `/Lang` for each annotation that states one.
    languages: BTreeMap<ObjectId, String>,
    /// §12.7's control for each widget annotation of a field with a widget on this page.
    controls: BTreeMap<ObjectId, pdf_model::form::Control>,
    /// The name §14.9.3 says a user interface shows for each such widget's field (ADR 1394).
    fields: BTreeMap<ObjectId, String>,
    /// §12.7.4.3's text for each such widget whose field is a text field or a combo box (ADR 1489).
    values: BTreeMap<ObjectId, pdf_model::view::ShownValue>,
    /// Where §12.7.4.3's layout placed each glyph of those values, in default user space
    /// (ADR 1501).
    glyphs: BTreeMap<ObjectId, Vec<pdf_model::view::FieldGlyph>>,
}

/// The field's control with §12.7.5.2's on state replaced by **this widget's**.
///
/// **§14.7.5.3's object reference names one widget annotation**, so a node built from it is a
/// button rather than a field — and `pdf_model::form::Control::RadioButton`'s `on` is "\[w]hether
/// any widget of the set is on", which is the field's answer and the wrong one here.
/// `pdf_model::form::Widget::on` is the right one and sits beside it, saying so in its own doc
/// comment: "[`Control::CheckBox`] carries the same fact for the field as a whole; this is the
/// per-widget answer a radio set needs."
///
/// Without it a clause is broken, and it was measured on a real AT-SPI bus (ADR 0630): a screen
/// reader is told **every** button of a set is selected as soon as one of them is. ISO 32000-2
/// §12.7.5.2.4:
///
/// > Like check boxes, individual radio buttons have two states, on and off.
///
/// and §12.7.5.2.3 makes the exclusion a `shall` where Table 229 bit 26 is clear: "at most one
/// radio button in a field shall be set at a time". A tree that says three are is the sharpest
/// form of trap 5 — the person it misleads is the one for whom the picture is no answer.
///
/// The fallback is the one both native hosts already apply to their own buttons: a widget whose
/// `/AP` names no on state at all cannot be on by name, so the field's answer is what is left.
///
/// [`Control::CheckBox`]: pdf_model::form::Control::CheckBox
fn this_widgets_control(
    control: &pdf_model::form::Control,
    widget: &pdf_model::form::Widget,
) -> pdf_model::form::Control {
    match control {
        pdf_model::form::Control::CheckBox { on } => pdf_model::form::Control::CheckBox {
            on: widget.on || (*on && widget.on_state.is_none()),
        },
        pdf_model::form::Control::RadioButton {
            on,
            no_toggle_to_off,
            in_unison,
        } => pdf_model::form::Control::RadioButton {
            on: widget.on || (*on && widget.on_state.is_none()),
            no_toggle_to_off: *no_toggle_to_off,
            in_unison: *in_unison,
        },
        // Every other type states one value for the field and no widget of its own has another.
        other => other.clone(),
    }
}
