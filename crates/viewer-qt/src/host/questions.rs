//! The dialogues a person answers: `CLAUDE.md`'s *ask* level, §7.6.4.1's password and a script's
//! question, each held here and put in a window the C++ side opens (ADRs 1155, 1616; filed here by
//! ADR 1908).

use viewer_core::{Command, DocumentId};
use viewer_host::native_window::{Asked, NativeWindow};

use crate::bridge::ffi::QtScriptEntry;

use super::Host;

impl Host {
    /// §7.6.4.1: a person typed a password, or closed the prompt with nothing in it.
    ///
    /// Exhaustive over `Supplied` on purpose, which is what holds three hosts level.
    pub(crate) fn supply_password(&mut self, password: &str) {
        let document = self.prompting;
        self.password_supplied(document, password.to_owned().into());
    }

    /// §7.6.4.1: what the prompt says, worded by [`viewer_host::password`] for all three hosts.
    pub(crate) fn password_prompt(&self) -> String {
        self.prompt.clone()
    }

    /// `CLAUDE.md`'s *ask* level: holds the question and asks C++ for a window to put it in.
    ///
    /// The two paragraphs are joined here because this bridge carries one string per dialogue and
    /// the C++ side wraps it in one `QLabel`; `viewer-gtk` draws them as two labels, which is the
    /// difference `viewer_host::Question` keeps them apart for.
    pub(super) fn put_the_question(&mut self, about: Asked, words: &viewer_host::Question) {
        self.question = Some((about, format!("{}\n\n{}", words.reasons, words.choice)));
        self.update.question = true;
    }

    /// RFC 0008 section 4.2's `app.alert` and `app.response`: a dialogue of its own, since its
    /// buttons are the script's and an entry may stand under its text (ADR 1628).
    pub(super) fn ask_for_a_script(
        &mut self,
        document: DocumentId,
        question: viewer_core::ScriptQuestion,
    ) {
        let name = self.state.documents.label_of(document);
        self.say(&viewer_host::script_asks::put(&name, &question));
        self.script_question = Some((document, name, question));
        self.update.script_question = true;
    }

    /// What the question's window is called, by what the question is about.
    pub(crate) fn question_title(&self) -> String {
        let subject = self
            .question
            .as_ref()
            .map_or(viewer_host::Subject::Restricted, |(about, _)| {
                about.subject()
            });
        subject.title().to_owned()
    }

    /// `CLAUDE.md`'s *ask* level: what the question says, worded by [`viewer_host`].
    pub(crate) fn question_prompt(&self) -> String {
        self.question
            .as_ref()
            .map(|(_, words)| words.clone())
            .unwrap_or_default()
    }

    /// What the person answered, carried out as every window carries it out (ADR 2105). Taking
    /// the question rather than reading it is what makes a dialogue closed twice answer once.
    pub(crate) fn answer_question(&mut self, proceed: bool) {
        let Some((about, _)) = self.question.take() else {
            return;
        };
        self.answer_the_question(about, proceed);
    }

    /// The script's question's dialogue title, naming the document that asks (ADR 1628).
    pub(crate) fn script_question_title(&self) -> String {
        self.script_question
            .as_ref()
            .map(|(_, name, question)| viewer_host::script_asks::title(name, question))
            .unwrap_or_default()
    }

    /// The script's message or question.
    pub(crate) fn script_question_text(&self) -> String {
        self.script_question
            .as_ref()
            .map(|(_, _, question)| viewer_host::script_asks::text(question).to_owned())
            .unwrap_or_default()
    }

    /// The line under the text: an alert's icon as a word, or a response's label.
    pub(crate) fn script_question_detail(&self) -> String {
        match self
            .script_question
            .as_ref()
            .map(|(_, _, question)| question)
        {
            Some(viewer_core::ScriptQuestion::Alert { icon, .. }) => icon.word().to_owned(),
            Some(viewer_core::ScriptQuestion::Response { label, .. }) => {
                label.clone().unwrap_or_default()
            }
            None => String::new(),
        }
    }

    /// The dialogue's buttons, the affirming one last: an alert's
    /// [`viewer_core::AlertButtons::buttons`], or Cancel and OK under a response's entry.
    pub(crate) fn script_question_buttons(&self) -> Vec<String> {
        script_buttons(
            self.script_question
                .as_ref()
                .map(|(_, _, question)| question),
        )
        .iter()
        .map(|button| button.label().to_owned())
        .collect()
    }

    /// Whether the dialogue carries an entry, and what it starts with and hides: an
    /// `app.response`'s default, and whether it is a password.
    pub(crate) fn script_question_entry(&self) -> QtScriptEntry {
        match self
            .script_question
            .as_ref()
            .map(|(_, _, question)| question)
        {
            Some(viewer_core::ScriptQuestion::Response {
                default, password, ..
            }) => QtScriptEntry {
                shown: true,
                start: default.clone(),
                password: *password,
            },
            _ => QtScriptEntry {
                shown: false,
                start: String::new(),
                password: false,
            },
        }
    }

    /// [`Event::Beep`]: `QApplication::beep` flagged for C++, and the line that says which of the
    /// reference's five was asked for (ADR 1702).
    pub(super) fn beep(&mut self, document: DocumentId, sound: pdf_model::view::Sound) {
        self.update.beep = true;
        let name = self.state.documents.label_of(document);
        self.say(&viewer_host::script_timers::played(&name, sound, "Qt"));
    }

    /// [`Event::ScriptQuestionWithdrawn`]: that document's question is taken, so the dialogue's
    /// next [`Self::poll_script_question`] sees it gone (ADR 1643).
    pub(super) fn withdraw_script_question(&mut self, document: DocumentId) {
        if self
            .script_question
            .as_ref()
            .is_some_and(|(asked, ..)| *asked == document)
        {
            self.script_question = None;
        }
        let name = self.state.documents.label_of(document);
        self.say(&viewer_host::script_asks::withdrawn(&name));
    }

    /// How long the dialogue waits before asking [`Self::poll_script_question`], in milliseconds:
    /// `viewer_host::script_asks::wake_after` (ADR 1643).
    #[expect(
        clippy::unused_self,
        reason = "a `cxx` bridge method is called on the host, and the wait is the reader's"
    )]
    pub(crate) fn script_question_wait(&self) -> u32 {
        u32::try_from(viewer_host::script_asks::wake_after().as_millis()).unwrap_or(u32::MAX)
    }

    /// Sends a tick of no time, after which the viewer asks whether the script's question was
    /// withdrawn, and answers whether it is still outstanding — `false` is a dialogue to close
    /// without answering (ADR 1643).
    pub(crate) fn poll_script_question(&mut self) -> bool {
        self.pump(vec![Command::Tick { millis: 0 }]);
        self.script_question.is_some()
    }

    /// What the person answered: the index of the button pressed into
    /// [`Self::script_question_buttons`], or a negative number for a dialogue closed without one,
    /// with the entry's text beside it.
    pub(crate) fn answer_script_question(&mut self, button: i32, typed: &str) {
        let Some((document, _, question)) = self.script_question.take() else {
            return;
        };
        let pressed = usize::try_from(button)
            .ok()
            .and_then(|index| script_buttons(Some(&question)).get(index).copied());
        let answer = match (&question, pressed) {
            (_, None) => viewer_core::ScriptAnswer::dismissed(&question),
            (viewer_core::ScriptQuestion::Alert { .. }, Some(button)) => {
                viewer_core::ScriptAnswer::Pressed(button)
            }
            (viewer_core::ScriptQuestion::Response { .. }, Some(viewer_core::AlertButton::Ok)) => {
                viewer_core::ScriptAnswer::Typed(Some(typed.to_owned()))
            }
            (viewer_core::ScriptQuestion::Response { .. }, Some(_)) => {
                viewer_core::ScriptAnswer::Typed(None)
            }
        };
        let password = matches!(
            question,
            viewer_core::ScriptQuestion::Response { password: true, .. }
        );
        self.say(&viewer_host::script_asks::answered(&answer, password));
        self.dispatch(Command::AnswerScript { document, answer });
    }

    /// What the button that lets the operation go ahead says.
    #[expect(
        clippy::unused_self,
        reason = "the bridge declares it as a method on `Host` and cxx calls it as one; a free \
                  function could not cross"
    )]
    pub(crate) fn go_ahead(&self) -> String {
        viewer_host::restriction::GO_AHEAD.to_owned()
    }

    /// What the button that leaves it undone says.
    #[expect(clippy::unused_self, reason = "`go_ahead`'s reason, one method up")]
    pub(crate) fn do_not(&self) -> String {
        viewer_host::restriction::DO_NOT.to_owned()
    }
}

/// A script question's buttons, the affirming one last: an alert's own, or Cancel and OK for a
/// response.
fn script_buttons(
    question: Option<&viewer_core::ScriptQuestion>,
) -> &'static [viewer_core::AlertButton] {
    match question {
        Some(viewer_core::ScriptQuestion::Alert { buttons, .. }) => buttons.buttons(),
        Some(viewer_core::ScriptQuestion::Response { .. }) => &[
            viewer_core::AlertButton::Cancel,
            viewer_core::AlertButton::Ok,
        ],
        None => &[],
    }
}
