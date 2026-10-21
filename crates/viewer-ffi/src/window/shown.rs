//! What the window shows over and beside the page, read and committed through `quorra_window`:
//! the panels' rows, the controls placed over the page, §12.5.6.14's popups, and the shapes the
//! find bar, Annex O's highlight and the selection draw (RFC 0010 section 3.2a, ADR 2104).
//!
//! **What changed is a record and what it changed to is read**, which is `viewer-qt`'s update
//! flags beside its getters (ADR 0246) and this window's own shape (ADR 2088): a
//! `QUORRA_OWED_PANELS` says the rows are to be read again, a `QUORRA_OWED_CONTROLS` that the set
//! of controls moved and the widgets are to be rebuilt rather than moved, a `QUORRA_OWED_POPUPS`
//! that a popup opened, closed or moved, and a `QUORRA_OWED_REPAINT` which of the page area's
//! frames, their places and the marks over them changed (`repainted`, ADR 2168). The rows
//! themselves cross as the handles the vocabulary layer already hands over, read through the same
//! [`Reads`].
//!
//! **A commit is addressed the way the toolkit windows address it**: a field by §12.7.4.2's
//! qualified name, a check box or radio button by its widget annotation, a layer by its group and
//! an attachment by its `/EmbeddedFiles` key. Each goes through the decision every window takes —
//! `viewer_host::form::edit_of` for §12.7.5.3's file-select field, `viewer_host::toggling` for
//! §12.7.5.2's two toggling kinds — so this window and the toolkits cannot answer one click two
//! ways.

use pdf_model::view::Entered;
use pdf_syntax::ObjectId;
use viewer_core::{Answer, Command, Edit, FormField, Query};
use viewer_host::native_window::NativeWindow;

use super::Window;
use super::owed::Owed;
use crate::{Matches, Reads, Status};

/// What the window last told its caller the page carries, so that a drain owes a rebuild only
/// where the set changed.
#[derive(Debug, Default)]
pub(super) struct Shown {
    /// The controls placed over the page, by the two names that identify one between two frames:
    /// the field's qualified name and its widget, as `viewer-qt`'s `Placement::key` names them.
    controls: Vec<(String, ObjectId)>,
    /// §12.5.6.14's open windows as the viewer last answered them.
    popups: Vec<viewer_core::PopupWindow>,
    /// The pictures and the marks over them, as the last repaint told them (ADR 2168).
    pub(super) painted: super::repainted::Painted,
    /// The magnification at which every control on the page fits its `/Rect`, from what the
    /// caller last measured, where they do not fit now (ADR 2169).
    pub(super) fit: Option<f32>,
}

impl Window {
    /// What the window's viewer answers, read as a `quorra_viewer *`'s would be (ADR 2104).
    #[must_use]
    pub const fn reads(&self) -> Reads<'_> {
        Reads::of(&self.state.viewer)
    }

    /// Every occurrence of what the find bar holds on the page shown, and none while it holds
    /// nothing — `viewer-qt`'s `matches`, asked on every repaint because it reads a readback that
    /// is already there.
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] where no document is focused.
    pub fn matches(&self) -> Result<Matches, Status> {
        if self.state.needle.is_empty() {
            return Ok(Matches::new(Vec::new()));
        }
        self.reads().find(&self.state.needle)
    }

    /// Activates an object: an outline item's or a thread's row, or a push-button's widget, whose
    /// look the producer drew and whose click is the page's (ADR 1357).
    pub fn activate(&mut self, number: u32, generation: u16) {
        self.dispatch(Command::Activate(ObjectId::new(number, generation)));
        self.settle();
    }

    /// A text control's or an editable combo box's text, as typed: §12.7.5.3's file-select field
    /// takes the file the text names, which `viewer_host::form::edit_of` tells apart (ADR 1216),
    /// and a refusal is said.
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] where no field of that name has a widget on the page shown.
    pub fn set_field_text(&mut self, field: &str, text: String) -> Result<(), Status> {
        let kind = viewer_host::control_kind(&self.field(field)?.control);
        match viewer_host::form::edit_of(Some(&kind), field, Entered::Text(text)) {
            Ok(edit) => self.dispatch(Command::Edit(edit)),
            Err(refusal) => self.say(&refusal),
        }
        self.settle();
        Ok(())
    }

    /// §12.7.5.4: which of Table 234's `/Opt` entries are selected now; none is a field with
    /// nothing chosen, which the clause makes `/V`'s null default rather than a reason to send
    /// nothing.
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] where no field of that name has a widget on the page shown.
    pub fn set_field_options(&mut self, field: &str, chosen: Vec<usize>) -> Result<(), Status> {
        self.edit_field(field, Entered::Chosen(chosen))
    }

    /// Empties a field: §12.7.6.3's "its V entry shall be removed".
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] where no field of that name has a widget on the page shown.
    pub fn clear_field(&mut self, field: &str) -> Result<(), Status> {
        self.edit_field(field, Entered::Cleared)
    }

    /// A control finished with: Enter in a single-line field, or the keyboard taken elsewhere,
    /// which is Table 197's `/Bl` for a focus the caller moved rather than the page (ADR 1592).
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] where no field of that name has a widget on the page shown.
    pub fn commit_field(&mut self, field: &str) -> Result<(), Status> {
        let field = self.field(field)?.name.qualified.clone();
        self.dispatch(Command::CommitField { field });
        self.settle();
        Ok(())
    }

    /// §12.7.5.2.3's check box or §12.7.5.2.4's radio button, by its widget, was clicked to `on`.
    ///
    /// The appearance-state name the value takes is the file's own, chosen by
    /// `viewer_host::toggling` for every window; a refusal it states — read-only, or a selected
    /// radio button that `NoToggleToOff` keeps — is said, and the field is left alone.
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] where no widget on the page shown is that annotation, and
    /// [`Status::WrongKind`] where it is not a check box's or a radio button's.
    pub fn toggle_field(&mut self, number: u32, generation: u16, on: bool) -> Result<(), Status> {
        let annotation = ObjectId::new(number, generation);
        let fields = self.fields_on_the_page();
        let (field, widget) = fields
            .iter()
            .find_map(|field| {
                field
                    .widgets
                    .iter()
                    .find(|widget| widget.annotation == annotation)
                    .map(|widget| (field, widget))
            })
            .ok_or(Status::NoAnswer)?;
        let no_toggle_to_off = match viewer_host::control_kind(&field.control) {
            viewer_host::ControlKind::Check { .. } => false,
            viewer_host::ControlKind::Radio {
                no_toggle_to_off, ..
            } => no_toggle_to_off,
            _ => return Err(Status::WrongKind),
        };
        let clicked = viewer_host::toggling(
            &field.name,
            field.read_only,
            on,
            no_toggle_to_off,
            widget.on_state.as_deref(),
        );
        if let Some(said) = clicked.note() {
            self.say(&said);
        }
        if let viewer_host::Clicked::Toggles { name, value } = clicked {
            self.dispatch(Command::Edit(Edit::SetField {
                field: name.qualified,
                value: Entered::Text(value),
            }));
        }
        self.settle();
        Ok(())
    }

    /// §8.11's switch on a layer's row was moved.
    ///
    /// Table 99's `/Locked` puts a group's state out of reach of an interactive processor's user
    /// interface. The model refuses the click as well; this is the second half of the clause, said
    /// where the command would be sent, as `viewer-qt` says it.
    pub fn set_group(&mut self, number: u32, generation: u16, on: bool) {
        let group = ObjectId::new(number, generation);
        let layers = match self.state.viewer.query(Query::Layers) {
            Answer::Layers(layers) => layers,
            _ => Vec::new(),
        };
        if locked(&layers, group) {
            self.say("that group is locked (Table 99) and its state cannot be changed");
        } else {
            self.dispatch(Command::SetGroup { group, on });
        }
        self.settle();
    }

    /// §7.11.4: an attachment's row was activated, and the file it names is taken out — written,
    /// or opened beside under the level the window holds (ADR 1331).
    pub fn extract(&mut self, name: String) {
        self.dispatch(Command::Extract { name });
        self.settle();
    }

    /// What changed of the controls and the popups since the caller was last told, owed as the
    /// record that says so — compared rather than inferred from `Event::Damage`, which is "a bound
    /// on what changed" and not a list (ADR 0244 finding 5).
    pub(super) fn owe_what_moved(&mut self) {
        let controls: Vec<(String, ObjectId)> = self
            .fields_on_the_page()
            .iter()
            .filter(|field| viewer_host::control_kind(&field.control).is_placed())
            .flat_map(|field| {
                field
                    .widgets
                    .iter()
                    .map(|widget| (field.name.qualified.clone(), widget.annotation))
            })
            .collect();
        if controls != self.shown.controls {
            self.shown.controls = controls;
            self.owing.owe(Owed::Controls);
        }
        let popups = match self.state.viewer.query(Query::Popups) {
            Answer::Popups(windows) => windows,
            _ => Vec::new(),
        };
        if popups != self.shown.popups {
            self.shown.popups = popups;
            self.owing.owe(Owed::Popups);
        }
    }

    /// What the caller measured of each control it placed: the rectangle it asked the control to
    /// occupy and the least the toolkit says the control can be, `[asked width, asked height,
    /// minimum width, minimum height]` in logical pixels — `viewer-qt`'s `measured`, through
    /// `viewer_host::ControlFit` so that no two windows can disagree about the number (ADRs 0245,
    /// 2169). Answers the magnification at which every control fits, where they do not fit now,
    /// which is what `w` sends.
    pub fn controls_measured(&mut self, measured: &[[i32; 4]]) -> Option<f32> {
        let mut fit = viewer_host::ControlFit::default();
        for [asked_width, asked_height, least_width, least_height] in measured {
            fit.record((*asked_width, *asked_height), (*least_width, *least_height));
        }
        self.shown.fit = self
            .showing_at()
            .and_then(|showing| fit.magnification(showing));
        self.shown.fit
    }

    /// `w`: the page magnified until every control fits, from the caller's last measure, or a
    /// sentence saying they already do — `viewer-qt`'s answer, said in its words.
    pub(super) fn fit_the_controls(&mut self) {
        match self.shown.fit {
            Some(wanted) => {
                self.say(&format!("fitting §12.7's controls at {wanted:.3}"));
                self.dispatch(Command::Zoom {
                    zoom: viewer_core::Zoom::Scale(wanted),
                    at: None,
                });
            }
            None => self.say("every control on this page already fits its /Rect"),
        }
    }

    /// The magnification the page in front is drawn at, in `viewer_core::Zoom::Scale`'s units —
    /// logical pixels per unit of user space — as `viewer-qt`'s `showing_at` reads it.
    fn showing_at(&self) -> Option<f32> {
        let Answer::Page { index, .. } = self.state.viewer.query(Query::CurrentPage) else {
            return None;
        };
        let Answer::Geometry(geometry) = self.state.viewer.query(Query::PageGeometry(index)) else {
            return None;
        };
        let display = if self.scale.is_finite() && self.scale >= 1.0 {
            self.scale
        } else {
            1.0
        };
        let logical = geometry.scale / display;
        (logical.is_finite() && logical > 0.0).then_some(logical)
    }

    /// One edit to a field the page shows, by name.
    fn edit_field(&mut self, field: &str, value: Entered) -> Result<(), Status> {
        let field = self.field(field)?.name.qualified.clone();
        self.dispatch(Command::Edit(Edit::SetField { field, value }));
        self.settle();
        Ok(())
    }

    /// The field of that qualified name with a widget on the page shown.
    fn field(&self, qualified: &str) -> Result<FormField, Status> {
        self.fields_on_the_page()
            .into_iter()
            .find(|field| field.name.qualified == qualified)
            .ok_or(Status::NoAnswer)
    }

    /// §12.7's fields with a widget on the page shown.
    fn fields_on_the_page(&self) -> Vec<FormField> {
        match self.state.viewer.query(Query::Fields) {
            Answer::Fields(fields) => fields,
            _ => Vec::new(),
        }
    }
}

/// Whether `/Order` holds `group` as a locked group, at any depth.
fn locked(layers: &[viewer_core::Layer], group: ObjectId) -> bool {
    layers.iter().any(|layer| match layer {
        viewer_core::Layer::Group {
            group: held,
            locked,
            ..
        } => *held == group && *locked,
        viewer_core::Layer::Collection { children, .. } => locked(children, group),
    })
}
