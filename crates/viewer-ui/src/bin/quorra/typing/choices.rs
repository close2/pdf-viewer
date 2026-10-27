//! §12.7.5.4's open list: drawing it, scrolling it with the arrows and committing a row.

use viewer_core::{Answer, Command, Edit, Entered, Query};
use viewer_host::form::{ControlKind, control_kind};

use super::{Aim, Choosing};
use crate::app::App;

impl App {
    /// Opens §12.7.5.4's options where the press was on a choice field.
    ///
    /// **This is the control, not a decoration.** A tier-1 host places a `GtkDropDown` or a
    /// `QComboBox` and a person picks a row out of it; this host draws the page's own appearance,
    /// so until now the *only* value it could give a choice field was characters — which Table 233
    /// bit 19 admits for one combo box in two and no list box at all. Both of §12.7.5.4's forms
    /// list their options here, because the clause's two controls differ in how the options are
    /// shown and not in whether a person may choose one.
    ///
    /// Answers whether a list is now open, so that a press on a choice field does not also start a
    /// drag on the page underneath it.
    pub(crate) fn open_choices(&mut self, at: (f32, f32)) -> bool {
        let Some(pressed) = self.control_at(at) else {
            return false;
        };
        if !matches!(
            pressed.kind,
            ControlKind::Combo { .. } | ControlKind::List { .. }
        ) {
            return false;
        }
        // Table 227 bit 1: "the field shall not be modified by the user". Checked here as well as
        // in the core for `App::toggle_button`'s reason — the core refuses the edit, and a program
        // that opened a list nothing could be picked out of looks broken rather than obedient.
        if pressed.read_only {
            println!("note: the field {} is read-only (Table 227)", pressed.field);
            return false;
        }
        self.choosing = Some(Choosing {
            field: pressed.field,
            annotation: pressed.annotation,
            first: None,
        });
        self.redraw();
        true
    }

    /// §12.7.5.4's open list: where its rows are, and which control they belong to.
    ///
    /// Re-derived per frame and per press rather than kept, which is [`Choosing`]'s own reason:
    /// the page moves under an open list, and one derivation used for both the drawing and the
    /// hit test is what keeps the row shown and the row picked the same row.
    pub(crate) fn choices(&self) -> Option<(viewer_ui::chrome::ChoiceList, ControlKind)> {
        let choosing = self.choosing.as_ref()?;
        let chrome = self.chrome.as_ref()?;
        let window = self.window()?;
        let Answer::Fields(fields) = self.viewer.query(Query::Fields) else {
            return None;
        };
        let field = fields
            .iter()
            .find(|field| field.name.qualified == choosing.field)?;
        let widget = field
            .widgets
            .iter()
            .find(|widget| widget.annotation == choosing.annotation)?;
        let kind = control_kind(&field.control);
        // Where the list starts. Table 234's `/TI` is the clause's own answer for a scrollable
        // list box — "the index in the Opt array of the first option visible in the list" — and it
        // says nothing about a drop-down, so a combo box starts where its value is: a list that
        // opened at row 0 with the selection forty rows below would be showing the document's data
        // and hiding its answer.
        let (options, selected, first) = match &kind {
            ControlKind::Combo {
                options, selected, ..
            } => (
                options.clone(),
                selected.map(|index| vec![index]).unwrap_or_default(),
                selected.unwrap_or_default(),
            ),
            ControlKind::List {
                options,
                selected,
                top,
                ..
            } => (options.clone(), selected.clone(), *top),
            _ => return None,
        };
        let first = choosing.first.unwrap_or(first);
        // Device pixels of the *page's* viewport, which begins where the panel ends — the same one
        // addition every other overlay makes.
        #[expect(
            clippy::cast_precision_loss,
            reason = "a panel width in pixels, which is hundreds"
        )]
        let edge = self.inset() as f32;
        let mut quad = widget.quad;
        for x in quad.iter_mut().step_by(2) {
            *x += edge;
        }
        let list =
            viewer_ui::chrome::ChoiceList::of(chrome, quad, &options, &selected, first, window)?;
        Some((list, kind))
    }

    /// Takes §12.7.5.4's options off the screen.
    pub(crate) fn close_choices(&mut self) -> bool {
        if self.choosing.take().is_none() {
            return false;
        }
        self.redraw();
        true
    }

    /// A press while §12.7.5.4's options are on the screen. Answers whether the list took it.
    ///
    /// A press on a row is the selection; a press anywhere else dismisses the list and goes no
    /// further, because a list that closed *and* acted on what was behind it would act on
    /// something the person pressing could not see.
    pub(crate) fn press_on_choices(&mut self, at: (f32, f32)) -> bool {
        let Some((list, kind)) = self.choices() else {
            return false;
        };
        #[expect(
            clippy::cast_precision_loss,
            reason = "a panel width in pixels, which is hundreds"
        )]
        let point = (at.0 + self.inset() as f32, at.1);
        let Some(option) = list.option_at(point) else {
            self.close_choices();
            return true;
        };
        let Some(choosing) = self.choosing.clone() else {
            return true;
        };
        let (value, stays) = match &kind {
            // Table 233 bit 22 set: "more than one of the field's option items may be selected
            // simultaneously". A press adds or removes one and the list stays up, because a person
            // choosing several has not finished after the first.
            ControlKind::List {
                selected,
                multi: true,
                ..
            } => {
                let mut chosen: Vec<usize> = selected.clone();
                if let Some(place) = chosen.iter().position(|held| *held == option) {
                    chosen.remove(place);
                } else {
                    chosen.push(option);
                    // Table 234's `/I` wants them "sorted in ascending order", and this is the
                    // list that becomes it.
                    chosen.sort_unstable();
                }
                (Entered::Chosen(chosen), true)
            }
            // "at most one item shall be selected", and a drop-down shows one value.
            _ => (Entered::Chosen(vec![option]), false),
        };
        self.dispatch(Command::Edit(Edit::SetField {
            field: choosing.field,
            value,
        }));
        if !stays {
            self.close_choices();
        }
        self.redraw();
        true
    }

    /// An up or down arrow while §12.7.5.4's options are on the screen. Answers whether the list
    /// took it.
    ///
    /// The arrow selects one option ([`viewer_ui::chrome::ChoiceList::stepped`]) and the list stays
    /// up, scrolled where the option has left its rows. The window says the arrows are down for the
    /// edit alone — `Command::Keys` before it and after it — so the field's Table 199 `/K`, which
    /// runs when a person "modifies the selection in a scrollable list box", reads Adobe's
    /// `event.keyDown` true for this selection and false for a press (ADRs 1762, 1786). An arrow at
    /// the end of the list selects what is already selected, which modifies nothing and sends
    /// nothing.
    pub(crate) fn arrow_on_choices(&mut self, down: bool) -> bool {
        let Some(choosing) = self.choosing.clone() else {
            return false;
        };
        let Some((_, kind)) = self.choices() else {
            return false;
        };
        let (selected, count, first) = match &kind {
            ControlKind::Combo {
                options, selected, ..
            } => (
                selected.map(|index| vec![index]).unwrap_or_default(),
                options.len(),
                selected.unwrap_or_default(),
            ),
            ControlKind::List {
                options,
                selected,
                top,
                ..
            } => (selected.clone(), options.len(), *top),
            _ => return false,
        };
        let Some(option) = viewer_ui::chrome::ChoiceList::stepped(&selected, count, down) else {
            return true;
        };
        let first = choosing.first.unwrap_or(first);
        if let Some(open) = self.choosing.as_mut() {
            open.first = Some(viewer_ui::chrome::ChoiceList::showing(first, option, count));
        }
        if selected != [option] {
            let held = |arrows| {
                Command::Keys(pdf_model::view::Keys {
                    shift: self.shift,
                    modifier: self.control,
                    arrows,
                })
            };
            let (down, up) = (held(true), held(false));
            println!(
                "note: {}: option {option} of Table 234's /Opt selected with an arrow",
                choosing.field
            );
            self.dispatch(down);
            self.dispatch(Command::Edit(Edit::SetField {
                field: choosing.field,
                value: Entered::Chosen(vec![option]),
            }));
            self.dispatch(up);
        }
        self.redraw();
        true
    }

    /// Reads §12.7.5.3's file-select control's file, where the field being left is one.
    ///
    /// Does nothing at all for every other field: its value is already in the log, put there by
    /// the keystrokes that typed it. `viewer_host::form::edit_of` is the decision, shared with the
    /// two native hosts so that three windows agree about what a person's typing into such a
    /// field means (ADR 1216).
    pub(super) fn commit_chosen_file(&mut self, aim: &Aim, value: &str) {
        let Aim::Field(field) = aim else { return };
        let Answer::Fields(fields) = self.viewer.query(Query::Fields) else {
            return;
        };
        let Some(kind) = fields
            .iter()
            .find(|one| one.name.qualified == *field)
            .map(|one| control_kind(&one.control))
        else {
            return;
        };
        match viewer_host::form::edit_of(Some(&kind), field, Entered::Text(value.to_owned())) {
            Ok(edit @ Edit::ChooseFile { .. }) => self.dispatch(Command::Edit(edit)),
            // An ordinary field, whose value every keystroke has already sent.
            Ok(_) => {}
            Err(refusal) => println!("note: {refusal}"),
        }
    }
}
