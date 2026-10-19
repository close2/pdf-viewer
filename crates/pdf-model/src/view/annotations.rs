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

    /// Which of Table 170's appearances this annotation shows now.
    ///
    /// [`Appearance::Normal`] for every annotation the pointer is not on, which §12.5.5 makes
    /// the case rather than a default: the normal appearance "shall be used when the
    /// annotation is not interacting with the user".
    #[must_use]
    pub fn appearance_for(&self, annotation: ObjectId) -> Appearance {
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
    /// entries — "[f]or backward compatibility, the A entry in an annotation dictionary, if
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

/// Whether a person may interact with one annotation at all, by the bits [`annotation_at`] reads.
///
/// §12.5.3's `Hidden`, `NoView` and `ReadOnly` each say whether an annotation may "interact with the
/// user", and a node an assistive technology is offered to act on is interaction: a widget a click
/// cannot reach is not one to announce as a control. `false` for an object that is not a
/// dictionary. ADR 1369.
#[must_use]
pub fn annotation_interacts(document: &Document, annotation: ObjectId, view: &ViewState) -> bool {
    let resolved = document.get(annotation);
    resolved.as_dict().is_some_and(|dict| {
        crate::annotation::interacts(document, dict, view.annotation(annotation))
    })
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
    let (x, y) = (f64::from(x), f64::from(y));
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
        // §12.5.3's Hidden, NoView and ReadOnly each say "interact with the user", and
        // `annotation::interacts` is those three bits and nothing else — an annotation that
        // renders no ink still has an activation region, which is what a link is.
        if !crate::annotation::interacts(document, dict, view.annotation(id)) {
            continue;
        }
        if rectangle_covers(document, dict, x, y) {
            found = Some(id);
        }
    }
    found
}

/// Whether Table 166's `/Rect` covers a point in default user space.
///
/// The rectangle "shall be two opposite corners" and states no order, so both are normalised
/// before the point is tested against them.
pub(super) fn rectangle_covers(
    document: &Document,
    annotation: &Dictionary,
    x: f64,
    y: f64,
) -> bool {
    let rect = document.get_key(annotation, "Rect");
    let Some(rect) = rect.as_array() else {
        return false;
    };
    let mut corners = rect
        .iter()
        .filter_map(|value| document.resolve(value).as_number());
    let (Some(x0), Some(y0), Some(x1), Some(y1)) = (
        corners.next(),
        corners.next(),
        corners.next(),
        corners.next(),
    ) else {
        return false;
    };
    x >= x0.min(x1) && x <= x0.max(x1) && y >= y0.min(y1) && y <= y0.max(y1)
}
