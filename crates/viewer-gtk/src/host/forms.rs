//! §12.7's controls placed over the page, what a change to one sends to the viewer, and the value written back into the control.

use std::rc::Rc;

use gtk4::glib;
use gtk4::prelude::*;
use viewer_core::{Answer, Command, Entered, FormField, Query};

use crate::controls::{FieldChange, Placed};
use viewer_host::ControlFit;
use viewer_host::trace::Topic;

use crate::controls;

use super::window::logical;
use super::{Host, with};

impl Host {
    /// §12.7's controls, built once and moved thereafter.
    ///
    /// Rebuilt only when the set of fields and widgets on the page changes, because rebuilding
    /// takes the keyboard away from whatever a person is typing into and the page moves under
    /// them on every scroll.
    pub(super) fn place_fields(&mut self, fields: &[FormField]) {
        let scale = f64::from(self.scale);
        if controls::signature(fields)
            != self
                .placed
                .iter()
                .map(|p| p.key.clone())
                .collect::<Vec<_>>()
        {
            for placed in self.placed.drain(..) {
                self.ui.fixed.remove(&placed.widget);
            }
            let change = self.field_sink();
            for field in fields {
                for widget in &field.widgets {
                    if let Some(placed) = controls::build(field, widget, &self.suppress, &change) {
                        self.ui.fixed.put(&placed.widget, 0.0, 0.0);
                        self.placed.push(placed);
                    }
                }
            }
            self.trace.say(
                Topic::Panel,
                format_args!(
                    "{} field(s) on the page, {} control(s) placed",
                    fields.len(),
                    self.placed.len()
                ),
            );
        }
        // The quadrilaterals are in device pixels of the viewport, the same form
        // `Selected::quads` and `Answer::Focus` take — one arithmetic in one place (ADR 0118).
        self.suppress.set(true);
        let mut fit = ControlFit::default();
        let focused = GtkWindowExt::focus(&self.ui.window);
        for field in fields {
            for widget in &field.widgets {
                let key = (field.name.qualified.clone(), widget.annotation);
                let Some(placed) = self.placed.iter().find(|placed| placed.key == key) else {
                    continue;
                };
                let (x, y, width, height) = viewer_host::bounds(widget.quad);
                self.ui
                    .fixed
                    .move_(&placed.widget, f64::from(x) / scale, f64::from(y) / scale);
                let (asked_width, asked_height) = (
                    logical(f64::from(width), scale),
                    logical(f64::from(height), scale),
                );
                placed.widget.set_size_request(asked_width, asked_height);
                // `measure` with `-1` in both directions is GTK's "for no particular size", which
                // is the minimum itself; asking for a height at a width below the control's
                // minimum is the case GTK warns about on the console.
                let (minimum_width, ..) = placed.widget.measure(gtk4::Orientation::Horizontal, -1);
                let (minimum_height, ..) = placed.widget.measure(gtk4::Orientation::Vertical, -1);
                fit.record((asked_width, asked_height), (minimum_width, minimum_height));
                let holds = focused.as_ref().is_some_and(|focused| {
                    focused.is_ancestor(&placed.widget) || *focused == placed.widget
                });
                write_back(placed, field, widget, holds);
            }
        }
        self.suppress.set(false);
        self.report_fit(&fit);
    }

    /// What the controls' minimum sizes say about the magnification, said once per placement.
    ///
    /// **ADR 0245 left this as a third decision and it needed no message.** The count is what
    /// ADR 0244 and ADR 0246 measured by hand on the two toolkits; the magnification beside it is
    /// `viewer_host::ControlFit`'s arithmetic over the same numbers, and pressing `w` sends it as
    /// `Zoom::Scale`, which the vocabulary has (ADR 0116).
    fn report_fit(&mut self, fit: &ControlFit) {
        let (placed, wider, taller, widest, tallest) = fit.counts();
        if placed == 0 {
            self.fit_magnification = None;
            return;
        }
        self.fit_magnification = self
            .showing_at()
            .and_then(|showing| fit.magnification(showing));
        self.trace.say(
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

    /// The magnification the page is drawn at now, in the units [`Zoom::Scale`](viewer_core::Zoom::Scale) takes.
    ///
    /// `PageGeometry::scale` is device pixels per user space unit — "the zoom and the display's
    /// scale together" — and `Zoom::Scale` is *logical* pixels per user space unit, so the
    /// display's own factor comes back out. Getting that wrong on a doubled display would put the
    /// answer out by two, which is exactly the class of arithmetic ADR 0118 keeps in one place.
    fn showing_at(&self) -> Option<f32> {
        let Answer::Page { index, .. } = self.viewer.query(Query::CurrentPage) else {
            return None;
        };
        let Answer::Geometry(geometry) = self.viewer.query(Query::PageGeometry(index)) else {
            return None;
        };
        let display = self.scale.max(1);
        #[expect(
            clippy::cast_precision_loss,
            reason = "a display scale is 1, 2 or 3; the lint is about the general case"
        )]
        let logical = geometry.scale / display as f32;
        (logical.is_finite() && logical > 0.0).then_some(logical)
    }

    /// What a control does when a person changes it.
    fn field_sink(&self) -> Rc<dyn Fn(FieldChange)> {
        let me = self.me.clone();
        Rc::new(move |change| {
            with(&me, |host| match change {
                FieldChange::Set { field, value } => {
                    if let Entered::Chosen(chosen) = &value {
                        host.trace.say(
                            Topic::Panel,
                            format_args!(
                                "{field}: option(s) {chosen:?} of Table 234's /Opt selected"
                            ),
                        );
                    }
                    // §12.7.5.3's file-select control takes a *file* where every other text
                    // field takes text, and `viewer_host::form::edit_of` is the one place the
                    // three windows tell the two apart (ADR 1216).
                    let control = host
                        .placed
                        .iter()
                        .find(|placed| placed.key.0 == field)
                        .map(|placed| placed.kind.clone());
                    match viewer_host::form::edit_of(control.as_ref(), &field, value) {
                        Ok(edit) => host.dispatch(Command::Edit(edit)),
                        Err(refusal) => host.say(&refusal),
                    }
                }
                FieldChange::Commit { field, enter } => {
                    host.dispatch(Command::CommitField { field });
                    if enter {
                        // From an idle, as every focus move here is: GTK finishes its own handling
                        // of the key after this returns. The control losing the keyboard commits
                        // again, which finds nothing typed and does nothing (ADR 1592), and the
                        // refresh it ends in writes what the field displays (ADR 1604).
                        let page = host.ui.page_area.clone();
                        glib::idle_add_local_once(move || {
                            page.grab_focus();
                        });
                    }
                }
                FieldChange::Arrows { held } => {
                    if held {
                        host.trace.say(
                            Topic::Panel,
                            format_args!("the selection is an arrow key's"),
                        );
                    }
                    host.tell_keys_with(held);
                }
                FieldChange::Holds { .. } => {
                    let fields = match host.viewer.query(Query::Fields) {
                        Answer::Fields(fields) => fields,
                        _ => Vec::new(),
                    };
                    host.place_fields(&fields);
                }
            });
        })
    }
}

/// The value a field now holds, written back into its control.
///
/// ADR 0201: a host keeps the *point* it clicked and never the text, because §12.7.5.3's
/// truncation means the field can take less than was typed. So the control shows what the field
/// took — except where the answer says the string is not the field's characters, which is Table
/// 231 bit 14's password field: writing that back would replace what a person typed with a row of
/// dots and send those as the next value.
///
/// **The refusal is on the answer rather than on the control**, since ADR 0247. It used to be the
/// `password: false` in the pattern below, which is this host inferring from the control's kind
/// what the answer now states; `ShownValue::obscured` is the statement, and a control kind that
/// stopped agreeing with it would have been a silent bug in exactly the place this one was found.
///
/// **§12.7.5.2's two toggling kinds** (ADR 0630): a `GtkCheckButton`'s state follows the field, not
/// only the moment the control was *built*. Otherwise a value the field acquired any other way — an
/// undo, an imported §12.7.8 data set, a click an assistive technology asked for, or the other
/// button of a radio set going on — would leave the picture saying one thing and `/V` another.
/// §12.7.5.2.4 is what makes the last of those a clause rather than an untidiness: with
/// `RadiosInUnison` clear, "at most one radio button in a field shall be set at a time", and two
/// buttons would show a tick together.
///
/// **A text field's control shows one of two strings, and the keyboard decides which** (ADR 1604):
/// the field's characters while it holds the keyboard, so that typing starts from the value the
/// format was made of, and [`FormField::displayed`] — Table 199's `/F` applied — while it does not,
/// which is what the page shows under it everywhere else. `holds` is whether it has the keyboard.
fn write_back(placed: &Placed, field: &FormField, widget: &viewer_core::FormWidget, holds: bool) {
    if let Some(button) = placed.widget.downcast_ref::<gtk4::CheckButton>() {
        // The same expression `controls::toggle` builds the button with, re-derived from what
        // `Query::Fields` says *now*: a widget is on when the field's value names its appearance
        // state, and a widget whose `/AP` names no on state at all falls back to the field's own.
        let on = widget.on
            || (matches!(
                viewer_host::control_kind(&field.control),
                viewer_host::form::ControlKind::Check { on: true }
                    | viewer_host::form::ControlKind::Radio { on: true, .. }
            ) && widget.on_state.is_none());
        if button.is_active() != on {
            button.set_active(on);
        }
        return;
    }
    let Some(shown) = field.value.as_ref() else {
        return;
    };
    if shown.obscured {
        return;
    }
    let value = shown.text.as_str();
    let written = if holds {
        value
    } else {
        field.displayed.as_deref().unwrap_or(value)
    };
    match &placed.kind {
        viewer_host::form::ControlKind::Entry {
            multiline: false, ..
        } => {
            if let Some(entry) = placed.widget.downcast_ref::<gtk4::Entry>()
                && entry.text() != written
            {
                // GTK selects an entry's whole text as it takes the keyboard; the characters that
                // replace the displayed string keep that, and otherwise the caret is after them.
                let whole = entry.selection_bounds().is_some_and(|(start, end)| {
                    start == 0 && usize::try_from(end).ok() == Some(entry.text().chars().count())
                });
                entry.set_text(written);
                if holds && whole {
                    entry.select_region(0, -1);
                } else if holds {
                    entry.set_position(-1);
                }
            }
        }
        viewer_host::form::ControlKind::Entry {
            multiline: true, ..
        } => {
            if let Some(scroller) = placed.widget.downcast_ref::<gtk4::ScrolledWindow>()
                && let Some(view) = scroller.child().and_downcast::<gtk4::TextView>()
            {
                let buffer = view.buffer();
                let (start, end) = buffer.bounds();
                if buffer.text(&start, &end, false) != written {
                    buffer.set_text(written);
                }
            }
        }
        // Table 233 bit 19 set: the text box half of `controls::editable_combo`, which is where a
        // row picked out of the drop-down arrives — the list sends a position and the field
        // resolves it to the string Table 234's `/Opt` entry displays, so the entry shows what the
        // *field* holds rather than what this host guessed it would.
        viewer_host::form::ControlKind::Combo { editable: true, .. } => {
            if let Some(entry) = placed
                .widget
                .downcast_ref::<gtk4::Box>()
                .and_then(WidgetExt::first_child)
                .and_downcast::<gtk4::Entry>()
                && entry.text() != value
            {
                entry.set_text(value);
            }
        }
        _ => {}
    }
}
