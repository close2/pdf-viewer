//! How an annotation is looked at: its appearance state under the pointer, whether it is hidden,
//! the view a host reads of it, and which annotation lies under a point.
//!
//! Reading only. Placing an annotation is `markup`'s, and the actions that hide one are `actions`'.

use pdf_syntax::{Dictionary, Document, ObjectId};

use super::{AnnotationView, Appearance, FieldValue, Pointer, ViewState};

impl ViewState {
    /// Puts the pointer on an annotation, or takes it off every annotation.
    ///
    /// A viewer calls this from its own hit testing: §12.5.5 speaks of the cursor being "into
    /// the annotation's active area", which is a question about a window's coordinates and a
    /// `/Rect` that this crate has no events to answer. What it decides here is which of
    /// Table 170's appearances [`Self::appearance_for`] chooses, and therefore what the next
    /// render draws.
    pub fn set_pointer(&mut self, at: Option<(ObjectId, Pointer)>) {
        self.pointer = at;
    }

    /// Puts a playing sound in a screen annotation's rectangle, or takes it out.
    ///
    /// ISO 32000-2 Table 294's `/W` 3 plays a media object in "\[t]he rectangle occupied by the
    /// screen annotation (see 12.5.6.18, "Screen annotations") associated with the media
    /// rendition", and its `/B` "shall be used if the media object does not entirely cover the
    /// rectangle". A sound covers none of it, so while one plays the rectangle is painted with the
    /// background over the annotation's own appearance; the window calls this when its player
    /// says the sound has started and again when it has ended (ADR 3407).
    pub fn set_playing_in(&mut self, playing: Option<(ObjectId, crate::media::Background)>) {
        self.playing_in = playing;
    }

    /// The screen annotation a sound plays in now and its background, if one does.
    #[must_use]
    pub fn playing_in(&self) -> Option<(ObjectId, crate::media::Background)> {
        self.playing_in
    }

    /// Which of Table 170's appearances this annotation shows now.
    ///
    /// [`Appearance::Normal`] for every annotation the pointer is not on, which §12.5.5 makes
    /// the case rather than a default: the normal appearance "shall be used when the
    /// annotation is not interacting with the user".
    ///
    /// And for every annotation while the purpose is printing, wherever the pointer is: the
    /// same bullet of ISO 32000-2 §12.5.5 goes on
    ///
    /// > This appearance is also used for printing the annotation.
    ///
    /// so a press the screen is showing is not something a printed sheet shows (ADR 2813).
    #[must_use]
    pub fn appearance_for(&self, annotation: ObjectId) -> Appearance {
        if self.purpose == crate::optional_content::Purpose::Print {
            return Appearance::Normal;
        }
        match self.pointer {
            Some((id, Pointer::Over)) if id == annotation => Appearance::Rollover,
            Some((id, Pointer::Down)) if id == annotation => Appearance::Down,
            _ => Appearance::Normal,
        }
    }

    /// Whether §12.6.4.11 has hidden this annotation, shown it, or said nothing about it.
    ///
    /// `Some(true)` means an action hid it, `Some(false)` that an action showed one the file
    /// marks hidden, and `None` that the annotation's own `/F` decides (§12.5.3).
    #[must_use]
    pub fn annotation_hidden(&self, annotation: ObjectId) -> Option<bool> {
        if self.hidden.contains(&annotation) {
            Some(true)
        } else if self.shown.contains(&annotation) {
            Some(false)
        } else {
            None
        }
    }

    /// Everything this state says about one annotation, gathered once.
    ///
    /// The five questions are asked together, per annotation and per page, so asking them in one
    /// call is one lookup per set rather than five walks of the page's annotation array.
    #[must_use]
    pub fn annotation(&self, annotation: ObjectId) -> AnnotationView<'_> {
        AnnotationView {
            purpose: self.purpose,
            paper: self.paper,
            hidden_by_action: self.annotation_hidden(annotation),
            appearance: self.appearance_for(annotation),
            value: if let Some(edit) = self.edited.get(&annotation) {
                edit.as_field_value()
            } else if let Some(import) = self.imported.get(&annotation) {
                FieldValue::Imported {
                    value: import.value.as_ref(),
                    flags: import.field_flags,
                    rich: import.rich_value.as_deref(),
                }
            } else if self.reset.contains(&annotation) {
                FieldValue::Default
            } else {
                FieldValue::Stored
            },
            flags: self.imported.get(&annotation).and_then(|import| {
                (!import.annotation_flags.is_unchanged()).then_some(import.annotation_flags)
            }),
            icon_fit: self
                .imported
                .get(&annotation)
                .and_then(|import| import.icon_fit.as_ref()),
            imported_appearance: self
                .imported
                .get(&annotation)
                .and_then(|import| import.appearance.as_ref()),
            contents: self.retyped.get(&annotation).map(String::as_str),
            editing: self.is_editing(annotation),
            displayed: self.displayed(annotation),
            scripted: self.scripted(annotation),
        }
    }

    /// §12.7.8's `/A` and `/AA`, where an FDF file stated either for this widget's field.
    ///
    /// A dictionary holding whichever of the two the file stated, which is what
    /// [`crate::action::for_annotation`] reads: Table 197 states a precedence *between* the two
    /// entries — "\[f]or backward compatibility, the A entry in an annotation dictionary, if
    /// present, takes precedence over this entry" — so a caller composes this over the widget's
    /// own dictionary and asks the one function, rather than deciding the precedence twice.
    ///
    /// `None` for a widget no import has given either entry, which is every widget of every
    /// document until an import-data action runs. ADR 1223.
    #[must_use]
    pub fn imported_actions(&self, widget: ObjectId) -> Option<&Dictionary> {
        self.imported
            .get(&widget)
            .and_then(|import| import.actions.as_ref())
    }

    /// [`Self::annotation`] for an annotation that may have no object number of its own.
    ///
    /// A direct annotation dictionary in a page's `/Annots` array has no identity a person could
    /// have interacted with, so every per-annotation answer is the file's — but
    /// [`AnnotationView::purpose`] and [`AnnotationView::paper`] are facts about the *output*,
    /// and an annotation the file wrote inline is printed on the same sheet as the ones beside
    /// it. Taking [`AnnotationView::default`] for it would have decided §12.5.3's bit 3 by the
    /// screen's row on exactly those annotations. ADR 1179.
    #[must_use]
    pub fn annotation_of(&self, annotation: Option<ObjectId>) -> AnnotationView<'_> {
        match annotation {
            Some(id) => self.annotation(id),
            None => AnnotationView {
                purpose: self.purpose,
                paper: self.paper,
                ..AnnotationView::default()
            },
        }
    }
}

/// Whether pressing this annotation changes what is drawn.
///
/// Two clauses can make it so, and a caller has to ask about both. Table 170's `/D` is the
/// appearance §12.5.5 shows while a button is down, and Table 191's `/H` is the highlighting
/// mode §12.5.6.19 states for the same moment — and `/H`'s default is `I`, so an annotation that
/// states *neither* entry still inverts.
///
/// Asked before the pointer state is changed at all, because changing it invalidates the page's
/// display list: a cursor crossing an annotation for which a press would change nothing would
/// otherwise re-interpret the page for a picture that cannot differ. That is a real cost — 2 000 M
/// instructions on the benchmark page — and it is why this exists rather than the caller looking
/// for an `/AP` `/D` and stopping there, which would leave §12.5.6.19 unreachable from the one
/// program that has a mouse (ADR 0123).
///
/// **A third clause joins them** (ADR 0177): §12.5.3's
/// `ToggleNoView` decides whether the annotation is drawn *at all* while the pointer is on it,
/// which is the largest change a press can make to a picture.
#[must_use]
pub fn press_changes_appearance(
    document: &Document,
    annotation: ObjectId,
    view: AnnotationView<'_>,
) -> bool {
    let object = document.get(annotation);
    let Some(dict) = object.as_dict() else {
        return false;
    };
    crate::annotation::press_changes(document, dict, view)
}

/// Whether the cursor arriving on this annotation changes what is drawn.
///
/// The hovering half of [`press_changes_appearance`], and it exists for the same reason: the
/// pointer state invalidates the page's display list, so it is only changed where the picture can
/// differ. Table 170's `/R` is the appearance §12.5.5 shows "when the user moves the cursor into
/// the annotation's active area without pressing the mouse button", and §12.5.3's `ToggleNoView`
/// decides whether the annotation is drawn at all while it is there.
#[must_use]
pub fn hover_changes_appearance(
    document: &Document,
    annotation: ObjectId,
    view: AnnotationView<'_>,
) -> bool {
    let object = document.get(annotation);
    let Some(dict) = object.as_dict() else {
        return false;
    };
    crate::annotation::hover_changes(document, dict, view)
}

/// Whether the pointer may act on one annotation, by the bits [`annotation_at`] reads.
///
/// §12.5.3's `Hidden`, `NoView` and `ReadOnly` each say whether an annotation may "interact with the
/// user", and for a widget the field's Table 227 `ReadOnly` says it, since it "subsumes" the
/// widget's own bit 7. What is withdrawn from a person altogether — the focus, an assistive
/// technology's tree — is [`annotation_present`]'s narrower question. `false` for an object that
/// is not a dictionary. ADRs 1369, 3413.
#[must_use]
pub fn annotation_interacts(document: &Document, annotation: ObjectId, view: &ViewState) -> bool {
    let resolved = document.get(annotation);
    resolved
        .as_dict()
        .is_some_and(|dict| passes(Gate::Responds, document, Some(annotation), dict, Some(view)))
}

/// Whether an annotation is there for a person at all — [`annotation_interacts`] but for a
/// widget's field `ReadOnly`, which Table 227 defines as not responding to the pointer.
///
/// What the keyboard's focus and an assistive technology ask: a read-only field is still reached,
/// read and announced, and a click on it is refused by name (ADR 3413).
#[must_use]
pub fn annotation_present(document: &Document, annotation: ObjectId, view: &ViewState) -> bool {
    let resolved = document.get(annotation);
    resolved
        .as_dict()
        .is_some_and(|dict| passes(Gate::Present, document, Some(annotation), dict, Some(view)))
}

/// Which of the two questions of Table 167's interaction bits a route asks (ADR 3413).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Gate {
    /// `crate::annotation::present`: `Hidden`, `NoView`, a non-widget's own `ReadOnly`, a
    /// watermark. Asked by what *names* or *reaches by keyboard*: a field's name under the
    /// pointer, which a host refuses a read-only field's click with, and the focus.
    Present,
    /// `crate::annotation::interacts`: [`Gate::Present`] and, for a widget, the field's `ReadOnly`
    /// that "subsumes" bit 7 — Table 227's "should not respond to mouse clicks nor change their
    /// appearance in response to mouse motions". Asked by everything the pointer *does*: an
    /// appearance, a trigger, a link's activation, a caret.
    Responds,
}

/// The interaction gate, one function every route asks with the [`Gate`] it needs.
///
/// A script's `readonly` on a widget's field is the view state's, and stands over the file's flag
/// as it does for the value (ADR 1603). `view` is `None` for a caller that asks about the file
/// alone, [`super::field_at`]'s. ADR 3413.
pub(super) fn passes(
    gate: Gate,
    document: &Document,
    id: Option<ObjectId>,
    dict: &Dictionary,
    view: Option<&ViewState>,
) -> bool {
    let annotation = view.map_or_else(AnnotationView::default, |view| view.annotation_of(id));
    match gate {
        Gate::Present => crate::annotation::present(document, dict, annotation),
        Gate::Responds => {
            let scripted = view
                .zip(id)
                .and_then(|(view, id)| view.widget_script_read_only(document, id));
            crate::annotation::interacts_under(document, dict, annotation, scripted)
        }
    }
}

/// Whether a point in default user space reaches one annotation of `page`: the gate and then
/// the active area, where §12.5.3's alteration has drawn it. The test every hit test of this
/// module shares, so that a pointer, a click and a field's name cannot disagree. ADRs 3413, 3414.
pub(super) fn reaches(
    gate: Gate,
    document: &Document,
    page: &crate::Page,
    view: Option<&ViewState>,
    (id, dict): (Option<ObjectId>, &Dictionary),
    (x, y): (f32, f32),
) -> bool {
    if !passes(gate, document, id, dict, view) {
        return false;
    }
    let annotation = view.map_or_else(AnnotationView::default, |view| view.annotation_of(id));
    let geometry = crate::annotation::ViewGeometry {
        rotate: page.rotate,
        // A pointer is on a screen, so the screen's magnification is the one §12.5.3 holds a
        // NoZoom annotation against; a caller with no view state states none, and NoZoom then
        // changes nothing, as it does for the drawing (ADR 2813).
        magnification: view.and_then(ViewState::magnification),
        media_box: page.media_box,
    };
    let (x, y) = crate::annotation::unaltered_point(document, dict, annotation, geometry, (x, y));
    active_area_covers(document, dict, x, y)
}

/// The field a point in default user space is on, with its fully qualified name.
///
/// **Any subtype**, which is what separates this from [`super::field_at`] and from
/// `crate::link::at`: §12.6.3's trigger events are Table 197's, and Table 197 belongs to every
/// annotation dictionary rather than to links or widgets.
///
/// The **last** match, because a page lists its annotations in painting order and the one on top
/// is the one under the pointer.
#[must_use]
pub fn annotation_at(
    document: &Document,
    page: &crate::Page,
    view: &ViewState,
    x: f32,
    y: f32,
) -> Option<ObjectId> {
    let annotations = document.get_key(&page.dict, "Annots");
    let annotations = annotations.as_array()?;
    let mut found = None;
    for annotation in annotations {
        let Some(id) = annotation.as_reference() else {
            continue;
        };
        let resolved = document.resolve(annotation);
        let Some(dict) = resolved.as_dict() else {
            continue;
        };
        // An annotation that renders no ink still has an activation region, which is what a
        // link is, so the gate is the interaction bits and not whether anything was drawn.
        if reaches(
            Gate::Responds,
            document,
            page,
            Some(view),
            (Some(id), dict),
            (x, y),
        ) {
            found = Some(id);
        }
    }
    found
}

/// The link a point activates on one page, where one does: [`annotation_at`]'s test, gate
/// included, over the page's §12.5.6.5 link annotations alone.
///
/// The route a click's activation, a hover's cursor and a host's `LinkAt` query all take, so
/// that a link Table 167 forbids to interact is neither shown as one nor followed (ADR 3413).
/// The last match, for [`annotation_at`]'s reason.
#[must_use]
pub fn link_at(
    document: &Document,
    page: &crate::Page,
    view: &ViewState,
    x: f32,
    y: f32,
) -> Option<crate::link::Link> {
    let annotations = document.get_key(&page.dict, "Annots");
    let annotations = annotations.as_array()?;
    annotations.iter().rev().find_map(|annotation| {
        let id = annotation.as_reference();
        let resolved = document.resolve(annotation);
        let dict = resolved.as_dict()?;
        let link = crate::link::read(document, id, dict)?;
        reaches(
            Gate::Responds,
            document,
            page,
            Some(view),
            (id, dict),
            (x, y),
        )
        .then_some(link)
    })
}

/// Whether an annotation's active area covers a point in default user space.
///
/// Table 166's `/Rect` for every subtype but one: a link's area is Table 176's `/QuadPoints`
/// where the clause admits them, "the region in which the link should be activated", so a
/// press inside `/Rect` and outside every quadrilateral is a press on nothing (ADR 2814).
fn active_area_covers(document: &Document, annotation: &Dictionary, x: f32, y: f32) -> bool {
    let is_link = document
        .get_key(annotation, "Subtype")
        .as_name()
        .is_some_and(|subtype| subtype.as_bytes() == b"Link");
    if is_link {
        crate::link::region_covers(document, annotation, x, y)
    } else {
        rectangle_covers(document, annotation, f64::from(x), f64::from(y))
    }
}

/// Whether Table 166's `/Rect` covers a point in default user space.
///
/// The rectangle is `crate::annotation::rectangle`'s reading — §7.9.5's four numbers, normalised —
/// so that a `/Rect` the drawing cannot place is not one a pointer lands on either.
pub(super) fn rectangle_covers(
    document: &Document,
    annotation: &Dictionary,
    x: f64,
    y: f64,
) -> bool {
    crate::annotation::rectangle(document, annotation, "Rect").is_some_and(|[x0, y0, x1, y1]| {
        x >= f64::from(x0) && x <= f64::from(x1) && y >= f64::from(y0) && y <= f64::from(y1)
    })
}
