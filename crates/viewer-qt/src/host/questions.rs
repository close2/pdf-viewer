//! The dialogues a person answers: `CLAUDE.md`'s *ask* level, §7.6.4.1's password and a script's
//! question, each held here and put in a window the C++ side opens (ADRs 1155, 1616; filed here by
//! ADR 1908).

use std::collections::VecDeque;
use std::path::PathBuf;

use viewer_core::{Command, DocumentId, Purpose};
use viewer_host::native_window::NativeWindow;

use crate::bridge::ffi::QtScriptEntry;

use super::Host;

/// What the one outstanding question is about, at `CLAUDE.md`'s *ask* level.
///
/// **Two subjects and one dialogue**, because a person meets one prompt and this host builds one:
/// an operation `viewer-core` is holding until an answer comes back, or §12.6.4.8's URI, which no
/// core is holding at all because starting a program on this machine is the host's own act.
/// Closed, so that a third subject cannot be answered by the wrong branch (ADR 1155).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Pending {
    /// An edit the viewer is holding, answered by `viewer_core::Command::Answer`.
    Restricted {
        /// Which document it is holding it for.
        document: DocumentId,
        /// Which operation the document restricts.
        operation: pdf_model::restriction::Operation,
    },
    /// §12.6.4.8's resolved URI, answered by `viewer_host::answered`.
    Link {
        /// The URI as `viewer_host::resolve_uri` left it.
        uri: String,
    },
    /// §12.7.6.2's composed request, answered by sending it or by saying it was not (ADR 1291).
    Submit {
        /// The document whose form it is, which is where an FDF answer goes.
        document: DocumentId,
        /// The request exactly as `pdf_model::submission::compose` made it.
        submission: Box<pdf_model::submission::Submission>,
    },
    /// An import's form data named by an absolute URI, answered by fetching it (ADR 1527).
    Fetch {
        /// The document that asked, which is where the answer is imported.
        document: DocumentId,
        /// The URI as the fragment or the action named it.
        url: String,
    },
    /// A signature policy's published copy, answered by fetching it (ADR 1738).
    Policy {
        /// The document whose signature named it, which a bound copy opens beside.
        document: DocumentId,
        /// The URL, the identifier and the digest the signer committed to.
        policy: Box<viewer_host::PublishedPolicy>,
    },
    /// A file a document named, answered by reading it and supplying it (ADRs 1227, 1239).
    RemoteDocument {
        /// Which of the three purposes asked, so that the answer goes back to the right one.
        purpose: Purpose,
        /// The file as the *document* named it, for the sentence a decline prints.
        name: String,
        /// Where `viewer_host::resolve_import` put it, which is what would be read.
        path: PathBuf,
    },
    /// §O.2.1's embedded document, answered by opening it in a tab of its own (ADR 1331).
    Embedded {
        /// The document holding it.
        document: DocumentId,
        /// The name the document filed it under.
        name: String,
        /// Its bytes, out of the holding document.
        bytes: Vec<u8>,
        /// What followed `ef` in the fragment, which applies to it.
        fragment: Option<String>,
    },
    /// RFC 0008 section 6.3's one question per document, answered by
    /// `viewer_core::Command::AnswerScripts` (ADR 1616).
    Scripts {
        /// The document whose scripts are waiting.
        document: DocumentId,
    },
}

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
    pub(super) fn put_the_question(&mut self, about: Pending, words: &viewer_host::Question) {
        self.question = Some((about, format!("{}\n\n{}", words.reasons, words.choice)));
        self.update.question = true;
    }

    /// `CLAUDE.md`'s *ask* level over an operation the document restricts, in the one dialogue,
    /// held as [`Pending::Restricted`]: the words are `viewer_host::restriction`'s and the dialogue
    /// is C++'s, because Rust does not call a Qt object (ADR 1145).
    pub(super) fn ask_whether_to_proceed(
        &mut self,
        document: DocumentId,
        operation: pdf_model::restriction::Operation,
        notes: &[String],
    ) {
        self.put_the_question(
            Pending::Restricted {
                document,
                operation,
            },
            &viewer_host::asked(operation, notes),
        );
    }

    /// RFC 0008 section 6.3's one question per document before its scripts run, in the one
    /// dialogue, held as [`Pending::Scripts`] (ADR 1616).
    pub(super) fn ask_whether_to_run_scripts(
        &mut self,
        document: DocumentId,
        script: &str,
        first_line: &str,
    ) {
        self.put_the_question(
            Pending::Scripts { document },
            &viewer_host::asked_to_run_scripts(script, first_line),
        );
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
        let subject = match self.question.as_ref().map(|(about, _)| about) {
            Some(Pending::Link { .. }) => viewer_host::Subject::Link,
            Some(Pending::Submit { .. }) => viewer_host::Subject::Submission,
            Some(Pending::Fetch { .. }) => viewer_host::Subject::Fetch,
            Some(Pending::Policy { .. }) => viewer_host::Subject::Policy,
            Some(Pending::RemoteDocument { .. }) => viewer_host::Subject::Document,
            Some(Pending::Embedded { .. }) => viewer_host::Subject::Embedded,
            Some(Pending::Scripts { .. }) => viewer_host::Subject::Scripts,
            Some(Pending::Restricted { .. }) | None => viewer_host::Subject::Restricted,
        };
        subject.title().to_owned()
    }

    /// `CLAUDE.md`'s *ask* level: what the question says, worded by [`viewer_host`].
    pub(crate) fn question_prompt(&self) -> String {
        self.question
            .as_ref()
            .map(|(_, words)| words.clone())
            .unwrap_or_default()
    }

    /// What the person answered, on its way to the viewer that is holding the operation.
    ///
    /// The decline is said out loud because `viewer-core` says nothing at all on a `no` —
    /// deliberately (ADR 0814) — and a person is still owed the fact that what they asked for did
    /// not happen. Taking the question rather than reading it is what makes a dialogue closed
    /// twice answer once.
    pub(crate) fn answer_question(&mut self, proceed: bool) {
        let Some((about, _)) = self.question.take() else {
            return;
        };
        match about {
            // §O.2.1: the act is this host's own, so the answer decides whether the file opens
            // at all (ADR 1331).
            Pending::Embedded {
                document,
                name,
                bytes,
                fragment,
            } => {
                if proceed {
                    self.open_embedded(document, &name, bytes, fragment);
                } else {
                    self.say(&viewer_host::embedded_declined(&name));
                }
            }
            Pending::Restricted {
                document,
                operation,
            } => {
                if !proceed {
                    self.say(&viewer_host::declined(operation));
                }
                self.dispatch(Command::Answer { document, proceed });
                // §12.11.6's `no` puts the document down unopened, so one on its way to a tab
                // never arrives and the next may start.
                if !proceed && operation == pdf_model::restriction::Operation::Process {
                    self.given_up(document);
                }
            }
            // §12.6.4.8: the act is this host's own rather than an edit the core is holding, so
            // what the answer decides is whether the URI reaches `xdg-open` (ADR 1155).
            Pending::Link { uri } => self.say(&viewer_host::answered(&uri, proceed)),
            // RFC 0008 section 6.3: the viewer holds the document's scripts, so the answer goes to
            // it whichever it is (ADR 1616).
            Pending::Scripts { document } => {
                self.say(&viewer_host::scripts_answered(proceed));
                self.dispatch(Command::AnswerScripts { document, proceed });
            }
            // §12.7.6.2: the act is this host's own, so the answer decides whether the request
            // leaves this machine at all (ADR 1291).
            Pending::Submit {
                document,
                submission,
            } => {
                if proceed {
                    self.send_form(document, *submission, None);
                } else {
                    self.say(&viewer_host::policy::submission_note(
                        &submission,
                        Some(&format!(
                            "you answered \"{}\"",
                            viewer_host::restriction::DO_NOT
                        )),
                    ));
                }
            }
            // Table Annex O.4's `fdf` from a server: a `no` fetches nothing, and the core is told
            // so the import says it declined (ADR 1527).
            Pending::Fetch { document, url } => {
                if proceed {
                    self.start_fetch(document, url, None);
                } else {
                    self.say(&viewer_host::fetch_note(
                        &url,
                        Some(&format!(
                            "you answered \"{}\"",
                            viewer_host::restriction::DO_NOT
                        )),
                    ));
                    self.dispatch(Command::Supply {
                        purpose: Purpose::ImportData,
                        bytes: None,
                    });
                }
            }
            // A signature policy's copy from a server: a `no` fetches nothing and says so (ADR
            // 1738).
            Pending::Policy { document, policy } => {
                if proceed {
                    self.start_policy_fetch(document, *policy, None);
                } else {
                    self.say(&viewer_host::policy::signature_policy_declined(
                        &policy,
                        &format!("you answered \"{}\"", viewer_host::restriction::DO_NOT),
                    ));
                }
            }
            // §12.6.4.3: the act is opening a document in place of this one, so a `no` supplies
            // nothing and the core says the link declined (ADR 1227).
            Pending::RemoteDocument {
                purpose,
                name,
                path,
            } => {
                let mut queue = VecDeque::new();
                let bytes = if proceed {
                    self.read_remote(purpose, &name, &path)
                } else {
                    self.say(&viewer_host::remote_declined(purpose, &name));
                    None
                };
                if bytes.is_some() {
                    self.offer_a_name(
                        &path,
                        bytes.as_deref().unwrap_or_default(),
                        purpose,
                        &mut queue,
                    );
                }
                queue.push_back(Command::Supply { purpose, bytes });
                self.pump(queue.into());
            }
        }
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
