//! Which control a press or a focus addresses, and whether it takes characters at all.

use viewer_core::{Answer, Command, Edit, Entered, Query};
use viewer_host::form::{Clicked, ControlKind, control_kind};

use super::{Aim, Pressed, Target, Typing};
use crate::app::App;

impl App {
    /// Starts or stops typing, at a point of the page's own viewport.
    ///
    /// **The point is the caller's** (ADR 0425), because a pointer is not the only thing that
    /// clicks: an assistive technology asking `org.a11y.atspi.Action` for a click names a *node*,
    /// and [`App::click_page`] turns that into the same three steps a mouse takes. Reading
    /// `self.cursor` here would aim the keyboard wherever the mouse happened to be resting.
    ///
    /// A press inside a field somebody can type into aims the keyboard at it; a press anywhere
    /// else puts the keyboard back on the page. §12.7.5.1's four field types are not equal here —
    /// a button has no text and a signature field's value is a dictionary — and the *core* is what
    /// draws that line: `Answer::Field`'s value is `None` for a field whose value is not text and
    /// `Some("")` for an empty one, which is the same distinction §12.7.4.3 makes when it decides
    /// what to lay out.
    pub(crate) fn aim_at_field(&mut self, at: (f32, f32)) {
        let was = self.typing.is_some();
        // §12.5.6.6 first, because the core hit-tests it first: an annotation a person added is
        // drawn after the page's own `/Annots` and the thing on top is the thing under the
        // pointer. Asking in the other order would put the keyboard in a field underneath a note.
        if let Some(typing) = self.aim_at_free_text(at) {
            self.typing = Some(typing);
            return;
        }
        self.typing = match self.viewer.query(Query::FieldAt(at)) {
            // Table 231 bit 14, refused rather than mishandled (ADR 0247's third amendment). This
            // host reads a field's value back after every keystroke (ADR 0201) and a password field
            // answers with bullets, so what it would send as the next value is those bullets with a
            // character appended. Refusing is trap 5: `viewer-gtk` and `viewer-qt` type into a
            // `GtkPasswordEntry` and a `QLineEdit` in `Password` echo mode, which are the
            // platform's own secure controls, and this host draws its own page and has none.
            Answer::Field {
                name,
                value: Some(shown),
            } if shown.obscured => {
                println!(
                    "note: {} is a Table 231 bit 14 password field, which this host does not type \
                     into: its value answers as bullets and cannot be read back (ADR 0247)",
                    name.shown()
                );
                None
            }
            // **Table 233 bit 19's other half, which this host disobeyed for its whole life.**
            // `Answer::Field` answers a combo box with characters whether or not the flag is set —
            // correctly, because the value *is* text and §12.7.4.3 lays it out — and this host read
            // "has a text value" as "takes typed characters". So a person could type *Purple* into
            // a drop-down whose options are Red and Blue, and the file took it. The bit binds in
            // both directions and `ControlKind::takes_typed_characters` is where that sentence now
            // lives for all three hosts. ADR 0596.
            Answer::Field {
                name,
                value: Some(_),
            } if !self.takes_characters_at(at) => {
                println!(
                    "note: {} states Table 233 bit 19 clear, so it shall include only a drop-down \
                     list — pick one of its options rather than typing a value",
                    name.shown()
                );
                None
            }
            Answer::Field {
                name,
                value: Some(value),
            } => {
                println!("note: typing into the field {}", name.shown());
                // **The caret goes where the click went**, which is `Query::Offset` — the inverse
                // of `Query::Caret` (ADR 0225). The point names the field and is also the point
                // measured, because a press is one place; a drag then asks the same question with
                // the pointer's place instead. The end of the value is what a field whose layout
                // could not answer falls back to.
                let caret = match self.viewer.query(Query::Offset { at, point: at }) {
                    Answer::Offset(offset) => offset,
                    _ => value.text.len(),
                };
                Some(Typing::at_offset(Target::Field, at, caret))
            }
            _ => None,
        };
        if was && self.typing.is_none() {
            println!("note: the keyboard is back on the page");
        }
    }

    /// The control a point is on: §12.7.4.2's name, the widget pressed, and what kind it is.
    ///
    /// **Two questions and not one**, because they answer about different things: `Query::FieldAt`
    /// says which *field* a point is in and whether its value is text, and `Query::Fields` says
    /// what §12.7.5's flags make the *control*. Table 233 bit 19 is a fact about the control, so
    /// nothing in the first answer could carry it. `App::toggle_button` has asked this pair since
    /// ADR 0235 for §12.7.5.2's own flag; this is that walk named once instead of twice.
    ///
    /// The *last* widget covering the point, because §12.5.2 draws them in `/Annots` order and the
    /// one on top is the one under the pointer.
    pub(super) fn control_at(&self, at: (f32, f32)) -> Option<Pressed> {
        let Answer::Field { name, .. } = self.viewer.query(Query::FieldAt(at)) else {
            return None;
        };
        let qualified = name.qualified;
        let Answer::Fields(fields) = self.viewer.query(Query::Fields) else {
            return None;
        };
        let field = fields
            .iter()
            .find(|field| field.name.qualified == qualified)?;
        let widget = field
            .widgets
            .iter()
            .rev()
            .find(|widget| viewer_host::covers(widget.quad, at))?;
        Some(Pressed {
            field: qualified,
            annotation: widget.annotation,
            kind: control_kind(&field.control),
            read_only: field.read_only,
        })
    }

    /// Enter in a single-line field: the commit, and the keyboard back on the page.
    pub(super) fn commit_on_enter(&mut self, aim: Aim) -> bool {
        let Aim::Field(field) = aim else {
            return false;
        };
        self.typing = None;
        self.dispatch(Command::CommitField { field });
        println!("note: the keyboard is back on the page");
        self.redraw();
        true
    }

    /// Whether the control at a point is a text field Table 231 bit 13 keeps to one line, or a
    /// combo box's edit — the two controls whose Enter is the commit rather than a character.
    pub(super) fn single_line(&self, at: (f32, f32)) -> bool {
        self.control_at(at).is_some_and(|pressed| {
            matches!(
                pressed.kind,
                ControlKind::Entry {
                    multiline: false,
                    ..
                } | ControlKind::Combo { .. }
            )
        })
    }

    /// Whether the control at a point takes characters from a keyboard at all.
    ///
    /// A point on no control at all answers **true**, which is deliberate: this is a *refusal*
    /// and it may only refuse something it has identified. §12.5.6.6's free text has already been
    /// answered before this is asked, and a field the form walk cannot find is one the existing
    /// value question has already decided about.
    fn takes_characters_at(&self, at: (f32, f32)) -> bool {
        self.control_at(at)
            .is_none_or(|pressed| pressed.kind.takes_typed_characters())
    }

    /// Aims the keyboard at §12.5.6.6's annotation under the point, whoever wrote it.
    ///
    /// The same two questions a field takes, in the same order and for the same reasons:
    /// `Query::FreeTextAt` names the annotation and hands back Table 166's `/Contents` as it reads
    /// now, and `Query::Offset` says where inside that text the press landed. Neither is a second
    /// implementation of anything — the core answers both from §12.7.4.3's own layout, which is
    /// the subclause §12.5.6.6 sends this subtype to.
    ///
    /// **This host needed not one line to reach the file's own annotations** (ADR 0304): the core
    /// answered `Query::FreeTextAt` for an annotation a person had added and now answers it for
    /// the producer's too, and `Edit::SetFreeText` already named its target by object. A capability
    /// that arrives through an existing message is what `doc/ui-boundary.md` is arguing for.
    pub(super) fn aim_at_free_text(&mut self, at: (f32, f32)) -> Option<Typing> {
        let Answer::FreeText { annotation, text } = self.viewer.query(Query::FreeTextAt { at })
        else {
            return None;
        };
        println!(
            "note: typing into the free text annotation {} {}",
            annotation.number, annotation.generation
        );
        let caret = match self.viewer.query(Query::Offset { at, point: at }) {
            Answer::Offset(offset) => offset,
            _ => text.len(),
        };
        Some(Typing::at_offset(Target::FreeText(annotation), at, caret))
    }

    /// §12.7.5.2's two toggling kinds, clicked at a point of the page's own viewport.
    ///
    /// The point is the caller's, for the reason [`App::aim_at_field`] gives.
    ///
    /// **What `Query::Fields` made possible and nothing else could.** `Edit::SetField` takes a
    /// string, and for a check box or a radio button the only strings that mean anything are the
    /// names Table 170's appearance dictionary is keyed by — §12.7.5.2.3 makes `/V` "a name object
    /// representing the check box's appearance state, which shall be used to select the
    /// appropriate appearance from the appearance dictionary" — and those names are the file's own
    /// invention. This host had no way to learn one, so it could type into a form and not tick a
    /// box in it. `FormWidget::on_state` is the name to send. ADR 0235.
    ///
    /// Which widget: `Query::FieldAt` is the hit test, because it is the model's own and because a
    /// second one here could disagree with it; the quadrilaterals then say *which of the field's
    /// widgets* the point was in, which is the question §12.7.5.2.4's set makes real — "at most
    /// one button in a set may be on at any given time", and the one that goes on is the one under
    /// the pointer.
    ///
    /// What to send: §12.7.5.2.4 states the rule for turning one off, and it is a flag rather than
    /// a convention — "\[i]f set, exactly one radio button shall be selected at all times;
    /// selecting the currently selected button has no effect. If clear, clicking the selected
    /// button deselects it, leaving no button selected."
    ///
    /// Table 227 bit 1 is checked here as well as in the core, and neither is redundant: the core
    /// refuses the edit, and a host that sent it anyway would be a program that looks broken
    /// rather than one that obeys the document.
    ///
    /// **The rule itself is [`viewer_host::form::clicked`]**, one copy for three windows, so that
    /// each asks Table 227 before deciding §12.7.5.2 rather than reading its own toolkit's button
    /// (ADR 0630). The match is exhaustive in all three windows, which is what makes a case added
    /// to [`viewer_host::Clicked`] a compile error in three places.
    ///
    /// [`viewer_host::form::clicked`]: viewer_host::clicked
    /// [`viewer_host::Clicked`]: viewer_host::Clicked
    pub(crate) fn toggle_button(&mut self, at: (f32, f32)) {
        let clicked = viewer_host::clicked(&self.viewer, at);
        // `false`: this host draws the page's own widget appearances, so a click on a text or
        // choice field is one it aims a caret or a list at rather than one it cannot reach.
        if let Some(said) = clicked.note() {
            println!("note: {said}");
        }
        match clicked {
            Clicked::Toggles { name, value } => {
                println!("note: setting the field {} to {value}", name.shown());
                self.dispatch(Command::Edit(Edit::SetField {
                    field: name.qualified,
                    value: Entered::Text(value),
                }));
            }
            // Each of these has already been said by `note` above, or is not this method's:
            // `aim_at_field` and `open_choices` ran before it for `Aimed`, and `Command::Pointer`
            // carries §12.6.3's triggers for `Pointed` and for the page.
            Clicked::ReadOnly { .. }
            | Clicked::Stays { .. }
            | Clicked::Unnamed { .. }
            | Clicked::Pointed { .. }
            | Clicked::Aimed { .. }
            | Clicked::Page => {}
        }
    }

    /// Aims the keyboard at whatever §12.5.1's tab walk just landed on, where it takes text.
    ///
    /// **The decision `doc/todo/33` left open, and it needed no new message.** The worry it
    /// recorded was that a focus ring on a *button* means something else — a press activates it,
    /// it does not take characters — and the answer is the one this host already uses for a
    /// click: `Answer::Field`'s value is `Some` only for a field §12.7.4.3 lays text out for, so
    /// the same question decides both. What was missing was only the *point*, and `Query::Focus`
    /// answers with the annotation's quadrilateral in the same device pixels `Query::FieldAt`
    /// takes, so the centre of the ring is the point to ask about.
    ///
    /// A walk onto anything else takes the keyboard back to the page, which is what makes Tab out
    /// of a field stop typing without a second binding for it.
    pub(crate) fn aim_at_focus(&mut self) {
        let Answer::Focus { quad, .. } = self.viewer.query(Query::Focus) else {
            self.typing = None;
            return;
        };
        // The centre of the ring, which is inside the widget's `/Rect` by construction: §12.5.5
        // places the appearance *on* that rectangle and `Query::Focus` answers with it.
        let at = ((quad[0] + quad[4]) * 0.5, (quad[1] + quad[5]) * 0.5);
        self.typing = match self.viewer.query(Query::FieldAt(at)) {
            // The same refusal a click makes, for the same reason — see `aim_at_field`.
            Answer::Field {
                value: Some(shown), ..
            } if shown.obscured => None,
            // Table 233 bit 19, refused exactly as the click above refuses it: §12.5.1's walk is a
            // second way into a field and a rule that held for one of them would be a rule with a
            // hole in it.
            Answer::Field {
                name,
                value: Some(_),
            } if !self.takes_characters_at(at) => {
                println!(
                    "note: {} states Table 233 bit 19 clear, so it shall include only a drop-down \
                     list — pick one of its options rather than typing a value",
                    name.shown()
                );
                None
            }
            Answer::Field {
                name,
                value: Some(value),
            } => {
                println!("note: typing into the field {}", name.shown());
                // The end of the value, and here that is not a fallback: a tab press names no
                // point inside the value, so there is nothing for `Query::Offset` to measure and
                // the end is the place ADR 0211 chose for a walk that arrives without one.
                Some(Typing::at_offset(Target::Field, at, value.text.len()))
            }
            _ => None,
        };
        self.redraw();
    }

    /// What the thing being typed into says now, and how an edit to it is addressed.
    ///
    /// One question per keystroke, asked twice — once before the key is applied and once after,
    /// because what the *document* took is what the caret has to be clamped to (ADR 0197). Which
    /// question it is depends on the target and nothing else does: a field is addressed by
    /// §12.7.4.2's qualified name and §12.5.6.6's annotation by its object, because an annotation
    /// has no name for anything to address it by.
    ///
    /// `None` where the thing is no longer under the point — a page turned under the pointer —
    /// which is what takes the keyboard back to the page.
    pub(super) fn aimed(&self, typing: Typing) -> Option<(String, Aim)> {
        match typing.target {
            Target::Field => match self.viewer.query(Query::FieldAt(typing.at)) {
                // A value that is Table 231 bit 14's echo is not a value to append to, which is
                // why `aim_at_field` never puts the keyboard here; this is the same refusal one
                // step later, so that the two cannot come apart. ADR 0247.
                Answer::Field {
                    value: Some(shown), ..
                } if shown.obscured => None,
                Answer::Field { name, value } => Some((
                    value.map_or_else(String::new, |shown| shown.text),
                    Aim::Field(name.qualified),
                )),
                _ => None,
            },
            Target::FreeText(annotation) => {
                match self.viewer.query(Query::FreeTextAt { at: typing.at }) {
                    Answer::FreeText { text, .. } => Some((text, Aim::FreeText(annotation))),
                    _ => None,
                }
            }
            // A note's window is typed into by `typed_into_note`, which reads the window's text.
            Target::Note(_) => None,
        }
    }
}
