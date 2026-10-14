//! §12.7's form as the controls Qt places over the page, what a person does in them, and
//! §12.5.1's focus among them (ADRs 0630, 1357; filed here by ADR 1908).

use viewer_core::{Answer, Command, Edit, Entered, FormField, Query};
use viewer_host::ControlFit;
use viewer_host::form::{ControlKind, control_kind};
use viewer_host::trace::Topic;

use crate::bridge::ffi::{QtControl, QtMeasure, QtQuad};

use super::{Host, quad};

/// One control placed over the page, and which field's widget it is for.
#[derive(Debug, Clone)]
pub(super) struct Placement {
    /// The two names §14.9.3 makes a processor distinguish: one to address an edit to, one to say.
    ///
    /// §12.7.4.2's qualified name alone would name a field the way a *file* does rather than the
    /// way the clause says a user interface shall (ADR 0630).
    name: pdf_model::view::FieldName,
    /// The widget annotation, which with the name above identifies this control.
    annotation: pdf_syntax::ObjectId,
    /// What kind of control it is.
    kind: ControlKind,
    /// The appearance state §12.7.5.2.3 makes a check box's on value, where the file names one.
    on_state: Option<String>,
    /// Table 229 bit 15: whether clicking the selected radio button of a set turns it off.
    no_toggle_to_off: bool,
    /// Table 227 bit 1: "the field shall not be modified by the user".
    ///
    /// The control is disabled, so a person cannot reach it — and an assistive technology's click
    /// does not go through the control, which is why the flag has to be carried rather than left
    /// to the widget's own sensitivity (ADR 0630).
    read_only: bool,
}

impl Placement {
    /// What identifies this control between two frames: the field's name and its widget.
    pub(super) fn key(&self) -> (String, pdf_syntax::ObjectId) {
        (self.name.qualified.clone(), self.annotation)
    }
}

impl Host {
    /// A person finished with a line edit: Enter, or the keyboard taken elsewhere.
    ///
    /// Table 231 bit 13 clear restricts a field's text "to a single line", so Enter there is no
    /// character and commits, and a control losing the keyboard is Table 197's `/Bl` for a focus
    /// Qt moved rather than the page. A field the page's own focus move already committed commits
    /// nothing (ADR 1592).
    pub(crate) fn commit_control(&mut self, index: usize) {
        let Some(placed) = self.placed.get(index) else {
            return;
        };
        let field = placed.name.qualified.clone();
        self.dispatch(Command::CommitField { field });
    }

    /// A control's value was typed into.
    pub(crate) fn set_control(&mut self, index: usize, value: &str) {
        let Some(placed) = self.placed.get(index) else {
            return;
        };
        let field = placed.name.qualified.clone();
        // §12.7.5.3's file-select control takes a *file* where every other text field takes text,
        // and `viewer_host::form::edit_of` is the one place the three windows tell the two apart
        // (ADR 1216).
        let kind = placed.kind.clone();
        match viewer_host::form::edit_of(Some(&kind), &field, Entered::Text(value.to_owned())) {
            Ok(edit) => self.dispatch(Command::Edit(edit)),
            Err(refusal) => self.say(&refusal),
        }
    }

    /// §12.7.5.4: which of Table 234's `/Opt` entries are selected now.
    ///
    /// An empty set is a field with nothing chosen, which the clause makes a real state — "[t]he
    /// default value of V is null , indicating that no item is currently selected" — and not a
    /// reason to send nothing.
    pub(crate) fn choose_control(&mut self, index: usize, chosen: &[u32]) {
        let Some(placed) = self.placed.get(index) else {
            return;
        };
        let field = placed.name.qualified.clone();
        let chosen: Vec<usize> = chosen
            .iter()
            .filter_map(|index| usize::try_from(*index).ok())
            .collect();
        self.state.trace.say(
            Topic::Panel,
            format_args!("{field}: option(s) {chosen:?} of Table 234's /Opt selected"),
        );
        self.dispatch(Command::Edit(Edit::SetField {
            field,
            value: Entered::Chosen(chosen),
        }));
    }

    /// §12.7.5.2.3's check box or §12.7.5.2.4's radio button was clicked.
    ///
    /// The *name* the value takes is this side's, because it is a clause and not a widget:
    /// §12.7.5.2.3 makes `/V` select among Table 170's appearance states by name and the names
    /// are the file's own, so a host that sent "on" would be inventing one.
    ///
    /// **The rule is [`viewer_host::toggling`]**, shared with the other two windows and with this
    /// host's own accessibility click, so a `QCheckBox`'s `toggled` and an assistive technology's
    /// `DoAction` cannot come to different answers about one clause (ADR 0630). What this method
    /// still owns is the *index*, which is the one thing only a placed control knows.
    pub(crate) fn toggle_control(&mut self, index: usize, on: bool) {
        let Some(placed) = self.placed.get(index) else {
            return;
        };
        let clicked = viewer_host::toggling(
            &placed.name,
            placed.read_only,
            on,
            placed.no_toggle_to_off,
            placed.on_state.as_deref(),
        );
        // `false`: the click did reach the control — it *is* the control's signal — so there is
        // nothing about a page coordinate to report.
        if let Some(said) = clicked.note() {
            self.say(&said);
        }
        match clicked {
            viewer_host::Clicked::Toggles { name, value } => {
                self.dispatch(Command::Edit(Edit::SetField {
                    field: name.qualified,
                    value: Entered::Text(value),
                }));
            }
            // A refusal leaves the field alone, and the `QAbstractButton` goes back to whatever
            // `Query::Fields` says on the next `applyUpdates` — which it does, unconditionally,
            // for exactly this reason.
            viewer_host::Clicked::ReadOnly { .. }
            | viewer_host::Clicked::Stays { .. }
            | viewer_host::Clicked::Unnamed { .. }
            | viewer_host::Clicked::Pointed { .. }
            | viewer_host::Clicked::Aimed { .. }
            | viewer_host::Clicked::Page => {}
        }
    }

    /// Which placed control §12.5.1's focus is on, as its index in `placed`, or -1 for none.
    pub(crate) fn focused_control(&self) -> i32 {
        let Answer::Focus { object, .. } = self.state.viewer.query(Query::Focus) else {
            return -1;
        };
        self.placed
            .iter()
            .position(|placed| placed.annotation == object)
            .and_then(|index| i32::try_from(index).ok())
            .unwrap_or(-1)
    }

    /// Which placed control an assistive technology's click aimed the keyboard at, as its index in
    /// `placed`, or -1 for none; asked once after each drain, and answered once (ADR 1566).
    ///
    /// Resolved against the controls as they stand when C++ asks, after the drain's own updates
    /// have been applied, so the index is one into the controls C++ now holds. A widget no control
    /// stands over is said by name rather than dropped (trap 5).
    pub(crate) fn aimed_control(&mut self) -> i32 {
        let Some((name, annotation)) = self.aimed.take() else {
            return -1;
        };
        let found = self
            .placed
            .iter()
            .position(|placed| placed.annotation == annotation)
            .and_then(|index| i32::try_from(index).ok());
        if found.is_none() {
            let said = format!("the field {} has no control placed over it", name.shown());
            self.state
                .trace
                .say(Topic::Access, format_args!("refused: {said}"));
            eprintln!("note: {said}");
        }
        found.unwrap_or(-1)
    }

    /// Carries out a key's press on a push-button: its activation, or the refusal said by name.
    pub(super) fn press(&mut self, pressed: &viewer_host::Pressed) {
        if let Some(said) = pressed.note() {
            self.say(&said);
            return;
        }
        match pressed {
            viewer_host::Pressed::Activates { annotation, .. } => {
                self.dispatch(Command::Activate(*annotation));
            }
            viewer_host::Pressed::ReadOnly { .. } => {}
        }
    }

    /// Every control the window has just placed, measured against the rectangle it was given.
    ///
    /// The counting used to be in `cpp/window.cpp`, which meant the two native hosts computed the
    /// same finding twice from the same numbers and only one of them could offer a magnification.
    /// It is `viewer_host::ControlFit`'s now — `panel.rs`'s reason, and ADR 0346's: a mapping from
    /// rectangles to a number is not a statement about a document, and two hosts measuring the
    /// same thing must not be able to disagree about it.
    pub(crate) fn measured(&mut self, controls: &[QtMeasure]) {
        let mut fit = ControlFit::default();
        for control in controls {
            fit.record(
                (control.asked_width, control.asked_height),
                (control.minimum_width, control.minimum_height),
            );
        }
        let (placed, wider, taller, widest, tallest) = fit.counts();
        if placed == 0 {
            self.fit_magnification = None;
            return;
        }
        self.fit_magnification = self
            .showing_at()
            .and_then(|showing| fit.magnification(showing));
        self.state.trace.say(
            Topic::Panel,
            format_args!(
                "{wider} of {placed} control(s) wider than their /Rect (worst +{} on {} px), \
                 {taller} taller (worst +{} on {} px){}",
                widest.0,
                widest.1,
                tallest.0,
                tallest.1,
                match self.fit_magnification {
                    Some(wanted) => format!("; every control fits at {wanted:.3}, which `w` sends"),
                    None => String::from("; every control fits at this magnification"),
                }
            ),
        );
    }

    /// Every control the page's form wants.
    pub(crate) fn controls(&self) -> Vec<QtControl> {
        let mut controls = Vec::with_capacity(self.placed.len());
        for (placed, (field, widget)) in self.placed.iter().zip(self.widgets()) {
            let (x, y, width, height) = viewer_host::bounds(widget.quad);
            let (kind, max_len, multi, editable) = describe_kind(&placed.kind);
            controls.push(QtControl {
                x,
                y,
                width,
                height,
                kind,
                field: placed.name.qualified.clone(),
                annotation: widget.annotation.number,
                value: field
                    .value
                    .as_ref()
                    .map_or_else(String::new, |shown| shown.text.clone()),
                displayed: field.displayed.clone().unwrap_or_else(|| {
                    field
                        .value
                        .as_ref()
                        .map_or_else(String::new, |shown| shown.text.clone())
                }),
                obscured: field.value.as_ref().is_some_and(|shown| shown.obscured),
                read_only: field.read_only,
                on: widget.on || (on_by_value(&placed.kind) && widget.on_state.is_none()),
                max_len,
                multi,
                editable,
                top: top_option(&placed.kind),
                tooltip: tooltip(field),
                choose_file: viewer_host::may_choose_file(Some(&placed.kind)).is_ok(),
            });
        }
        controls
    }

    /// Table 234's `/Opt` labels for one control.
    pub(crate) fn control_options(&self, index: usize) -> Vec<String> {
        match self.placed.get(index).map(|placed| &placed.kind) {
            Some(ControlKind::Combo { options, .. } | ControlKind::List { options, .. }) => {
                options.clone()
            }
            _ => Vec::new(),
        }
    }

    /// Which of them are selected.
    pub(crate) fn control_selection(&self, index: usize) -> Vec<u32> {
        let chosen: Vec<usize> = match self.placed.get(index).map(|placed| &placed.kind) {
            Some(ControlKind::Combo { selected, .. }) => selected.iter().copied().collect(),
            Some(ControlKind::List { selected, .. }) => selected.clone(),
            _ => Vec::new(),
        };
        chosen
            .into_iter()
            .filter_map(|index| u32::try_from(index).ok())
            .collect()
    }

    /// §12.5.1's focus ring: one quadrilateral, or none.
    pub(crate) fn focus(&self) -> Vec<QtQuad> {
        match self.state.viewer.query(Query::Focus) {
            Answer::Focus { quad: ring, .. } => vec![quad(ring)],
            _ => Vec::new(),
        }
    }

    /// Every widget a control was placed over, in the order the controls are in.
    fn widgets(&self) -> impl Iterator<Item = (&FormField, &viewer_core::FormWidget)> {
        self.fields.iter().flat_map(|field| {
            let kind = control_kind(&field.control);
            field
                .widgets
                .iter()
                .filter(move |widget| placement(field, widget, &kind).is_some())
                .map(move |widget| (field, widget))
        })
    }
}

/// The control one widget of one field becomes, or nothing where the page draws the field itself.
///
/// [`ControlKind::is_placed`] is the set, and it is the one `pdf_model::form::Control::is_delegable`
/// leaves on the page: a push-button's `/AP` is the producer's whole statement of what it looks
/// like, so a toolkit button in its place would be marks the producer did not draw — its click is
/// the pointer's and its key is `viewer_host::pressed`'s (ADR 1357). §12.7.5.5's signature has no
/// control to build and Table 226's absent `/FT` names none.
pub(super) fn placement(
    field: &FormField,
    widget: &viewer_core::FormWidget,
    kind: &ControlKind,
) -> Option<Placement> {
    match kind {
        kind if !kind.is_placed() => None,
        _ => Some(Placement {
            name: field.name.clone(),
            annotation: widget.annotation,
            kind: kind.clone(),
            on_state: widget.on_state.clone(),
            no_toggle_to_off: matches!(
                kind,
                ControlKind::Radio {
                    no_toggle_to_off: true,
                    ..
                }
            ),
            read_only: field.read_only,
        }),
    }
}

/// Which number the bridge carries for a control, and the three flags that go with it.
pub(super) fn describe_kind(kind: &ControlKind) -> (u8, i32, bool, bool) {
    match kind {
        ControlKind::Entry {
            multiline,
            password,
            max_len,
            // Table 231 bit 21 changes what the text *means* rather than what the control is.
            file_select: _,
        } => {
            // Table 231 bit 14 outranks bit 13: a field that is both is a password field, and a
            // password field is never multiline.
            let which = match (*password, *multiline) {
                (true, _) => 2,
                (false, true) => 1,
                (false, false) => 0,
            };
            let cap = max_len
                .and_then(|len| i32::try_from(len).ok())
                .unwrap_or(-1);
            (which, cap, false, false)
        }
        ControlKind::Check { .. } => (3, -1, false, false),
        ControlKind::Radio { .. } => (4, -1, false, false),
        ControlKind::Combo { editable, .. } => (6, -1, false, *editable),
        ControlKind::List { multi, .. } => (7, -1, *multi, false),
        // Unreachable: `placement` returns `None` for all three, so no control of any of them is
        // ever in `placed`. Answered rather than asserted away, because a refusal that cannot
        // happen still has to say something if it does.
        ControlKind::Push | ControlKind::Signature | ControlKind::Unstated => {
            (255, -1, false, false)
        }
    }
}

/// Table 234's `/TI` for a list box, and 0 for every other control.
///
/// The clause states it of the list rather than of the value — "the index in the Opt array of the
/// first option visible in the list" — so it decides where a scrollable list *starts*, which is a
/// question about the control and not about what is selected in it. `pdf-model` reads the entry and
/// the page's own appearance obeys it (ADR 0407); a host that started every list at row 0 would
/// show a different first option than the picture underneath it.
fn top_option(kind: &ControlKind) -> u32 {
    match kind {
        ControlKind::List { top, .. } => u32::try_from(*top).unwrap_or(0),
        _ => 0,
    }
}

/// Whether a control's on-ness comes from the field's value rather than the widget's state.
fn on_by_value(kind: &ControlKind) -> bool {
    match kind {
        ControlKind::Check { on } | ControlKind::Radio { on, .. } => *on,
        _ => false,
    }
}

/// Table 226's `/TU`, as the two hosts show it.
fn tooltip(field: &FormField) -> String {
    let shown = field.name.shown();
    if shown.is_empty() {
        return String::new();
    }
    if field.required {
        return format!("{shown} (required)");
    }
    shown.to_owned()
}
