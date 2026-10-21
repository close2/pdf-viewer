//! Script answers, keyboard decisions and the document's clock.

use super::Session;
use crate::events::Events;
use crate::keyboard::ActKind;
use viewer_core::{Answer, Command, DocumentId, Query};
use viewer_host::Decided;

impl Session {
    /// How many milliseconds of [`Self::tick`] may pass before a script's timer is owed, or `None`
    /// where no document holds one — every session's answer at the level off (ADRs 1702, 2061).
    /// Asked of every open document, so it is never refused for want of a focused one.
    #[must_use]
    pub fn timer_due(&self) -> Option<u32> {
        match self.viewer.query(Query::TimerDue) {
            Answer::TimerDue(due) => due,
            _ => None,
        }
    }

    /// §12.4.4.1's clock, driven by the caller, because rule 3 leaves this crate none.
    #[must_use]
    pub fn tick(&mut self, millis: u32) -> Events {
        self.handle(Command::Tick { millis })
    }

    /// §7.9.4's instant from the caller's clock, which Table 166's `/M` is written as; `None`
    /// withdraws it.
    #[must_use]
    pub fn clock(&mut self, now: Option<pdf_syntax::Date>) -> Events {
        self.handle(Command::Clock(now))
    }

    /// §12.7.8's form data a server answered a submission or a fetched import with.
    #[must_use]
    pub fn respond(
        &mut self,
        document: u64,
        answers: viewer_core::Answered,
        source: String,
        format: pdf_model::action::DataFormat,
        bytes: Vec<u8>,
    ) -> Events {
        self.handle(Command::Respond {
            document: DocumentId(document),
            answers,
            source,
            format,
            bytes,
        })
    }

    /// What the keyboard holds, for a script's `event.shift` and `event.modifier`.
    #[must_use]
    pub fn keys(&mut self, keys: pdf_model::view::Keys) -> Events {
        self.handle(Command::Keys(keys))
    }

    /// RFC 0008 section 6.3's level for every document's scripts.
    ///
    /// **The level becomes a runner where every window's does**, in
    /// [`viewer_host::policy::scripting`]: at `ask`, `warn` and `on` each document is handed a
    /// `pdf-script-worker` that starts at its first trigger and never before, found beside the
    /// running executable or where `PDF_SCRIPT_WORKER` names it. This library is a C caller's host
    /// and supplies the runner a window supplies (ADR 2061).
    #[must_use]
    pub fn scripts(&mut self, level: viewer_host::Scripts) -> Events {
        self.handle(Command::Scripts(viewer_host::policy::scripting(level)))
    }

    /// The person's answer to [`viewer_core::Event::AskingToRunScripts`].
    #[must_use]
    pub fn answer_scripts(&mut self, document: u64, proceed: bool) -> Events {
        self.handle(Command::AnswerScripts {
            document: DocumentId(document),
            proceed,
        })
    }

    /// The person's answer to [`viewer_core::Event::ScriptAsking`].
    #[must_use]
    pub fn answer_script(&mut self, document: u64, answer: viewer_core::ScriptAnswer) -> Events {
        self.handle(Command::AnswerScript {
            document: DocumentId(document),
            answer,
        })
    }

    /// A key press that reached the page, acted on as every window acts on one.
    ///
    /// What needs nothing of the window is done here — the command the table means is sent, and
    /// its events are the answer — and what does is handed back as the act and the distance a
    /// scroll carries. A press the person is owed a sentence about answers [`ActKind::Say`] and
    /// leaves the sentence for [`Self::said`]; every other press clears it.
    #[must_use]
    pub fn press(
        &mut self,
        key: viewer_host::Key,
        held: viewer_host::Modifiers,
        mode: viewer_host::Mode,
        waiting: viewer_host::Waiting,
    ) -> (Events, ActKind, f32) {
        self.said = None;
        let page = viewer_host::Keyboard::Page;
        match viewer_host::decide(&self.viewer, key, held, mode, waiting, page) {
            Decided::Activate { annotation, .. } => (
                self.handle(Command::Activate(annotation)),
                ActKind::Nothing,
                0.0,
            ),
            Decided::Send(command) => (self.handle(command), ActKind::Nothing, 0.0),
            Decided::Window(act) => {
                let (kind, by) = ActKind::of(act);
                (Events::new(Vec::new()), kind, by)
            }
            Decided::Say(sentence) => {
                self.said = Some(sentence);
                (Events::new(Vec::new()), ActKind::Say, 0.0)
            }
            Decided::Nothing => (Events::new(Vec::new()), ActKind::Nothing, 0.0),
        }
    }

    /// The sentence the last [`Self::press`] owes the person, if it owed one.
    #[must_use]
    pub fn said(&self) -> Option<&str> {
        self.said.as_deref()
    }
}
